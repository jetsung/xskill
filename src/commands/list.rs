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

    // 标题：项目级 vs 全局级
    if global {
        println!("{}\n", "Global Skills".bold());
    } else {
        println!("{}\n", "Project Skills".bold());
    }

    // 收集 skills：
    // - 指定平台时，只扫描该平台实际可用的 skills（合并目录，同名去重、规范目录优先）
    // - 未指定时，扫描规范目录与所有平台目录，收集完整的 agent 关联列表
    let skills = if let Some(platform_name) = agent {
        if platform_name == "*" {
            anyhow::bail!(
                "--agent '*' is not supported for list; omit --agent to list all skills"
            );
        }
        // 验证平台名称
        validate_agent(&config, platform_name)?;
        scan_platform_skills(&config, platform_name, global)?
    } else {
        scan_all_skills(&config, global)?
    };

    if skills.is_empty() {
        println!("{}", "No skills installed".bright_black());
        return Ok(());
    }

    // 按路径排序
    let mut sorted_skills: Vec<(&String, &InstalledSkill)> = skills.iter().collect();
    sorted_skills.sort_by_key(|(_, v)| display_path(&v.display_path));

    // 计算列宽
    let max_name_len = sorted_skills
        .iter()
        .map(|(k, _)| k.len())
        .max()
        .unwrap_or(0);
    let max_path_len = sorted_skills
        .iter()
        .map(|(_, v)| display_path(&v.display_path).len())
        .max()
        .unwrap_or(0);

    // 输出：指定平台时仅名称 + 路径两列；未指定时附加 Agents 列
    for (name, info) in &sorted_skills {
        let path_str = display_path(&info.display_path);
        if agent.is_some() {
            println!(
                "{:<name_w$}    {:<path_w$}",
                name.yellow(),
                path_str.bright_black(),
                name_w = max_name_len,
                path_w = max_path_len,
            );
        } else {
            let status_str = if info.platforms.is_empty() {
                String::new()
            } else {
                format!("{} {}", "Agents:".bright_black(), info.platforms.join(", "))
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
    }

    Ok(())
}

/// 扫描规范目录与所有平台目录，收集全部已安装 skills 及平台关联列表
fn scan_all_skills(config: &Config, global: bool) -> Result<BTreeMap<String, InstalledSkill>> {
    let canonical_dir = canonical_skills_dir(global);
    let mut skills: BTreeMap<String, InstalledSkill> = BTreeMap::new();

    // 扫描规范目录
    for name in scan_skills_dir(&canonical_dir)? {
        skills
            .entry(name.clone())
            .or_insert_with(|| InstalledSkill {
                display_path: canonical_dir.join(&name),
                in_canonical: true,
                platforms: Vec::new(),
            });
    }

    // 扫描各平台目录（始终扫描所有平台，以收集完整的 agent 列表）
    for pname in config.platform_names() {
        let platform = config.platforms.get(pname);
        let is_agents_compat = platform.map(|p| p.agents_compat).unwrap_or(false);

        if is_agents_compat {
            // agents_compat 平台直接读取规范目录，视为已链接所有规范目录中的 skill
            for entry in skills.values_mut() {
                if entry.in_canonical && !entry.platforms.contains(&pname.to_string()) {
                    entry.platforms.push(pname.to_string());
                }
            }
        } else {
            let platform_skills = scan_platform_with_paths(config, pname, global)?;
            for (name, platform_path) in platform_skills {
                let entry = skills
                    .entry(name.clone())
                    .or_insert_with(|| InstalledSkill {
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
    }

    Ok(skills)
}

/// 扫描指定平台实际可用的 skills（合并其目录列表，同名去重、规范目录优先）
fn scan_platform_skills(
    config: &Config,
    platform_name: &str,
    global: bool,
) -> Result<BTreeMap<String, InstalledSkill>> {
    let mut skills: BTreeMap<String, InstalledSkill> = BTreeMap::new();

    if let Some(dirs) = platform_skills_dirs(config, platform_name, global) {
        for dir in dirs {
            for (name, path) in scan_skills_dir_with_paths(&dir)? {
                skills
                    .entry(name.clone())
                    .or_insert_with(|| InstalledSkill {
                        display_path: path,
                        in_canonical: true,
                        platforms: Vec::new(),
                    });
            }
        }
    }

    Ok(skills)
}

/// 根据平台配置解析实际扫描目录列表：
/// - agents_compat 平台：返回 [规范目录, 平台自身 skills 目录]（规范目录在前）
/// - 非兼容平台：返回 [平台自身 skills 目录]
/// - 平台不存在或 skills 目录配置为空：返回 None
fn platform_skills_dirs(
    config: &Config,
    platform_name: &str,
    global: bool,
) -> Option<Vec<PathBuf>> {
    let platform = config.get_platform(platform_name)?;
    if platform.skills.is_empty() {
        return None;
    }

    let base_dir = if global {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"))
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };
    let platform_skills_dir = base_dir.join(&platform.path).join(&platform.skills);

    if platform.agents_compat {
        Some(vec![canonical_skills_dir(global), platform_skills_dir])
    } else {
        Some(vec![platform_skills_dir])
    }
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
                path: tmp
                    .path()
                    .join(".test-platform")
                    .to_string_lossy()
                    .to_string(),
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

    // --- platform_skills_dirs ---

    #[test]
    fn test_platform_skills_dirs_non_compat_project() {
        let mut config = Config::default();
        config.platforms.insert(
            "custom-plain".to_string(),
            Platform {
                path: ".custom-plain".to_string(),
                skills: "skills".to_string(),
                agents: "CUSTOM.md".to_string(),
                source: "AGENTS.md".to_string(),
                agents_compat: false,
            },
        );

        let dirs = platform_skills_dirs(&config, "custom-plain", false).unwrap();
        assert_eq!(dirs.len(), 1);
        let cwd = std::env::current_dir().unwrap();
        assert_eq!(dirs[0], cwd.join(".custom-plain").join("skills"));
    }

    #[test]
    fn test_platform_skills_dirs_non_compat_global() {
        let mut config = Config::default();
        config.platforms.insert(
            "custom-plain".to_string(),
            Platform {
                path: ".custom-plain".to_string(),
                skills: "skills".to_string(),
                agents: "CUSTOM.md".to_string(),
                source: "AGENTS.md".to_string(),
                agents_compat: false,
            },
        );

        let dirs = platform_skills_dirs(&config, "custom-plain", true).unwrap();
        assert_eq!(dirs.len(), 1);
        let home = dirs::home_dir().unwrap();
        assert_eq!(dirs[0], home.join(".custom-plain").join("skills"));
    }

    #[test]
    fn test_platform_skills_dirs_agents_compat_project() {
        let mut config = Config::default();
        config.platforms.insert(
            "custom-compat".to_string(),
            Platform {
                path: ".custom-compat".to_string(),
                skills: "skills".to_string(),
                agents: "CUSTOM.md".to_string(),
                source: "AGENTS.md".to_string(),
                agents_compat: true,
            },
        );

        let dirs = platform_skills_dirs(&config, "custom-compat", false).unwrap();
        assert_eq!(dirs.len(), 2);
        let cwd = std::env::current_dir().unwrap();
        // 规范目录在前
        assert_eq!(dirs[0], cwd.join(".agents").join("skills"));
        assert_eq!(dirs[1], cwd.join(".custom-compat").join("skills"));
    }

    #[test]
    fn test_platform_skills_dirs_agents_compat_global() {
        let mut config = Config::default();
        config.platforms.insert(
            "custom-compat".to_string(),
            Platform {
                path: ".custom-compat".to_string(),
                skills: "skills".to_string(),
                agents: "CUSTOM.md".to_string(),
                source: "AGENTS.md".to_string(),
                agents_compat: true,
            },
        );

        let dirs = platform_skills_dirs(&config, "custom-compat", true).unwrap();
        assert_eq!(dirs.len(), 2);
        let home = dirs::home_dir().unwrap();
        // 规范目录在前
        assert_eq!(dirs[0], home.join(".agents").join("skills"));
        assert_eq!(dirs[1], home.join(".custom-compat").join("skills"));
    }

    #[test]
    fn test_platform_skills_dirs_platform_not_found() {
        let config = Config::default();
        assert!(platform_skills_dirs(&config, "nonexistent", false).is_none());
    }

    #[test]
    fn test_platform_skills_dirs_empty_skills_config() {
        let mut config = Config::default();
        config.platforms.insert(
            "custom-empty".to_string(),
            Platform {
                path: ".custom-empty".to_string(),
                skills: String::new(),
                agents: "CUSTOM.md".to_string(),
                source: "AGENTS.md".to_string(),
                agents_compat: false,
            },
        );

        assert!(platform_skills_dirs(&config, "custom-empty", false).is_none());
    }
}
