use anyhow::{Context, Result};
use colored::Colorize;
use std::fs;
use std::path::Path;

/// Create a new skill project
pub fn run(name: &str, description: &str, _template: &str) -> Result<()> {
    let project_dir = Path::new(name);

    // Check if directory already exists
    if project_dir.exists() {
        anyhow::bail!("Directory '{}' already exists", name);
    }

    // Create project directory
    fs::create_dir_all(project_dir)
        .with_context(|| format!("Failed to create directory: {}", name))?;

    // Generate SKILL.md
    let skill_md = generate_skill_md(name, description);
    fs::write(project_dir.join("SKILL.md"), skill_md)
        .context("Failed to create SKILL.md")?;

    println!("{}: {}", "Created skill project".green(), name);
    println!("  {}/SKILL.md", name.dimmed());

    Ok(())
}

/// Generate SKILL.md content with YAML frontmatter
fn generate_skill_md(name: &str, description: &str) -> String {
    let desc = if description.is_empty() {
        "TODO: Add description"
    } else {
        description
    };

    format!(
        r#"---
name: {name}
description: {desc}
metadata:
  version: 0.1.0
---

# {name}

{desc}

## Usage

TODO: Describe how to use this skill.

## Examples

TODO: Add examples.
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_create_project() {
        let dir = tempdir().unwrap();
        let project_path = dir.path().join("test-skill");

        // Create project using absolute path
        let name = project_path.to_string_lossy().to_string();
        run(&name, "A test skill", "basic").unwrap();

        // Verify files exist
        assert!(project_path.join("SKILL.md").exists());
    }

    #[test]
    fn test_directory_already_exists() {
        let dir = tempdir().unwrap();
        let project_path = dir.path().join("existing-skill");
        fs::create_dir(&project_path).unwrap();

        let name = project_path.to_string_lossy().to_string();
        let result = run(&name, "", "basic");
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("already exists"));
    }

    #[test]
    fn test_skill_md_content() {
        let content = generate_skill_md("my-skill", "A cool skill");
        assert!(content.contains("name: my-skill"));
        assert!(content.contains("description: A cool skill"));
        assert!(content.contains("version: 0.1.0"));
    }
}
