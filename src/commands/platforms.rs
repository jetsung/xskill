use crate::config::{Config, default_platforms};
use crate::output::print_table;
use anyhow::{Result, bail};
use colored::Colorize;
use ratatui::text::{Line as TuiLine, Span as TuiSpan};
use skim::prelude::*;
use std::io::IsTerminal;

fn compat_str(agents_compat: bool) -> String {
    if agents_compat {
        "✓".green().to_string()
    } else {
        "✗".red().to_string()
    }
}

pub fn run(detailed: bool) -> Result<()> {
    let config = Config::load()?;

    if config.platforms.is_empty() {
        println!("{}", "No platforms configured".yellow());
        return Ok(());
    }

    let mut sorted: Vec<_> = config.platforms.iter().collect();
    sorted.sort_by_key(|(name, _)| name.to_lowercase());

    if detailed {
        let headers = &["NAME", "PATH", "SKILLS", "AGENTS", "SOURCE", "COMPAT"];
        let rows: Vec<Vec<String>> = sorted
            .iter()
            .map(|(name, platform)| {
                let skills_dir = platform
                    .skills_dir()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let agents_file = platform
                    .agents_file()
                    .map(|p| p.to_string_lossy().into_owned())
                    .unwrap_or_default();
                let source_file = platform.source_file().to_string_lossy().into_owned();
                let compat = compat_str(platform.agents_compat);
                vec![
                    name.to_string(),
                    platform.path.clone(),
                    skills_dir,
                    agents_file,
                    source_file,
                    compat,
                ]
            })
            .collect();
        print_table(headers, &rows);
    } else {
        let headers = &["NAME", "PATH", "COMPAT"];
        let rows: Vec<Vec<String>> = sorted
            .iter()
            .map(|(name, platform)| {
                vec![
                    name.to_string(),
                    platform.path.clone(),
                    compat_str(platform.agents_compat),
                ]
            })
            .collect();
        print_table(headers, &rows);
    }

    Ok(())
}

/// 重置模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ResetMode {
    /// 完全替换：platforms 重置为默认列表（丢弃自定义）
    Replace,
    /// 覆盖：默认值覆盖内置平台，保留自定义平台
    Merge,
    /// 取消
    Cancel,
}

/// 返回当前配置中的自定义平台名称（不在默认列表中的，排序）
fn custom_platform_names(config: &Config) -> Vec<String> {
    let defaults = default_platforms();
    let mut names: Vec<String> = config
        .platforms
        .keys()
        .filter(|k| !defaults.contains_key(*k))
        .cloned()
        .collect();
    names.sort_unstable();
    names
}

/// 按模式应用重置，修改 config.platforms
fn apply_reset_mode(config: &mut Config, mode: ResetMode) {
    let defaults = default_platforms();
    config.platforms = match mode {
        // 完全替换：直接使用默认列表
        ResetMode::Replace => defaults,
        // 覆盖：默认列表 + 自定义平台（内置平台保持默认值，自定义平台原样保留）
        ResetMode::Merge => {
            let mut merged = defaults;
            for (name, platform) in std::mem::take(&mut config.platforms) {
                merged.entry(name).or_insert(platform);
            }
            merged
        }
        ResetMode::Cancel => return,
    };
}

/// 重置 platforms：弹出一个单选 TUI，由用户选择重置方式
pub fn run_reset() -> Result<()> {
    if !std::io::stdin().is_terminal() {
        bail!("'platforms reset' requires an interactive terminal.");
    }

    let mut config = Config::load()?;

    let custom = custom_platform_names(&config);
    if !custom.is_empty() {
        println!("{}: {}", "Custom platforms".yellow(), custom.join(", "));
    }

    let mode = select_reset_mode()?;
    if mode == ResetMode::Cancel {
        println!("{}", "Cancelled.".yellow());
        return Ok(());
    }

    apply_reset_mode(&mut config, mode);
    config.save()?;

    let desc = match mode {
        ResetMode::Replace => "replaced with defaults (custom dropped)",
        ResetMode::Merge => "merged (custom kept)",
        ResetMode::Cancel => unreachable!(),
    };
    println!(
        "{}: {} platforms, {}",
        "Platforms reset".green(),
        config.platforms.len(),
        desc
    );
    Ok(())
}

/// 显示 skim 单选 TUI，返回选中的模式
fn select_reset_mode() -> Result<ResetMode> {
    let items: Vec<ResetItem> = ResetMode::OPTIONS
        .iter()
        .map(|(mode, title, desc)| ResetItem {
            title: title.to_string(),
            desc: desc.to_string(),
            mode: *mode,
        })
        .collect();

    let opts = SkimOptionsBuilder::default()
        .multi(false)
        .prompt("Reset platforms: ".to_string())
        .exact(true)
        .highlight_line(true)
        .multiline(Some("\n".to_string()))
        .reverse(true)
        .color("current:bg:236,current_match:fg:151:bg:236".to_string())
        .header(" \nup/down navigate | enter select | esc cancel\n ".to_string())
        .build()
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let output = Skim::run_items(opts, items).map_err(|e| anyhow::anyhow!("{}", e))?;

    if output.is_abort {
        return Ok(ResetMode::Cancel);
    }

    let matched = match output.current {
        Some(item) => item,
        None => bail!("No mode selected."),
    };

    matched
        .downcast_item::<ResetItem>()
        .map(|item| item.mode)
        .ok_or_else(|| anyhow::anyhow!("Failed to retrieve selected mode"))
}

/// 单选 TUI 的可选择项（两行式：第一行操作名，第二行效果说明）
struct ResetItem {
    title: String,
    desc: String,
    mode: ResetMode,
}

impl ResetMode {
    /// 定义顺序即显示顺序（reverse(true) 下 items[0] 显示在最上方，回车默认选中第一项）。
    const OPTIONS: [(ResetMode, &'static str, &'static str); 3] = [
        (
            ResetMode::Replace,
            "完全恢复",
            "所有平台恢复内置默认配置，移除自定义平台",
        ),
        (ResetMode::Merge, "谨慎合并", "只更新内置渠道，自定义平台保留"),
        (ResetMode::Cancel, "取消", "不修改任何配置"),
    ];
}

impl SkimItem for ResetItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Owned(format!("{}\n{}", self.title, self.desc))
    }

    fn output(&self) -> Cow<'_, str> {
        Cow::Owned(match self.mode {
            ResetMode::Replace => "replace".to_string(),
            ResetMode::Merge => "merge".to_string(),
            ResetMode::Cancel => "cancel".to_string(),
        })
    }

    fn display(&self, context: DisplayContext) -> TuiLine<'_> {
        use ratatui::style::Color;
        let base = context.base_style;
        let is_selected = base.bg.is_some();
        let title_style = if is_selected {
            base.fg(Color::Blue)
        } else {
            base
        };
        let desc_style = base.fg(Color::DarkGray);
        TuiLine::from(vec![
            TuiSpan::styled(self.title.clone(), title_style),
            TuiSpan::raw("\n"),
            TuiSpan::styled(self.desc.clone(), desc_style),
        ])
    }
}

#[cfg(test)]
mod tests {
    use super::{ResetMode, compat_str, default_platforms};
    use crate::config::{Config, Platform};
    use colored::Colorize;

    #[test]
    fn test_compat_str() {
        assert_eq!(compat_str(true), "✓".green().to_string());
        assert_eq!(compat_str(false), "✗".red().to_string());
    }

    #[test]
    fn test_platform_config_fields() {
        let platform = Platform {
            path: ".claude".to_string(),
            skills: "skills".to_string(),
            agents: "CLAUDE.md".to_string(),
            source: "AGENTS.md".to_string(),
            agents_compat: false,
        };
        assert!(platform.skills_dir().is_some());
        assert!(platform.agents_file().is_some());
        assert!(!platform.agents_compat);
        assert_eq!(
            platform.source_file(),
            std::path::PathBuf::from(".agents/AGENTS.md")
        );
    }

    #[test]
    fn test_platform_agents_compat() {
        let platform = Platform {
            path: ".opencode".to_string(),
            skills: "skills".to_string(),
            agents: "AGENTS.md".to_string(),
            source: "AGENTS.md".to_string(),
            agents_compat: true,
        };
        assert!(platform.agents_compat);
    }

    #[test]
    fn test_platform_no_skills_no_agents() {
        let platform = Platform {
            path: ".gemini".to_string(),
            skills: String::new(),
            agents: String::new(),
            source: "AGENTS.md".to_string(),
            agents_compat: false,
        };
        assert!(platform.skills_dir().is_none());
        assert!(platform.agents_file().is_none());
        assert!(!platform.agents_compat);
    }

    fn platform(name: &str) -> Platform {
        Platform {
            path: format!(".{}", name),
            skills: "skills".to_string(),
            agents: "AGENTS.md".to_string(),
            source: "AGENTS.md".to_string(),
            agents_compat: true,
        }
    }

    fn config_with(extra: &[(&str, Platform)]) -> Config {
        let mut config = Config {
            platforms: default_platforms(),
            ..Config::default()
        };
        for (name, p) in extra {
            config.platforms.insert(name.to_string(), p.clone());
        }
        config
    }

    #[test]
    fn test_custom_platform_names() {
        let config = config_with(&[("my-custom", platform("my-custom"))]);
        let custom = super::custom_platform_names(&config);
        assert_eq!(custom, vec!["my-custom"]);
    }

    #[test]
    fn test_custom_platform_names_empty() {
        let config = config_with(&[]);
        assert!(super::custom_platform_names(&config).is_empty());
    }

    #[test]
    fn test_apply_reset_mode_replace() {
        let mut config = config_with(&[("my-custom", platform("my-custom"))]);
        super::apply_reset_mode(&mut config, ResetMode::Replace);
        let defaults = default_platforms();
        assert_eq!(config.platforms.len(), defaults.len());
        for name in defaults.keys() {
            assert!(config.platforms.contains_key(name));
        }
        assert!(!config.platforms.contains_key("my-custom"));
    }

    #[test]
    fn test_apply_reset_mode_merge() {
        let mut config = config_with(&[("my-custom", platform("my-custom"))]);
        super::apply_reset_mode(&mut config, ResetMode::Merge);
        assert!(config.platforms.contains_key("my-custom"));
        assert_eq!(config.platforms.len(), default_platforms().len() + 1);
        // 内置平台被默认值覆盖
        let claude = &config.platforms["claude"];
        assert_eq!(claude.agents, "CLAUDE.md");
        assert_eq!(claude.path, ".claude");
    }
}
