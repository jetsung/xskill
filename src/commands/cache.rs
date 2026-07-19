use crate::cache::{self, CacheData, CachedSkill, SourceCache};
use crate::config::Config;
use crate::git;
use crate::skill_meta::SkillMeta;
use crate::utils::{normalize_url, strip_git_suffix};
use anyhow::{Context, Result};
use chrono::Utc;
use colored::Colorize;
use std::fs;
use std::path::Path;

/// Check if a string looks like a URL.
fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

/// Update cache for sources
pub fn run_update(from: Option<&str>) -> Result<()> {
    let config = Config::load()?;

    // Handle --from with URL not in sources: save to URL cache
    if let Some(src) = from {
        if is_url(src) && config.get_source(src).is_none() {
            let url = normalize_url(src);
            let skills = collect_source_skills(&url, src)?;
            let count = skills.len();
            // Normalize URL: strip .git suffix for consistent source name
            let normalized = src.strip_suffix(".git").unwrap_or(src);
            let data = CacheData {
                updated_at: Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string(),
                sources: vec![SourceCache {
                    source: normalized.to_string(),
                    url: Some(url),
                    registry_url: None,
                    skills,
                }],
            };
            cache::save_url_cache(src, &data)?;
            println!("{}: {} {}", normalized, count, "skills".cyan());
            return Ok(());
        }
    }

    if config.sources.is_empty() {
        if config.is_registry_enabled() {
            println!("{}", "No local sources configured. Registry will be used for skill discovery.".yellow());
        } else {
            println!("{}", "No sources configured. Add sources with 'xskill sources add' or enable registry with 'xskill config --set registry.enabled=true'.".yellow());
        }
        return Ok(());
    }

    // Determine which sources to update
    let sources_to_update = if let Some(name) = from {
        let s = config.get_source(name).ok_or_else(|| {
            let existing: Vec<String> = config.sources.iter().map(|s| s.effective_name()).collect();
            anyhow::anyhow!("Source '{}' not found.\nAvailable: {}", name, existing.join(", "))
        })?;
        vec![s]
    } else {
        config.sources.iter().collect()
    };

    // Load existing cache or create new
    let mut data = CacheData::load().unwrap_or_default();

    let mut total_skills = 0;
    let mut success = 0;
    let mut failed = 0;

    for source in &sources_to_update {
        let source_name = source.effective_name();
        match collect_source_skills(&source.url, &source_name) {
            Ok(skills) => {
                let count = skills.len();
                println!("{}: {} {}", source_name, count, "skills".cyan());
                total_skills += count;

                // Update or add source cache
                if let Some(existing) = data.sources.iter_mut().find(|s| s.source == source_name) {
                    existing.skills = skills;
                } else {
                    data.sources.push(SourceCache {
                        source: source_name,
                        url: Some(source.url.clone()),
                        registry_url: None,
                        skills,
                    });
                }
                success += 1;
            }
            Err(e) => {
                eprintln!("{}: {} - {}", source_name, "failed".red(), e);
                failed += 1;
            }
        }
    }

    data.updated_at = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    data.save()?;

    println!();
    if failed > 0 {
        println!("{}: {} sources, {} skills total ({} failed)", "Cache updated".green(), success, total_skills, format!("{} failed", failed).red());
    } else {
        println!("{}: {} sources, {} skills total", "Cache updated".green(), success, total_skills);
    }
    Ok(())
}

/// Clear cache (empty skills list, update timestamp)
pub fn run_clear(from: Option<&str>) -> Result<()> {
    let mut data = match CacheData::load() {
        Some(d) => d,
        None => {
            println!("{}", "No cache to clear".yellow());
            return Ok(());
        }
    };

    if let Some(name) = from {
        // Remove specific source entry (normalize URL for comparison)
        let before = data.sources.len();
        data.sources.retain(|s| strip_git_suffix(&s.source) != strip_git_suffix(name));
        if data.sources.len() < before {
            data.updated_at = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
            data.save()?;
            let url_cache_count = cache::clear_url_caches()?;
            println!("{} for '{}'", "Cache cleared".green(), name);
            if url_cache_count > 0 {
                println!("{} {} URL cache files removed", "Cleaned".green(), url_cache_count);
            }
        } else {
            println!("{} '{}' not found in cache", "Source".yellow(), name);
        }
    } else {
        // Clear all sources
        let source_count = data.sources.len();
        data.sources.clear();
        data.updated_at = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
        data.save()?;
        let url_cache_count = cache::clear_url_caches()?;
        println!("{}: {} sources cleared", "Cache cleared".green(), source_count);
        if url_cache_count > 0 {
            println!("{} {} URL cache files removed", "Cleaned".green(), url_cache_count);
        }
    }
    Ok(())
}

/// Collect skills from a single source
fn collect_source_skills(url: &str, _source_name: &str) -> Result<Vec<CachedSkill>> {
    let url = normalize_url(url);
    let tmp_dir = git::clone_for_listing(&url)?;
    let skills_dir = tmp_dir.path().join("skills");

    if !skills_dir.exists() {
        return Ok(vec![]);
    }

    collect_skills_from_repo(tmp_dir.path(), &skills_dir)
}

/// Recursively collect skills from a repo directory, parsing SKILL.md frontmatter
fn collect_skills_from_repo(repo_root: &Path, dir: &Path) -> Result<Vec<CachedSkill>> {
    let mut skills = Vec::new();
    collect_recursive(repo_root, dir, &mut skills)?;
    Ok(skills)
}

fn collect_recursive(
    repo_root: &Path,
    dir: &Path,
    skills: &mut Vec<CachedSkill>,
) -> Result<()> {
    for entry in fs::read_dir(dir)
        .with_context(|| format!("Failed to read directory: {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();

        if !path.is_dir() {
            continue;
        }

        let dir_name = entry.file_name().to_string_lossy().to_string();
        let skill_md = path.join("SKILL.md");

        if skill_md.exists() {
            let meta = SkillMeta::from_file(&path).unwrap_or_default();
            let full_rel = path
                .strip_prefix(repo_root)
                .unwrap_or(&path)
                .to_string_lossy()
                .to_string();

            skills.push(CachedSkill {
                name: meta.display_name(&dir_name),
                path: full_rel,
                description: meta.display_description(),
                version: meta.display_version(),
            });
        }

        // Recurse into subdirectories
        collect_recursive(repo_root, &path, skills)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::config::Config;

    #[test]
    fn test_empty_config_has_no_sources() {
        let config = Config::default();
        assert!(config.sources.is_empty());
    }

    #[test]
    fn test_config_with_sources() {
        let mut config = Config::default();
        config.sources.push(crate::config::Source {
            name: "antfu".to_string(),
            source_type: "git".to_string(),
            url: "https://github.com/antfu/skills".to_string(),
        });
        assert_eq!(config.sources.len(), 1);
        assert_eq!(config.sources[0].name, "antfu");
    }
}
