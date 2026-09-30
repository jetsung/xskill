use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::PathBuf;

use crate::utils::looks_like_org_repo;

/// 锁文件结构
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LockFile {
    pub version: u32,
    pub skills: HashMap<String, LockEntry>,
    /// 锁文件最后更新时间（增删改 skill 时更新）
    #[serde(default)]
    pub updated_at: String,
}

impl Default for LockFile {
    fn default() -> Self {
        Self {
            version: 1,
            skills: HashMap::new(),
            updated_at: String::new(),
        }
    }
}

/// 锁文件条目
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LockEntry {
    /// 源名称（-f 传入的参数值）
    pub source: String,
    /// 源类型（git 或 api）
    pub source_type: String,
    /// 源 URL
    pub source_url: String,
    /// skill 相对于 Git 仓库的路径
    pub skill_path: String,
    /// skill 文件夹的 git tree hash
    pub skill_folder_hash: String,
    /// 首次安装时间
    pub installed_at: String,
    /// 最后更新时间
    pub updated_at: String,
}

/// 锁文件路径
fn lock_file_path(global: bool) -> PathBuf {
    if global {
        // 全局：~/.agents/.xskill-lock.json
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".agents")
            .join(".xskill-lock.json")
    } else {
        // 项目级：.xskill-lock.json（当前工作目录）
        std::env::current_dir()
            .unwrap_or_else(|_| PathBuf::from("."))
            .join(".xskill-lock.json")
    }
}

/// 归一化存量简写记录：`source` 为 `owner/repo` 简写、且 `source_url` 恰为其
/// GitHub URL、且不是 `settings.json` 中已配置渠道的名称时，将 `source` 替换为
/// 完整 URL；其余形式（已配置渠道名、完整 URL、source_url 与简写不对应的记录）
/// 保持不变，保证幂等。
fn normalize_legacy_source_entries(
    skills: &mut HashMap<String, LockEntry>,
    configured_names: &HashSet<String>,
) {
    for entry in skills.values_mut() {
        if !configured_names.contains(&entry.source)
            && looks_like_org_repo(&entry.source)
            && entry.source_url == format!("https://github.com/{}", entry.source)
        {
            entry.source = entry.source_url.clone();
        }
    }
}

/// 收集 `settings.json` 中所有渠道的有效名称（含形如 `owner/repo` 的名称）。
/// 配置加载失败时返回空集合，归一化退化为纯形式判断。
fn configured_source_names() -> HashSet<String> {
    match crate::config::Config::load() {
        Ok(config) => config.sources.iter().map(|s| s.effective_name()).collect(),
        Err(_) => HashSet::new(),
    }
}

impl LockFile {
    /// 加载锁文件
    pub fn load(global: bool) -> Result<Self> {
        let path = lock_file_path(global);
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read lock file: {}", path.display()))?;

        let lock_file: Self = serde_json::from_str(&content).unwrap_or_else(|_| {
            // 如果格式错误，返回默认值
            Self::default()
        });

        let mut lock_file = lock_file;
        let configured_names = configured_source_names();
        normalize_legacy_source_entries(&mut lock_file.skills, &configured_names);
        Ok(lock_file)
    }

    /// 保存锁文件
    pub fn save(&self, global: bool) -> Result<()> {
        let path = lock_file_path(global);

        // 确保目录存在
        if let Some(parent) = path.parent()
            && !parent.exists()
        {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }

        let content =
            serde_json::to_string_pretty(self).with_context(|| "Failed to serialize lock file")?;

        fs::write(&path, content)
            .with_context(|| format!("Failed to write lock file: {}", path.display()))?;

        Ok(())
    }

    /// 添加或更新 skill 记录
    pub fn upsert_skill(&mut self, name: &str, entry: LockEntry) {
        self.skills.insert(name.to_string(), entry);
    }

    /// 删除 skill 记录
    pub fn remove_skill(&mut self, name: &str) {
        self.skills.remove(name);
    }

    /// 清空所有 skill 记录
    pub fn clear_skills(&mut self) {
        self.skills.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_entry(name: &str) -> LockEntry {
        LockEntry {
            source: "test-source".to_string(),
            source_type: "git".to_string(),
            source_url: format!("https://example.com/{}.git", name),
            skill_path: format!("skills/{}/SKILL.md", name),
            skill_folder_hash: "abc123".to_string(),
            installed_at: "2026-07-17T00:00:00.000Z".to_string(),
            updated_at: "2026-07-17T00:00:00.000Z".to_string(),
        }
    }

    #[test]
    fn test_default_lock_file() {
        let lock = LockFile::default();
        assert_eq!(lock.version, 1);
        assert!(lock.skills.is_empty());
        assert!(lock.updated_at.is_empty());
    }

    #[test]
    fn test_upsert_and_remove() {
        let mut lock = LockFile::default();
        lock.upsert_skill("vue", make_entry("vue"));
        assert_eq!(lock.skills.len(), 1);
        assert!(lock.skills.contains_key("vue"));

        lock.upsert_skill("react", make_entry("react"));
        assert_eq!(lock.skills.len(), 2);

        lock.remove_skill("vue");
        assert_eq!(lock.skills.len(), 1);
        assert!(!lock.skills.contains_key("vue"));
    }

    #[test]
    fn test_clear_skills() {
        let mut lock = LockFile::default();
        lock.upsert_skill("a", make_entry("a"));
        lock.upsert_skill("b", make_entry("b"));
        assert_eq!(lock.skills.len(), 2);

        lock.clear_skills();
        assert!(lock.skills.is_empty());
    }

    #[test]
    fn test_upsert_overwrites() {
        let mut lock = LockFile::default();
        lock.upsert_skill("vue", make_entry("vue"));
        let mut updated = make_entry("vue");
        updated.skill_folder_hash = "newhash".to_string();
        lock.upsert_skill("vue", updated);
        assert_eq!(lock.skills.len(), 1);
        assert_eq!(lock.skills["vue"].skill_folder_hash, "newhash");
    }

    fn entry_with(source: &str, source_url: &str) -> LockEntry {
        LockEntry {
            source: source.to_string(),
            source_type: "git".to_string(),
            source_url: source_url.to_string(),
            skill_path: "skills/x/SKILL.md".to_string(),
            skill_folder_hash: "abc123".to_string(),
            installed_at: "2026-07-17T00:00:00.000Z".to_string(),
            updated_at: "2026-07-17T00:00:00.000Z".to_string(),
        }
    }

    #[test]
    fn test_normalize_legacy_shorthand_source() {
        // 存量简写记录：source 为 owner/repo 且 source_url 恰为其 GitHub URL → 替换
        let mut skills = HashMap::new();
        skills.insert(
            "archify".to_string(),
            entry_with("tt-a1i/archify", "https://github.com/tt-a1i/archify"),
        );
        normalize_legacy_source_entries(&mut skills, &HashSet::new());
        assert_eq!(
            skills["archify"].source,
            "https://github.com/tt-a1i/archify"
        );
    }

    #[test]
    fn test_normalize_keeps_source_name() {
        // 已配置源名（无斜杠）不被修改
        let mut skills = HashMap::new();
        skills.insert(
            "vue".to_string(),
            entry_with("myskills", "https://github.com/example/skills.git"),
        );
        normalize_legacy_source_entries(&mut skills, &HashSet::new());
        assert_eq!(skills["vue"].source, "myskills");
    }

    #[test]
    fn test_normalize_keeps_mismatched_url() {
        // source 为简写但 source_url 与之不对应 → 不修改
        let mut skills = HashMap::new();
        skills.insert(
            "x".to_string(),
            entry_with("a/b", "https://gitcode.com/example/repo.git"),
        );
        normalize_legacy_source_entries(&mut skills, &HashSet::new());
        assert_eq!(skills["x"].source, "a/b");
    }

    #[test]
    fn test_normalize_idempotent() {
        // 已归一化（source 为完整 URL）再次处理不变
        let mut skills = HashMap::new();
        skills.insert(
            "archify".to_string(),
            entry_with(
                "https://github.com/tt-a1i/archify",
                "https://github.com/tt-a1i/archify",
            ),
        );
        normalize_legacy_source_entries(&mut skills, &HashSet::new());
        assert_eq!(
            skills["archify"].source,
            "https://github.com/tt-a1i/archify"
        );
    }

    #[test]
    fn test_normalize_keeps_configured_source_name() {
        // source 为已配置渠道名（形如 owner/repo）→ 即使形式匹配也不归一化
        let mut skills = HashMap::new();
        skills.insert(
            "vue".to_string(),
            entry_with("jetsung/skills", "https://github.com/jetsung/skills"),
        );
        let mut configured = HashSet::new();
        configured.insert("jetsung/skills".to_string());
        normalize_legacy_source_entries(&mut skills, &configured);
        assert_eq!(skills["vue"].source, "jetsung/skills");
        // 未列入配置时按形式规则归一化
        normalize_legacy_source_entries(&mut skills, &HashSet::new());
        assert_eq!(skills["vue"].source, "https://github.com/jetsung/skills");
    }

    #[test]
    fn test_load_save_roundtrip_normalizes_legacy_entry() {
        // 文件级全链路：磁盘上的存量简写记录 → load 归一化 → save 落盘为完整 URL
        // 本测试切换进程工作目录，须与依赖 cwd/XSKILL_CONFIG 的测试串行
        let _guard = crate::config::test_env_lock()
            .lock()
            .unwrap_or_else(|e| e.into_inner());
        let tmp = tempfile::tempdir().unwrap();
        let lock_path = tmp.path().join(".xskill-lock.json");
        let mut skills = HashMap::new();
        skills.insert(
            "archify".to_string(),
            entry_with("tt-a1i/archify", "https://github.com/tt-a1i/archify"),
        );
        skills.insert(
            "vue".to_string(),
            entry_with("myskills", "https://github.com/example/skills.git"),
        );
        let lock = LockFile {
            version: 1,
            skills,
            updated_at: "2026-09-30T00:00:00.000Z".to_string(),
        };
        std::fs::write(&lock_path, serde_json::to_string_pretty(&lock).unwrap()).unwrap();

        // LockFile::load(false)/save(false) 基于当前工作目录解析，切到临时目录
        let orig_cwd = std::env::current_dir().unwrap();
        std::env::set_current_dir(tmp.path()).unwrap();
        let loaded = LockFile::load(false).unwrap();
        loaded.save(false).unwrap();
        std::env::set_current_dir(orig_cwd).unwrap();

        let saved: LockFile =
            serde_json::from_str(&std::fs::read_to_string(&lock_path).unwrap()).unwrap();
        // 存量简写记录落盘为完整 URL
        assert_eq!(
            saved.skills["archify"].source,
            "https://github.com/tt-a1i/archify"
        );
        // 源名记录不受影响
        assert_eq!(saved.skills["vue"].source, "myskills");
    }
}
