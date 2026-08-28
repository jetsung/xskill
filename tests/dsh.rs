//! 集成测试：验证 DeepSeek Harness 渠道（dsh）
//!
//! 覆盖（与 docs/PLATFORMS.md 一致）：
//! - dsh 默认启用：出现在 `xskill platforms`（非 --all）与 `platforms list --enabled` 中
//! - 配置目录为 `.dsh`、agents 文件为 `AGENTS.md`、source 为 `AGENTS.md`
//! - `agents_compat=true`：`xskill link --agent dsh` 被安全跳过，不创建 `.dsh` 目录
//! - 未启用渠道显式指定仍可用（dsh 已默认启用，此处校验渠道名可被识别）

use std::fs;
use std::path::Path;
use std::process::Command;

/// 返回编译后的 xskill 二进制路径（集成测试中由 cargo 自动提供）
fn bin() -> &'static str {
    env!("CARGO_BIN_EXE_xskill")
}

/// 在指定工作目录运行 xskill，传入环境变量（隔离用户真实配置）
fn run_xskill(workdir: &Path, args: &[&str], extra_env: &[(String, String)]) -> std::process::Output {
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
    env.iter().map(|(k, v)| ((*k).to_string(), v.clone())).collect()
}

/// 创建测试用的 skill 项目（通过 `xskill new`）并放入规范目录
fn create_skill_project(workdir: &Path, skill_name: &str) {
    let env = isolated_config_env(workdir);
    let out = run_xskill(
        workdir,
        &["new", "--name", skill_name, "--description", "integration test skill"],
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
fn test_dsh_in_default_platforms_list() {
    let tmp = tempfile::tempdir().unwrap();
    let workdir = tmp.path();
    let env = isolated_config_env(workdir);

    // 不带 --all 的 platforms 仅显示启用渠道；dsh 默认启用，应出现
    let out = run_xskill(workdir, &["platforms"], &as_refs(&env));
    assert!(
        out.status.success(),
        "xskill platforms failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("DeepSeek Harness") && stdout.contains(".dsh"),
        "dsh is enabled by default and should appear in `platforms`, got:\n{}",
        stdout
    );
}

#[test]
fn test_dsh_appears_in_platforms_enabled_detailed() {
    let tmp = tempfile::tempdir().unwrap();
    let workdir = tmp.path();
    let env = isolated_config_env(workdir);

    let out = run_xskill(workdir, &["platforms", "list", "--enabled"], &as_refs(&env));
    assert!(
        out.status.success(),
        "xskill platforms list --enabled failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("DeepSeek Harness"),
        "platforms list --enabled should list DeepSeek Harness, got:\n{}",
        stdout
    );
    // 详细视图列：PATH=.dsh、SKILLS=skills、AGENTS=.dsh/AGENTS.md、SOURCE=.agents/AGENTS.md
    assert!(
        stdout.contains(".dsh") && stdout.contains(".dsh/AGENTS.md") && stdout.contains(".agents/AGENTS.md"),
        "dsh row should show .dsh path with AGENTS.md agents file and .agents/AGENTS.md source, got:\n{}",
        stdout
    );
}

#[test]
fn test_dsh_link_skipped_due_to_agents_compat() {
    let tmp = tempfile::tempdir().unwrap();
    let workdir = tmp.path();
    let env = isolated_config_env(workdir);

    let skill_name = "dsh-demo-skill";
    create_skill_project(workdir, skill_name);

    let out = run_xskill(
        workdir,
        &["link", "--skill", skill_name, "--agent", "dsh"],
        &as_refs(&env),
    );
    assert!(
        out.status.success(),
        "xskill link dsh should succeed (skipped): {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Skipped") && stdout.contains("dsh"),
        "expected 'Skipped: dsh (agents_compat)', got:\n{}",
        stdout
    );

    // agents_compat 渠道不应创建自身配置目录
    let dsh_dir = workdir.join(".dsh");
    assert!(
        !dsh_dir.exists(),
        "linking to agents_compat platform must not create {}, got {}",
        dsh_dir.display(),
        dsh_dir.display()
    );
}
