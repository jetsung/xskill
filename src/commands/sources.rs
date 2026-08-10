use crate::config::{Config, Source, validate_source_name, validate_source_url};
use crate::output::print_table;
use anyhow::Result;
use colored::Colorize;

/// Resolve source index by priority: --name/--url > index > error
fn resolve_source_index(
    sources: &[Source],
    name: Option<&str>,
    url: Option<&str>,
    index: Option<usize>,
) -> Result<usize> {
    // --name/--url 优先级更高
    if name.is_some() || url.is_some() {
        return sources
            .iter()
            .position(|s| {
                let name_match = name.map_or(true, |n| s.effective_name() == n);
                let url_match = url.map_or(true, |u| s.url == u);
                name_match && url_match
            })
            .ok_or_else(|| anyhow::anyhow!("No matching source found."));
    }

    // 其次使用索引（1-based）
    if let Some(idx) = index {
        if idx < 1 || idx > sources.len() {
            anyhow::bail!("Source index {} out of range (1..{}).", idx, sources.len());
        }
        return Ok(idx - 1);
    }

    anyhow::bail!("At least one of --name, --url, or index is required.")
}

/// List all configured sources
pub fn run() -> Result<()> {
    let config = Config::load()?;

    if config.sources.is_empty() {
        println!("{}", "No sources configured".yellow());
        return Ok(());
    }

    let headers = &["#", "NAME", "TYPE", "URL"];
    let rows: Vec<Vec<String>> = config
        .sources
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let display_name = if s.name.is_empty() { "-" } else { &s.name };
            vec![
                (i + 1).to_string(),
                display_name.to_string(),
                s.effective_type(),
                s.url.clone(),
            ]
        })
        .collect();
    print_table(headers, &rows);

    Ok(())
}

/// Normalize URL: strip trailing .git and slashes
fn normalize_url(url: &str) -> String {
    url.trim_end_matches(".git")
        .trim_end_matches('/')
        .to_string()
}

/// Extract path from URL as default source name.
/// e.g. "https://github.com/user/repo.git" -> "user/repo"
///      "https://example.com/group/sub/repo" -> "group/sub/repo"
fn extract_name_from_url(url: &str) -> String {
    url.split("://")
        .nth(1)
        .and_then(|s| s.splitn(2, '/').nth(1))
        .map(|p| p.trim_end_matches(".git").trim_end_matches('/'))
        .filter(|p| !p.is_empty())
        .unwrap_or(url)
        .to_string()
}

/// Add a new source
pub fn run_add(name: Option<&str>, url: &str, source_type: &str) -> Result<()> {
    validate_source_url(url)?;
    let url = normalize_url(url);

    if source_type != "git" && source_type != "api" {
        anyhow::bail!(
            "Invalid source type '{}'. Must be 'git' or 'api'.",
            source_type
        );
    }

    // 未指定 --name 时从 URL 中提取路径作为默认名称
    let default_name;
    let raw_name = match name {
        Some(n) => n,
        None => {
            default_name = extract_name_from_url(&url);
            &default_name
        }
    };
    validate_source_name(raw_name)?;

    let mut config = Config::load()?;

    // URL 冲突检测
    if let Some(existing) = config.sources.iter().find(|s| s.url == url) {
        if existing.name.is_empty() {
            anyhow::bail!("URL '{}' already exists.", url);
        } else {
            anyhow::bail!(
                "URL '{}' already exists in source '{}'.",
                url,
                existing.name
            );
        }
    }

    // name 非空时做唯一性检查
    if !raw_name.is_empty() {
        let conflict = config
            .sources
            .iter()
            .any(|s| !s.name.is_empty() && s.name == raw_name);
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

    let display = if raw_name.is_empty() { &url } else { raw_name };
    config.save()?;
    println!("{} '{}' added successfully.", "Source".green(), display);
    Ok(())
}

/// Remove a source by name, url, or index
pub fn run_remove(name: Option<&str>, url: Option<&str>, index: Option<usize>) -> Result<()> {
    let mut config = Config::load()?;
    let idx = resolve_source_index(&config.sources, name, url, index)?;

    let display = config.sources[idx].effective_name();
    config.sources.remove(idx);
    config.save()?;
    println!("{} '{}' removed successfully.", "Source".green(), display);
    Ok(())
}

/// Rename an existing source (url and type are immutable)
pub fn run_rename(
    name: Option<&str>,
    url: Option<&str>,
    new_name: &str,
    index: Option<usize>,
) -> Result<()> {
    validate_source_name(new_name)?;

    let mut config = Config::load()?;
    let idx = resolve_source_index(&config.sources, name, url, index)?;

    // new_name 非空时做唯一性检查
    if !new_name.is_empty() {
        let conflict = config
            .sources
            .iter()
            .enumerate()
            .any(|(i, s)| i != idx && !s.name.is_empty() && s.name == new_name);
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
        assert!(validate_source_name("user/repo").is_ok());
        assert!(validate_source_name("a/b/c").is_ok());
        assert!(validate_source_name("org/team/repo").is_ok());
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
    fn test_source_name_with_slashes() {
        let mut config = Config::default();
        config.sources.push(Source {
            name: "user/repo".to_string(),
            source_type: "git".to_string(),
            url: "https://github.com/user/repo.git".to_string(),
        });
        config.sources.push(Source {
            name: "a/b/c".to_string(),
            source_type: "api".to_string(),
            url: "https://example.com/a/b/c".to_string(),
        });

        // Lookup by slash name
        assert!(config.get_source("user/repo").is_some());
        assert!(config.get_source("a/b/c").is_some());
        assert_eq!(config.get_source("a/b/c").unwrap().effective_type(), "api");

        // Non-existent
        assert!(config.get_source("x/y").is_none());
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
        let conflict = config
            .sources
            .iter()
            .find(|s| s.url == "https://example.com/repo.git");
        assert!(conflict.is_some());
        assert_eq!(conflict.unwrap().name, "src1");

        // No conflict
        let no_conflict = config
            .sources
            .iter()
            .find(|s| s.url == "https://other.com/repo.git");
        assert!(no_conflict.is_none());
    }

    #[test]
    fn test_extract_name_from_url() {
        assert_eq!(
            extract_name_from_url("https://github.com/user/repo.git"),
            "user/repo"
        );
        assert_eq!(
            extract_name_from_url("https://github.com/user/repo"),
            "user/repo"
        );
        assert_eq!(
            extract_name_from_url("https://example.com/group/sub/repo.git"),
            "group/sub/repo"
        );
        assert_eq!(extract_name_from_url("https://example.com/a/b/c/"), "a/b/c");
        assert_eq!(
            extract_name_from_url("https://example.com/org/team/project.git"),
            "org/team/project"
        );
        // Path-only URL: no domain separator after scheme
        assert_eq!(
            extract_name_from_url("https://example.com"),
            "https://example.com"
        );
    }
}
