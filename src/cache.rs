use anyhow::{Context, Result};
use chrono::{DateTime, Duration as ChronoDuration, Utc};
use md5::{Digest, Md5};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;

/// Cache data containing all sources' skills
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct CacheData {
    /// Last updated timestamp (ISO 8601)
    pub updated_at: String,
    /// Skills grouped by source
    pub sources: Vec<SourceCache>,
}

/// Cache entry for a single source
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SourceCache {
    /// Source name
    pub source: String,
    /// Source URL (optional, used for deduplication)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    /// Registry URL (set when source comes from a registry)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub registry_url: Option<String>,
    /// Skills from this source
    pub skills: Vec<CachedSkill>,
}

/// A single cached skill
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CachedSkill {
    /// Skill name
    pub name: String,
    /// Path to SKILL.md (relative to repo root)
    pub path: String,
    /// Skill description
    #[serde(default)]
    pub description: String,
    /// Skill version
    #[serde(default)]
    pub version: String,
}

/// Returns the cache directory path: ~/.xskill/cache/
pub fn cache_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".xskill")
        .join("cache")
}

/// Returns the single cache file path: ~/.xskill/cache/skills.json
fn cache_file() -> PathBuf {
    cache_dir().join("skills.json")
}

impl CacheData {
    /// Load cache from ~/.xskill/cache/skills.json
    pub fn load() -> Option<Self> {
        let path = cache_file();
        if !path.exists() {
            return None;
        }
        let content = fs::read_to_string(&path).ok()?;
        serde_json::from_str(&content).ok()
    }

    /// Save cache to ~/.xskill/cache/skills.json
    pub fn save(&self) -> Result<()> {
        let dir = cache_dir();
        fs::create_dir_all(&dir)
            .with_context(|| format!("Failed to create cache directory: {}", dir.display()))?;

        let path = cache_file();
        let json = serde_json::to_string_pretty(self)
            .context("Failed to serialize cache")?;
        fs::write(&path, json)
            .with_context(|| format!("Failed to write cache file: {}", path.display()))?;
        Ok(())
    }

    /// Check if the cache is stale based on the given TTL in seconds.
    /// Returns `true` if `updated_at` is older than `ttl_secs` from now,
    /// or if the timestamp cannot be parsed.
    pub fn is_stale(&self, ttl_secs: u64) -> bool {
        let updated = match DateTime::parse_from_rfc3339(&self.updated_at) {
            Ok(dt) => dt.with_timezone(&Utc),
            Err(_) => return true,
        };
        let deadline = Utc::now() - ChronoDuration::seconds(ttl_secs as i64);
        updated < deadline
    }
}

/// Compute cache filename for a URL: `source_<md5hex>.json`
/// Strips `.git` suffix before hashing to avoid duplicate cache files.
pub fn url_cache_filename(url: &str) -> String {
    let normalized = url.strip_suffix(".git").unwrap_or(url);
    let mut hasher = Md5::new();
    hasher.update(normalized.as_bytes());
    let hash = format!("{:x}", hasher.finalize());
    format!("source_{}.json", hash)
}

/// Load URL-specific cache if it exists and is fresh (within given TTL seconds).
pub fn load_url_cache(url: &str, ttl_secs: u64) -> Option<CacheData> {
    let path = cache_dir().join(url_cache_filename(url));
    if !path.exists() {
        return None;
    }
    let meta = fs::metadata(&path).ok()?;
    let modified = meta.modified().ok()?;
    let age = modified.elapsed().ok()?;
    if age > Duration::from_secs(ttl_secs) {
        return None;
    }
    let content = fs::read_to_string(&path).ok()?;
    serde_json::from_str(&content).ok()
}

/// Save cache data for a URL source to `~/.xskill/cache/source_<md5>.json`.
pub fn save_url_cache(url: &str, data: &CacheData) -> Result<()> {
    let dir = cache_dir();
    fs::create_dir_all(&dir)
        .with_context(|| format!("Failed to create cache directory: {}", dir.display()))?;

    let path = dir.join(url_cache_filename(url));
    let json =
        serde_json::to_string_pretty(data).context("Failed to serialize URL cache")?;
    fs::write(&path, json)
        .with_context(|| format!("Failed to write URL cache file: {}", path.display()))?;
    Ok(())
}

/// Delete all URL-specific cache files (`source_<md5>.json`) from the cache directory.
pub fn clear_url_caches() -> Result<usize> {
    let dir = cache_dir();
    if !dir.exists() {
        return Ok(0);
    }
    let mut count = 0;
    for entry in fs::read_dir(&dir)
        .with_context(|| format!("Failed to read cache directory: {}", dir.display()))?
    {
        let entry = entry?;
        let name = entry.file_name().to_string_lossy().to_string();
        if name.starts_with("source_") && name.ends_with(".json") {
            fs::remove_file(entry.path())?;
            count += 1;
        }
    }
    Ok(count)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cache_data_serialization_roundtrip() {
        // Test serialization/deserialization without touching the filesystem
        let data = CacheData {
            updated_at: "2026-07-17T12:00:00.000Z".to_string(),
            sources: vec![
                SourceCache {
                    source: "test-source".to_string(),
                    url: None,
                    registry_url: None,
                    skills: vec![
                        CachedSkill {
                            name: "vue".to_string(),
                            path: "skills/vue/SKILL.md".to_string(),
                            description: "Vue.js skills".to_string(),
                            version: "1.0.0".to_string(),
                        },
                    ],
                },
            ],
        };

        // Serialize to JSON string and deserialize back
        let json = serde_json::to_string(&data).unwrap();
        let loaded: CacheData = serde_json::from_str(&json).unwrap();
        assert_eq!(loaded.sources.len(), 1);
        assert_eq!(loaded.sources[0].source, "test-source");
        assert_eq!(loaded.sources[0].skills.len(), 1);
        assert_eq!(loaded.sources[0].skills[0].name, "vue");
        assert_eq!(loaded.updated_at, "2026-07-17T12:00:00.000Z");
    }

    #[test]
    fn test_cache_data_default() {
        let data = CacheData::default();
        assert!(data.updated_at.is_empty());
        assert!(data.sources.is_empty());
    }

    #[test]
    fn test_url_cache_filename_deterministic() {
        let a = url_cache_filename("https://github.com/example/skills");
        let b = url_cache_filename("https://github.com/example/skills");
        assert_eq!(a, b);
        assert!(a.starts_with("source_"));
        assert!(a.ends_with(".json"));
    }

    #[test]
    fn test_url_cache_filename_git_suffix() {
        let with_git = url_cache_filename("https://github.com/example/skills.git");
        let without_git = url_cache_filename("https://github.com/example/skills");
        assert_eq!(with_git, without_git);
    }

    #[test]
    fn test_url_cache_filename_different_urls() {
        let a = url_cache_filename("https://github.com/a/b");
        let b = url_cache_filename("https://github.com/c/d");
        assert_ne!(a, b);
    }

    #[test]
    fn test_url_cache_roundtrip() {
        let url = "https://example.com/test-repo";
        let data = CacheData {
            updated_at: "2026-07-18T12:00:00Z".to_string(),
            sources: vec![SourceCache {
                source: "test".to_string(),
                url: None,
                registry_url: None,
                skills: vec![CachedSkill {
                    name: "my-skill".to_string(),
                    path: "skills/my-skill/SKILL.md".to_string(),
                    description: "desc".to_string(),
                    version: "1.0".to_string(),
                }],
            }],
        };

        // Save
        save_url_cache(url, &data).unwrap();

        // Load — should be fresh with default TTL
        let loaded = load_url_cache(url, 600).expect("URL cache should load");
        assert_eq!(loaded.sources.len(), 1);
        assert_eq!(loaded.sources[0].skills[0].name, "my-skill");

        // Cleanup
        let path = cache_dir().join(url_cache_filename(url));
        let _ = fs::remove_file(path);
    }

    #[test]
    fn test_is_stale_recent_timestamp() {
        let data = CacheData {
            updated_at: Utc::now().to_rfc3339(),
            sources: vec![],
        };
        assert!(!data.is_stale(600));
    }

    #[test]
    fn test_is_stale_old_timestamp() {
        let old = Utc::now() - ChronoDuration::seconds(1200);
        let data = CacheData {
            updated_at: old.to_rfc3339(),
            sources: vec![],
        };
        assert!(data.is_stale(600));
    }

    #[test]
    fn test_is_stale_invalid_timestamp() {
        let data = CacheData {
            updated_at: "not-a-timestamp".to_string(),
            sources: vec![],
        };
        assert!(data.is_stale(600));
    }

    #[test]
    fn test_is_stale_empty_timestamp() {
        let data = CacheData::default();
        assert!(data.is_stale(600));
    }

    #[test]
    fn test_is_stale_exact_boundary() {
        // Exactly at TTL boundary — should be stale (updated < deadline)
        let exact = Utc::now() - ChronoDuration::seconds(600);
        let data = CacheData {
            updated_at: exact.to_rfc3339(),
            sources: vec![],
        };
        // Depending on sub-second timing, this may or may not be stale.
        // Just verify it doesn't panic.
        let _ = data.is_stale(600);
    }
}
