use crate::git;
use crate::lock::{LockFile, LockEntry};
use crate::skill_meta::SkillMeta;
use anyhow::Result;
use colored::Colorize;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;

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

pub fn run(global: bool, skill: Option<&str>) -> Result<()> {
    // Determine update scope
    let is_global = global;

    // Load lock file
    let lock_file = LockFile::load(is_global)?;

    if lock_file.skills.is_empty() {
        if is_global {
            println!("{}", "No skills installed in global lock file".yellow());
        } else {
            println!("{}", "No skills installed in project lock file".yellow());
        }
        return Ok(());
    }

    // Determine skills to update
    let skills_to_update: Vec<(&String, &LockEntry)> = if let Some(skill_name) = skill {
        if skill_name == "*" {
            // Update all skills
            lock_file.skills.iter().collect()
        } else {
            // Update specific skill
            if let Some(_entry) = lock_file.skills.get(skill_name) {
                lock_file.skills.iter().filter(|(k, _)| k.as_str() == skill_name).collect()
            } else {
                anyhow::bail!("Skill not found in lock file: {}", skill_name);
            }
        }
    } else {
        // Default: update all skills
        lock_file.skills.iter().collect()
    };

    if skills_to_update.is_empty() {
        println!("{}", "No skills to update".yellow());
        return Ok(());
    }

    println!("{}", "Updating skills...".cyan());
    println!();

    // Determine target directory
    let base_dir = if is_global {
        global_agents_skills_dir()
    } else {
        project_agents_skills_dir()
    };

    // Load lock file for updates
    let mut updated_lock_file = LockFile::load(is_global)?;
    let mut success_count = 0;
    let mut fail_count = 0;
    let now = chrono::Utc::now();
    let top_timestamp = now.format("%Y-%m-%dT%H:%M:%S%.3fZ").to_string();

    // Group skills by source_url to clone each repo only once
    let mut groups: HashMap<String, Vec<(&String, &LockEntry)>> = HashMap::new();
    for (skill_name, entry) in &skills_to_update {
        groups.entry(entry.source_url.clone()).or_default().push((skill_name, entry));
    }

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
            println!("  {}: {}", "Updating".cyan(), skill_name);

            // Extract skill directory name from skillPath
            let skill_dir_name = entry.skill_path
                .replace("/SKILL.md", "")
                .replace("skills/", "");
            let source_dir = skills_dir.join(&skill_dir_name);

            if !source_dir.exists() {
                println!("    {}: {}", skill_name, "skill directory not found in repo".red());
                fail_count += 1;
                println!();
                continue;
            }

            // Read new version info from the clone
            let meta = SkillMeta::from_file(&source_dir).unwrap_or_default();
            println!("    {}: {}", "Name".cyan().bold(), meta.display_name(skill_name).yellow());
            println!("    {}: {}", "Description".cyan().bold(), meta.display_description());
            if let Some(version) = meta.metadata.as_ref().and_then(|m| m.version.clone()) {
                if !version.is_empty() {
                    println!("    {}: {}", "Version".cyan().bold(), version);
                }
            }

            // Copy skill to destination
            let dest_dir = base_dir.join(skill_name);
            if dest_dir.exists() {
                fs::remove_dir_all(&dest_dir)?;
            }
            fs::create_dir_all(&dest_dir)?;
            crate::commands::restore::copy_dir_recursive(&source_dir, &dest_dir)?;

            // Get skill_folder_hash from the shared clone
            let skill_folder_hash = git::get_skill_folder_hash(tmp_dir.path(), skill_name)
                .unwrap_or_default();

            // Update lock file entry
            let entry_timestamp = top_timestamp.clone();

            let updated_entry = LockEntry {
                source: entry.source.clone(),
                source_type: entry.source_type.clone(),
                source_url: entry.source_url.clone(),
                skill_path: entry.skill_path.clone(),
                skill_folder_hash,
                installed_at: entry.installed_at.clone(), // Preserve original install time
                updated_at: entry_timestamp,
            };

            updated_lock_file.upsert_skill(skill_name, updated_entry);

            println!("    {}: {}", "Updated".green(), dest_dir.display());
            success_count += 1;
            println!();
        }
    }

    // Save updated lock file
    updated_lock_file.updated_at = top_timestamp;
    updated_lock_file.save(is_global)?;

    // Print statistics
    println!("{}: {} succeeded, {} failed", "Update complete".green(), format!("{}", success_count).green(), format!("{}", fail_count).red());

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::lock::LockFile;

    #[test]
    fn test_default_lock_file_is_empty() {
        let lock = LockFile::default();
        assert!(lock.skills.is_empty());
        assert_eq!(lock.version, 1);
    }

    #[test]
    fn test_lock_file_skill_lookup() {
        let mut lock = LockFile::default();
        assert!(lock.skills.get("vue").is_none());

        lock.upsert_skill("vue", crate::lock::LockEntry {
            source: "test".to_string(),
            source_type: "git".to_string(),
            source_url: "https://example.com".to_string(),
            skill_path: "skills/vue/SKILL.md".to_string(),
            skill_folder_hash: "abc".to_string(),
            installed_at: "2026-07-17T00:00:00.000Z".to_string(),
            updated_at: "2026-07-17T00:00:00.000Z".to_string(),
        });

        assert!(lock.skills.get("vue").is_some());
        assert_eq!(lock.skills["vue"].source, "test");
    }
}
