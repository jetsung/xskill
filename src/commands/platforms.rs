use crate::config::Config;
use crate::output::print_table;
use anyhow::Result;
use colored::Colorize;

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
        let rows: Vec<Vec<String>> = sorted.iter().map(|(name, platform)| {
            let skills_dir = platform.skills_dir()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            let agents_file = platform.agents_file()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            let source_file = platform.source_file().to_string_lossy().into_owned();
            let compat = if platform.agents_compat { "✓" } else { "" }.to_string();
            vec![name.to_string(), platform.path.clone(), skills_dir, agents_file, source_file, compat]
        }).collect();
        print_table(headers, &rows);
    } else {
        let headers = &["NAME", "PATH"];
        let rows: Vec<Vec<String>> = sorted.iter().map(|(name, platform)| {
            vec![name.to_string(), platform.path.clone()]
        }).collect();
        print_table(headers, &rows);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::config::Platform;

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
}
