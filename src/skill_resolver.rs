use crate::cache::{self, CacheData, CachedSkill, SourceCache};
use crate::config::Config;
use crate::git;
use crate::skill_meta::SkillMeta;
use crate::utils::{normalize_url, resolve_source};
use anyhow::Result;
use colored::Colorize;
use std::collections::HashSet;
use std::fs;

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Resolve skills based on configuration and options.
///
/// Returns a `CacheData` that both `find` and `query` can consume.
///
/// Priority:
/// 1. If `from` is specified → load from that source directly (no cache)
/// 2. If registry.enabled → merge central + local sources
/// 3. If registry not enabled:
///    - If cache.enabled → load from cache (fallback to cloning if missing)
///    - Else → clone all sources real-time
pub fn resolve_skills(config: &Config, from: Option<&str>) -> Result<CacheData> {
    // 1. --from specified: load directly from that source
    if let Some(src) = from {
        return resolve_from_source(config, src);
    }

    // 2. Central repo enabled: merge central + local
    if config.is_registry_enabled() {
        let central_data = fetch_registry(config);
        let local_data = load_local_skills(config);
        return Ok(merge_skills(local_data, central_data));
    }

    // 3. Central repo not enabled
    if config.is_cache_enabled() {
        // Try cache first, check staleness and empty sources
        if let Some(cached) = CacheData::load() {
            let stale = cached.is_stale(config.cache.ttl);
            let empty_but_sources_exist =
                cached.sources.is_empty() && !config.sources.is_empty();
            if !stale && !empty_but_sources_exist {
                return Ok(cached);
            }
        }
        // Cache missing, stale, or empty with configured sources — refresh
        eprintln!(
            "{}: Cache missing or stale, fetching from sources directly.",
            "Warning".yellow()
        );
        let fresh = clone_all_sources(config);
        // Persist refreshed data so subsequent calls hit fresh cache
        let _ = fresh.save();
        return Ok(fresh);
    }

    // Cache not enabled — clone all sources
    Ok(clone_all_sources(config))
}

/// A matched skill from a specific source (for disambiguation).
#[derive(Debug, Clone)]
pub struct SkillMatch {
    pub source_name: String,
    pub source_url: String,
    pub skill_path: String,
    pub is_registry: bool,
}

/// Find ALL matching skills across all sources (for disambiguation).
///
/// Unlike `find_skill` which returns the first match, this returns every
/// source that contains a skill with the given name, including registry.
pub fn find_all_skills(
    config: &Config,
    skill_name: &str,
    prefer_source: Option<&str>,
) -> Vec<SkillMatch> {
    let mut results = Vec::new();
    let mut seen_urls: HashSet<String> = HashSet::new();

    // 1. Try preferred source first
    if let Some(src) = prefer_source {
        if let Some(m) = search_source_for_match(config, src, skill_name) {
            seen_urls.insert(normalize_url(&m.source_url));
            results.push(m);
        }
    }

    // 2. Search local cache
    if let Some(cached) = CacheData::load() {
        for source_cache in &cached.sources {
            let url = source_cache
                .url
                .clone()
                .unwrap_or_else(|| source_cache.source.clone());
            let norm = normalize_url(&url);
            if seen_urls.contains(&norm) {
                continue;
            }
            for skill in &source_cache.skills {
                if skill.name == skill_name {
                    let source_name = if source_cache.source.is_empty() {
                        url.clone()
                    } else {
                        source_cache.source.clone()
                    };
                    let is_registry = source_cache.registry_url.is_some();
                    seen_urls.insert(norm.clone());
                    results.push(SkillMatch {
                        source_name,
                        source_url: url.clone(),
                        skill_path: skill.path.clone(),
                        is_registry,
                    });
                    break;
                }
            }
        }
    }

    // 3. Search all configured sources (clone)
    for source in &config.sources {
        if Some(source.effective_name().as_str()) == prefer_source {
            continue;
        }
        let norm = normalize_url(&source.url);
        if seen_urls.contains(&norm) {
            continue;
        }
        if let Some(m) = search_source_for_match(config, &source.effective_name(), skill_name) {
            seen_urls.insert(norm);
            results.push(m);
        }
    }

    // 4. Search registry (may return multiple matches from different sources)
    if config.is_registry_enabled() {
        let registry_matches = search_registry_for_match(config, skill_name, &seen_urls);
        results.extend(registry_matches);
    }

    results
}

/// Search a specific source for a skill match.
fn search_source_for_match(
    config: &Config,
    src: &str,
    skill_name: &str,
) -> Option<SkillMatch> {
    let resolved = resolve_source(config, src).ok()?;
    let url = normalize_url(&resolved.url);
    let tmp_dir = git::clone_for_listing(&url).ok()?;
    let skills_dir = tmp_dir.path().join("skills");

    if !skills_dir.exists() {
        return None;
    }

    let source_name = config
        .get_source(src)
        .map(|s| s.effective_name())
        .unwrap_or_else(|| src.to_string());

    search_skills_in_dir(&skills_dir, skill_name, &source_name, &resolved.url, "")
        .map(|(name, url, path)| SkillMatch {
            source_name: name,
            source_url: url,
            skill_path: path,
            is_registry: false,
        })
}

/// Search the registry for a skill match, skipping already-seen URLs.
fn search_registry_for_match(
    config: &Config,
    skill_name: &str,
    seen_urls: &HashSet<String>,
) -> Vec<SkillMatch> {
    let central_url = match config.effective_registry_url() {
        u if !u.is_empty() => u,
        _ => return Vec::new(),
    };
    let body = match fetch_json(&central_url) {
        Ok(b) => b,
        Err(_) => return Vec::new(),
    };
    let cache_data: CacheData = match serde_json::from_str(&body) {
        Ok(d) => d,
        Err(_) => return Vec::new(),
    };

    // Collect local source names for conflict detection
    let local_names: HashSet<String> = config
        .sources
        .iter()
        .map(|s| s.effective_name())
        .collect();

    let mut results = Vec::new();

    for source_cache in &cache_data.sources {
        let src_url = source_cache.url.as_deref().unwrap_or("");
        if !src_url.is_empty() && seen_urls.contains(&normalize_url(src_url)) {
            continue;
        }

        for skill in &source_cache.skills {
            if skill.name == skill_name {
                let url = source_cache
                    .url
                    .clone()
                    .unwrap_or_else(|| source_cache.source.clone());

                // Source name rules (matching find command behavior):
                // - Empty → "-"
                // - Same name as local source but different URL → "-"
                // - Name equals URL → "-"
                let display_name = if source_cache.source.is_empty() {
                    "-".to_string()
                } else if local_names.contains(source_cache.source.as_str())
                    && normalize_url(&source_cache.source) != normalize_url(&url)
                {
                    "-".to_string()
                } else if source_cache.source == url {
                    "-".to_string()
                } else {
                    source_cache.source.clone()
                };

                results.push(SkillMatch {
                    source_name: display_name,
                    source_url: url,
                    skill_path: skill.path.clone(),
                    is_registry: true,
                });
                break; // one match per source
            }
        }
    }

    results
}

// ---------------------------------------------------------------------------
// Source resolution
// ---------------------------------------------------------------------------

/// Resolve skills from a specific source (--from flag).
fn resolve_from_source(config: &Config, src: &str) -> Result<CacheData> {
    // If it's a URL not in configured sources, use URL cache
    if is_url(src) && config.get_source(src).is_none() {
        return load_or_fetch_url_cache(src, config.cache.ttl);
    }

    // Otherwise resolve as a named source
    let resolved = resolve_source(config, src)?;
    let skills = clone_and_collect(&resolved.url)?;

    let source_name = config
        .get_source(src)
        .map(|s| s.effective_name())
        .unwrap_or_else(|| src.to_string());

    Ok(CacheData {
        updated_at: chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string(),
        sources: vec![SourceCache {
            source: source_name,
            url: Some(resolved.url),
            registry_url: None,
            skills,
        }],
    })
}

// ---------------------------------------------------------------------------
// Local skills loading
// ---------------------------------------------------------------------------

/// Load local skills from cache or by cloning sources.
/// Checks TTL and empty sources before accepting cached data.
/// Persists refreshed data back to cache when applicable.
fn load_local_skills(config: &Config) -> CacheData {
    if config.is_cache_enabled() {
        if let Some(cached) = CacheData::load() {
            let stale = cached.is_stale(config.cache.ttl);
            let empty_but_sources_exist =
                cached.sources.is_empty() && !config.sources.is_empty();
            if !stale && !empty_but_sources_exist {
                return cached;
            }
        }
        let fresh = clone_all_sources(config);
        let _ = fresh.save();
        return fresh;
    }
    clone_all_sources(config)
}

/// Clone all configured sources and collect skills.
fn clone_all_sources(config: &Config) -> CacheData {
    let mut sources = Vec::new();

    for source in &config.sources {
        let source_name = source.effective_name();
        match clone_and_collect(&source.url) {
            Ok(skills) => {
                sources.push(SourceCache {
                    source: source_name,
                    url: Some(source.url.clone()),
                    registry_url: None,
                    skills,
                });
            }
            Err(e) => {
                eprintln!(
                    "{}: Failed to fetch source '{}': {}",
                    "Warning".yellow(),
                    source_name,
                    e
                );
            }
        }
    }

    CacheData {
        updated_at: chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string(),
        sources,
    }
}

// ---------------------------------------------------------------------------
// Registry
// ---------------------------------------------------------------------------

/// Fetch skills from the registry.
fn fetch_registry(config: &Config) -> CacheData {
    let central_url = config.effective_registry_url();

    let body = match fetch_json(&central_url) {
        Ok(b) => b,
        Err(e) => {
            eprintln!(
                "{}: Failed to fetch registry: {}",
                "Warning".yellow(),
                e
            );
            return CacheData::default();
        }
    };

    match serde_json::from_str::<CacheData>(&body) {
        Ok(d) => d,
        Err(e) => {
            eprintln!(
                "{}: Failed to parse registry response: {}",
                "Warning".yellow(),
                e
            );
            CacheData::default()
        }
    }
}

/// Merge local and central skills, preferring local for duplicates.
fn merge_skills(local: CacheData, central: CacheData) -> CacheData {
    // Collect normalized local source URLs for deduplication
    let local_urls: HashSet<String> = local
        .sources
        .iter()
        .filter_map(|s| s.url.as_deref())
        .map(|u| normalize_url(u))
        .collect();

    // Collect local source names for name-based dedup
    let local_names: HashSet<String> = local
        .sources
        .iter()
        .map(|s| s.source.clone())
        .collect();

    let mut merged_sources = local.sources;

    for central_src in central.sources {
        let central_url = central_src.url.as_deref().unwrap_or("");

        // 1. URL match → skip entirely (local takes precedence)
        if !central_url.is_empty() && local_urls.contains(&normalize_url(central_url)) {
            continue;
        }

        // Different URL → keep all skills (including those with same name as local)
        let filtered_skills = central_src.skills;

        if !filtered_skills.is_empty() {
            // 2. Same name but different URL → clear source name to "-"
            let display_name = if !central_src.source.is_empty()
                && local_names.contains(central_src.source.as_str())
            {
                String::new()
            } else {
                central_src.source.clone()
            };

            merged_sources.push(SourceCache {
                source: display_name,
                url: central_src.url.clone(),
                registry_url: central_src.url,
                skills: filtered_skills,
            });
        }
    }

    CacheData {
        updated_at: chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string(),
        sources: merged_sources,
    }
}

/// Recursively search for a skill in a directory tree.
fn search_skills_in_dir(
    dir: &std::path::Path,
    target: &str,
    source_name: &str,
    source_url: &str,
    prefix: &str,
) -> Option<(String, String, String)> {
    let entries = fs::read_dir(dir).ok()?;

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let dir_name = entry.file_name().to_string_lossy().to_string();
        let rel_path = if prefix.is_empty() {
            dir_name.clone()
        } else {
            format!("{}/{}", prefix, dir_name)
        };

        let skill_md = path.join("SKILL.md");
        if skill_md.exists() {
            let meta = SkillMeta::from_file(&path).unwrap_or_default();
            let display = meta.display_name(&dir_name);
            if display == target || dir_name == target {
                return Some((
                    source_name.to_string(),
                    source_url.to_string(),
                    format!("skills/{}/SKILL.md", rel_path),
                ));
            }
        }

        if let Some(found) =
            search_skills_in_dir(&path, target, source_name, source_url, &rel_path)
        {
            return Some(found);
        }
    }

    None
}

// ---------------------------------------------------------------------------
// Utility functions
// ---------------------------------------------------------------------------

/// Check if a string looks like a URL.
pub fn is_url(s: &str) -> bool {
    s.starts_with("http://") || s.starts_with("https://")
}

/// Clone a repo and collect all skills.
fn clone_and_collect(url: &str) -> Result<Vec<CachedSkill>> {
    let url = normalize_url(url);
    let tmp_dir = git::clone_for_listing(&url)?;
    let skills_dir = tmp_dir.path().join("skills");

    let mut skills = Vec::new();
    if skills_dir.exists() {
        collect_skills_from_dir(&skills_dir, &mut skills, &mut String::new());
    }

    Ok(skills)
}

/// Load from URL cache if fresh, otherwise clone and save.
fn load_or_fetch_url_cache(url: &str, ttl_secs: u64) -> Result<CacheData> {
    if let Some(cached) = cache::load_url_cache(url, ttl_secs) {
        return Ok(cached);
    }

    let skills = clone_and_collect(url)?;
    // Normalize URL: strip .git suffix for consistent source name
    let normalized = url.strip_suffix(".git").unwrap_or(url);

    let data = CacheData {
        updated_at: chrono::Utc::now()
            .format("%Y-%m-%dT%H:%M:%S%.3fZ")
            .to_string(),
        sources: vec![SourceCache {
            source: normalized.to_string(),
            url: Some(normalized.to_string()),
            registry_url: None,
            skills,
        }],
    };

    // Save to URL cache (ignore errors — non-critical)
    let _ = cache::save_url_cache(url, &data);

    Ok(data)
}

/// Recursively collect skills from a directory tree.
pub fn collect_skills_from_dir(
    dir: &std::path::Path,
    skills: &mut Vec<CachedSkill>,
    current_path: &mut String,
) {
    let entries = match fs::read_dir(dir) {
        Ok(e) => e,
        Err(_) => return,
    };

    for entry in entries.flatten() {
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }

        let dir_name = entry.file_name().to_string_lossy().to_string();
        let saved = current_path.clone();
        let rel_path = if current_path.is_empty() {
            dir_name.clone()
        } else {
            format!("{}/{}", current_path, dir_name)
        };
        *current_path = rel_path.clone();

        let skill_md = path.join("SKILL.md");
        if skill_md.exists() {
            let meta = SkillMeta::from_file(&path).unwrap_or_default();
            skills.push(CachedSkill {
                name: meta.display_name(&dir_name),
                path: format!("{}/SKILL.md", rel_path),
                description: meta.display_description(),
                version: meta
                    .metadata
                    .as_ref()
                    .and_then(|m| m.version.clone())
                    .unwrap_or_default(),
            });
        }

        collect_skills_from_dir(&path, skills, current_path);
        *current_path = saved;
    }
}

/// Fetch JSON content from a URL (curl with wget fallback).
pub fn fetch_json(url: &str) -> Result<String> {
    use std::process::Command;

    // Try curl first
    let output = Command::new("curl")
        .args(["-sL", "--max-time", "10", url])
        .output();

    match output {
        Ok(out) if out.status.success() => {
            let body = String::from_utf8_lossy(&out.stdout).to_string();
            if !body.is_empty() {
                return Ok(body);
            }
        }
        _ => {}
    }

    // Fallback to wget
    let output = Command::new("wget")
        .args(["-qO-", "--timeout=10", url])
        .output()?;

    if output.status.success() {
        Ok(String::from_utf8_lossy(&output.stdout).to_string())
    } else {
        anyhow::bail!("Failed to fetch URL: {}", url)
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cache::{CacheData, CachedSkill, SourceCache};

    #[test]
    fn test_is_url() {
        assert!(is_url("https://github.com/example/skills"));
        assert!(is_url("http://example.com/repo"));
        assert!(!is_url("antfu"));
        assert!(!is_url("org/repo"));
    }

    #[test]
    fn test_merge_skills_no_overlap() {
        let local = CacheData {
            updated_at: String::new(),
            sources: vec![SourceCache {
                source: "local-src".to_string(),
                url: Some("https://local/repo".to_string()),
                registry_url: None,
                skills: vec![CachedSkill {
                    name: "vue".to_string(),
                    path: "skills/vue/SKILL.md".to_string(),
                    description: String::new(),
                    version: String::new(),
                }],
            }],
        };
        let central = CacheData {
            updated_at: String::new(),
            sources: vec![SourceCache {
                source: "central-src".to_string(),
                url: Some("https://central/repo".to_string()),
                registry_url: None,
                skills: vec![CachedSkill {
                    name: "react".to_string(),
                    path: "skills/react/SKILL.md".to_string(),
                    description: String::new(),
                    version: String::new(),
                }],
            }],
        };

        let merged = merge_skills(local, central);
        assert_eq!(merged.sources.len(), 2);
        assert_eq!(merged.sources[0].source, "local-src");
        assert_eq!(merged.sources[1].source, "central-src");
        assert_eq!(merged.sources[1].skills.len(), 1);
        assert_eq!(merged.sources[1].skills[0].name, "react");
    }

    #[test]
    fn test_merge_skills_same_url_skipped() {
        let local = CacheData {
            updated_at: String::new(),
            sources: vec![SourceCache {
                source: "antfu".to_string(),
                url: Some("https://github.com/antfu/skills".to_string()),
                registry_url: None,
                skills: vec![CachedSkill {
                    name: "vue".to_string(),
                    path: "skills/vue/SKILL.md".to_string(),
                    description: String::new(),
                    version: String::new(),
                }],
            }],
        };
        let central = CacheData {
            updated_at: String::new(),
            sources: vec![SourceCache {
                source: "antfu".to_string(),
                url: Some("https://github.com/antfu/skills".to_string()),
                registry_url: None,
                skills: vec![CachedSkill {
                    name: "vue".to_string(),
                    path: "skills/vue/SKILL.md".to_string(),
                    description: String::new(),
                    version: String::new(),
                }],
            }],
        };

        let merged = merge_skills(local, central);
        assert_eq!(merged.sources.len(), 1);
        assert_eq!(merged.sources[0].source, "antfu");
    }

    #[test]
    fn test_merge_skills_different_url_same_name_kept() {
        let local = CacheData {
            updated_at: String::new(),
            sources: vec![SourceCache {
                source: "local".to_string(),
                url: Some("https://local/repo".to_string()),
                registry_url: None,
                skills: vec![CachedSkill {
                    name: "vue".to_string(),
                    path: "skills/vue/SKILL.md".to_string(),
                    description: "local vue".to_string(),
                    version: "1.0".to_string(),
                }],
            }],
        };
        let central = CacheData {
            updated_at: String::new(),
            sources: vec![SourceCache {
                source: "other".to_string(),
                url: Some("https://other/repo".to_string()),
                registry_url: None,
                skills: vec![
                    CachedSkill {
                        name: "vue".to_string(),
                        path: "skills/vue/SKILL.md".to_string(),
                        description: "central vue".to_string(),
                        version: "2.0".to_string(),
                    },
                    CachedSkill {
                        name: "react".to_string(),
                        path: "skills/react/SKILL.md".to_string(),
                        description: "react".to_string(),
                        version: "1.0".to_string(),
                    },
                ],
            }],
        };

        let merged = merge_skills(local, central);
        // Different URL → keep all skills (including same-name "vue")
        let central_src = merged
            .sources
            .iter()
            .find(|s| s.source == "other")
            .unwrap();
        assert_eq!(central_src.skills.len(), 2);
        assert_eq!(central_src.skills[0].name, "vue");
        assert_eq!(central_src.skills[1].name, "react");
    }

    #[test]
    fn test_merge_skills_name_collision_different_url() {
        let local = CacheData {
            updated_at: String::new(),
            sources: vec![SourceCache {
                source: "myrepo".to_string(),
                url: Some("https://local/myrepo".to_string()),
                registry_url: None,
                skills: vec![CachedSkill {
                    name: "skill-a".to_string(),
                    path: "skills/skill-a/SKILL.md".to_string(),
                    description: "local".to_string(),
                    version: "1.0".to_string(),
                }],
            }],
        };
        let central = CacheData {
            updated_at: String::new(),
            sources: vec![SourceCache {
                source: "myrepo".to_string(),
                url: Some("https://central/myrepo".to_string()),
                registry_url: None,
                skills: vec![
                    CachedSkill {
                        name: "skill-a".to_string(),
                        path: "skills/skill-a/SKILL.md".to_string(),
                        description: "central".to_string(),
                        version: "2.0".to_string(),
                    },
                    CachedSkill {
                        name: "skill-b".to_string(),
                        path: "skills/skill-b/SKILL.md".to_string(),
                        description: String::new(),
                        version: String::new(),
                    },
                ],
            }],
        };

        let merged = merge_skills(local, central);
        // Same name but different URL → source name cleared, all skills kept
        assert_eq!(merged.sources[1].source, "");
        assert_eq!(merged.sources[1].skills.len(), 2);
    }

    #[test]
    fn test_merge_skills_empty() {
        let local = CacheData::default();
        let central = CacheData::default();
        let merged = merge_skills(local, central);
        assert!(merged.sources.is_empty());
    }
}
