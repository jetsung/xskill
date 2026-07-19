use crate::config::Config;
use anyhow::{bail, Result};
use regex::Regex;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

/// 源解析结果
#[allow(dead_code)]
pub struct ResolvedSource {
    pub url: String,
    pub source_type: String,
}

/// 解析源来源为 URL
/// 优先级：config sources name → ORG/REPO 补全 → 原样 URL
pub fn resolve_source(config: &Config, source: &str) -> Result<ResolvedSource> {
    // 1. 在 sources 中按 effective_name 查找
    for s in &config.sources {
        if s.effective_name() == source {
            return Ok(ResolvedSource {
                url: normalize_url(&s.url),
                source_type: s.effective_type(),
            });
        }
    }

    // 2. ORG/REPO 格式 → GitHub URL
    if looks_like_org_repo(source) {
        return Ok(ResolvedSource {
            url: format!("https://github.com/{}", source),
            source_type: "git".to_string(),
        });
    }

    // 3. 原样作为 URL
    if source.starts_with("http://") || source.starts_with("https://") || source.starts_with("git@") {
        return Ok(ResolvedSource {
            url: source.to_string(),
            source_type: "git".to_string(),
        });
    }

    bail!("Source not found: {}", source)
}

/// 归一化 URL（处理 .git 后缀等）
pub fn normalize_url(url: &str) -> String {
    let url = url.trim_end_matches('/').to_string();
    if url.ends_with(".git") {
        // 保留 .git，git 需要它
    }
    url
}

/// 去除 .git 后缀用于比较
pub fn strip_git_suffix(url: &str) -> &str {
    url.strip_suffix(".git").unwrap_or(url)
}

/// 判断是否形如 ORG/REPO
pub fn looks_like_org_repo(s: &str) -> bool {
    let re = Regex::new(r"^[a-zA-Z0-9_.-]+/[a-zA-Z0-9_.-]+$").unwrap();
    re.is_match(s) && !s.contains("://") && !s.contains('@')
}

/// 验证 skill 路径安全性：仅允许 skills/ 内直接子目录名
pub fn validate_skill_path(skill_path: &str) -> Result<()> {
    if skill_path.is_empty() {
        bail!("Skill path cannot be empty");
    }

    let path = PathBuf::from(skill_path);

    // Reject absolute paths
    if path.is_absolute() {
        bail!("Invalid path: absolute path not allowed '{}'", skill_path);
    }

    // Reject paths containing ..
    for component in path.components() {
        let s = component.as_os_str().to_string_lossy();
        if s == ".." {
            bail!("Invalid path: path traversal rejected '{}'", skill_path);
        }
    }

    // 拒绝包含路径分隔符的（必须为直接子目录名）
    // 注意：这里允许嵌套名如 "category/skill" 用于安装源查找，
    // 但移除操作只允许单层
    Ok(())
}

/// 验证移除路径：仅允许 skills/ 内的单层子目录名
#[allow(dead_code)]
pub fn validate_remove_path(skill_path: &str) -> Result<()> {
    validate_skill_path(skill_path)?;

    let path = PathBuf::from(skill_path);
    // 移除操作不允许嵌套路径
    if path.components().count() > 1 {
        bail!("Invalid path: only direct subdirectories under skills/ allowed '{}'", skill_path);
    }

    Ok(())
}

/// 验证自定义路径安全性：必须在项目内，不可逃逸
#[allow(dead_code)]
pub fn validate_custom_path(path: &str) -> Result<()> {
    if path.is_empty() {
        bail!("Path cannot be empty");
    }

    let path_buf = PathBuf::from(path);

    // Reject absolute paths
    if path_buf.is_absolute() {
        bail!("Path out of bounds: {}", path);
    }

    // Reject paths containing ..
    for component in path_buf.components() {
        let s = component.as_os_str().to_string_lossy();
        if s == ".." {
            bail!("Path out of bounds: {}", path);
        }
    }

    // Reject paths starting with ~
    if path.starts_with('~') {
        bail!("Path out of bounds: {}", path);
    }

    Ok(())
}

/// 对比版本并返回变更描述
pub fn compare_versions(old_version: &str, new_version: &str) -> String {
    if old_version.is_empty() || new_version.is_empty() {
        return "Installed".to_string();
    }
    if old_version == new_version {
        format!("No change ({})", new_version)
    } else {
        format!("Updated ({} → {})", old_version, new_version)
    }
}

/// 环境变量替换：$VAR → 环境变量值，未定义则保留原样
#[allow(dead_code)]
pub fn expand_env_vars(s: &str) -> String {
    let re = Regex::new(r"\$([A-Z_][A-Z0-9_]*)").unwrap();
    re.replace_all(s, |caps: &regex::Captures| {
        let var_name = &caps[1];
        env::var(var_name).unwrap_or_else(|_| format!("${}", var_name))
    })
    .into_owned()
}

/// 获取本地 skills 目录路径
#[allow(dead_code)]
pub fn skills_dir() -> PathBuf {
    env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join("skills")
}

/// 获取规范目录（canonical directory）路径
/// - 全局：~/.agents/skills
/// - 本地：./.agents/skills
pub fn canonical_skills_dir(global: bool) -> PathBuf {
    if global {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".agents")
            .join("skills")
    } else {
        env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".agents")
            .join("skills")
    }
}

/// 创建相对路径软链接
/// - 已存在且指向同一目标 → 跳过，返回 true
/// - 已存在但指向不同目标 → 删除重建
/// - 创建失败 → 返回 false（调用方应回退到文件复制）
/// - 自动创建父目录
pub fn create_relative_symlink(target: &Path, link_path: &Path) -> Result<bool> {
    // 如果链接已存在
    if link_path.exists() || link_path.is_symlink() {
        // 解析现有链接目标
        if link_path.is_symlink() {
            if let Ok(existing_target) = fs::read_link(link_path) {
                // 解析为绝对路径进行比较
                let resolved_existing = if existing_target.is_absolute() {
                    existing_target.clone()
                } else {
                    link_path.parent().unwrap_or(link_path).join(&existing_target)
                };
                let resolved_target = if target.is_absolute() {
                    target.to_path_buf()
                } else {
                    // 规范化目标路径
                    std::fs::canonicalize(target).unwrap_or_else(|_| target.to_path_buf())
                };
                // 尝试规范化两者
                let canon_existing = std::fs::canonicalize(&resolved_existing).unwrap_or(resolved_existing);
                let canon_target = std::fs::canonicalize(&resolved_target).unwrap_or(resolved_target);
                if canon_existing == canon_target {
                    return Ok(true); // 相同目标，跳过
                }
            }
            // 不同目标，删除旧链接
            fs::remove_file(link_path)?;
        } else {
            // 是一个实际目录，删除
            fs::remove_dir_all(link_path)?;
        }
    }

    // 创建父目录
    if let Some(parent) = link_path.parent() {
        fs::create_dir_all(parent)?;
    }

    // 计算相对路径
    let link_dir = link_path.parent().unwrap_or(Path::new("."));
    let relative_path = pathdiff::diff_paths(target, link_dir)
        .unwrap_or_else(|| target.to_path_buf());

    // 创建软链接
    #[cfg(unix)]
    {
        match std::os::unix::fs::symlink(&relative_path, link_path) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false), // 创建失败，调用方回退到 copy
        }
    }
    #[cfg(windows)]
    {
        // Windows: 目录使用 junction 或 symlink
        match std::os::windows::fs::symlink_dir(&relative_path, link_path) {
            Ok(()) => Ok(true),
            Err(_) => Ok(false),
        }
    }
}

/// 安全删除软链接（不删除目标内容）
pub fn remove_symlink(link_path: &Path) -> Result<()> {
    if link_path.is_symlink() {
        fs::remove_file(link_path)?;
    } else if link_path.exists() {
        fs::remove_dir_all(link_path)?;
    }
    Ok(())
}

/// 格式化路径显示：将 home 目录替换为 ~/
pub fn display_path(path: &Path) -> String {
    let path_str = path.display().to_string();
    if let Some(home) = dirs::home_dir() {
        let home_str = home.display().to_string();
        if path_str.starts_with(&home_str) {
            return path_str.replacen(&home_str, "~", 1);
        }
    }
    path_str
}

/// 验证 agent/平台名称是否存在于配置中
/// - `*` 通配符直接放行
/// - 具体名称不存在时报错，使用统一格式：
///   Invalid agents: <输入值>（褐色）
///   Valid agents: <平台1>, <平台2>, ...（黑灰色）
pub fn validate_agent(config: &Config, agent: &str) -> Result<()> {
    if agent == "*" {
        return Ok(());
    }
    if config.get_platform(agent).is_some() {
        return Ok(());
    }
    let available = config.platform_names();
    bail!("{}", AgentValidationError {
        agent: agent.to_string(),
        available: available.iter().map(|s| s.to_string()).collect(),
    });
}

/// 自定义错误类型，确保 ANSI 颜色码不被 anyhow 截断
struct AgentValidationError {
    agent: String,
    available: Vec<String>,
}

impl std::fmt::Display for AgentValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        use colored::Colorize;
        write!(
            f,
            "{}\n{}",
            format!("Invalid agents: {}", self.agent).yellow(),
            format!("Valid agents: {}", self.available.join(", ")).bright_black()
        )
    }
}

impl std::fmt::Debug for AgentValidationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}

impl std::error::Error for AgentValidationError {}

/// 获取 URL 末尾路径名（不含 .git 后缀）
#[allow(dead_code)]
pub fn url_last_segment(url: &str) -> String {
    let url = url.trim_end_matches('/');
    let url = url.trim_end_matches(".git");
    Path::new(url)
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_else(|| url.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_source_by_name() {
        let mut config = crate::config::Config::default();
        config.sources.push(crate::config::Source {
            name: "antfu".to_string(),
            source_type: "git".to_string(),
            url: "https://github.com/antfu/skills".to_string(),
        });
        let resolved = resolve_source(&config, "antfu").unwrap();
        assert_eq!(resolved.url, "https://github.com/antfu/skills");
    }

    #[test]
    fn test_resolve_source_org_repo() {
        let config = crate::config::Config::default();
        let resolved = resolve_source(&config, "antfu/skills").unwrap();
        assert_eq!(resolved.url, "https://github.com/antfu/skills");
    }

    #[test]
    fn test_resolve_source_url() {
        let config = crate::config::Config::default();
        let resolved = resolve_source(&config, "https://example.com/repo.git").unwrap();
        assert_eq!(resolved.url, "https://example.com/repo.git");
    }

    #[test]
    fn test_resolve_source_not_found() {
        let config = crate::config::Config::default();
        assert!(resolve_source(&config, "nonexistent").is_err());
    }

    #[test]
    fn test_normalize_url() {
        assert_eq!(normalize_url("https://example.com/repo"), "https://example.com/repo");
        assert_eq!(normalize_url("https://example.com/repo/"), "https://example.com/repo");
        assert_eq!(normalize_url("https://example.com/repo.git"), "https://example.com/repo.git");
    }

    #[test]
    fn test_looks_like_org_repo() {
        assert!(looks_like_org_repo("antfu/skills"));
        assert!(looks_like_org_repo("org/repo.name"));
        assert!(!looks_like_org_repo("https://example.com"));
        assert!(!looks_like_org_repo("just-a-name"));
        assert!(!looks_like_org_repo("org/repo/extra"));
    }

    #[test]
    fn test_validate_skill_path_ok() {
        assert!(validate_skill_path("my-skill").is_ok());
        assert!(validate_skill_path("category/skill").is_ok());
    }

    #[test]
    fn test_validate_skill_path_empty() {
        assert!(validate_skill_path("").is_err());
    }

    #[test]
    fn test_validate_skill_path_absolute() {
        assert!(validate_skill_path("/etc/passwd").is_err());
    }

    #[test]
    fn test_validate_skill_path_traversal() {
        assert!(validate_skill_path("../etc/passwd").is_err());
        assert!(validate_skill_path("foo/../bar").is_err());
    }

    #[test]
    fn test_validate_custom_path_ok() {
        assert!(validate_custom_path("my-dir").is_ok());
        assert!(validate_custom_path("sub/dir").is_ok());
    }

    #[test]
    fn test_validate_custom_path_empty() {
        assert!(validate_custom_path("").is_err());
    }

    #[test]
    fn test_validate_custom_path_absolute() {
        assert!(validate_custom_path("/tmp/foo").is_err());
    }

    #[test]
    fn test_validate_custom_path_traversal() {
        assert!(validate_custom_path("../escape").is_err());
    }

    #[test]
    fn test_validate_custom_path_tilde() {
        assert!(validate_custom_path("~/secret").is_err());
    }

    #[test]
    fn test_compare_versions() {
        assert_eq!(compare_versions("", ""), "Installed");
        assert_eq!(compare_versions("", "1.0.0"), "Installed");
        assert_eq!(compare_versions("1.0.0", "1.0.0"), "No change (1.0.0)");
        assert_eq!(compare_versions("1.0.0", "2.0.0"), "Updated (1.0.0 → 2.0.0)");
    }

    #[test]
    fn test_url_last_segment() {
        assert_eq!(url_last_segment("https://example.com/repo.git"), "repo");
        assert_eq!(url_last_segment("https://example.com/repo"), "repo");
        assert_eq!(url_last_segment("https://example.com/repo/"), "repo");
    }

    // --- canonical_skills_dir ---

    #[test]
    fn test_canonical_skills_dir_local() {
        let dir = canonical_skills_dir(false);
        assert!(dir.ends_with(".agents/skills"));
        // 本地模式使用 cwd 作为基础
        let cwd = env::current_dir().unwrap();
        assert!(dir.starts_with(&cwd));
    }

    #[test]
    fn test_canonical_skills_dir_global() {
        let dir = canonical_skills_dir(true);
        assert!(dir.ends_with(".agents/skills"));
        let home = dirs::home_dir().unwrap_or_default();
        assert!(dir.starts_with(home));
    }

    // --- create_relative_symlink ---

    #[test]
    fn test_create_relative_symlink_basic() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("target_dir");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("file.txt"), "hello").unwrap();

        let link = tmp.path().join("link_dir");
        let result = create_relative_symlink(&target, &link).unwrap();
        assert!(result);
        assert!(link.is_symlink());
        assert!(link.exists());
        assert_eq!(fs::read_to_string(link.join("file.txt")).unwrap(), "hello");
    }

    #[test]
    fn test_create_relative_symlink_idempotent() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("target_dir");
        fs::create_dir_all(&target).unwrap();

        let link = tmp.path().join("link_dir");
        let r1 = create_relative_symlink(&target, &link).unwrap();
        let r2 = create_relative_symlink(&target, &link).unwrap();
        assert!(r1);
        assert!(r2); // 同一目标，跳过
    }

    #[test]
    fn test_create_relative_symlink_different_target_rebuilds() {
        let tmp = tempfile::tempdir().unwrap();
        let target1 = tmp.path().join("target1");
        let target2 = tmp.path().join("target2");
        fs::create_dir_all(&target1).unwrap();
        fs::create_dir_all(&target2).unwrap();
        fs::write(target1.join("a.txt"), "a").unwrap();
        fs::write(target2.join("b.txt"), "b").unwrap();

        let link = tmp.path().join("link");
        create_relative_symlink(&target1, &link).unwrap();
        assert_eq!(fs::read_to_string(link.join("a.txt")).unwrap(), "a");

        // 指向不同目标，重建
        create_relative_symlink(&target2, &link).unwrap();
        assert!(link.is_symlink());
        assert_eq!(fs::read_to_string(link.join("b.txt")).unwrap(), "b");
        // 旧目标不再通过链接可达
        assert!(!link.join("a.txt").exists());
    }

    #[test]
    fn test_create_relative_symlink_creates_parent_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("target");
        fs::create_dir_all(&target).unwrap();

        let link = tmp.path().join("nested").join("deep").join("link");
        let result = create_relative_symlink(&target, &link).unwrap();
        assert!(result);
        assert!(link.is_symlink());
        assert!(link.exists());
    }

    #[test]
    fn test_create_relative_symlink_uses_relative_path() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("target");
        fs::create_dir_all(&target).unwrap();

        let link = tmp.path().join("link");
        create_relative_symlink(&target, &link).unwrap();

        let readback = fs::read_link(&link).unwrap();
        // 相对路径不应包含绝对路径
        assert!(!readback.is_absolute());
        assert!(!readback.to_string_lossy().contains('/'));
    }

    // --- remove_symlink ---

    #[test]
    fn test_remove_symlink_removes_link_only() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("target");
        fs::create_dir_all(&target).unwrap();
        fs::write(target.join("file.txt"), "data").unwrap();

        let link = tmp.path().join("link");
        create_relative_symlink(&target, &link).unwrap();
        assert!(link.is_symlink());

        remove_symlink(&link).unwrap();
        assert!(!link.exists());
        // 目标不受影响
        assert!(target.exists());
        assert_eq!(fs::read_to_string(target.join("file.txt")).unwrap(), "data");
    }

    #[test]
    fn test_remove_symlink_removes_dir() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("some_dir");
        fs::create_dir_all(dir.join("sub")).unwrap();
        fs::write(dir.join("file.txt"), "data").unwrap();

        remove_symlink(&dir).unwrap();
        assert!(!dir.exists());
    }

    #[test]
    fn test_remove_symlink_nonexistent_is_noop() {
        let tmp = tempfile::tempdir().unwrap();
        let path = tmp.path().join("does_not_exist");
        // 不应报错
        remove_symlink(&path).unwrap();
    }

    #[test]
    fn test_remove_symlink_dangling() {
        let tmp = tempfile::tempdir().unwrap();
        let target = tmp.path().join("target");
        fs::create_dir_all(&target).unwrap();

        let link = tmp.path().join("link");
        create_relative_symlink(&target, &link).unwrap();
        assert!(link.is_symlink());

        // 删除目标使链接断裂
        fs::remove_dir_all(&target).unwrap();
        assert!(!link.exists()); // 目标不存在时链接 "不存在"
        assert!(link.is_symlink()); // 但 is_symlink 仍为 true

        remove_symlink(&link).unwrap();
        assert!(!link.is_symlink());
    }

    // --- display_path ---

    #[test]
    fn test_display_path_with_home() {
        let home = dirs::home_dir().unwrap();
        let path = home.join(".agents").join("skills");
        let displayed = display_path(&path);
        assert!(displayed.starts_with('~'), "expected ~/... prefix, got: {}", displayed);
        assert!(displayed.contains(".agents"));
        assert!(!displayed.contains(&home.display().to_string()));
    }

    #[test]
    fn test_display_path_without_home() {
        let path = PathBuf::from("/tmp/something");
        let displayed = display_path(&path);
        assert_eq!(displayed, "/tmp/something");
    }

    #[test]
    fn test_display_path_relative() {
        let path = PathBuf::from(".agents/skills");
        let displayed = display_path(&path);
        assert_eq!(displayed, ".agents/skills");
    }

    // --- validate_agent ---

    #[test]
    fn test_validate_agent_wildcard() {
        let config = crate::config::Config::default();
        assert!(validate_agent(&config, "*").is_ok());
    }

    #[test]
    fn test_validate_agent_valid_platform() {
        let mut config = crate::config::Config::default();
        config.platforms.insert(
            "claude".to_string(),
            crate::config::Platform {
                path: ".claude".to_string(),
                skills: "skills".to_string(),
                agents: "CLAUDE.md".to_string(),
                source: "AGENTS.md".to_string(),
                agents_compat: false,
            },
        );
        assert!(validate_agent(&config, "claude").is_ok());
    }

    #[test]
    fn test_validate_agent_invalid_platform() {
        let config = crate::config::Config::default();
        let result = validate_agent(&config, "nonexistent");
        assert!(result.is_err());
        let err = result.unwrap_err().to_string();
        assert!(err.contains("Invalid agents: nonexistent"));
        assert!(err.contains("Valid agents:"));
    }
}
