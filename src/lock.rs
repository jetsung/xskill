use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

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

impl LockFile {
    /// 加载锁文件
    pub fn load(global: bool) -> Result<Self> {
        let path = lock_file_path(global);
        if !path.exists() {
            return Ok(Self::default());
        }

        let content = fs::read_to_string(&path)
            .with_context(|| format!("Failed to read lock file: {}", path.display()))?;

        let lock_file: Self = serde_json::from_str(&content)
            .unwrap_or_else(|_| {
                // 如果格式错误，返回默认值
                Self::default()
            });

        Ok(lock_file)
    }

    /// 保存锁文件
    pub fn save(&self, global: bool) -> Result<()> {
        let path = lock_file_path(global);

        // 确保目录存在
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent)
                    .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
            }
        }

        let content = serde_json::to_string_pretty(self)
            .with_context(|| "Failed to serialize lock file")?;

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
}
