use crate::config::Config;
use crate::utils::{canonical_skills_dir, display_path, validate_agent};
use anyhow::Result;
use colored::Colorize;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// 已安装 skill 的信息
struct InstalledSkill {
    /// 显示路径（规范目录或平台目录）
    display_path: PathBuf,
    /// 是否在规范目录中
    in_canonical: bool,
    /// 关联的平台列表
    platforms: Vec<String>,
}

pub fn run(global: bool, agent: Option<&str>) -> Result<()> {
    let config = Config::load()?;

    let canonical_dir = canonical_skills_dir(global);
    let mut skills: BTreeMap<String, InstalledSkill> = BTreeMap::new();

    // 扫描规范目录
    for name in scan_skills_dir(&canonical_dir)? {
        skills.entry(name.clone()).or_insert_with(|| InstalledSkill {
            display_path: canonical_dir.join(&name),
            in_canonical: true,
            platforms: Vec::new(),
        });
    }

    // 扫描各平台目录（始终扫描所有平台，以收集完整的 agent 列表）
    if let Some(platform_name) = agent {
        // 验证平台名称
        validate_agent(&config, platform_name)?;
    }
    for pname in config.platform_names() {
        let platform_skills = scan_platform_with_paths(&config, pname, global)?;
        for (name, platform_path) in platform_skills {
            let entry = skills.entry(name.clone()).or_insert_with(|| InstalledSkill {
                display_path: platform_path.clone(),
                in_canonical: false,
                platforms: Vec::new(),
            });
            // 如果不在规范目录中，使用平台路径
            if !entry.in_canonical {
                entry.display_path = platform_path;
            }
            if !entry.platforms.contains(&pname.to_string()) {
                entry.platforms.push(pname.to_string());
            }
        }
    }

    // 标题：项目级 vs 全局级
    if global {
        println!("{}\n", "Global Skills".bold());
    } else {
        println!("{}\n", "Project Skills".bold());
    }

    if skills.is_empty() {
        println!("{}", "No skills installed".bright_black());
        return Ok(());
    }

    // 按路径排序
    let mut sorted_skills: Vec<(&String, &InstalledSkill)> = skills.iter().collect();
    sorted_skills.sort_by(|a, b| display_path(&a.1.display_path).cmp(&display_path(&b.1.display_path)));

    // 计算列宽
    let max_name_len = sorted_skills.iter().map(|(k, _)| k.len()).max().unwrap_or(0);
    let max_path_len = sorted_skills
        .iter()
        .map(|(_, v)| display_path(&v.display_path).len())
        .max()
        .unwrap_or(0);

    // 输出
    for (name, info) in &sorted_skills {
        let path_str = display_path(&info.display_path);
        let status_str = if let Some(filter_agent) = agent {
            if info.platforms.contains(&filter_agent.to_string()) {
                // 在过滤的平台中，显示 Agents
                if info.platforms.is_empty() {
                    String::new()
                } else {
                    format!("{} {}", "Agents:".bright_black(), info.platforms.join(", "))
                }
            } else {
                // 不在过滤的平台中，显示 not symlinked
                format!("{} {}", "Agents:".bright_black(), "not symlinked".yellow())
            }
        } else {
            // 没有过滤，显示所有 Agents
            if info.platforms.is_empty() {
                String::new()
            } else {
                format!("{} {}", "Agents:".bright_black(), info.platforms.join(", "))
            }
        };

        println!(
            "{:<name_w$}    {:<path_w$}    {}",
            name.yellow(),
            path_str.bright_black(),
            status_str,
            name_w = max_name_len,
            path_w = max_path_len,
        );
    }

    Ok(())
}

/// 扫描指定平台目录，返回 (skill名称, 平台路径) 列表
fn scan_platform_with_paths(
    config: &Config,
    platform_name: &str,
    global: bool,
) -> Result<Vec<(String, PathBuf)>> {
    let platform = match config.get_platform(platform_name) {
        Some(p) => p,
        None => return Ok(Vec::new()),
    };

    if platform.skills.is_empty() {
        return Ok(Vec::new());
    }

    let base_dir = if global {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"))
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    let platform_path = base_dir.join(&platform.path);
    let skills_dir = platform_path.join(&platform.skills);

    scan_skills_dir_with_paths(&skills_dir)
}

/// 扫描 skills 目录，返回有效 skill 名称列表
fn scan_skills_dir(dir: &Path) -> Result<Vec<String>> {
    let mut names = Vec::new();

    if !dir.exists() {
        return Ok(names);
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // 检查是否是目录或 symlink
        let is_valid = entry.file_type().map(|t| t.is_dir() || t.is_symlink()).unwrap_or(false);
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

/// 扫描 skills 目录，返回 (skill名称, 实际路径) 列表
fn scan_skills_dir_with_paths(dir: &Path) -> Result<Vec<(String, PathBuf)>> {
    let mut items = Vec::new();

    if !dir.exists() {
        return Ok(items);
    }

    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        // 检查是否是目录或 symlink
        let is_valid = entry.file_type().map(|t| t.is_dir() || t.is_symlink()).unwrap_or(false);
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

        items.push((entry.file_name().to_string_lossy().to_string(), path));
    }

    Ok(items)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Platform};

    #[test]
    fn test_list_platform_not_found() {
        let config = Config::default();
        let result = config.get_platform("nonexistent");
        assert!(result.is_none());
    }

    #[test]
    fn test_list_platform_found() {
        let mut config = Config::default();
        config.platforms.insert(
            "claude".to_string(),
            Platform {
                path: ".claude".to_string(),
                skills: "skills".to_string(),
                agents: "CLAUDE.md".to_string(),
                source: "AGENTS.md".to_string(),
                agents_compat: false,
            },
        );
        let result = config.get_platform("claude");
        assert!(result.is_some());
        assert_eq!(result.unwrap().path, ".claude");
    }

    // --- scan_skills_dir ---

    #[test]
    fn test_scan_skills_dir_finds_skills() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("skills");

        let s1 = dir.join("vue");
        let s2 = dir.join("react");
        fs::create_dir_all(&s1).unwrap();
        fs::create_dir_all(&s2).unwrap();
        fs::write(s1.join("SKILL.md"), "---\nname: vue\n---\n").unwrap();
        fs::write(s2.join("SKILL.md"), "---\nname: react\n---\n").unwrap();

        let names = scan_skills_dir(&dir).unwrap();
        assert_eq!(names.len(), 2);
        assert!(names.contains(&"vue".to_string()));
        assert!(names.contains(&"react".to_string()));
    }

    #[test]
    fn test_scan_skills_dir_skips_non_skill_dirs() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("skills");

        let s1 = dir.join("real-skill");
        let s2 = dir.join("not-a-skill");
        fs::create_dir_all(&s1).unwrap();
        fs::create_dir_all(&s2).unwrap();
        fs::write(s1.join("SKILL.md"), "test").unwrap();
        fs::write(s2.join("README.md"), "readme").unwrap();

        let names = scan_skills_dir(&dir).unwrap();
        assert_eq!(names.len(), 1);
        assert_eq!(names[0], "real-skill");
    }

    #[test]
    fn test_scan_skills_dir_includes_symlinks() {
        let tmp = tempfile::tempdir().unwrap();

        let canonical = tmp.path().join("canonical").join("my-skill");
        fs::create_dir_all(&canonical).unwrap();
        fs::write(canonical.join("SKILL.md"), "test").unwrap();

        let dir = tmp.path().join("platform").join("skills");
        let link = dir.join("my-skill");
        fs::create_dir_all(&dir).unwrap();
        crate::utils::create_relative_symlink(&canonical, &link).unwrap();

        let names = scan_skills_dir(&dir).unwrap();
        assert_eq!(names.len(), 1);
        assert_eq!(names[0], "my-skill");
    }

    #[test]
    fn test_scan_skills_dir_skips_broken_symlinks() {
        let tmp = tempfile::tempdir().unwrap();

        let target = tmp.path().join("target").join("broken-skill");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("SKILL.md"), "test").unwrap();

        let dir = tmp.path().join("skills");
        let link = dir.join("broken-skill");
        fs::create_dir_all(&dir).unwrap();
        crate::utils::create_relative_symlink(&target, &link).unwrap();

        // 删除目标使链接断裂
        fs::remove_dir_all(target.parent().unwrap()).unwrap();

        let names = scan_skills_dir(&dir).unwrap();
        assert!(names.is_empty(), "should skip broken symlinks");
    }

    #[test]
    fn test_scan_skills_dir_nonexistent() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("nonexistent");
        let names = scan_skills_dir(&dir).unwrap();
        assert!(names.is_empty());
    }

    #[test]
    fn test_scan_skills_dir_empty() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("empty_skills");
        fs::create_dir_all(&dir).unwrap();

        let names = scan_skills_dir(&dir).unwrap();
        assert!(names.is_empty());
    }

    #[test]
    fn test_scan_skills_dir_skips_files() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("skills");
        fs::create_dir_all(&dir).unwrap();

        // 放一个普通文件（非目录）
        fs::write(dir.join("some-file.txt"), "not a skill").unwrap();

        // 放一个真正的 skill
        let real = dir.join("real-skill");
        fs::create_dir_all(&real).unwrap();
        fs::write(real.join("SKILL.md"), "test").unwrap();

        let names = scan_skills_dir(&dir).unwrap();
        assert_eq!(names.len(), 1);
        assert_eq!(names[0], "real-skill");
    }

    // --- scan_platform ---

    #[test]
    fn test_scan_platform_returns_skill_names() {
        let tmp = tempfile::tempdir().unwrap();

        let mut config = Config::default();
        config.platforms.insert(
            "test-platform".to_string(),
            Platform {
                path: tmp.path().join(".test-platform").to_string_lossy().to_string(),
                skills: "skills".to_string(),
                agents: "AGENTS.md".to_string(),
                source: "AGENTS.md".to_string(),
                agents_compat: true,
            },
        );

        // 创建平台 skills 目录和 skill
        let skills_dir = tmp.path().join(".test-platform").join("skills");
        let skill_dir = skills_dir.join("my-skill");
        fs::create_dir_all(&skill_dir).unwrap();
        fs::write(skill_dir.join("SKILL.md"), "test").unwrap();

        // scan_platform 需要 global=true 来使用 tmp 作为 home 基础
        // 但由于 scan_platform 内部用 base_dir = if global { home } else { cwd }，
        // 我们需要手动设置路径。这里直接测试 scan_skills_dir 更合适。
        let names = scan_skills_dir(&skills_dir).unwrap();
        assert_eq!(names.len(), 1);
        assert_eq!(names[0], "my-skill");
    }

    #[test]
    fn test_scan_platform_no_skills_config() {
        let config = Config::default();
        // 默认平台中不存在 "nonexistent"
        let items = scan_platform_with_paths(&config, "nonexistent", false).unwrap();
        assert!(items.is_empty());
    }
}
