use crate::cache::{CacheData, CachedSkill};
use crate::config::Config;
use crate::git;
use crate::lock::{LockEntry, LockFile};
use crate::skill_resolver;
use crate::utils::resolve_source;
use anyhow::{Result, bail};
use colored::Colorize;
use ratatui::style::Color;
use ratatui::text::{Line, Span};
use skim::prelude::*;
use std::fs;
use std::io::IsTerminal;
use std::path::PathBuf;

// ---------------------------------------------------------------------------
// Skim item types
// ---------------------------------------------------------------------------

/// A skill item for the fuzzy-find TUI.
#[derive(Clone)]
struct FindItem {
    display: String,
    skill: CachedSkill,
    source: String,
    is_registry: bool,
    source_url: Option<String>,
}

impl SkimItem for FindItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.display)
    }

    fn output(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.skill.name)
    }

    fn display(&self, context: DisplayContext) -> Line<'_> {
        let base = context.base_style;
        let is_selected = base.bg.is_some();
        // Selected item has a background color (bg:236 from --color theme)
        let name_style = if is_selected {
            base.fg(Color::Blue)
        } else {
            base
        };
        let registry_style = if is_selected {
            base.fg(Color::Green)
        } else {
            base.fg(Color::DarkGray)
        };
        let source_style = base.fg(Color::DarkGray);

        let mut spans = vec![Span::styled(self.skill.name.clone(), name_style)];

        if self.is_registry {
            spans.push(Span::styled(" [registry]".to_string(), registry_style));
        }

        let display_source = if self.source.is_empty() {
            self.source_url.as_deref().unwrap_or("-")
        } else {
            &self.source
        };
        spans.push(Span::styled(format!(" [{}]", display_source), source_style));

        Line::from(spans)
    }
}

/// A generic selectable item for scope / platform TUI steps.
struct SelectItem {
    display: String,
    value: String,
    disabled: bool,
}

impl SkimItem for SelectItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.display)
    }

    fn output(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.value)
    }

    fn disabled(&self) -> bool {
        self.disabled
    }

    fn display(&self, context: DisplayContext) -> Line<'_> {
        let base = context.base_style;
        let style = if base.bg.is_some() {
            base.fg(Color::Blue)
        } else {
            base
        };
        Line::from(vec![Span::styled(self.display.clone(), style)])
    }
}

// ---------------------------------------------------------------------------
// Public entry point
// ---------------------------------------------------------------------------

/// Find and install a skill interactively via multi-step TUI.
pub fn run(skill: Option<&str>, source: Option<&str>, global: bool) -> Result<()> {
    // Detect non-interactive terminal
    if !std::io::stdin().is_terminal() {
        bail!("'find' requires an interactive terminal. Use 'query' for non-interactive listing.");
    }

    let config = Config::load()?;

    // ---- Step 1: Load skills & fuzzy-find ----
    let cache_data = load_skills(&config, source)?;
    let items = collect_items(&cache_data, source);

    if items.is_empty() {
        if let Some(filter_source) = source {
            bail!("Source '{}' not found in cache.", filter_source);
        } else {
            bail!("No skills found in cache.");
        }
    }

    let selected_skill = run_skill_tui(items, skill)?;

    // ---- Step 2: Platform selection (multi-select) ----
    let platforms = run_platform_tui(&config)?;

    // ---- Step 3: Install ----
    let source_name = if selected_skill.source.is_empty() {
        selected_skill.source_url.as_deref().unwrap_or("")
    } else {
        &selected_skill.source
    };
    let skill_name = &selected_skill.skill.name;

    {
        // Resolve source and clone once
        let resolved = resolve_source(&config, source_name)?;
        let skill_path = skill_name.to_string();
        let (_tmp_dir, source_dir) =
            git::install_skill_sparse(&resolved.url, &skill_path, skill_name)?;

        let mut failed: Vec<String> = Vec::new();

        // 1. Always install to canonical .agents directory
        let canonical_dir = if global {
            dirs::home_dir()
                .unwrap_or_default()
                .join(".agents")
                .join("skills")
        } else {
            std::env::current_dir()
                .unwrap_or_default()
                .join(".agents")
                .join("skills")
        }
        .join(skill_name);

        if let Err(e) = fs::create_dir_all(&canonical_dir) {
            bail!("Failed to create canonical directory: {}", e);
        }
        if canonical_dir.exists() {
            let _ = fs::remove_dir_all(&canonical_dir);
        }
        if let Err(e) = git::copy_dir_recursive(&source_dir, &canonical_dir) {
            bail!("Failed to install skill: {}", e);
        }
        println!(
            "{}: {}",
            "Installed".green(),
            crate::utils::display_path(&canonical_dir)
        );

        // 2. Create symlinks for selected platforms
        let mut linked: Vec<String> = Vec::new();
        for platform in &platforms {
            let dest_dir = match resolve_platform_dest(&config, platform, skill_name, global) {
                Some(dir) => dir,
                None => {
                    failed.push(format!("{} (platform not found)", platform));
                    continue;
                }
            };

            if let Err(e) = fs::create_dir_all(&dest_dir.parent().unwrap_or(&dest_dir)) {
                failed.push(format!("{} ({})", platform, e));
                continue;
            }

            // Remove old dir/link if exists
            if dest_dir.exists() || dest_dir.is_symlink() {
                let _ = fs::remove_dir_all(&dest_dir);
            }

            match crate::utils::create_relative_symlink(&canonical_dir, &dest_dir) {
                Ok(true) => linked.push(platform.to_string()),
                _ => {
                    // Fallback to copy
                    if let Err(e) = git::copy_dir_recursive(&source_dir, &dest_dir) {
                        failed.push(format!("{} ({})", platform, e));
                        continue;
                    }
                    linked.push(format!("{} (copy)", platform));
                }
            }
        }

        if !linked.is_empty() {
            println!("{}: {}", "Symlinked".green(), linked.join(", "));
        }

        // Update lock file
        update_lock_file(source_name, &resolved, &skill_path, skill_name, global)?;

        if !failed.is_empty() {
            println!();
            println!("{}: {}", "Failed".red(), failed.join(", "));
        }
    }

    Ok(())
}

// ---------------------------------------------------------------------------
// Data loading
// ---------------------------------------------------------------------------

/// Load skills using the unified skill resolver.
fn load_skills(config: &Config, source: Option<&str>) -> Result<CacheData> {
    skill_resolver::resolve_skills(config, source)
}

// ---------------------------------------------------------------------------
// TUI steps
// ---------------------------------------------------------------------------

/// Step 1: Fuzzy-find a skill. Returns the selected FindItem.
fn run_skill_tui(items: Vec<FindItem>, initial_query: Option<&str>) -> Result<FindItem> {
    let mut builder = SkimOptionsBuilder::default();
    builder.multi(false);
    builder.prompt("Search skills: ".to_string());
    builder.exact(true);
    builder.highlight_line(true);
    builder.color("current:bg:236,current_match:fg:151:bg:236".to_string());
    builder.header(" \nup/down navigate | enter select | esc cancel\n ".to_string());

    if let Some(query) = initial_query {
        builder.query(query.to_string());
    }
    let opts = builder.build().map_err(|e| anyhow::anyhow!("{}", e))?;

    let output = Skim::run_items(opts, items).map_err(|e| anyhow::anyhow!("{}", e))?;

    if output.is_abort {
        bail!("Cancelled.");
    }

    let matched = match output.current {
        Some(item) => item,
        None => bail!("No skill selected."),
    };

    matched
        .downcast_item::<FindItem>()
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Failed to retrieve selected skill"))
}


/// Step 2: Select target platforms (multi-select).
fn run_platform_tui(config: &Config) -> Result<Vec<String>> {
    let mut items = vec![SelectItem {
        display: "Default".to_string(),
        value: "-".to_string(),
        disabled: true,
    }];

    for name in config.platform_names() {
        items.push(SelectItem {
            display: name.to_string(),
            value: name.to_string(),
            disabled: false,
        });
    }

    let header = " \nTAB: select/deselect\n ";

    let opts = SkimOptionsBuilder::default()
        .multi(true)
        .highlight_line(true)
        .color("current:bg:236,current_match:fg:151:bg:236".to_string())
        .header(header.to_string())
        .build()
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let output = Skim::run_items(opts, items).map_err(|e| anyhow::anyhow!("{}", e))?;

    if output.is_abort {
        bail!("Cancelled.");
    }

    let selected: Vec<String> = output
        .selected_items
        .iter()
        .filter_map(|item| item.downcast_item::<SelectItem>().map(|s| s.value.clone()))
        .collect();

    Ok(selected)
}

// ---------------------------------------------------------------------------
// Install helpers
// ---------------------------------------------------------------------------

/// Resolve the destination directory for a named platform.
fn resolve_platform_dest(
    config: &Config,
    platform_name: &str,
    skill_name: &str,
    is_global: bool,
) -> Option<PathBuf> {
    let platform = config.get_platform(platform_name)?;
    if platform.skills.is_empty() {
        return None;
    }
    let base = if is_global {
        dirs::home_dir().unwrap_or_default()
    } else {
        std::env::current_dir().unwrap_or_default()
    };
    Some(
        base.join(&platform.path)
            .join(&platform.skills)
            .join(skill_name),
    )
}

/// Update the lock file after successful installation.
fn update_lock_file(
    source: &str,
    resolved: &crate::utils::ResolvedSource,
    _skill_path: &str,
    skill_name: &str,
    global: bool,
) -> Result<()> {
    let config = Config::load()?;
    let mut lock_file = LockFile::load(global)?;

    let source_type = if source.contains('/') {
        "git".to_string()
    } else {
        config
            .get_source(source)
            .map(|s| s.effective_type())
            .unwrap_or_else(|| "git".to_string())
    };

    let skill_path_in_repo = format!("skills/{}/SKILL.md", skill_name);
    let tmp_dir = git::clone_for_listing(&resolved.url)?;
    let skill_folder_hash =
        git::get_skill_folder_hash(tmp_dir.path(), skill_name).unwrap_or_default();

    let now = chrono::Utc::now();
    let timestamp = now.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    let installed_at = if let Some(existing) = lock_file.skills.get(skill_name) {
        existing.installed_at.clone()
    } else {
        timestamp.clone()
    };

    let entry = LockEntry {
        source: source.to_string(),
        source_type,
        source_url: resolved.url.clone(),
        skill_path: skill_path_in_repo,
        skill_folder_hash,
        installed_at,
        updated_at: timestamp,
    };

    lock_file.upsert_skill(skill_name, entry);
    lock_file.save(global)?;

    Ok(())
}

// ---------------------------------------------------------------------------
// Display helpers
// ---------------------------------------------------------------------------

/// Collect FindItem entries from cache data, optionally filtered by source.
fn collect_items(cache_data: &CacheData, source: Option<&str>) -> Vec<FindItem> {
    let mut items: Vec<FindItem> = Vec::new();
    for source_cache in &cache_data.sources {
        if let Some(filter_source) = source
            && source_cache.source != filter_source
        {
            continue;
        }
        let is_registry = source_cache.registry_url.is_some();
        for cached_skill in &source_cache.skills {
            let display = format_display(
                cached_skill,
                &source_cache.source,
                is_registry,
                source_cache.url.as_deref(),
            );
            items.push(FindItem {
                display,
                skill: cached_skill.clone(),
                source: source_cache.source.clone(),
                is_registry,
                source_url: source_cache.url.clone(),
            });
        }
    }
    items
}

/// Format display string for fuzzy matching.
/// Normal: "name [source]"
/// Registry: "name [registry] [source]"
/// Registry + name collision (source empty): "name [registry] [source_url]"
fn format_display(
    skill: &CachedSkill,
    source: &str,
    is_registry: bool,
    source_url: Option<&str>,
) -> String {
    let effective_source = if source.is_empty() {
        source_url.unwrap_or("-")
    } else {
        source
    };
    if is_registry {
        format!("{} [registry] [{}]", skill.name, effective_source)
    } else {
        format!("{} [{}]", skill.name, effective_source)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::{CacheData, CachedSkill, SourceCache};

    #[test]
    fn test_format_display() {
        let skill = CachedSkill {
            name: "vue".to_string(),
            path: "skills/vue/SKILL.md".to_string(),
            description: "ignored".to_string(),
            version: "1.0.0".to_string(),
        };
        assert_eq!(format_display(&skill, "antfu", false, None), "vue [antfu]");
    }

    #[test]
    fn test_format_display_no_description() {
        let skill = CachedSkill {
            name: "git-commit".to_string(),
            path: "skills/git-commit/SKILL.md".to_string(),
            description: String::new(),
            version: String::new(),
        };
        assert_eq!(
            format_display(&skill, "other", false, None),
            "git-commit [other]"
        );
    }

    #[test]
    fn test_format_display_registry() {
        let skill = CachedSkill {
            name: "vue".to_string(),
            path: "skills/vue/SKILL.md".to_string(),
            description: String::new(),
            version: String::new(),
        };
        assert_eq!(
            format_display(&skill, "antfu", true, None),
            "vue [registry] [antfu]"
        );
    }

    #[test]
    fn test_format_display_registry_name_collision() {
        let skill = CachedSkill {
            name: "vue".to_string(),
            path: "skills/vue/SKILL.md".to_string(),
            description: String::new(),
            version: String::new(),
        };
        // Source name empty due to collision, fallback to URL
        assert_eq!(
            format_display(&skill, "", true, Some("https://example.com/repo")),
            "vue [registry] [https://example.com/repo]"
        );
    }

    #[test]
    fn test_format_display_registry_no_url_fallback() {
        let skill = CachedSkill {
            name: "vue".to_string(),
            path: "skills/vue/SKILL.md".to_string(),
            description: String::new(),
            version: String::new(),
        };
        // Source name empty, no URL → fallback to "-"
        assert_eq!(format_display(&skill, "", true, None), "vue [registry] [-]");
    }

    fn make_cache() -> CacheData {
        CacheData {
            updated_at: "2026-07-18T12:00:00Z".to_string(),
            sources: vec![
                SourceCache {
                    source: "antfu".to_string(),
                    url: None,
                    registry_url: None,
                    skills: vec![
                        CachedSkill {
                            name: "vue".to_string(),
                            path: "skills/vue/SKILL.md".to_string(),
                            description: "Vue.js skills".to_string(),
                            version: "1.0.0".to_string(),
                        },
                        CachedSkill {
                            name: "git-commit".to_string(),
                            path: "skills/git-commit/SKILL.md".to_string(),
                            description: "Git commit helper".to_string(),
                            version: String::new(),
                        },
                    ],
                },
                SourceCache {
                    source: "other".to_string(),
                    url: None,
                    registry_url: None,
                    skills: vec![CachedSkill {
                        name: "react".to_string(),
                        path: "skills/react/SKILL.md".to_string(),
                        description: "React skills".to_string(),
                        version: "2.0.0".to_string(),
                    }],
                },
            ],
        }
    }

    #[test]
    fn test_collect_items_all_sources() {
        let cache = make_cache();
        let items = collect_items(&cache, None);
        assert_eq!(items.len(), 3);
    }

    #[test]
    fn test_collect_items_filter_by_source_found() {
        let cache = make_cache();
        let items = collect_items(&cache, Some("antfu"));
        assert_eq!(items.len(), 2);
        assert!(items.iter().all(|i| i.source == "antfu"));
    }

    #[test]
    fn test_collect_items_filter_by_source_not_found() {
        let cache = make_cache();
        let items = collect_items(&cache, Some("nonexistent"));
        assert!(items.is_empty());
    }

    #[test]
    fn test_collect_items_registry_source() {
        let cache = CacheData {
            updated_at: "2026-07-18T12:00:00Z".to_string(),
            sources: vec![
                SourceCache {
                    source: "local-src".to_string(),
                    url: Some("https://example.com/local".to_string()),
                    registry_url: None,
                    skills: vec![CachedSkill {
                        name: "vue".to_string(),
                        path: "skills/vue/SKILL.md".to_string(),
                        description: String::new(),
                        version: String::new(),
                    }],
                },
                SourceCache {
                    source: "remote-src".to_string(),
                    url: Some("https://example.com/remote".to_string()),
                    registry_url: Some("https://xskill.gcli.cn/skills.json".to_string()),
                    skills: vec![CachedSkill {
                        name: "react".to_string(),
                        path: "skills/react/SKILL.md".to_string(),
                        description: String::new(),
                        version: String::new(),
                    }],
                },
            ],
        };
        let items = collect_items(&cache, None);
        assert_eq!(items.len(), 2);
        // Local item
        assert!(!items[0].is_registry);
        assert_eq!(items[0].display, "vue [local-src]");
        // Registry item
        assert!(items[1].is_registry);
        assert_eq!(items[1].display, "react [registry] [remote-src]");
    }

    #[test]
    fn test_collect_items_empty_cache() {
        let cache = CacheData::default();
        let items = collect_items(&cache, None);
        assert!(items.is_empty());
    }

    #[test]
    fn test_find_item_text_and_output() {
        let item = FindItem {
            display: "vue [antfu]".to_string(),
            skill: CachedSkill {
                name: "vue".to_string(),
                path: String::new(),
                description: String::new(),
                version: String::new(),
            },
            source: "antfu".to_string(),
            is_registry: false,
            source_url: None,
        };
        assert_eq!(item.text(), "vue [antfu]");
        assert_eq!(item.output(), "vue");
    }

    #[test]
    fn test_find_item_display_colored() {
        use ratatui::style::Style;
        let item = FindItem {
            display: "vue [antfu]".to_string(),
            skill: CachedSkill {
                name: "vue".to_string(),
                path: String::new(),
                description: String::new(),
                version: String::new(),
            },
            source: "antfu".to_string(),
            is_registry: false,
            source_url: None,
        };

        let context = DisplayContext {
            score: 0,
            matches: skim::Matches::None,
            container_width: 80,
            base_style: Style::default(),
            matched_style: Style::default(),
        };

        let line = item.display(context);
        assert_eq!(line.spans.len(), 2);
        // First span: skill name in default color (no bg = not selected)
        assert_eq!(line.spans[0].content.as_ref(), "vue");
        assert_eq!(line.spans[0].style.fg, None);
        // Second span: source in dark gray
        assert_eq!(line.spans[1].content.as_ref(), " [antfu]");
        assert_eq!(line.spans[1].style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn test_find_item_display_registry() {
        use ratatui::style::Style;
        let item = FindItem {
            display: "vue [registry] [antfu]".to_string(),
            skill: CachedSkill {
                name: "vue".to_string(),
                path: String::new(),
                description: String::new(),
                version: String::new(),
            },
            source: "antfu".to_string(),
            is_registry: true,
            source_url: Some("https://github.com/antfu/skills".to_string()),
        };

        let context = DisplayContext {
            score: 0,
            matches: skim::Matches::None,
            container_width: 80,
            base_style: Style::default(),
            matched_style: Style::default(),
        };

        let line = item.display(context);
        // name + [registry] + [source] = 3 spans
        assert_eq!(line.spans.len(), 3);
        assert_eq!(line.spans[0].content.as_ref(), "vue");
        assert_eq!(line.spans[1].content.as_ref(), " [registry]");
        assert_eq!(line.spans[1].style.fg, Some(Color::DarkGray));
        assert_eq!(line.spans[2].content.as_ref(), " [antfu]");
        assert_eq!(line.spans[2].style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn test_find_item_display_registry_name_collision() {
        use ratatui::style::Style;
        let item = FindItem {
            display: "vue [registry] [https://example.com/repo]".to_string(),
            skill: CachedSkill {
                name: "vue".to_string(),
                path: String::new(),
                description: String::new(),
                version: String::new(),
            },
            source: String::new(), // empty due to name collision
            is_registry: true,
            source_url: Some("https://example.com/repo".to_string()),
        };

        let context = DisplayContext {
            score: 0,
            matches: skim::Matches::None,
            container_width: 80,
            base_style: Style::default(),
            matched_style: Style::default(),
        };

        let line = item.display(context);
        assert_eq!(line.spans.len(), 3);
        // Falls back to URL when source is empty
        assert_eq!(
            line.spans[2].content.as_ref(),
            " [https://example.com/repo]"
        );
    }

    #[test]
    fn test_find_item_display_registry_selected() {
        use ratatui::style::Style;
        let item = FindItem {
            display: "vue [registry] [antfu]".to_string(),
            skill: CachedSkill {
                name: "vue".to_string(),
                path: String::new(),
                description: String::new(),
                version: String::new(),
            },
            source: "antfu".to_string(),
            is_registry: true,
            source_url: Some("https://github.com/antfu/skills".to_string()),
        };

        // Simulate selected state with bg color
        let context = DisplayContext {
            score: 0,
            matches: skim::Matches::None,
            container_width: 80,
            base_style: Style::default().bg(ratatui::style::Color::Indexed(236)),
            matched_style: Style::default(),
        };

        let line = item.display(context);
        assert_eq!(line.spans.len(), 3);
        // Name: blue when selected
        assert_eq!(line.spans[0].style.fg, Some(Color::Blue));
        // [registry]: green when selected
        assert_eq!(line.spans[1].content.as_ref(), " [registry]");
        assert_eq!(line.spans[1].style.fg, Some(Color::Green));
        // [source]: dark gray always
        assert_eq!(line.spans[2].content.as_ref(), " [antfu]");
        assert_eq!(line.spans[2].style.fg, Some(Color::DarkGray));
    }

    #[test]
    fn test_select_item_text_and_output() {
        let item = SelectItem {
            display: "Project [.agents/skills]".to_string(),
            value: "project".to_string(),
            disabled: false,
        };
        assert_eq!(item.text(), "Project [.agents/skills]");
        assert_eq!(item.output(), "project");
    }

    #[test]
    fn test_non_interactive_terminal_detection() {
        if !std::io::stdin().is_terminal() {
            let result = run(None, None, false);
            assert!(result.is_err());
            assert!(
                result
                    .unwrap_err()
                    .to_string()
                    .contains("interactive terminal")
            );
        }
    }

    #[test]
    fn test_resolve_platform_dest_known_platform() {
        use crate::config::{Config, Platform};
        use std::collections::HashMap;

        let mut platforms = HashMap::new();
        platforms.insert(
            "claude".to_string(),
            Platform {
                path: ".claude".to_string(),
                skills: "skills".to_string(),
                agents: "CLAUDE.md".to_string(),
                source: "AGENTS.md".to_string(),
                agents_compat: false,
            },
        );
        let config = Config {
            platforms,
            ..Default::default()
        };

        let result = resolve_platform_dest(&config, "claude", "my-skill", false);
        assert!(result.is_some());
        let path = result.unwrap();
        assert!(path.ends_with(".claude/skills/my-skill"));
    }

    #[test]
    fn test_resolve_platform_dest_unknown_platform() {
        let config = Config::default();
        let result = resolve_platform_dest(&config, "nonexistent", "my-skill", false);
        assert!(result.is_none());
    }

    #[test]
    fn test_resolve_platform_dest_no_skills_dir() {
        use crate::config::{Config, Platform};
        use std::collections::HashMap;

        let mut platforms = HashMap::new();
        platforms.insert(
            "minimal".to_string(),
            Platform {
                path: ".minimal".to_string(),
                skills: String::new(),
                agents: String::new(),
                source: "AGENTS.md".to_string(),
                agents_compat: false,
            },
        );
        let config = Config {
            platforms,
            ..Default::default()
        };

        let result = resolve_platform_dest(&config, "minimal", "my-skill", false);
        assert!(result.is_none());
    }
}
