use anyhow::{Context, Result};
use regex::Regex;
use serde::Deserialize;
use std::fs;
use std::path::Path;

/// SKILL.md 元信息
#[derive(Debug, Deserialize, Default, Clone)]
pub struct SkillMeta {
    pub name: Option<String>,
    pub description: Option<String>,
    pub metadata: Option<Metadata>,
}

#[derive(Debug, Deserialize, Default, Clone)]
pub struct Metadata {
    pub version: Option<String>,
}

impl SkillMeta {
    /// 从 SKILL.md 文件解析元信息
    pub fn from_file(skill_dir: &Path) -> Result<Self> {
        let file_path = skill_dir.join("SKILL.md");
        if !file_path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&file_path)
            .with_context(|| format!("Failed to read SKILL.md: {}", file_path.display()))?;
        Self::parse(&content)
    }

    /// 从文件内容解析 YAML frontmatter
    pub fn parse(content: &str) -> Result<Self> {
        let re = Regex::new(r"(?s)\A---\s*\n(.*?)\n---")?;
        let meta = match re.captures(content) {
            Some(caps) => {
                let yaml_str = caps.get(1).unwrap().as_str();
                serde_yaml::from_str(yaml_str).unwrap_or_default()
            }
            None => Self::default(),
        };
        Ok(meta)
    }

    /// 获取显示名
    pub fn display_name(&self, dir_name: &str) -> String {
        self.name.clone().unwrap_or_else(|| dir_name.to_string())
    }

    /// 获取描述
    pub fn display_description(&self) -> String {
        self.description.clone().unwrap_or_else(|| "N/A".to_string())
    }

    /// 获取版本（不存在时返回空字符串）
    pub fn display_version(&self) -> String {
        self.metadata
            .as_ref()
            .and_then(|m| m.version.clone())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_frontmatter() {
        let content = r#"---
name: my-skill
description: A useful skill
metadata:
  version: 1.0.0
---

# my-skill

Some content here.
"#;
        let meta = SkillMeta::parse(content).unwrap();
        assert_eq!(meta.name, Some("my-skill".to_string()));
        assert_eq!(meta.description, Some("A useful skill".to_string()));
        assert_eq!(meta.metadata.as_ref().unwrap().version, Some("1.0.0".to_string()));
    }

    #[test]
    fn test_parse_no_frontmatter() {
        let content = "# Just a markdown file\n\nNo frontmatter here.";
        let meta = SkillMeta::parse(content).unwrap();
        assert_eq!(meta.name, None);
        assert_eq!(meta.description, None);
    }

    #[test]
    fn test_parse_partial_frontmatter() {
        let content = r#"---
name: partial-skill
---

# partial
"#;
        let meta = SkillMeta::parse(content).unwrap();
        assert_eq!(meta.name, Some("partial-skill".to_string()));
        assert_eq!(meta.description, None);
    }

    #[test]
    fn test_display_name_fallback() {
        let meta = SkillMeta::default();
        assert_eq!(meta.display_name("dir-name"), "dir-name");

        let meta = SkillMeta {
            name: Some("custom-name".to_string()),
            ..Default::default()
        };
        assert_eq!(meta.display_name("dir-name"), "custom-name");
    }

    #[test]
    fn test_display_description_fallback() {
        let meta = SkillMeta::default();
        assert_eq!(meta.display_description(), "N/A");

        let meta = SkillMeta {
            description: Some("A cool skill".to_string()),
            ..Default::default()
        };
        assert_eq!(meta.display_description(), "A cool skill");
    }

    #[test]
    fn test_display_version_fallback() {
        let meta = SkillMeta::default();
        assert_eq!(meta.display_version(), "");

        let meta = SkillMeta {
            metadata: Some(Metadata { version: Some("2.0.0".to_string()) }),
            ..Default::default()
        };
        assert_eq!(meta.display_version(), "2.0.0");
    }
}
