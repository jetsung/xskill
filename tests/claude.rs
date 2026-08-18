//! 集成测试：验证非 agents_compat 渠道的链接行为（以 Claude Code 为代表）
//!
//! commandcode 集成测试已覆盖 `agents_compat=true`（链接被跳过）分支；
//! 本文件覆盖 `agents_compat=false` 分支：link 应真正创建 skills 软链并处理 agents 文件。
//! 仅选取 claude 作为代表渠道，避免 20 个渠道重复测试。

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

/// 创建测试用的 skill 项目并放入规范目录（模拟已安装到 .agents/skills）
fn install_skill_to_canonical(workdir: &Path, skill_name: &str) -> std::path::PathBuf {
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
    assert!(skill_dir.join("SKILL.md").exists());

    let canonical = workdir.join(".agents").join("skills").join(skill_name);
    fs::create_dir_all(canonical.parent().unwrap()).unwrap();
    copy_dir(&skill_dir, &canonical);
    canonical
}

#[test]
fn test_claude_link_creates_skill_symlink() {
    let tmp = tempfile::tempdir().unwrap();
    let workdir = tmp.path();
    let env = isolated_config_env(workdir);

    let skill_name = "demo-skill";
    install_skill_to_canonical(workdir, skill_name);

    // claude 为 agents_compat=false，link 应真正创建 .claude/skills/<skill> 软链
    let out = run_xskill(
        workdir,
        &["link", "--skill", skill_name, "--agent", "claude"],
        &as_refs(&env),
    );
    assert!(
        out.status.success(),
        "xskill link claude failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // 软链存在且指向规范目录
    let link_path = workdir.join(".claude").join("skills").join(skill_name);
    assert!(
        link_path.exists(),
        "expected symlink at {}",
        link_path.display()
    );
    assert!(
        link_path.is_symlink(),
        "expected {} to be a symlink",
        link_path.display()
    );

    let canonical = workdir.join(".agents").join("skills").join(skill_name);
    let target = fs::read_link(&link_path).unwrap();
    let target_display = target.to_string_lossy().into_owned();
    // create_relative_symlink 生成的是相对软链（便于目录迁移），需解析为绝对路径再比对
    let resolved = if target.is_absolute() {
        target
    } else {
        link_path.parent().unwrap().join(&target).canonicalize().unwrap()
    };
    assert_eq!(
        resolved,
        canonical.canonicalize().unwrap(),
        "symlink target mismatch: {:?} != {:?}",
        target_display,
        canonical
    );

    // 非 compat 渠道不会打印 Skipped（agents_compat 分支不应命中）
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("Skipped"),
        "claude is agents_compat=false; should not be skipped, got:\n{}",
        stdout
    );
}

#[test]
fn test_claude_link_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let workdir = tmp.path();
    let env = isolated_config_env(workdir);

    let skill_name = "idempotent-skill";
    install_skill_to_canonical(workdir, skill_name);

    let first = run_xskill(
        workdir,
        &["link", "--skill", skill_name, "--agent", "claude"],
        &as_refs(&env),
    );
    assert!(first.status.success());

    let link_path = workdir.join(".claude").join("skills").join(skill_name);
    assert!(link_path.exists() && link_path.is_symlink());

    // 重复链接不应报错（幂等）
    let second = run_xskill(
        workdir,
        &["link", "--skill", skill_name, "--agent", "claude"],
        &as_refs(&env),
    );
    assert!(
        second.status.success(),
        "re-linking claude should be idempotent: {}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert!(link_path.exists() && link_path.is_symlink());
}
