use crate::skill_meta::SkillMeta;
use anyhow::{bail, Context, Result};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

/// 安装结果
#[allow(dead_code)]
pub struct InstallResult {
    pub dest: String,
    pub version_change: String,
}

/// 从 Git 仓库安装 skill（仅拉取目标子树）
///
/// 流程：
/// 1. 归一化 URL
/// 2. sparse-checkout 仅检出 skills/<skill_path>
/// 3. 检测默认分支
/// 4. 迁移到本地 dest_dir/
/// 5. 清理临时目录
pub fn install_skill(repo_url: &str, skill_path: &str, dest_name: &str, dest_dir: &Path) -> Result<InstallResult> {
    let dest = dest_name.to_string();

    // 检查是否已安装，记录旧版本
    let old_meta = if dest_dir.exists() {
        SkillMeta::from_file(&dest_dir).ok()
    } else {
        None
    };
    let old_version = old_meta
        .as_ref()
        .and_then(|m| m.metadata.as_ref())
        .and_then(|m| m.version.clone())
        .unwrap_or_default();

    // 创建临时目录
    let tmp_dir = TempDir::new()?;
    let tmp_path = tmp_dir.path().to_path_buf();

    // 使用 git CLI 进行 sparse checkout
    clone_sparse(repo_url, skill_path, &tmp_path)?;

    // 将子树内容迁移到目标目录
    let sparse_checkout_dir = tmp_path.join("skills").join(skill_path);
    if !sparse_checkout_dir.exists() {
        bail!(
            "Skill not found in repo {}: skills/{}",
            repo_url,
            skill_path
        );
    }

    // 确保目标目录存在
    if dest_dir.exists() {
        fs::remove_dir_all(&dest_dir)
            .with_context(|| format!("Failed to remove old directory: {}", dest_dir.display()))?;
    }
    fs::create_dir_all(&dest_dir)
        .with_context(|| format!("Failed to create directory: {}", dest_dir.display()))?;

    // 复制文件
    copy_dir_recursive(&sparse_checkout_dir, &dest_dir)?;

    // 读取新版本
    let new_meta = SkillMeta::from_file(&dest_dir)?;
    let new_version = new_meta
        .metadata
        .as_ref()
        .and_then(|m| m.version.clone())
        .unwrap_or_default();

    let version_change = crate::utils::compare_versions(&old_version, &new_version);

    Ok(InstallResult {
        dest,
        version_change,
    })
}

/// 使用 git CLI 进行 sparse checkout 克隆（静默模式）
fn clone_sparse(repo_url: &str, skill_path: &str, dest: &Path) -> Result<()> {
    let sparse_path = format!("skills/{}", skill_path);

    // 1. 检测默认分支
    let default_branch = detect_default_branch(repo_url)?;

    // 2. 克隆仓库（浅克隆 + sparse checkout，静默模式）
    let status = Command::new("git")
        .args([
            "clone",
            "--filter=blob:none",
            "--depth=1",
            "--sparse",
            &format!("--branch={}", default_branch),
            repo_url,
            dest.to_str().unwrap_or(""),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .with_context(|| "Failed to run git clone")?;

    if !status.success() {
        bail!("git clone failed");
    }

    // 3. Initialize sparse checkout (cone mode)
    let status = Command::new("git")
        .current_dir(dest)
        .args(["sparse-checkout", "init", "--cone"])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .with_context(|| "Failed to run git sparse-checkout init")?;

    if !status.success() {
        bail!("git sparse-checkout init failed");
    }

    // 4. Set sparse checkout path (silent mode)
    let status = Command::new("git")
        .current_dir(dest)
        .args(["sparse-checkout", "set", &sparse_path])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .with_context(|| "Failed to run git sparse-checkout set")?;

    if !status.success() {
        bail!("git sparse-checkout set failed");
    }

    Ok(())
}

/// 检测远程仓库的默认分支
fn detect_default_branch(repo_url: &str) -> Result<String> {
    let output = Command::new("git")
        .args(["ls-remote", "--symref", repo_url, "HEAD"])
        .output()
        .with_context(|| "Failed to run git ls-remote")?;

    if !output.status.success() {
        bail!("git ls-remote failed");
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    // 输出格式: ref: refs/heads/main\tHEAD
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("ref: refs/heads/") {
            let branch = rest.split('\t').next().unwrap_or("main");
            return Ok(branch.to_string());
        }
    }

    // 默认回退到 main
    Ok("main".to_string())
}

/// 递归复制目录
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

/// Sparse checkout clone, returns (TempDir, skill source dir path).
/// The caller can copy from the returned path to multiple targets.
/// The TempDir must be kept alive until copying is done.
pub fn install_skill_sparse(repo_url: &str, skill_path: &str, dest_name: &str) -> Result<(TempDir, PathBuf)> {
    let tmp_dir = TempDir::new()?;
    let tmp_path = tmp_dir.path().to_path_buf();
    clone_sparse(repo_url, skill_path, &tmp_path)?;

    let source_dir = tmp_path.join("skills").join(dest_name);
    if !source_dir.exists() {
        bail!("Skill not found in repo: skills/{}", skill_path);
    }

    Ok((tmp_dir, source_dir))
}

/// 临时克隆仓库并列出 skills 目录内容（用于 show/query 等只读操作）
pub fn clone_for_listing(repo_url: &str) -> Result<TempDir> {
    let tmp_dir = TempDir::new()?;
    let tmp_path = tmp_dir.path().to_path_buf();

    // 1. 检测默认分支
    let default_branch = detect_default_branch(repo_url)?;

    // 2. 浅克隆（静默模式）
    let status = Command::new("git")
        .args([
            "clone",
            "--filter=blob:none",
            "--depth=1",
            &format!("--branch={}", default_branch),
            repo_url,
            tmp_path.to_str().unwrap_or(""),
        ])
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .status()
        .with_context(|| "Failed to run git clone")?;

    if !status.success() {
        bail!("git clone failed");
    }

    Ok(tmp_dir)
}

/// Get the git tree hash of a skill folder
pub fn get_skill_folder_hash(repo_dir: &Path, skill_name: &str) -> Result<String> {
    let skill_path = format!("skills/{}", skill_name);

    let output = Command::new("git")
        .current_dir(repo_dir)
        .args(["rev-parse", &format!("HEAD:{}", skill_path)])
        .output()
        .with_context(|| "Failed to run git rev-parse")?;

    if !output.status.success() {
        // 如果无法获取 hash，返回空字符串
        return Ok(String::new());
    }

    let hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(hash)
}
