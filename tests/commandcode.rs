//! 集成测试：验证 Command Code 渠道（commandcode）
//!
//! 覆盖：
//! - `xskill platforms list --all` 输出包含 Command Code 及其配置目录 `.commandcode`、agents 文件 `AGENTS.md`
//! - commandcode 默认禁用，不应出现在 `xskill platforms`（非 --all）中
//! - `xskill link --agent commandcode` 因 `agents_compat=true` 被安全跳过（不报错、不创建 `.commandcode`），
//!   与其它 agents_compat 渠道行为一致
//! - 非法渠道名校验

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

/// 创建测试用的 skill 项目（通过 `xskill new`）
///
/// `new` 仅生成项目脚手架，而 `link` 从规范目录 `.agents/skills/<skill>` 读取，
/// 因此此处在创建后将其拷贝进规范目录，模拟“已安装到规范目录”的状态。
fn create_skill_project(workdir: &Path, skill_name: &str) -> std::path::PathBuf {
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

    // 拷贝到规范目录，使 link 能找到该 skill
    let canonical = workdir.join(".agents").join("skills").join(skill_name);
    fs::create_dir_all(canonical.parent().unwrap()).unwrap();
    copy_dir(&skill_dir, &canonical);
    canonical
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
fn test_commandcode_link_skipped_due_to_agents_compat() {
    let tmp = tempfile::tempdir().unwrap();
    let workdir = tmp.path();
    let env = isolated_config_env(workdir);

    // 创建 skill 并放入规范目录（模拟已安装）
    let skill_name = "demo-skill";
    create_skill_project(workdir, skill_name);

    // 链接到 commandcode 渠道：因 agents_compat=true 应被安全跳过
    let out = run_xskill(
        workdir,
        &["link", "--skill", skill_name, "--agent", "commandcode"],
        &as_refs(&env),
    );
    assert!(
        out.status.success(),
        "xskill link commandcode should succeed (skipped): {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Skipped") && stdout.contains("commandcode"),
        "expected 'Skipped: commandcode (agents_compat)', got:\n{}",
        stdout
    );

    // 不应创建任何 .commandcode 目录（skills 软链与 agents 文件均跳过）
    let commandcode_dir = workdir.join(".commandcode");
    assert!(
        !commandcode_dir.exists(),
        "linking to agents_compat platform must not create {}, got {}",
        commandcode_dir.display(),
        commandcode_dir.display()
    );

    // 重复链接同样安全跳过（幂等）
    let again = run_xskill(
        workdir,
        &["link", "--skill", skill_name, "--agent", "commandcode"],
        &as_refs(&env),
    );
    assert!(
        again.status.success(),
        "re-linking commandcode should be idempotent: {}",
        String::from_utf8_lossy(&again.stderr)
    );
    assert!(!commandcode_dir.exists());
}

#[test]
fn test_commandcode_invalid_agent_rejected() {
    let tmp = tempfile::tempdir().unwrap();
    let workdir = tmp.path();
    let env = isolated_config_env(workdir);

    let skill_name = "bad-agent-skill";
    create_skill_project(workdir, skill_name);

    // 不存在的渠道名应被拒绝
    let out = run_xskill(
        workdir,
        &["link", "--skill", skill_name, "--agent", "not-a-real-agent"],
        &as_refs(&env),
    );
    assert!(
        !out.status.success(),
        "expected non-existent agent to be rejected"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("not-a-real-agent"),
        "error should mention the invalid agent name, got: {}",
        stderr
    );
}

#[test]
fn test_commandcode_appears_in_platforms_detailed() {
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
        stdout.contains("Command Code"),
        "platforms list --all should list Command Code, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains(".commandcode"),
        "platforms list --all should show .commandcode path, got:\n{}",
        stdout
    );
    assert!(
        stdout.contains("AGENTS.md"),
        "commandcode agents file should be AGENTS.md, got:\n{}",
        stdout
    );
}

#[test]
fn test_commandcode_not_in_default_enabled_list() {
    let tmp = tempfile::tempdir().unwrap();
    let workdir = tmp.path();
    let env = isolated_config_env(workdir);

    // 不带 --all 的 platforms 仅显示启用的渠道；commandcode 默认禁用，不应出现
    let out = run_xskill(workdir, &["platforms"], &as_refs(&env));
    assert!(out.status.success());

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("Command Code"),
        "commandcode is disabled by default and should NOT appear in `platforms` (non-detailed), got:\n{}",
        stdout
    );
}
