use crate::skill_meta::SkillMeta;
use anyhow::{Context, Result, bail};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::Ordering;
use tempfile::TempDir;

/// 获取 verbose 模式状态
fn is_verbose() -> bool {
    crate::VERBOSE.load(Ordering::Relaxed)
}

/// 根据 verbose 模式返回 Stdio：verbose 时继承，否则丢弃
fn stderr_stdio() -> std::process::Stdio {
    if is_verbose() {
        std::process::Stdio::inherit()
    } else {
        std::process::Stdio::null()
    }
}

/// 安装结果
#[allow(dead_code)]
pub struct InstallResult {
    pub dest: String,
    pub version_change: String,
    pub skill_folder_hash: String,
}

/// 从 Git 仓库安装 skill（仅拉取目标子树）
///
/// 流程：
/// 1. 归一化 URL
/// 2. sparse-checkout 仅检出 skills/<skill_path>
/// 3. 检测默认分支
/// 4. 迁移到本地 dest_dir/
/// 5. 清理临时目录
pub fn install_skill(
    repo_url: &str,
    skill_path: &str,
    dest_name: &str,
    dest_dir: &Path,
) -> Result<InstallResult> {
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
    // skill_path is already the full relative path (e.g., "skills/name" or "name")
    let sparse_checkout_dir = tmp_path.join(skill_path);
    if !sparse_checkout_dir.exists() {
        bail!("Skill not found in repo {}: {}", repo_url, skill_path);
    }

    // 确保目标目录存在
    crate::utils::remove_symlink(dest_dir)
        .with_context(|| format!("Failed to remove old directory: {}", dest_dir.display()))?;
    fs::create_dir_all(&dest_dir)
        .with_context(|| format!("Failed to create directory: {}", dest_dir.display()))?;

    // 复制文件
    copy_dir_recursive(&sparse_checkout_dir, &dest_dir)?;

    // 在 tmp_dir 存活时计算 tree hash（避免后续再次克隆）
    let skill_folder_hash = get_skill_folder_hash(&tmp_path, skill_path).unwrap_or_default();

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
        skill_folder_hash,
    })
}

/// 使用 git CLI 进行 sparse checkout 克隆（静默模式）
fn clone_sparse(repo_url: &str, skill_path: &str, dest: &Path) -> Result<()> {
    // skill_path is already the full relative path (e.g., "skills/name" or "name")

    // 1. 检测默认分支
    let default_branch = detect_default_branch(repo_url)?;

    // 2. 克隆仓库（浅克隆 + sparse checkout，静默模式）
    if is_verbose() {
        eprintln!(
            "[verbose] git clone --filter=blob:none --depth=1 --sparse --branch={} {} {}",
            default_branch,
            repo_url,
            dest.display()
        );
    }
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
        .stderr(stderr_stdio())
        .status()
        .with_context(|| "Failed to run git clone")?;

    if !status.success() {
        bail!(
            "git clone failed (exit code: {})",
            status.code().unwrap_or(-1)
        );
    }

    // 3. Initialize sparse checkout (cone mode)
    if is_verbose() {
        eprintln!(
            "[verbose] git sparse-checkout init --cone (in {})",
            dest.display()
        );
    }
    let status = Command::new("git")
        .current_dir(dest)
        .args(["sparse-checkout", "init", "--cone"])
        .stdout(std::process::Stdio::null())
        .stderr(stderr_stdio())
        .status()
        .with_context(|| "Failed to run git sparse-checkout init")?;

    if !status.success() {
        bail!(
            "git sparse-checkout init failed (exit code: {})",
            status.code().unwrap_or(-1)
        );
    }

    // 4. Set sparse checkout path (silent mode)
    if is_verbose() {
        eprintln!(
            "[verbose] git sparse-checkout set {} (in {})",
            skill_path,
            dest.display()
        );
    }
    let status = Command::new("git")
        .current_dir(dest)
        .args(["sparse-checkout", "set", skill_path])
        .stdout(std::process::Stdio::null())
        .stderr(stderr_stdio())
        .status()
        .with_context(|| "Failed to run git sparse-checkout set")?;

    if !status.success() {
        bail!(
            "git sparse-checkout set failed (exit code: {})",
            status.code().unwrap_or(-1)
        );
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

    for entry in
        fs::read_dir(src).with_context(|| format!("Failed to read directory: {}", src.display()))?
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
#[allow(dead_code)]
pub fn install_skill_sparse(
    repo_url: &str,
    skill_path: &str,
    _dest_name: &str,
) -> Result<(TempDir, PathBuf)> {
    let tmp_dir = TempDir::new()?;
    let tmp_path = tmp_dir.path().to_path_buf();
    clone_sparse(repo_url, skill_path, &tmp_path)?;

    // skill_path is already the full relative path (e.g., "skills/name" or "name")
    let source_dir = tmp_path.join(skill_path);
    if !source_dir.exists() {
        bail!("Skill not found in repo: {}", skill_path);
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
        .stderr(stderr_stdio())
        .status()
        .with_context(|| "Failed to run git clone")?;

    if !status.success() {
        bail!("git clone failed");
    }

    Ok(tmp_dir)
}

/// Get the latest commit ID (SHA) of a git repository
pub fn get_latest_commit_hash(repo_dir: &Path) -> Result<String> {
    let output = Command::new("git")
        .current_dir(repo_dir)
        .args(["rev-parse", "HEAD"])
        .output()
        .with_context(|| "Failed to run git rev-parse HEAD")?;

    if !output.status.success() {
        return Ok(String::new());
    }

    let commit_hash = String::from_utf8_lossy(&output.stdout).trim().to_string();
    Ok(commit_hash)
}

/// Get the git tree hash of a skill folder
pub fn get_skill_folder_hash(repo_dir: &Path, skill_path: &str) -> Result<String> {
    // skill_path is the full relative path from repo root (e.g., "skills/name" or "name")
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
