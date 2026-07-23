use crate::config::Config;
use crate::utils::{canonical_skills_dir, create_relative_symlink, display_path, validate_agent};
use anyhow::{bail, Result};
use colored::Colorize;
use std::fs;
use std::path::{Path, PathBuf};

pub fn run(skill: &str, agent: &str, global: bool) -> Result<()> {
    let config = Config::load()?;

    // 校验平台名
    validate_agent(&config, agent)?;

    let canonical_dir = canonical_skills_dir(global);

    if !canonical_dir.exists() {
        bail!(
            "No skills found in canonical directory: {}",
            display_path(&canonical_dir)
        );
    }

    if skill == "*" {
        // 链接所有 skill
        let skills = scan_canonical_skills(&canonical_dir)?;
        if skills.is_empty() {
            println!("{}", "No skills found in canonical directory".yellow());
            return Ok(());
        }

        for name in &skills {
            if agent == "*" {
                symlink_skill_to_all_platforms(&config, name, global)?;
            } else {
                symlink_skill_to_platform(&config, name, agent, global)?;
            }
        }
    } else {
        // 链接指定 skill
        let skill_dir = canonical_dir.join(skill);
        if !skill_dir.exists() || !skill_dir.join("SKILL.md").exists() {
            bail!(
                "Skill '{}' not found in canonical directory: {}",
                skill,
                display_path(&skill_dir)
            );
        }

        if agent == "*" {
            symlink_skill_to_all_platforms(&config, skill, global)?;
        } else {
            symlink_skill_to_platform(&config, skill, agent, global)?;
        }
    }

    Ok(())
}

/// 扫描规范目录中所有有效 skill
fn scan_canonical_skills(canonical_dir: &Path) -> Result<Vec<String>> {
    let mut names = Vec::new();

    if !canonical_dir.exists() {
        return Ok(names);
    }

    for entry in fs::read_dir(canonical_dir)? {
        let entry = entry?;
        let path = entry.path();

        // 检查是否是目录或 symlink
        let is_valid = entry
            .file_type()
            .map(|t| t.is_dir() || t.is_symlink())
            .unwrap_or(false);
        if !is_valid {
            continue;
        }

        // 跳过断裂的 symlink
        if path.is_symlink() && !path.exists() {
            continue;
        }

        // 检查是否有 SKILL.md
        if !path.join("SKILL.md").exists() {
            continue;
        }

        names.push(entry.file_name().to_string_lossy().to_string());
    }

    Ok(names)
}

/// 创建 skill 到指定平台的 symlink
fn symlink_skill_to_platform(
    config: &Config,
    skill_name: &str,
    platform_name: &str,
    global: bool,
) -> Result<()> {
    let platform = match config.get_platform(platform_name) {
        Some(p) => p,
        None => bail!("Platform '{}' not found in config", platform_name),
    };

    if platform.agents_compat {
        println!(
            "{}: {} ({})",
            "Skipped".dimmed(),
            platform_name,
            "agents_compat"
        );
        return Ok(());
    }

    if platform.skills.is_empty() {
        bail!(
            "Platform {} has no skills directory configured",
            platform_name
        );
    }

    let base_dir = if global {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"))
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    let platform_path = base_dir.join(&platform.path);
    if !platform_path.exists() {
        fs::create_dir_all(&platform_path)?;
    }

    let canonical_skill_dir = canonical_skills_dir(global).join(skill_name);
    let link_path = platform_path.join(&platform.skills).join(skill_name);

    match create_relative_symlink(&canonical_skill_dir, &link_path) {
        Ok(true) => {
            println!(
                "{}: {} → {}",
                "Symlinked".green(),
                skill_name.yellow(),
                display_path(&link_path)
            );
        }
        Ok(false) => {
            // 回退到文件复制
            fs::create_dir_all(&link_path)?;
            copy_dir_recursive(&canonical_skill_dir, &link_path)?;
            println!(
                "{}: {} → {} (copy fallback)",
                "Installed".green(),
                skill_name.yellow(),
                display_path(&link_path)
            );
        }
        Err(e) => {
            // 回退到文件复制
            fs::create_dir_all(&link_path)?;
            copy_dir_recursive(&canonical_skill_dir, &link_path)?;
            println!(
                "{}: {} → {} (copy fallback: {})",
                "Installed".green(),
                skill_name.yellow(),
                display_path(&link_path),
                e
            );
        }
    }

    Ok(())
}

/// 创建 skill 到所有已存在平台的 symlink
fn symlink_skill_to_all_platforms(
    config: &Config,
    skill_name: &str,
    global: bool,
) -> Result<()> {
    let base_dir = if global {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"))
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    let canonical_skill_dir = canonical_skills_dir(global).join(skill_name);
    let mut linked_platforms = Vec::new();
    let mut skipped_platforms = Vec::new();

    for name in config.platform_names() {
        let platform = match config.platforms.get(name) {
            Some(p) => p,
            None => continue,
        };

        // 跳过 agents_compat 平台
        if platform.agents_compat {
            skipped_platforms.push(name.to_string());
            continue;
        }

        if platform.skills.is_empty() {
            continue;
        }

        // 仅链接已存在的平台目录
        let platform_path = base_dir.join(&platform.path);
        if !platform_path.exists() {
            continue;
        }

        let skills_dir = platform_path.join(&platform.skills);
        let link_path = skills_dir.join(skill_name);

        match create_relative_symlink(&canonical_skill_dir, &link_path) {
            Ok(true) => {
                linked_platforms.push(name.to_string());
            }
            Ok(false) => {
                // 回退到文件复制
                fs::create_dir_all(&link_path)?;
                copy_dir_recursive(&canonical_skill_dir, &link_path)?;
                linked_platforms.push(name.to_string());
            }
            Err(e) => {
                eprintln!(
                    "{}: Failed to link to {}: {}",
                    "Warning".yellow(),
                    name,
                    e
                );
            }
        }
    }

    if linked_platforms.is_empty() {
        println!(
            "{}: {} — no available platform directories found to link",
            skill_name.yellow(),
            "Skipped".dimmed()
        );
    } else {
        println!(
            "{}: {} → {}",
            "Symlinked".green(),
            skill_name.yellow(),
            linked_platforms.join(", ")
        );
    }

    if !skipped_platforms.is_empty() {
        println!(
            "{}: {} (agents_compat)",
            "Skipped".dimmed(),
            skipped_platforms.join(", ")
        );
    }

    Ok(())
}

/// 递归复制目录
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !src.is_dir() {
        return Ok(());
    }

    fs::create_dir_all(dst)?;

    for entry in fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Platform};

    #[test]
    fn test_scan_canonical_skills_finds_skills() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();

        let s1 = dir.join("vue");
        let s2 = dir.join("react");
        fs::create_dir_all(&s1).unwrap();
        fs::create_dir_all(&s2).unwrap();
        fs::write(s1.join("SKILL.md"), "---\nname: vue\n---\n").unwrap();
        fs::write(s2.join("SKILL.md"), "---\nname: react\n---\n").unwrap();

        let names = scan_canonical_skills(dir).unwrap();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"vue".to_string()));
        assert!(names.contains(&"react".to_string()));
    }

    #[test]
    fn test_scan_canonical_skills_skips_non_skill_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path();

        let s1 = dir.join("real-skill");
        let s2 = dir.join("not-a-skill");
        fs::create_dir_all(&s1).unwrap();
        fs::create_dir_all(&s2).unwrap();
        fs::write(s1.join("SKILL.md"), "test").unwrap();
        fs::write(s2.join("README.md"), "readme").unwrap();

        let names = scan_canonical_skills(dir).unwrap();
        assert_eq!(names.len(), 1);
        assert_eq!(names[0], "real-skill");
    }

    #[test]
    fn test_scan_canonical_skills_skips_broken_symlinks() {
        let tmp = tempfile::tempdir().unwrap();

        let target = tmp.path().join("target").join("broken-skill");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("SKILL.md"), "test").unwrap();

        let dir = tmp.path().join("canonical");
        let link = dir.join("broken-skill");
        fs::create_dir_all(&dir).unwrap();
        crate::utils::create_relative_symlink(&target, &link).unwrap();

        // 删除目标使链接断裂
        fs::remove_dir_all(target.parent().unwrap()).unwrap();

        let names = scan_canonical_skills(&dir).unwrap();
        assert!(names.is_empty(), "should skip broken symlinks");
    }

    #[test]
    fn test_scan_canonical_skills_nonexistent() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("nonexistent");
        let names = scan_canonical_skills(&dir).unwrap();
        assert!(names.is_empty());
    }

    #[test]
    fn test_scan_canonical_skills_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("empty");
        fs::create_dir_all(&dir).unwrap();

        let names = scan_canonical_skills(&dir).unwrap();
        assert!(names.is_empty());
    }

    #[test]
    fn test_symlink_skill_to_platform_creates_link() {
        let tmp = tempfile::tempdir().unwrap();

        // 创建规范目录中的 skill
        let canonical = tmp.path().join(".agents").join("skills").join("test-skill");
        fs::create_dir_all(&canonical).unwrap();
        fs::write(canonical.join("SKILL.md"), "---\nname: test\n---\n").unwrap();

        // 创建平台目录
        let platform_skills = tmp.path().join(".claude").join("skills");
        fs::create_dir_all(&platform_skills).unwrap();

        // 创建配置
        let mut config = Config::default();
        config.platforms.insert(
            "claude".to_string(),
            Platform {
                path: tmp
                    .path()
                    .join(".claude")
                    .to_string_lossy()
                    .to_string(),
                skills: "skills".to_string(),
                agents: "CLAUDE.md".to_string(),
                source: "AGENTS.md".to_string(),
                agents_compat: false,
            },
        );

        // 执行链接（使用全局模式，symlink 指向 ~/.agents/skills/ 即真实 home 目录）
        symlink_skill_to_platform(&config, "test-skill", "claude", true).unwrap();

        let link = platform_skills.join("test-skill");
        // is_symlink 不跟随链接，仅检查是否为符号链接
        assert!(link.is_symlink());
    }

    #[test]
    fn test_symlink_skill_to_platform_agents_compat_skipped() {
        let tmp = tempfile::tempdir().unwrap();

        let canonical = tmp.path().join(".agents").join("skills").join("test-skill");
        fs::create_dir_all(&canonical).unwrap();
        fs::write(canonical.join("SKILL.md"), "test").unwrap();

        let mut config = Config::default();
        config.platforms.insert(
            "zcode".to_string(),
            Platform {
                path: tmp.path().join(".zcode").to_string_lossy().to_string(),
                skills: "skills".to_string(),
                agents: "AGENTS.md".to_string(),
                source: "AGENTS.md".to_string(),
                agents_compat: true,
            },
        );

        // 应该跳过，不报错
        symlink_skill_to_platform(&config, "test-skill", "zcode", true).unwrap();

        // 平台目录不应被创建
        assert!(!tmp.path().join(".zcode").exists());
    }

    #[test]
    fn test_symlink_skill_to_all_platforms() {
        let tmp = tempfile::tempdir().unwrap();

        let canonical = tmp.path().join(".agents").join("skills").join("test-skill");
        fs::create_dir_all(&canonical).unwrap();
        fs::write(canonical.join("SKILL.md"), "test").unwrap();

        // 创建两个平台目录
        let claude_skills = tmp.path().join(".claude").join("skills");
        let codebuddy_skills = tmp.path().join(".codebuddy").join("skills");
        fs::create_dir_all(&claude_skills).unwrap();
        fs::create_dir_all(&codebuddy_skills).unwrap();

        let mut config = Config::default();
        config.platforms.insert(
            "claude".to_string(),
            Platform {
                path: tmp
                    .path()
                    .join(".claude")
                    .to_string_lossy()
                    .to_string(),
                skills: "skills".to_string(),
                agents: "CLAUDE.md".to_string(),
                source: "AGENTS.md".to_string(),
                agents_compat: false,
            },
        );
        config.platforms.insert(
            "codebuddy".to_string(),
            Platform {
                path: tmp
                    .path()
                    .join(".codebuddy")
                    .to_string_lossy()
                    .to_string(),
                skills: "skills".to_string(),
                agents: "CODEBUDDY.md".to_string(),
                source: "AGENTS.md".to_string(),
                agents_compat: false,
            },
        );
        // agents_compat 平台，应被跳过
        config.platforms.insert(
            "zcode".to_string(),
            Platform {
                path: tmp.path().join(".zcode").to_string_lossy().to_string(),
                skills: "skills".to_string(),
                agents: "AGENTS.md".to_string(),
                source: "AGENTS.md".to_string(),
                agents_compat: true,
            },
        );

        symlink_skill_to_all_platforms(&config, "test-skill", true).unwrap();

        // claude 和 codebuddy 应有 symlink
        assert!(claude_skills.join("test-skill").is_symlink());
        assert!(codebuddy_skills.join("test-skill").is_symlink());

        // zcode 不应有目录
        assert!(!tmp.path().join(".zcode").exists());
    }

    #[test]
    fn test_symlink_skill_to_platform_auto_creates_dir() {
        let tmp = tempfile::tempdir().unwrap();

        let canonical = tmp.path().join(".agents").join("skills").join("test-skill");
        fs::create_dir_all(&canonical).unwrap();
        fs::write(canonical.join("SKILL.md"), "test").unwrap();

        // 平台目录不存在
        let platform_skills = tmp.path().join(".newplatform").join("skills");
        assert!(!platform_skills.exists());

        let mut config = Config::default();
        config.platforms.insert(
            "newplatform".to_string(),
            Platform {
                path: tmp
                    .path()
                    .join(".newplatform")
                    .to_string_lossy()
                    .to_string(),
                skills: "skills".to_string(),
                agents: "AGENTS.md".to_string(),
                source: "AGENTS.md".to_string(),
                agents_compat: false,
            },
        );

        symlink_skill_to_platform(&config, "test-skill", "newplatform", true).unwrap();

        // 平台目录应被自动创建
        assert!(platform_skills.exists());
        assert!(platform_skills.join("test-skill").is_symlink());
    }

    #[test]
    fn test_copy_dir_recursive_basic() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        fs::create_dir_all(src.join("sub")).unwrap();
        fs::write(src.join("a.txt"), "hello").unwrap();
        fs::write(src.join("sub").join("b.txt"), "world").unwrap();

        copy_dir_recursive(&src, &dst).unwrap();
        assert_eq!(fs::read_to_string(dst.join("a.txt")).unwrap(), "hello");
        assert_eq!(
            fs::read_to_string(dst.join("sub").join("b.txt")).unwrap(),
            "world"
        );
    }

    #[test]
    fn test_copy_dir_recursive_nonexistent_src() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("nonexistent");
        let dst = tmp.path().join("dst");
        copy_dir_recursive(&src, &dst).unwrap();
        assert!(!dst.exists());
    }
}
