use crate::config::Config;
use crate::git;
use crate::lock::{LockFile, LockEntry};
use crate::skill_meta::SkillMeta;
use crate::utils::validate_agent;
use anyhow::{Context, Result};
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Extract skill directory name from skillPath (e.g. "skills/vue/SKILL.md" -> "vue")
fn extract_skill_name(skill_path: &str) -> String {
    skill_path
        .replace("/SKILL.md", "")
        .replace("skills/", "")
}

/// Get project-level .agents/skills directory
fn project_agents_skills_dir() -> PathBuf {
    std::env::current_dir()
        .unwrap_or_else(|_| PathBuf::from("."))
        .join(".agents")
        .join("skills")
}

/// Get global ~/.agents/skills directory
fn global_agents_skills_dir() -> PathBuf {
    dirs::home_dir()
        .unwrap_or_else(|| PathBuf::from("~"))
        .join(".agents")
        .join("skills")
}

/// Resolve restore target directory
fn resolve_restore_target(global: bool, agent: Option<&str>) -> Result<Vec<PathBuf>> {
    let config = Config::load()?;
    let mut targets = Vec::new();

    if let Some(agent_value) = agent {
        if agent_value == "*" {
            // All configured platforms
            for name in config.platform_names() {
                if let Some(platform) = config.get_platform(name) {
                    if let Some(skills_dir) = platform.skills_dir() {
                        targets.push(skills_dir);
                    }
                }
            }
        } else {
            // Specific platform
            validate_agent(&config, agent_value)?;
            let platform = config.get_platform(agent_value).unwrap();
            let skills_dir = platform.skills_dir()
                .ok_or_else(|| anyhow::anyhow!(
                    "Platform {} has no skills directory configured", agent_value
                ))?;
            targets.push(skills_dir);
        }
    } else if global {
        targets.push(global_agents_skills_dir());
    } else {
        targets.push(project_agents_skills_dir());
    }

    Ok(targets)
}

/// Copy skill directory to destination
fn copy_skill_to_dest(source_dir: &Path, dest_dir: &Path) -> Result<()> {
    if !source_dir.exists() {
        anyhow::bail!("Source directory not found: {}", source_dir.display());
    }

    if dest_dir.exists() {
        fs::remove_dir_all(dest_dir)?;
    }
    fs::create_dir_all(dest_dir)?;
    copy_dir_recursive(source_dir, dest_dir)?;

    println!("{}: {}", "Installed".green(), dest_dir.display());
    Ok(())
}

/// Recursively copy directory
pub fn copy_dir_recursive(src: &Path, dst: &Path) -> Result<()> {
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

pub fn run(global: bool, agent: Option<&str>, dry_run: bool) -> Result<()> {
    // Always read from project-level lock file
    let lock_file = LockFile::load(false)?;

    if lock_file.skills.is_empty() {
        println!("{}", "No skills to restore".yellow());
        return Ok(());
    }

    let targets = resolve_restore_target(global, agent)?;

    // Dry-run mode: preview only (grouped by skill name)
    if dry_run {
        println!("Skills to restore:");
        println!();

        // Calculate column widths
        let name_w = lock_file.skills.keys()
            .map(|k| k.len())
            .max()
            .unwrap_or(4)
            .max(4); // "NAME"
        let source_w = lock_file.skills.values()
            .map(|e| e.source_url.len())
            .max()
            .unwrap_or(6)
            .max(6); // "SOURCE"

        // Use colors only when multiple targets
        let use_color = targets.len() > 1;

        // Header
        if use_color {
            println!("{:<name_w$}  {:<source_w$}  {}",
                "NAME".blue(), "SOURCE".blue(), "TARGET".blue(),
                name_w = name_w, source_w = source_w);
        } else {
            println!("{:<name_w$}  {:<source_w$}  {}",
                "NAME", "SOURCE", "TARGET",
                name_w = name_w, source_w = source_w);
        }

        for (skill_name, entry) in &lock_file.skills {
            let mut first = true;
            for target_dir in &targets {
                let dest_dir = target_dir.join(skill_name);
                if first {
                    println!("{:<name_w$}  {:<source_w$}  {}",
                        skill_name, entry.source_url, dest_dir.display(),
                        name_w = name_w, source_w = source_w);
                    first = false;
                } else if use_color {
                    println!("{:<name_w$}  {:<source_w$}  {}",
                        "", "", dest_dir.display().to_string().bright_black(),
                        name_w = name_w, source_w = source_w);
                } else {
                    println!("{:<name_w$}  {:<source_w$}  {}",
                        "", "", dest_dir.display(),
                        name_w = name_w, source_w = source_w);
                }
            }
        }
        return Ok(());
    }

    println!("{}", "Restoring skills...".cyan());
    println!();

    // Determine which lock file to update
    let lock_is_global = global && agent.is_none();
    let mut updated_lock = LockFile::load(lock_is_global)?;
    let now = chrono::Utc::now();
    let timestamp = now.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    // Group skills by source_url to clone each repo only once
    let mut groups: HashMap<String, Vec<(&String, &LockEntry)>> = HashMap::new();
    for (skill_name, entry) in &lock_file.skills {
        groups.entry(entry.source_url.clone()).or_default().push((skill_name, entry));
    }

    let mut success_count = 0;
    let mut fail_count = 0;

    for (source_url, skills) in &groups {
        println!("{}: {}", "Source".cyan(), source_url);

        // Clone repo once for all skills from this source
        let tmp_dir = match git::clone_for_listing(source_url) {
            Ok(dir) => dir,
            Err(e) => {
                println!("  {}: {}", "Clone failed".red(), e);
                fail_count += skills.len();
                println!();
                continue;
            }
        };

        let skills_dir = tmp_dir.path().join("skills");

        for (skill_name, entry) in skills {
            let skill_dir_name = extract_skill_name(&entry.skill_path);
            let source_dir = skills_dir.join(&skill_dir_name);

            if !source_dir.exists() {
                println!("  {}: {}", skill_name, "skill directory not found in repo".red());
                fail_count += 1;
                continue;
            }

            // Read skill metadata from clone
            let meta = SkillMeta::from_file(&source_dir).unwrap_or_default();
            println!("  {}: {}", "Name".cyan().bold(), meta.display_name(skill_name).yellow());
            println!("  {}: {}", "Description".cyan().bold(), meta.display_description());
            if let Some(version) = meta.metadata.as_ref().and_then(|m| m.version.clone()) {
                if !version.is_empty() {
                    println!("  {}: {}", "Version".cyan().bold(), version);
                }
            }

            // Get skill_folder_hash from the shared clone
            let skill_folder_hash = git::get_skill_folder_hash(tmp_dir.path(), skill_name)
                .unwrap_or_default();

            // Copy to each target directory
            let mut any_target_ok = false;
            for target_dir in &targets {
                let dest_dir = target_dir.join(skill_name);
                match copy_skill_to_dest(&source_dir, &dest_dir) {
                    Ok(_) => { any_target_ok = true; }
                    Err(e) => {
                        println!("  {} for {}: {}", "Copy failed".red(), dest_dir.display(), e);
                    }
                }
            }

            if any_target_ok {
                // Preserve installed_at from existing target lock entry, or use source entry's
                let installed_at = if let Some(existing) = updated_lock.skills.get(*skill_name) {
                    existing.installed_at.clone()
                } else {
                    entry.installed_at.clone()
                };

                let updated_entry = LockEntry {
                    source: entry.source.clone(),
                    source_type: entry.source_type.clone(),
                    source_url: entry.source_url.clone(),
                    skill_path: entry.skill_path.clone(),
                    skill_folder_hash,
                    installed_at,
                    updated_at: timestamp.clone(),
                };
                updated_lock.upsert_skill(skill_name, updated_entry);
                success_count += 1;
            } else {
                fail_count += 1;
            }

            println!();
        }
    }

    // Save updated lock file
    if success_count > 0 {
        updated_lock.updated_at = timestamp;
        updated_lock.save(lock_is_global)?;
    }

    println!("{}: {} succeeded, {} failed", "Restore complete".green(), format!("{}", success_count).green(), format!("{}", fail_count).red());

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lock::LockEntry;

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
    fn test_default_lock_file_is_empty() {
        let lock = LockFile::default();
        assert!(lock.skills.is_empty());
        assert_eq!(lock.version, 1);
    }

    #[test]
    fn test_lock_file_skill_lookup() {
        let mut lock = LockFile::default();
        lock.upsert_skill("vue", make_entry("vue"));
        lock.upsert_skill("react", make_entry("react"));

        assert!(lock.skills.get("vue").is_some());
        assert!(lock.skills.get("react").is_some());
        assert!(lock.skills.get("angular").is_none());
        assert_eq!(lock.skills["vue"].source_url, "https://example.com/vue.git");
    }

    #[test]
    fn test_extract_skill_name() {
        assert_eq!(extract_skill_name("skills/vue/SKILL.md"), "vue");
        assert_eq!(extract_skill_name("skills/react/SKILL.md"), "react");
        assert_eq!(extract_skill_name("skills/my-plugin/SKILL.md"), "my-plugin");
        assert_eq!(extract_skill_name("skills/antfu-design/SKILL.md"), "antfu-design");
    }

    #[test]
    fn test_resolve_restore_targets_default() {
        let targets = resolve_restore_target(false, None).unwrap();
        assert_eq!(targets.len(), 1);
        assert!(targets[0].to_string_lossy().contains(".agents/skills"));
    }

    #[test]
    fn test_resolve_restore_targets_global() {
        let targets = resolve_restore_target(true, None).unwrap();
        assert_eq!(targets.len(), 1);
        assert!(targets[0].to_string_lossy().contains(".agents/skills"));
    }

    #[test]
    fn test_resolve_restore_targets_invalid_platform() {
        let result = resolve_restore_target(false, Some("nonexistent-platform"));
        assert!(result.is_err());
        let err_msg = result.unwrap_err().to_string();
        assert!(err_msg.contains("Invalid agents: nonexistent-platform"));
    }

    #[test]
    fn test_copy_dir_recursive() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();

        // Create source structure
        let src_dir = src.path().join("skills").join("vue");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("SKILL.md"), "# Vue Skill").unwrap();
        fs::write(src_dir.join("README.md"), "readme").unwrap();

        // Copy
        let dest_dir = dst.path().join("vue");
        copy_dir_recursive(&src_dir, &dest_dir).unwrap();

        // Verify
        assert!(dest_dir.join("SKILL.md").exists());
        assert!(dest_dir.join("README.md").exists());
        assert_eq!(fs::read_to_string(dest_dir.join("SKILL.md")).unwrap(), "# Vue Skill");
    }

    #[test]
    fn test_copy_skill_to_dest() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();

        let src_dir = src.path().join("vue");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("SKILL.md"), "# Vue").unwrap();

        let dest_dir = dst.path().join("vue");
        copy_skill_to_dest(&src_dir, &dest_dir).unwrap();

        assert!(dest_dir.join("SKILL.md").exists());
    }

    #[test]
    fn test_copy_skill_to_dest_overwrites_existing() {
        let src = tempfile::tempdir().unwrap();
        let dst = tempfile::tempdir().unwrap();

        let src_dir = src.path().join("vue");
        fs::create_dir_all(&src_dir).unwrap();
        fs::write(src_dir.join("SKILL.md"), "new content").unwrap();

        let dest_dir = dst.path().join("vue");
        fs::create_dir_all(&dest_dir).unwrap();
        fs::write(dest_dir.join("SKILL.md"), "old content").unwrap();
        fs::write(dest_dir.join("old-file.txt"), "to be removed").unwrap();

        copy_skill_to_dest(&src_dir, &dest_dir).unwrap();

        assert_eq!(fs::read_to_string(dest_dir.join("SKILL.md")).unwrap(), "new content");
        assert!(!dest_dir.join("old-file.txt").exists());
    }

    #[test]
    fn test_copy_skill_to_dest_missing_source() {
        let dst = tempfile::tempdir().unwrap();
        let result = copy_skill_to_dest(
            &PathBuf::from("/nonexistent/path"),
            &dst.path().join("vue"),
        );
        assert!(result.is_err());
    }

    #[test]
    fn test_group_skills_by_source_url() {
        let mut lock = LockFile::default();
        lock.upsert_skill("vue", LockEntry {
            source: "antfu".to_string(),
            source_type: "git".to_string(),
            source_url: "https://github.com/antfu/skills.git".to_string(),
            skill_path: "skills/vue/SKILL.md".to_string(),
            skill_folder_hash: "abc".to_string(),
            installed_at: "2026-07-17T00:00:00.000Z".to_string(),
            updated_at: "2026-07-17T00:00:00.000Z".to_string(),
        });
        lock.upsert_skill("react", LockEntry {
            source: "antfu".to_string(),
            source_type: "git".to_string(),
            source_url: "https://github.com/antfu/skills.git".to_string(),
            skill_path: "skills/react/SKILL.md".to_string(),
            skill_folder_hash: "def".to_string(),
            installed_at: "2026-07-17T00:00:00.000Z".to_string(),
            updated_at: "2026-07-17T00:00:00.000Z".to_string(),
        });
        lock.upsert_skill("tdd", LockEntry {
            source: "mattpocock".to_string(),
            source_type: "git".to_string(),
            source_url: "https://github.com/mattpocock/skills.git".to_string(),
            skill_path: "skills/tdd/SKILL.md".to_string(),
            skill_folder_hash: "ghi".to_string(),
            installed_at: "2026-07-17T00:00:00.000Z".to_string(),
            updated_at: "2026-07-17T00:00:00.000Z".to_string(),
        });

        // Group by source_url
        let mut groups: HashMap<String, Vec<(&String, &LockEntry)>> = HashMap::new();
        for (skill_name, entry) in &lock.skills {
            groups.entry(entry.source_url.clone()).or_default().push((skill_name, entry));
        }

        assert_eq!(groups.len(), 2);
        assert_eq!(groups["https://github.com/antfu/skills.git"].len(), 2);
        assert_eq!(groups["https://github.com/mattpocock/skills.git"].len(), 1);
    }

    #[test]
    fn test_installed_at_preservation() {
        let mut target_lock = LockFile::default();
        // Simulate existing entry in target lock with original installed_at
        target_lock.upsert_skill("vue", LockEntry {
            source: "antfu".to_string(),
            source_type: "git".to_string(),
            source_url: "https://github.com/antfu/skills.git".to_string(),
            skill_path: "skills/vue/SKILL.md".to_string(),
            skill_folder_hash: "old_hash".to_string(),
            installed_at: "2026-07-10T00:00:00.000Z".to_string(),
            updated_at: "2026-07-10T00:00:00.000Z".to_string(),
        });

        // Source entry from project lock
        let source_entry = LockEntry {
            source: "antfu".to_string(),
            source_type: "git".to_string(),
            source_url: "https://github.com/antfu/skills.git".to_string(),
            skill_path: "skills/vue/SKILL.md".to_string(),
            skill_folder_hash: "new_hash".to_string(),
            installed_at: "2026-07-17T00:00:00.000Z".to_string(),
            updated_at: "2026-07-17T00:00:00.000Z".to_string(),
        };

        // When target has existing entry, preserve its installed_at
        let installed_at = if let Some(existing) = target_lock.skills.get("vue") {
            existing.installed_at.clone()
        } else {
            source_entry.installed_at.clone()
        };
        assert_eq!(installed_at, "2026-07-10T00:00:00.000Z");

        // When target has no entry, use source's installed_at
        let installed_at_new = if let Some(existing) = target_lock.skills.get("react") {
            existing.installed_at.clone()
        } else {
            "2026-07-17T00:00:00.000Z".to_string()
        };
        assert_eq!(installed_at_new, "2026-07-17T00:00:00.000Z");
    }
}
