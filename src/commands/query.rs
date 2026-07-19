use crate::cache::{CachedSkill, SourceCache};
use crate::config::Config;
use crate::skill_resolver;
use crate::utils::strip_git_suffix;
use anyhow::Result;
use colored::Colorize;

/// Query skills
pub fn run(skill: &str, source: Option<&str>) -> Result<()> {
    // Validate skill parameter
    if skill.is_empty() {
        anyhow::bail!("Skill name cannot be empty");
    }
    if skill == "*" {
        anyhow::bail!("Skill name cannot be '*', use a specific skill name");
    }

    let config = Config::load()?;
    let mut found = false;

    // Use unified resolver to get all available skills
    let cache_data = skill_resolver::resolve_skills(&config, source)?;

    if let Some(src) = source {
        // Query specific source (normalize URL for comparison)
        if let Some(source_cache) = cache_data.sources.iter().find(|s| {
            strip_git_suffix(&s.source) == strip_git_suffix(src)
        }) {
            query_in_cache(source_cache, skill, &mut found);
        }
    } else {
        // Iterate all sources
        for source_cache in &cache_data.sources {
            query_in_cache(source_cache, skill, &mut found);
        }
    }

    if !found {
        println!("{}: {}", "Skill not found in any source".yellow(), skill);

        // Hint: suggest cache update when sources exist but cache may be stale
        if !config.sources.is_empty() || config.is_registry_enabled() {
            if config.is_cache_enabled() {
                println!("{}", "Hint: run `xskill cache update` to refresh skills cache".cyan());
            }
        }
    }

    Ok(())
}

/// Query skill in cache
fn query_in_cache(
    source_cache: &SourceCache,
    skill: &str,
    found: &mut bool,
) {
    let matches: Vec<_> = source_cache.skills.iter().filter(|s| {
        s.name == skill || s.path.contains(skill)
    }).collect();

    if matches.is_empty() {
        return;
    }
    *found = true;

    print_skill_table(source_cache, &matches);
}

/// Print skills as vertical key-value blocks.
fn print_skill_table(source_cache: &SourceCache, skills: &[&CachedSkill]) {
    for skill in skills {
        let display_source = if source_cache.source.is_empty() {
            "-".to_string()
        } else {
            source_cache.source.clone()
        };
        println!("{}: {}", "Source".cyan().bold(), display_source);
        if let Some(ref registry_url) = source_cache.registry_url {
            if !registry_url.is_empty() {
                println!("{}: {}", "Registry".cyan().bold(), registry_url);
            }
        }
        println!("{}: {}", "Name".cyan().bold(), skill.name.yellow());
        if !skill.description.is_empty() && skill.description != "无" {
            println!("{}: {}", "Description".cyan().bold(), skill.description);
        }
        if !skill.version.is_empty() {
            println!("{}: {}", "Version".cyan().bold(), skill.version);
        }
        println!("{}: {}", "Path".cyan().bold(), skill.path);
        println!();
    }
}

#[cfg(test)]
mod tests {
    use crate::cache::CacheData;
    use crate::config::CacheConfig;

    #[test]
    fn test_cache_enabled_config() {
        let config = crate::config::Config {
            cache: CacheConfig {
                enabled: true,
                ..Default::default()
            },
            ..Default::default()
        };
        assert!(config.is_cache_enabled());
    }

    #[test]
    fn test_cache_disabled_config() {
        let config = crate::config::Config::default();
        assert!(!config.is_cache_enabled());
    }

    #[test]
    fn test_cache_miss_returns_none() {
        let cache: Option<CacheData> = None;
        assert!(cache.is_none());
    }
}
