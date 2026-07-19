use crate::config::{Config, RecommendedSource};
use crate::output::print_table;
use anyhow::Result;
use colored::Colorize;

/// List all configured recommended sources
pub fn run() -> Result<()> {
    let config = Config::load()?;

    if config.recommended.is_empty() {
        println!("{}", "No recommended skills configured".yellow());
        return Ok(());
    }

    let headers = &["SOURCE", "NAME", "URL", "SKILLS"];
    let rows: Vec<Vec<String>> = config.recommended.iter().map(|item| {
        let found_source = if !item.name.is_empty() {
            config.get_source(&item.name)
        } else {
            None
        };

        let (display_url, is_valid) = if let Some(source) = found_source {
            if item.url.is_empty() {
                (source.url.clone(), true)
            } else if item.url == source.url {
                (item.url.clone(), true)
            } else {
                (format!("{}", "invalid".red()), false)
            }
        } else if item.url.is_empty() {
            (format!("{}", "invalid".red()), false)
        } else {
            (item.url.clone(), false)
        };

        let source_col = if is_valid { "true" } else { "false" }.to_string();

        vec![
            source_col,
            item.name.clone(),
            display_url,
            item.skills.join(", "),
        ]
    }).collect();
    print_table(headers, &rows);

    Ok(())
}

/// Add skills to a recommended source
///
/// Logic:
/// - When only name and skills: validate name exists in sources, add/update recommended
/// - When name, url, and skills:
///   - If name exists in sources AND url matches source url: save only name + skills
///   - If name exists in sources BUT url doesn't match: error
///   - If name doesn't exist in sources: save url + skills (name becomes url)
/// - If recommended entry already exists with skills, new skills are appended
pub fn run_add(name: Option<&str>, url: Option<&str>, skills: &str) -> Result<()> {
    let skills_list: Vec<String> = skills
        .split(',')
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
        .collect();

    if skills_list.is_empty() {
        anyhow::bail!("At least one skill is required");
    }

    let mut config = Config::load()?;

    // Resolve the recommended entry key and fields
    let (entry_name, entry_url) = match (name, url) {
        // Both name and url provided
        (Some(name), Some(url)) => {
            if name.is_empty() {
                anyhow::bail!("Name cannot be empty");
            }
            if url.is_empty() {
                anyhow::bail!("URL cannot be empty when provided");
            }

            // Check if name exists in sources
            if let Some(source) = config.get_source(name) {
                // Name exists in sources, check if url matches
                if source.url == url {
                    // Url matches, save only name (no url needed)
                    (name.to_string(), String::new())
                } else {
                    // Url doesn't match source
                    anyhow::bail!(
                        "Source '{}' exists with URL '{}', but provided URL '{}' doesn't match.",
                        name,
                        source.url,
                        url
                    );
                }
            } else {
                // Name doesn't exist in sources, use url as the identifier
                (url.to_string(), url.to_string())
            }
        }
        // Only name provided
        (Some(name), None) => {
            if name.is_empty() {
                anyhow::bail!("Name cannot be empty");
            }

            // Validate name exists in sources
            if config.get_source(name).is_none() {
                let existing: Vec<String> = config.sources.iter().map(|s| s.effective_name()).collect();
                anyhow::bail!(
                    "Source '{}' not found in sources. Available: {}",
                    name,
                    if existing.is_empty() {
                        "(none)".to_string()
                    } else {
                        existing.join(", ")
                    }
                );
            }

            (name.to_string(), String::new())
        }
        // Only url provided
        (None, Some(url)) => {
            if url.is_empty() {
                anyhow::bail!("URL cannot be empty when provided");
            }
            (url.to_string(), url.to_string())
        }
        // Neither provided
        (None, None) => {
            anyhow::bail!("At least one of --name or --url is required");
        }
    };

    // Check if recommended entry already exists
    if let Some(idx) = config
        .recommended
        .iter()
        .position(|r| r.name == entry_name)
    {
        // Entry exists, append new skills (avoid duplicates)
        let existing_skills = &mut config.recommended[idx].skills;
        let mut added_count = 0;
        for skill in &skills_list {
            if !existing_skills.contains(skill) {
                existing_skills.push(skill.clone());
                added_count += 1;
            }
        }

        if added_count == 0 {
            println!("{} already exist in '{}'.", "All skills".yellow(), entry_name);
        } else {
            println!(
                "{} {} skill(s) to '{}'. Total: {}",
                "Added".green(),
                added_count,
                entry_name,
                existing_skills.len()
            );
        }
    } else {
        // Create new entry
        config.recommended.push(RecommendedSource {
            name: entry_name.clone(),
            url: entry_url,
            skills: skills_list,
        });
        println!("{} '{}' added successfully.", "Recommended source".green(), entry_name);
    }

    config.save()?;
    Ok(())
}

/// Remove a recommended source or specific skills
///
/// Priority logic:
/// - When both name and url provided: prioritize url (fallback to name if url not found)
/// - When only name: delete entire entry with that name
/// - When name and skills: delete specific skills from entry with that name
/// - When url and skills: delete specific skills from entry with that url
pub fn run_remove(
    name: Option<&str>,
    url: Option<&str>,
    skills: Option<&str>,
) -> Result<()> {
    let mut config = Config::load()?;

    // Parse skills to remove
    let skills_to_remove: Option<Vec<String>> = skills.map(|s| {
        s.split(',')
            .map(|skill| skill.trim().to_string())
            .filter(|skill| !skill.is_empty())
            .collect()
    });

    // Find the target entry index
    let idx = match (name, url) {
        // Both name and url provided: prioritize url, fallback to name
        (Some(name), Some(url)) => {
            if let Some(idx) = config.recommended.iter().position(|r| r.url == url) {
                Some(idx)
            } else {
                config.recommended.iter().position(|r| r.name == name)
            }
        }
        // Only url provided
        (None, Some(url)) => config.recommended.iter().position(|r| r.url == url),
        // Only name provided
        (Some(name), None) => config.recommended.iter().position(|r| r.name == name),
        // Neither provided
        (None, None) => {
            anyhow::bail!("At least one of --name or --url is required");
        }
    };

    let idx = idx.ok_or_else(|| {
        let existing: Vec<&str> = config.recommended.iter().map(|r| r.name.as_str()).collect();
        anyhow::anyhow!(
            "Recommended source not found. Available: {}",
            if existing.is_empty() {
                "(none)".to_string()
            } else {
                existing.join(", ")
            }
        )
    })?;

    // Perform removal
    if let Some(skills_to_remove) = skills_to_remove {
        // Remove specific skills from the entry
        let entry = &mut config.recommended[idx];
        let original_count = entry.skills.len();
        entry.skills.retain(|s| !skills_to_remove.contains(s));
        let removed_count = original_count - entry.skills.len();

        if removed_count == 0 {
            println!("{}", "No matching skills found to remove.".yellow());
            return Ok(());
        }

        // If no skills left, remove the entire entry
        if entry.skills.is_empty() {
            config.recommended.remove(idx);
            println!("{}", "All skills removed, entry deleted.".green());
        } else {
            println!("{} {} skill(s) from '{}'.", "Removed".green(), removed_count, entry.name);
        }
    } else {
        // Remove the entire entry
        let removed = config.recommended.remove(idx);
        println!("{} '{}' removed successfully.", "Recommended source".green(), removed.name);
    }

    config.save()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    #[test]
    fn test_empty_recommended_config() {
        let config = Config::default();
        assert!(config.recommended.is_empty());
    }

    #[test]
    fn test_add_recommended_source() {
        let mut config = Config::default();
        config.recommended.push(RecommendedSource {
            name: "test".to_string(),
            url: "https://example.com".to_string(),
            skills: vec!["skill1".to_string(), "skill2".to_string()],
        });
        assert_eq!(config.recommended.len(), 1);
        assert_eq!(config.recommended[0].name, "test");
        assert_eq!(config.recommended[0].skills.len(), 2);
    }

    #[test]
    fn test_recommended_with_url_priority() {
        let item = RecommendedSource {
            name: "antfu".to_string(),
            url: "https://github.com/antfu/skills".to_string(),
            skills: vec!["vue".to_string()],
        };
        assert_eq!(item.url, "https://github.com/antfu/skills");
    }

    #[test]
    fn test_recommended_with_name_only() {
        let item = RecommendedSource {
            name: "antfu".to_string(),
            url: String::new(),
            skills: vec!["vue".to_string()],
        };
        assert_eq!(item.name, "antfu");
        assert!(item.url.is_empty());
    }
}
