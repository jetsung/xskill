use crate::config::Config;
use crate::git;
use crate::lock::{LockFile, LockEntry};
use crate::skill_meta::SkillMeta;
use crate::skill_resolver;
use crate::utils::{
    canonical_skills_dir, create_relative_symlink, display_path, resolve_source, validate_agent,
    ResolvedSource,
};
use anyhow::{bail, Context, Result};
use colored::Colorize;
use ratatui::style::Color;
use ratatui::text::{Line, Span};
use skim::prelude::*;
use std::fs;
use std::io::IsTerminal;
use std::path::{Path, PathBuf};

/// 安装目标
enum InstallTarget {
    /// 仅安装到规范目录
    Canonical,
    /// 安装到规范目录 + 创建到指定平台的 symlink
    CanonicalWithPlatform(String),
    /// 安装到规范目录 + 创建到所有已存在平台的 symlink
    CanonicalWithAllPlatforms,
}

pub fn run(url: Option<&str>, skill: &str, global: bool, agent: Option<&str>) -> Result<()> {
    let config = Config::load()?;

    // 解析安装目标
    let target = resolve_install_target(agent);

    // 处理 -s '*' 的情况（安装所有 skills，需要 --from）
    if skill == "*" {
        let url = url.ok_or_else(|| anyhow::anyhow!("--from is required when using '*' to install all skills"))?;
        let resolved = resolve_source(&config, url)?;
        return install_all_skills(&config, &resolved, &target, url, global);
    }

    // 查找 skill（支持 fallback：指定源 → 缓存 → 所有源 → 注册中心）
    let (source_name, source_url, skill_path, dest_name) =
        find_skill_with_fallback(&config, skill, url)?;

    // 显示源信息
    println!("{}: {} ({})", "Source".cyan().bold(), source_name, source_url);

    let resolved = ResolvedSource {
        url: source_url.clone(),
        source_type: config
            .get_source(&source_name)
            .map(|s| s.effective_type())
            .unwrap_or_else(|| "git".to_string()),
    };

    // 安装到规范目录
    let canonical_dir = canonical_skills_dir(global).join(&dest_name);
    install_to_canonical(&resolved, &skill_path, &dest_name, &canonical_dir, &source_name, global)?;

    // 根据目标创建软链接
    match &target {
        InstallTarget::Canonical => {
            // 仅规范目录，无需额外操作
        }
        InstallTarget::CanonicalWithPlatform(platform_name) => {
            symlink_to_platform(&config, &dest_name, platform_name, global)?;
        }
        InstallTarget::CanonicalWithAllPlatforms => {
            symlink_to_all_platforms(&config, &dest_name, global)?;
        }
    }

    Ok(())
}

/// 解析安装目标
fn resolve_install_target(agent: Option<&str>) -> InstallTarget {
    match agent {
        Some("*") => InstallTarget::CanonicalWithAllPlatforms,
        Some(name) => InstallTarget::CanonicalWithPlatform(name.to_string()),
        None => InstallTarget::Canonical,
    }
}

/// 安装 skill 到规范目录
fn install_to_canonical(
    resolved: &ResolvedSource,
    skill_path: &str,
    dest_name: &str,
    dest_dir: &Path,
    source: &str,
    global: bool,
) -> Result<()> {
    // 清理规范目录中的 skill 子目录
    if dest_dir.exists() || dest_dir.is_symlink() {
        fs::remove_dir_all(dest_dir)?;
    }

    // 安装
    let _result = git::install_skill(&resolved.url, skill_path, dest_name, dest_dir)
        .with_context(|| format!("Failed to install skill '{}'", dest_name))?;

    // 读取 SKILL.md 信息
    let meta = SkillMeta::from_file(dest_dir)?;

    // 显示 skill 信息
    println!("{}: {}", "Name".cyan().bold(), meta.display_name(dest_name).yellow());
    println!("{}: {}", "Description".cyan().bold(), meta.display_description());
    if let Some(version) = meta.metadata.as_ref().and_then(|m| m.version.clone()) {
        if !version.is_empty() {
            println!("{}: {}", "Version".cyan().bold(), version);
        }
    }

    // 显示安装路径
    println!("{}: {}", "Installed".green(), display_path(dest_dir));

    // 更新锁文件
    update_lock_file(source, resolved, skill_path, dest_name, global)?;

    Ok(())
}

/// 创建到指定平台的 symlink
fn symlink_to_platform(
    config: &Config,
    dest_name: &str,
    platform_name: &str,
    global: bool,
) -> Result<()> {
    validate_agent(config, platform_name)?;
    let platform = config.get_platform(platform_name).unwrap();

    let skills_dir = platform.skills_dir()
        .ok_or_else(|| anyhow::anyhow!("Platform {} has no skills directory configured", platform_name))?;

    // 平台目录不存在时自动创建
    let base_dir = if global {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"))
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };
    let platform_path = base_dir.join(&platform.path);
    if !platform_path.exists() {
        fs::create_dir_all(&platform_path)?;
    }

    let canonical_dir = canonical_skills_dir(global).join(dest_name);
    let link_path = skills_dir.join(dest_name);

    match create_relative_symlink(&canonical_dir, &link_path) {
        Ok(true) => {
            println!("{}: {}", "Symlinked".green(), display_path(&link_path));
        }
        Ok(false) => {
            // 回退到文件复制
            fs::create_dir_all(&link_path)?;
            copy_dir_recursive(&canonical_dir, &link_path)?;
            println!("{}: {} (copy fallback)", "Installed".green(), display_path(&link_path));
        }
        Err(e) => {
            // 回退到文件复制
            fs::create_dir_all(&link_path)?;
            copy_dir_recursive(&canonical_dir, &link_path)?;
            println!("{}: {} (copy fallback: {})", "Installed".green(), display_path(&link_path), e);
        }
    }

    Ok(())
}

/// 创建到所有已存在平台的 symlink
fn symlink_to_all_platforms(
    config: &Config,
    dest_name: &str,
    global: bool,
) -> Result<()> {
    let base_dir = if global {
        dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"))
    } else {
        std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
    };

    let canonical_dir = canonical_skills_dir(global).join(dest_name);
    let mut linked_platforms = Vec::new();

    for name in config.platform_names() {
        let platform = match config.platforms.get(name) {
            Some(p) => p,
            None => continue,
        };
        // 仅链接已存在的平台目录
        let platform_path = base_dir.join(&platform.path);
        if !platform_path.exists() {
            continue;
        }

        if platform.skills.is_empty() {
            continue;
        }

        let skills_dir = platform_path.join(&platform.skills);
        let link_path = skills_dir.join(dest_name);

        match create_relative_symlink(&canonical_dir, &link_path) {
            Ok(true) => {
                linked_platforms.push(name.to_string());
            }
            Ok(false) => {
                // 回退到文件复制
                fs::create_dir_all(&link_path)?;
                copy_dir_recursive(&canonical_dir, &link_path)?;
                linked_platforms.push(name.to_string());
            }
            Err(e) => {
                eprintln!("{}: Failed to link to {}: {}", "Warning".yellow(), name, e);
            }
        }
    }

    if linked_platforms.is_empty() {
        println!("{}", "No available platform directories found to link".yellow());
    } else {
        println!("{}: {}", "Symlinked".green(), linked_platforms.join(", "));
    }

    Ok(())
}

/// 安装所有 skills
fn install_all_skills(
    config: &Config,
    resolved: &ResolvedSource,
    target: &InstallTarget,
    source: &str,
    global: bool,
) -> Result<()> {
    // 克隆仓库并列出所有 skills
    let tmp_dir = git::clone_for_listing(&resolved.url)?;
    let skills_dir = tmp_dir.path().join("skills");

    if !skills_dir.exists() {
        println!("{}", "No skills directory found in this source".yellow());
        return Ok(());
    }

    // 递归收集所有包含 SKILL.md 的目录
    let mut skill_entries = Vec::new();
    collect_all_skills(&skills_dir, &mut skill_entries, &mut String::new())?;

    if skill_entries.is_empty() {
        println!("{}", "No skills available in this source".yellow());
        return Ok(());
    }

    // 显示源 URL
    println!("{}: {}", "Source URL".cyan().bold(), resolved.url);
    println!();

    // 加载锁文件
    let mut lock_file = LockFile::load(global)?;

    // 获取 sourceType
    let source_type = if source.contains('/') {
        "git".to_string()
    } else {
        config.get_source(source)
            .map(|s| s.effective_type())
            .unwrap_or_else(|| "git".to_string())
    };

    // 获取当前时间
    let now = chrono::Utc::now();
    let timestamp = now.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    // 安装每个 skill
    for (skill_path, dest_name) in &skill_entries {
        // 读取 SKILL.md 信息
        let skill_md_path = skills_dir.join(skill_path);
        let meta = SkillMeta::from_file(&skill_md_path).unwrap_or_default();

        // 显示 skill 信息
        println!("{}: {}", "Name".cyan().bold(), meta.display_name(dest_name).yellow());
        println!("{}: {}", "Description".cyan().bold(), meta.display_description());
        if let Some(version) = meta.metadata.as_ref().and_then(|m| m.version.clone()) {
            if !version.is_empty() {
                println!("{}: {}", "Version".cyan().bold(), version);
            }
        }

        // 获取 tree hash
        let skill_folder_hash = git::get_skill_folder_hash(tmp_dir.path(), skill_path)
            .unwrap_or_default();

        // 检查是否已存在，保留原始 installedAt
        let installed_at = if let Some(existing) = lock_file.skills.get(dest_name) {
            existing.installed_at.clone()
        } else {
            timestamp.clone()
        };

        // 构建 skillPath
        let skill_path_in_repo = format!("skills/{}/SKILL.md", skill_path);

        // 创建锁文件条目
        let lock_entry = LockEntry {
            source: source.to_string(),
            source_type: source_type.clone(),
            source_url: resolved.url.clone(),
            skill_path: skill_path_in_repo,
            skill_folder_hash,
            installed_at,
            updated_at: timestamp.clone(),
        };

        // 添加或更新记录
        lock_file.upsert_skill(dest_name, lock_entry);

        // 源目录
        let source_dir = skills_dir.join(skill_path);

        // 安装到规范目录
        let canonical_dir = canonical_skills_dir(global).join(dest_name);
        copy_skill_to_target(&source_dir, &canonical_dir)?;

        // 创建 symlink
        match target {
            InstallTarget::Canonical => {}
            InstallTarget::CanonicalWithPlatform(platform_name) => {
                symlink_to_platform(config, dest_name, platform_name, global)?;
            }
            InstallTarget::CanonicalWithAllPlatforms => {
                symlink_to_all_platforms(config, dest_name, global)?;
            }
        }

        println!();
    }

    // 保存锁文件
    lock_file.updated_at = timestamp;
    lock_file.save(global)?;

    println!("{}", "All skills installed".green());
    Ok(())
}

/// 复制 skill 到目标目录
fn copy_skill_to_target(
    source_dir: &Path,
    dest_dir: &Path,
) -> Result<()> {
    if !source_dir.exists() {
        bail!("Source directory not found: {}", source_dir.display());
    }

    // 清理目标（包括断裂的 symlink）
    if dest_dir.exists() || dest_dir.is_symlink() {
        fs::remove_dir_all(dest_dir)?;
    }
    fs::create_dir_all(dest_dir)?;
    copy_dir_recursive(source_dir, dest_dir)?;

    println!("{}: {}", "Installed".green(), display_path(dest_dir));
    Ok(())
}

/// 更新锁文件
fn update_lock_file(
    source: &str,
    resolved: &ResolvedSource,
    skill_path: &str,
    dest_name: &str,
    global: bool,
) -> Result<()> {
    let mut lock_file = LockFile::load(global)?;

    // 获取 sourceType
    let source_type = if source.contains('/') {
        "git".to_string()
    } else {
        // 从配置中查找
        let config = Config::load()?;
        config.get_source(source)
            .map(|s| s.effective_type())
            .unwrap_or_else(|| "git".to_string())
    };

    // 构建 skillPath（相对于 Git 仓库）
    let skill_path_in_repo = format!("skills/{}/SKILL.md", skill_path);

    // 克隆仓库以获取 tree hash
    let tmp_dir = git::clone_for_listing(&resolved.url)?;
    let skill_folder_hash = git::get_skill_folder_hash(tmp_dir.path(), skill_path)
        .unwrap_or_default();

    // 获取当前时间（格式化为 ISO 8601 with milliseconds）
    let now = chrono::Utc::now();
    let timestamp = now.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    // 检查是否已存在，保留原始 installedAt
    let installed_at = if let Some(existing) = lock_file.skills.get(dest_name) {
        existing.installed_at.clone()
    } else {
        timestamp.clone()
    };

    // 创建锁文件条目
    let entry = LockEntry {
        source: source.to_string(),
        source_type,
        source_url: resolved.url.clone(),
        skill_path: skill_path_in_repo,
        skill_folder_hash,
        installed_at,
        updated_at: timestamp.clone(),
    };

    // 添加或更新记录
    lock_file.upsert_skill(dest_name, entry);

    // 保存锁文件
    lock_file.updated_at = timestamp;
    lock_file.save(global)?;

    Ok(())
}

/// 查找 skill，支持 fallback：指定源 → 缓存 → 所有源 → 注册中心
fn find_skill_with_fallback(
    config: &Config,
    skill_name: &str,
    prefer_source: Option<&str>,
) -> Result<(String, String, String, String)> {
    // 1. If preferred source specified, try it first
    if let Some(src) = prefer_source {
        if let Ok(resolved) = resolve_source(config, src) {
            if let Ok(Some((path, dest))) = find_skill_in_repo(&resolved.url, skill_name) {
                let source_name = config
                    .get_source(src)
                    .map(|s| s.effective_name())
                    .unwrap_or_else(|| src.to_string());
                return Ok((source_name, resolved.url, path, dest));
            }
        }
        // Source not resolvable or skill not found — fall through to broader search
    }

    // 2. Search all sources (cache → configured → registry)
    let matches = skill_resolver::find_all_skills(config, skill_name, prefer_source);

    match matches.len() {
        0 => bail!(
            "Skill '{}' not found in any source{}",
            skill_name,
            prefer_source
                .map(|s| format!(" (including '{}')", s))
                .unwrap_or_default()
        ),
        1 => {
            let m = &matches[0];
            let (skill_path, dest_name) = extract_skill_path(&m.skill_path);
            Ok((m.source_name.clone(), m.source_url.clone(), skill_path, dest_name))
        }
        _ => {
            // Multiple sources have this skill — let the user choose
            if !std::io::stdin().is_terminal() {
                let sources: Vec<String> = matches
                    .iter()
                    .map(|m| {
                        let tag = if m.is_registry { " [registry]" } else { "" };
                        format!("  {}{}: {}", m.source_name, tag, m.source_url)
                    })
                    .collect();
                bail!(
                    "Multiple sources contain skill '{}':\n{}\n\nUse -f <source> to specify which one.",
                    skill_name,
                    sources.join("\n")
                );
            }
            let selected = run_source_select_tui(skill_name, &matches)?;
            let (skill_path, dest_name) = extract_skill_path(&selected.skill_path);
            Ok((selected.source_name, selected.source_url, skill_path, dest_name))
        }
    }
}

/// A selectable item for the source disambiguation TUI.
struct SourceItem {
    display: String,
    source_name: String,
    source_url: String,
    is_registry: bool,
}

impl SkimItem for SourceItem {
    fn text(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.display)
    }

    fn output(&self) -> Cow<'_, str> {
        Cow::Borrowed(&self.source_name)
    }

    fn display(&self, context: DisplayContext) -> Line<'_> {
        let base = context.base_style;
        let is_selected = base.bg.is_some();
        let name_style = if is_selected { base.fg(Color::Blue) } else { base };
        let registry_style = if is_selected {
            base.fg(Color::Green)
        } else {
            base.fg(Color::DarkGray)
        };
        let source_style = base.fg(Color::DarkGray);

        // Column 1: [registry] tag (10 chars padded)
        // Column 2: source_name
        // Column 3: url
        let tag = if self.is_registry { "[registry]" } else { "-" };
        let mut spans = vec![Span::styled(format!("{:<12}", tag), registry_style)];
        spans.push(Span::styled(format!("{:<16}", self.source_name), name_style));
        spans.push(Span::styled(self.source_url.clone(), source_style));
        Line::from(spans)
    }
}

/// Show a skim single-select TUI for choosing between multiple source matches.
fn run_source_select_tui(
    skill_name: &str,
    matches: &[skill_resolver::SkillMatch],
) -> Result<skill_resolver::SkillMatch> {
    let items: Vec<SourceItem> = matches
        .iter()
        .map(|m| SourceItem {
            display: format!("{} {} {}", if m.is_registry { "[registry]" } else { "" }, m.source_name, m.source_url),
            source_name: m.source_name.clone(),
            source_url: m.source_url.clone(),
            is_registry: m.is_registry,
        })
        .collect();

    let opts = SkimOptionsBuilder::default()
        .multi(false)
        .prompt(format!("Select source for '{}': ", skill_name))
        .exact(true)
        .highlight_line(true)
        .color("current:bg:236,current_match:fg:151:bg:236".to_string())
        .header(" \nup/down navigate | enter select | esc cancel\n ".to_string())
        .build()
        .map_err(|e| anyhow::anyhow!("{}", e))?;

    let output = Skim::run_items(opts, items).map_err(|e| anyhow::anyhow!("{}", e))?;

    if output.is_abort {
        bail!("Cancelled.");
    }

    let matched = match output.current {
        Some(item) => item,
        None => bail!("No source selected."),
    };

    let selected = matched
        .downcast_item::<SourceItem>()
        .ok_or_else(|| anyhow::anyhow!("Failed to retrieve selected source"))?;

    // Find the original SkillMatch by source_name + source_url
    matches
        .iter()
        .find(|m| m.source_name == selected.source_name && m.source_url == selected.source_url)
        .cloned()
        .ok_or_else(|| anyhow::anyhow!("Selected source not found"))
}

/// Extract (rel_path, dir_name) from "skills/rel_path/SKILL.md"
fn extract_skill_path(full_path: &str) -> (String, String) {
    let stripped = full_path
        .strip_prefix("skills/")
        .unwrap_or(full_path)
        .strip_suffix("/SKILL.md")
        .unwrap_or(full_path);
    let dest_name = stripped
        .split('/')
        .last()
        .unwrap_or(stripped)
        .to_string();
    (stripped.to_string(), dest_name)
}

/// 在仓库中查找 skill 的完整路径（递归搜索所有子目录）
fn find_skill_in_repo(repo_url: &str, skill_name: &str) -> Result<Option<(String, String)>> {
    let tmp_dir = git::clone_for_listing(repo_url)?;
    let workdir = tmp_dir.path().join("skills");

    if !workdir.exists() {
        return Ok(None);
    }

    // 递归搜索所有子目录
    let mut matches = Vec::new();
    collect_matching_skills(&workdir, skill_name, &mut matches, &mut String::new())?;

    match matches.len() {
        0 => Ok(None),
        1 => {
            let (path, dest) = matches.into_iter().next().unwrap();
            Ok(Some((path, dest)))
        }
        _ => {
            let options: Vec<String> = matches.iter().map(|(p, _)| p.clone()).collect();
            bail!(
                "Multiple skills matching '{}' found in source, please specify full path:\n{}",
                skill_name,
                options.join("\n")
            );
        }
    }
}

/// 递归收集匹配的 skill 目录
fn collect_matching_skills(
    dir: &std::path::Path,
    target: &str,
    matches: &mut Vec<(String, String)>,
    current_path: &mut String,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let dir_name = entry.file_name().to_string_lossy().to_string();
            let saved = current_path.clone();

            let rel_path = if current_path.is_empty() {
                dir_name.clone()
            } else {
                format!("{}/{}", current_path, dir_name)
            };
            *current_path = rel_path.clone();

            // 检查是否有 SKILL.md 且 name 匹配
            let skill_md = path.join("SKILL.md");
            if skill_md.exists() {
                if let Ok(meta) = SkillMeta::from_file(&path) {
                    let display = meta.display_name(&dir_name);
                    if display == target || dir_name == target {
                        matches.push((rel_path.clone(), dir_name.clone()));
                        *current_path = saved;
                        continue;
                    }
                }
            }

            // 继续递归搜索
            collect_matching_skills(&path, target, matches, current_path)?;
            *current_path = saved;
        }
    }

    Ok(())
}

/// 递归收集所有包含 SKILL.md 的目录
fn collect_all_skills(
    dir: &Path,
    entries: &mut Vec<(String, String)>,
    current_path: &mut String,
) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_dir() {
            let dir_name = entry.file_name().to_string_lossy().to_string();
            let saved = current_path.clone();

            let rel_path = if current_path.is_empty() {
                dir_name.clone()
            } else {
                format!("{}/{}", current_path, dir_name)
            };
            *current_path = rel_path.clone();

            // 检查是否有 SKILL.md（任何包含 SKILL.md 的目录都是 skill）
            let skill_md = path.join("SKILL.md");
            if skill_md.exists() {
                entries.push((rel_path.clone(), dir_name.clone()));
            }

            // 继续递归搜索子目录
            collect_all_skills(&path, entries, current_path)?;

            *current_path = saved;
        }
    }

    Ok(())
}

/// Recursively copy directory
fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
    if !src.is_dir() {
        return Ok(());
    }

    fs::create_dir_all(dst)
        .with_context(|| format!("Failed to create directory: {}", dst.display()))?;

    for entry in fs::read_dir(src)
        .with_context(|| format!("Failed to read directory: {}", src.display()))?
    {
        let entry = entry?;
        let src_path = entry.path();
        let dst_path = dst.join(entry.file_name());

        if src_path.is_dir() {
            copy_dir_recursive(&src_path, &dst_path)?;
        } else {
            fs::copy(&src_path, &dst_path)
                .with_context(|| format!("Failed to copy file: {}", src_path.display()))?;
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_install_target_default() {
        let target = resolve_install_target(None);
        assert!(matches!(target, InstallTarget::Canonical));
    }

    #[test]
    fn test_resolve_install_target_platform() {
        let target = resolve_install_target(Some("claude"));
        assert!(matches!(&target, InstallTarget::CanonicalWithPlatform(name) if name == "claude"));
    }

    #[test]
    fn test_resolve_install_target_all_platforms() {
        let target = resolve_install_target(Some("*"));
        assert!(matches!(target, InstallTarget::CanonicalWithAllPlatforms));
    }

    // --- extract_skill_path ---

    #[test]
    fn test_extract_skill_path_simple() {
        let (path, dest) = extract_skill_path("skills/vue/SKILL.md");
        assert_eq!(path, "vue");
        assert_eq!(dest, "vue");
    }

    #[test]
    fn test_extract_skill_path_nested() {
        let (path, dest) = extract_skill_path("skills/frontend/vue/SKILL.md");
        assert_eq!(path, "frontend/vue");
        assert_eq!(dest, "vue");
    }

    #[test]
    fn test_extract_skill_path_no_prefix() {
        let (path, dest) = extract_skill_path("vue/SKILL.md");
        assert_eq!(path, "vue");
        assert_eq!(dest, "vue");
    }

    #[test]
    fn test_extract_skill_path_no_suffix() {
        let (path, dest) = extract_skill_path("skills/vue");
        assert_eq!(path, "skills/vue");
        assert_eq!(dest, "vue");
    }

    // --- copy_dir_recursive ---

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
        assert_eq!(fs::read_to_string(dst.join("sub").join("b.txt")).unwrap(), "world");
    }

    #[test]
    fn test_copy_dir_recursive_nonexistent_src() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("nonexistent");
        let dst = tmp.path().join("dst");
        // 不报错，静默返回
        copy_dir_recursive(&src, &dst).unwrap();
        assert!(!dst.exists());
    }

    #[test]
    fn test_copy_dir_recursive_overwrites() {
        let tmp = tempfile::tempdir().unwrap();
        let src = tmp.path().join("src");
        let dst = tmp.path().join("dst");
        fs::create_dir_all(&src).unwrap();
        fs::create_dir_all(&dst).unwrap();
        fs::write(src.join("new.txt"), "new").unwrap();
        fs::write(dst.join("old.txt"), "old").unwrap();

        copy_dir_recursive(&src, &dst).unwrap();
        assert_eq!(fs::read_to_string(dst.join("new.txt")).unwrap(), "new");
    }

    // --- symlink_to_platform (integration with tempfile) ---

    #[test]
    fn test_symlink_to_platform_creates_link() {
        let tmp = tempfile::tempdir().unwrap();
        let home = tmp.path();

        // 设置环境：创建规范目录和平台目录
        let canonical = home.join(".agents").join("skills").join("test-skill");
        fs::create_dir_all(&canonical).unwrap();
        fs::write(canonical.join("SKILL.md"), "---\nname: test\n---\n").unwrap();

        let platform_skills = home.join(".codebuddy").join("skills");
        fs::create_dir_all(&platform_skills).unwrap();

        // 验证 symlink 创建
        let link = platform_skills.join("test-skill");
        assert!(!link.exists());

        // 手动创建 symlink（因为 symlink_to_platform 需要 Config，这里直接测试 utils 函数）
        let result = crate::utils::create_relative_symlink(&canonical, &link).unwrap();
        assert!(result);
        assert!(link.is_symlink());
        assert!(link.exists());
        assert!(link.join("SKILL.md").exists());
    }

    #[test]
    fn test_symlink_to_platform_auto_creates_platform_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let canonical = tmp.path().join(".agents").join("skills").join("test-skill");
        fs::create_dir_all(&canonical).unwrap();

        // 平台目录不存在
        let platform_skills = tmp.path().join(".newplatform").join("skills");
        assert!(!platform_skills.exists());

        // symlink 应自动创建父目录
        let link = platform_skills.join("test-skill");
        let result = crate::utils::create_relative_symlink(&canonical, &link).unwrap();
        assert!(result);
        assert!(link.is_symlink());
        assert!(platform_skills.exists());
    }
}
