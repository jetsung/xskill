//! 集成测试：验证 Junie CLI 渠道（junie）
//!
//! 覆盖（与 docs/PLATFORMS.md 一致）：
//! - junie 默认不启用：不出现在默认 `xskill platforms` 中，`platforms list --all` 可见
//! - 配置目录为 `.junie`、agents 文件为 `AGENTS.md`
//! - `agents_compat=true`：`xskill link --agent junie` 被安全跳过，不创建 `.junie` 目录

use std::fs;
use std::path::Path;
use std::process::Command;

/// 返回编译后的 xskill 二进制路径（集成测试中由 cargo 自动提供）
fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_xskill")
}

/// 在指定工作目录运行 xskill，传入环境变量（隔离用户真实配置）
fn run_xskill(
    workdir: &Path,
    args: &[&str],
    extra_env: &[(String, String)],
) -> std::process::Output {
    let mut cmd = Command::new(bin());
    cmd.current_dir(workdir);
    for (k, v) in extra_env {
        cmd.env(k, v);
    }
    cmd.args(args);
    cmd.output()
        .unwrap_or_else(|e| panic!("failed to run xskill {:?}: {}", args, e))
}

/// 生成一个指向不存在配置文件的 XSKILL_CONFIG 环境变量，强制使用内置默认平台列表
fn isolated_config_env(tmp: &Path) -> Vec<(&'static str, String)> {
    let cfg = tmp.join("nonexistent-settings.json");
    vec![("XSKILL_CONFIG", cfg.to_string_lossy().into_owned())]
}

/// 将 (&str, String) 转为 (String, String) 以供 run_xskill 借用
fn as_refs(env: &[(&str, String)]) -> Vec<(String, String)> {
    env.iter()
        .map(|(k, v)| ((*k).to_string(), v.clone()))
        .collect()
}

/// 创建测试用的 skill 项目（通过 `xskill new`）并放入规范目录
fn create_skill_project(workdir: &Path, skill_name: &str) {
    let env = isolated_config_env(workdir);
    let out = run_xskill(
        workdir,
        &[
            "new",
            "--name",
            skill_name,
            "--description",
            "integration test skill",
        ],
        &as_refs(&env),
    );
    assert!(
        out.status.success(),
        "xskill new failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let skill_dir = workdir.join(skill_name);
    assert!(
        skill_dir.join("SKILL.md").exists(),
        "expected SKILL.md to be created at {}",
        skill_dir.display()
    );

    let canonical = workdir.join(".agents").join("skills").join(skill_name);
    fs::create_dir_all(canonical.parent().unwrap()).unwrap();
    copy_dir(&skill_dir, &canonical);
}

/// 递归拷贝目录（测试辅助）
fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let src = entry.path();
        let dst = to.join(entry.file_name());
        if src.is_dir() {
            copy_dir(&src, &dst);
        } else {
            fs::copy(&src, &dst).unwrap();
        }
    }
}

#[test]
fn test_junie_in_default_platforms_list() {
    let tmp = tempfile::tempdir().unwrap();
    let workdir = tmp.path();
    let env = isolated_config_env(workdir);

    // 不带 --all 的 platforms 仅显示启用渠道；junie 默认不启用，不应出现
    let out = run_xskill(workdir, &["platforms"], &as_refs(&env));
    assert!(
        out.status.success(),
        "xskill platforms failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("Junie CLI"),
        "junie is disabled by default and should NOT appear in `platforms`, got:\n{}",
        stdout
    );

    // --all 视图中 junie 可见
    let out = run_xskill(workdir, &["platforms", "list", "--all"], &as_refs(&env));
    assert!(
        out.status.success(),
        "xskill platforms list --all failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Junie CLI") && stdout.contains(".junie"),
        "platforms list --all should include Junie CLI, got:\n{}",
        stdout
    );
}

#[test]
fn test_junie_appears_in_platforms_list_detailed() {
    let tmp = tempfile::tempdir().unwrap();
    let workdir = tmp.path();
    let env = isolated_config_env(workdir);

    let out = run_xskill(workdir, &["platforms", "list", "--all"], &as_refs(&env));
    assert!(
        out.status.success(),
        "xskill platforms list --all failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Junie CLI"),
        "platforms list --all should list Junie CLI, got:\n{}",
        stdout
    );
    // 详细视图列：PATH=.junie、SKILLS=skills、AGENTS=.junie/AGENTS.md
    assert!(
        stdout.contains(".junie") && stdout.contains(".junie/AGENTS.md"),
        "junie row should show .junie path with AGENTS.md agents file, got:\n{}",
        stdout
    );
}

#[test]
fn test_junie_link_skipped_due_to_agents_compat() {
    let tmp = tempfile::tempdir().unwrap();
    let workdir = tmp.path();
    let env = isolated_config_env(workdir);

    let skill_name = "junie-demo-skill";
    create_skill_project(workdir, skill_name);

    let out = run_xskill(
        workdir,
        &["link", "--skill", skill_name, "--agent", "junie"],
        &as_refs(&env),
    );
    assert!(
        out.status.success(),
        "xskill link junie should succeed (skipped): {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Skipped") && stdout.contains("junie"),
        "expected 'Skipped: junie (agents_compat)', got:\n{}",
        stdout
    );

    // agents_compat 渠道不应创建自身配置目录
    let junie_dir = workdir.join(".junie");
    assert!(
        !junie_dir.exists(),
        "linking to agents_compat platform must not create {}",
        junie_dir.display()
    );
}
