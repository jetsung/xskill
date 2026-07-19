use crate::config::{validate_source_name, validate_source_url, Config, Source};
use crate::output::print_table;
use anyhow::Result;
use colored::Colorize;

/// List all configured sources
pub fn run() -> Result<()> {
    let config = Config::load()?;

    if config.sources.is_empty() {
        println!("{}", "No sources configured".yellow());
        return Ok(());
    }

    let headers = &["NAME", "TYPE", "URL"];
    let rows: Vec<Vec<String>> = config.sources.iter().map(|s| {
        let display_name = if s.name.is_empty() { "-" } else { &s.name };
        vec![display_name.to_string(), s.effective_type(), s.url.clone()]
    }).collect();
    print_table(headers, &rows);

    Ok(())
}

/// Add a new source
pub fn run_add(name: Option<&str>, url: &str, source_type: &str) -> Result<()> {
    validate_source_url(url)?;

    if source_type != "git" && source_type != "api" {
        anyhow::bail!("Invalid source type '{}'. Must be 'git' or 'api'.", source_type);
    }

    let raw_name = name.unwrap_or("");
    validate_source_name(raw_name)?;

    let mut config = Config::load()?;

    // URL 冲突检测
    if let Some(existing) = config.sources.iter().find(|s| s.url == url) {
        if existing.name.is_empty() {
            anyhow::bail!("URL '{}' already exists.", url);
        } else {
            anyhow::bail!("URL '{}' already exists in source '{}'.", url, existing.name);
        }
    }

    // name 非空时做唯一性检查
    if !raw_name.is_empty() {
        let conflict = config.sources.iter().any(|s| !s.name.is_empty() && s.name == raw_name);
        if conflict {
            let existing: Vec<String> = config.sources.iter().map(|s| s.effective_name()).collect();
            anyhow::bail!(
                "Source name '{}' already exists. Existing sources: {}",
                raw_name,
                existing.join(", ")
            );
        }
    }

    config.sources.push(Source {
        name: raw_name.to_string(),
        source_type: source_type.to_string(),
        url: url.to_string(),
    });

    let display = if raw_name.is_empty() { url } else { raw_name };
    config.save()?;
    println!("{} '{}' added successfully.", "Source".green(), display);
    Ok(())
}

/// Remove a source by name and/or url
pub fn run_remove(name: Option<&str>, url: Option<&str>) -> Result<()> {
    let mut config = Config::load()?;

    // 至少指定 --name 或 --url 之一
    if name.is_none() && url.is_none() {
        anyhow::bail!("At least one of --name or --url is required.");
    }

    // 查找匹配的源
    let idx = config.sources.iter().position(|s| {
        let name_match = name.map_or(true, |n| s.effective_name() == n);
        let url_match = url.map_or(true, |u| s.url == u);
        name_match && url_match
    }).ok_or_else(|| anyhow::anyhow!("No matching source found."))?;

    let display = config.sources[idx].effective_name();
    config.sources.remove(idx);
    config.save()?;
    println!("{} '{}' removed successfully.", "Source".green(), display);
    Ok(())
}

/// Edit an existing source (only name can be changed; url and type are immutable)
pub fn run_edit(
    name: Option<&str>,
    url: Option<&str>,
    new_name: &str,
) -> Result<()> {
    validate_source_name(new_name)?;

    // 至少指定 --name 或 --url 之一
    if name.is_none() && url.is_none() {
        anyhow::bail!("At least one of --name or --url is required.");
    }

    let mut config = Config::load()?;

    // 查找匹配的源
    let idx = config.sources.iter().position(|s| {
        let name_match = name.map_or(true, |n| s.effective_name() == n);
        let url_match = url.map_or(true, |u| s.url == u);
        name_match && url_match
    }).ok_or_else(|| anyhow::anyhow!("No matching source found."))?;

    // new_name 非空时做唯一性检查
    if !new_name.is_empty() {
        let conflict = config.sources.iter().enumerate().any(|(i, s)| {
            i != idx && !s.name.is_empty() && s.name == new_name
        });
        if conflict {
            anyhow::bail!("Source name '{}' already exists.", new_name);
        }
    }

    let display = config.sources[idx].effective_name();
    config.sources[idx].name = new_name.to_string();

    config.save()?;
    println!("{} '{}' updated successfully.", "Source".green(), display);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{validate_source_name, validate_source_url};

    #[test]
    fn test_validate_source_name_valid() {
        assert!(validate_source_name("antfu").is_ok());
        assert!(validate_source_name("my-repo").is_ok());
        assert!(validate_source_name("repo_123").is_ok());
    }

    #[test]
    fn test_validate_source_name_invalid() {
        assert!(validate_source_name("my repo").is_err());
        assert!(validate_source_name("repo@name").is_err());
    }

    #[test]
    fn test_validate_source_url_valid() {
        assert!(validate_source_url("https://github.com/example/skills.git").is_ok());
        assert!(validate_source_url("http://example.com/skills").is_ok());
    }

    #[test]
    fn test_validate_source_url_invalid() {
        assert!(validate_source_url("ftp://example.com").is_err());
        assert!(validate_source_url("github.com/example").is_err());
        assert!(validate_source_url("").is_err());
    }

    #[test]
    fn test_source_conflict_detection() {
        let mut config = Config::default();
        config.sources.push(Source {
            name: "existing".to_string(),
            source_type: "git".to_string(),
            url: "https://example.com/repo.git".to_string(),
        });

        // Name conflict
        assert!(config.get_source("existing").is_some());
        assert!(config.get_source("nonexistent").is_none());
    }

    #[test]
    fn test_source_url_conflict_detection() {
        let mut config = Config::default();
        config.sources.push(Source {
            name: "src1".to_string(),
            source_type: "git".to_string(),
            url: "https://example.com/repo.git".to_string(),
        });

        // URL conflict
        let conflict = config.sources.iter().find(|s| s.url == "https://example.com/repo.git");
        assert!(conflict.is_some());
        assert_eq!(conflict.unwrap().name, "src1");

        // No conflict
        let no_conflict = config.sources.iter().find(|s| s.url == "https://other.com/repo.git");
        assert!(no_conflict.is_none());
    }
}
