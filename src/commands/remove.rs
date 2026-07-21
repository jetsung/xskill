use crate::config::Config;
use crate::lock::LockFile;
use crate::utils::{canonical_skills_dir, display_path, remove_symlink, validate_agent};
use anyhow::{Context, Result};
use chrono::Utc;
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

/// Remove target
enum RemoveTarget {
    /// 仅移除规范目录
    Canonical,
    /// 移除指定平台的 symlink（保留规范目录）
    Platform(String),
    /// 移除所有平台的 symlink + 规范目录
    AllPlatforms,
}

pub fn run(skill: &str, global: bool, agent: Option<&str>) -> Result<()> {
    let config = Config::load()?;

    // 解析移除目标
    let target = resolve_remove_target(agent);

    // Handle -s '*' (remove all skills)
    if skill == "*" {
        return remove_all_skills(&config, &target, global);
    }

    // Remove single skill
    let removed = match &target {
        RemoveTarget::Canonical => {
            // 移除各平台的 symlink（悬空文件夹）
            let n = remove_platform_symlinks(&config, skill, global)?;
            // 移除规范目录
            let canonical_dir = canonical_skills_dir(global).join(skill);
            let dir_removed = remove_from_target(skill, &canonical_dir)?;
            n + dir_removed
        }
        RemoveTarget::Platform(platform_name) => {
            remove_from_platform(&config, skill, platform_name, global)?
        }
        RemoveTarget::AllPlatforms => {
            // 移除各平台 symlink
            let n = remove_from_all_platforms(&config, skill, global)?;
            // 移除规范目录
            let canonical_dir = canonical_skills_dir(global).join(skill);
            let dir_removed = remove_from_target(skill, &canonical_dir)?;
            n + dir_removed
        }
    };

    if removed == 0 {
        println!("{}: skill '{}' not installed", "Nothing to remove".yellow(), skill);
    }

    // Update lock file
    let mut lock_file = LockFile::load(global)?;
    lock_file.remove_skill(skill);
    lock_file.updated_at = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    lock_file.save(global)?;

    Ok(())
}

/// 解析移除目标
fn resolve_remove_target(agent: Option<&str>) -> RemoveTarget {
    match agent {
        Some("*") => RemoveTarget::AllPlatforms,
        Some(name) => RemoveTarget::Platform(name.to_string()),
        None => RemoveTarget::Canonical,
    }
}

/// Remove all skills
fn remove_all_skills(
    config: &Config,
    target: &RemoveTarget,
    global: bool,
) -> Result<()> {
    let removed = match target {
        RemoveTarget::Canonical => {
            // 移除各平台的 symlink（悬空文件夹）
            let n1 = remove_all_platform_symlinks(config, global)?;
            // 移除规范目录
            let dir = canonical_skills_dir(global);
            let n2 = remove_all_from_dir(&dir)?;
            n1 + n2
        }
        RemoveTarget::Platform(platform_name) => {
            remove_all_from_platform(config, platform_name, global)?
        }
        RemoveTarget::AllPlatforms => {
            // 移除所有平台 symlink
            let n1 = remove_all_from_all_platforms(config, global)?;
            // 移除规范目录
            let dir = canonical_skills_dir(global);
            let n2 = remove_all_from_dir(&dir)?;
            n1 + n2
        }
    };

    if removed == 0 {
        println!("{}: no skills installed", "Nothing to remove".yellow());
    }

    // Clear lock file
    let mut lock_file = LockFile::load(global)?;
    lock_file.clear_skills();
    lock_file.updated_at = Utc::now().format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();
    lock_file.save(global)?;

    Ok(())
}

/// Remove all skills from directory
fn remove_all_from_dir(dir: &PathBuf) -> Result<usize> {
    if !dir.exists() {
        return Ok(0);
    }

    let entries: Vec<_> = fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            let is_dir = e.file_type().map(|t| t.is_dir() || t.is_symlink()).unwrap_or(false);
            is_dir && e.path().join("SKILL.md").exists()
        })
        .collect();

    let count = entries.len();
    for entry in entries {
        let name = entry.file_name().to_string_lossy().to_string();
        let path = entry.path();
        remove_symlink(&path)
            .with_context(|| format!("Failed to remove {}/{}", dir.display(), name))?;
        println!("{}: {}", "Removed".green(), display_path(&path));
    }

    Ok(count)
}

/// Remove skill from platform symlink
fn remove_from_platform(
    config: &Config,
    skill: &str,
    platform_name: &str,
    global: bool,
) -> Result<usize> {
    validate_agent(config, platform_name)?;
    let platform = config.get_platform(platform_name).unwrap();

    if platform.agents_compat {
        println!("{}: {} ({})", "Skipped".dimmed(), platform_name, "agents_compat");
        return Ok(0);
    }

    let base_dir = if global {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"))
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };
    let skills_dir = platform.skills_dir_with_base(&base_dir)
        .ok_or_else(|| anyhow::anyhow!("Platform {} has no skills directory configured", platform_name))?;

    let link_path = skills_dir.join(skill);

    // 处理 symlink 或目录
    if link_path.is_symlink() || link_path.exists() {
        remove_symlink(&link_path)?;
        println!("{}: {}", "Removed".green(), display_path(&link_path));
        Ok(1)
    } else {
        Ok(0)
    }
}

/// Remove skill from all existing platforms
fn remove_from_all_platforms(
    config: &Config,
    skill: &str,
    global: bool,
) -> Result<usize> {
    let base_dir = if global {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"))
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    let mut count = 0;
    for name in config.platform_names() {
        let platform = match config.platforms.get(name) {
            Some(p) => p,
            None => continue,
        };
        if platform.agents_compat {
            continue;
        }
        let platform_path = base_dir.join(&platform.path);
        if !platform_path.exists() || platform.skills.is_empty() {
            continue;
        }

        let skills_dir = platform_path.join(&platform.skills);
        let link_path = skills_dir.join(skill);

        if link_path.is_symlink() || link_path.exists() {
            remove_symlink(&link_path)?;
            println!("{}: {}", "Removed".green(), display_path(&link_path));
            count += 1;
        }
    }

    Ok(count)
}

/// Remove all skills from platform
fn remove_all_from_platform(
    config: &Config,
    platform_name: &str,
    global: bool,
) -> Result<usize> {
    validate_agent(config, platform_name)?;
    let platform = config.get_platform(platform_name).unwrap();

    if platform.agents_compat {
        println!("{}: {} ({})", "Skipped".dimmed(), platform_name, "agents_compat");
        return Ok(0);
    }

    let base_dir = if global {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"))
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };
    if let Some(skills_dir) = platform.skills_dir_with_base(&base_dir) {
        if skills_dir.exists() || skills_dir.is_symlink() {
            return remove_all_from_dir(&skills_dir);
        }
    }

    Ok(0)
}

/// Remove skill symlinks from all platform directories (不移除规范目录)
fn remove_platform_symlinks(config: &Config, skill: &str, global: bool) -> Result<usize> {
    let base_dir = if global {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"))
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    let mut count = 0;
    for name in config.platform_names() {
        let platform = match config.platforms.get(name) {
            Some(p) => p,
            None => continue,
        };
        if platform.agents_compat {
            continue;
        }
        let platform_path = base_dir.join(&platform.path);
        if !platform_path.exists() || platform.skills.is_empty() {
            continue;
        }

        let skills_dir = platform_path.join(&platform.skills);
        let link_path = skills_dir.join(skill);

        if link_path.is_symlink() {
            remove_symlink(&link_path)?;
            println!("{}: {}", "Removed".green(), display_path(&link_path));
            count += 1;
        }
    }

    Ok(count)
}

/// Remove skill from target directory (handles both symlink and directory)
fn remove_from_target(_skill: &str, target_dir: &PathBuf) -> Result<usize> {
    if target_dir.is_symlink() || target_dir.exists() {
        remove_symlink(target_dir)?;
        println!("{}: {}", "Removed".green(), display_path(target_dir));
        Ok(1)
    } else {
        Ok(0)
    }
}

/// Remove all skill symlinks from all platform directories (不移除规范目录)
fn remove_all_platform_symlinks(config: &Config, global: bool) -> Result<usize> {
    let base_dir = if global {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"))
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    let mut count = 0;
    for name in config.platform_names() {
        let platform = match config.platforms.get(name) {
            Some(p) => p,
            None => continue,
        };
        if platform.agents_compat {
            continue;
        }
        let platform_path = base_dir.join(&platform.path);
        if !platform_path.exists() || platform.skills.is_empty() {
            continue;
        }

        let skills_dir = platform_path.join(&platform.skills);
        if !skills_dir.exists() && !skills_dir.is_symlink() {
            continue;
        }

        let entries: Vec<_> = fs::read_dir(&skills_dir)?
            .filter_map(|e| e.ok())
            .filter(|e| e.file_type().map(|t| t.is_symlink()).unwrap_or(false))
            .collect();

        for entry in entries {
            let name = entry.file_name().to_string_lossy().to_string();
            let link_path = entry.path();
            remove_symlink(&link_path)
                .with_context(|| format!("Failed to remove {}/{}", skills_dir.display(), name))?;
            println!("{}: {}", "Removed".green(), display_path(&link_path));
            count += 1;
        }
    }

    Ok(count)
}

/// Remove all skills from all platforms
fn remove_all_from_all_platforms(config: &Config, global: bool) -> Result<usize> {
    let base_dir = if global {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"))
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    let mut count = 0;
    for name in config.platform_names() {
        let platform = match config.platforms.get(name) {
            Some(p) => p,
            None => continue,
        };
        if platform.agents_compat {
            continue;
        }
        let platform_path = base_dir.join(&platform.path);
        if !platform_path.exists() || platform.skills.is_empty() {
            continue;
        }

        let skills_dir = platform_path.join(&platform.skills);
        if skills_dir.exists() || skills_dir.is_symlink() {
            count += remove_all_from_dir(&skills_dir)?;
        }
    }

    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::utils;

    #[test]
    fn test_resolve_remove_target_default() {
        let target = resolve_remove_target(None);
        assert!(matches!(target, RemoveTarget::Canonical));
    }

    #[test]
    fn test_resolve_remove_target_platform() {
        let target = resolve_remove_target(Some("claude"));
        assert!(matches!(&target, RemoveTarget::Platform(name) if name == "claude"));
    }

    #[test]
    fn test_resolve_remove_target_all_platforms() {
        let target = resolve_remove_target(Some("*"));
        assert!(matches!(target, RemoveTarget::AllPlatforms));
    }

    #[test]
    fn test_platform_not_found() {
        let config = Config::default();
        let result = config.get_platform("nonexistent");
        assert!(result.is_none());
    }

    // --- remove_from_target ---

    #[test]
    fn test_remove_from_target_removes_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("skill");
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("SKILL.md"), "test").unwrap();
        assert!(dir.exists());

        remove_from_target("skill", &dir).unwrap();
        assert!(!dir.exists());
    }

    #[test]
    fn test_remove_from_target_removes_symlink() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("canonical").join("skill");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("SKILL.md"), "test").unwrap();

        let link = tmp.path().join("platform").join("skill");
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        utils::create_relative_symlink(&target, &link).unwrap();
        assert!(link.is_symlink());

        remove_from_target("skill", &link).unwrap();
        assert!(!link.exists());
        // 规范目录不受影响
        assert!(target.exists());
    }

    #[test]
    fn test_remove_from_target_dangling_symlink() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("canonical").join("skill");
        fs::create_dir_all(&target).unwrap();

        let link = tmp.path().join("platform").join("skill");
        fs::create_dir_all(link.parent().unwrap()).unwrap();
        utils::create_relative_symlink(&target, &link).unwrap();

        // 删除目标使链接断裂
        fs::remove_dir_all(&target).unwrap();
        assert!(link.is_symlink());
        assert!(!link.exists());

        // 应该能清理断裂链接
        remove_from_target("skill", &link).unwrap();
        assert!(!link.is_symlink());
    }

    #[test]
    fn test_remove_from_target_not_found_is_noop() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("nonexistent");
        // 不应报错
        remove_from_target("skill", &dir).unwrap();
    }

    // --- remove_all_from_dir ---

    #[test]
    fn test_remove_all_from_dir_removes_skills() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("skills");

        let s1 = dir.join("skill-1");
        let s2 = dir.join("skill-2");
        fs::create_dir_all(&s1).unwrap();
        fs::create_dir_all(&s2).unwrap();
        fs::write(s1.join("SKILL.md"), "test1").unwrap();
        fs::write(s2.join("SKILL.md"), "test2").unwrap();

        remove_all_from_dir(&dir).unwrap();
        assert!(!s1.exists());
        assert!(!s2.exists());
    }

    #[test]
    fn test_remove_all_from_dir_skips_non_skill_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("skills");

        let s1 = dir.join("real-skill");
        let s2 = dir.join("not-a-skill");
        fs::create_dir_all(&s1).unwrap();
        fs::create_dir_all(&s2).unwrap();
        fs::write(s1.join("SKILL.md"), "test").unwrap();
        fs::write(s2.join("README.md"), "readme").unwrap();

        remove_all_from_dir(&dir).unwrap();
        assert!(!s1.exists());
        assert!(s2.exists()); // 不是 skill，不应被删除
    }

    #[test]
    fn test_remove_all_from_dir_removes_symlinked_skills() {
        let tmp = tempfile::tempdir().unwrap();
        let canonical = tmp.path().join("canonical").join("skill-1");
        fs::create_dir_all(&canonical).unwrap();
        fs::write(canonical.join("SKILL.md"), "test").unwrap();

        let dir = tmp.path().join("platform").join("skills");
        let link = dir.join("skill-1");
        fs::create_dir_all(&dir).unwrap();
        utils::create_relative_symlink(&canonical, &link).unwrap();

        remove_all_from_dir(&dir).unwrap();
        assert!(!link.exists());
        assert!(canonical.exists()); // 规范目录不受影响
    }

    #[test]
    fn test_remove_all_from_dir_nonexistent() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("nonexistent");
        // 不应报错
        remove_all_from_dir(&dir).unwrap();
    }

    // --- remove_symlink 验证不跟删目标 ---

    #[test]
    fn test_remove_symlink_does_not_follow_target() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("target");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("important.txt"), "keep me").unwrap();

        let link = tmp.path().join("link");
        utils::create_relative_symlink(&target, &link).unwrap();

        utils::remove_symlink(&link).unwrap();
        assert!(!link.exists());
        assert!(target.join("important.txt").exists());
        assert_eq!(fs::read_to_string(target.join("important.txt")).unwrap(), "keep me");
    }
}
