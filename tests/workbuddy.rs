//! 集成测试：验证 WorkBuddy 渠道（workbuddy）
//!
//! workbuddy 沿用 codebuddy 渠道的约定：配置目录 `.workbuddy`、agents 文件 `CODEBUDDY.md`、
//! `agents_compat=false`（link 会真正创建软链，不走 Skipped 分支）。
//!
//! 覆盖（与 docs/PLATFORMS.md 一致）：
//! - workbuddy 默认启用：出现在 `xskill platforms` 与 `platforms list` 中
//! - 详细视图列：PATH=.workbuddy、AGENTS=.workbuddy/CODEBUDDY.md、SOURCE=.agents/AGENTS.md
//! - `agents_compat=false`：`xskill link --agent workbuddy` 创建 skills 软链且可重复执行（幂等）

use std::fs;
use std::path::Path;
use std::path::PathBuf;
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
fn install_skill_to_canonical(workdir: &Path, skill_name: &str) -> PathBuf {
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
    canonical
}

#[test]
fn test_workbuddy_in_default_platforms_list() {
    let tmp = tempfile::tempdir().unwrap();
    let workdir = tmp.path();
    let env = isolated_config_env(workdir);

    // 不带 --all 的 platforms 仅显示启用渠道；workbuddy 默认启用，应出现
    let out = run_xskill(workdir, &["platforms"], &as_refs(&env));
    assert!(
        out.status.success(),
        "xskill platforms failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("WorkBuddy") && stdout.contains(".workbuddy"),
        "workbuddy is enabled by default and should appear in `platforms`, got:\n{}",
        stdout
    );
}

#[test]
fn test_workbuddy_appears_in_platforms_list_detailed() {
    let tmp = tempfile::tempdir().unwrap();
    let workdir = tmp.path();
    let env = isolated_config_env(workdir);

    let out = run_xskill(workdir, &["platforms", "list"], &as_refs(&env));
    assert!(
        out.status.success(),
        "xskill platforms list failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("WorkBuddy"),
        "platforms list should list WorkBuddy, got:\n{}",
        stdout
    );
    // 详细视图列：PATH=.workbuddy、SKILLS=skills、AGENTS=.workbuddy/CODEBUDDY.md、SOURCE=.agents/AGENTS.md
    assert!(
        stdout.contains(".workbuddy")
            && stdout.contains(".workbuddy/CODEBUDDY.md")
            && stdout.contains(".agents/AGENTS.md"),
        "workbuddy row should show .workbuddy path with CODEBUDDY.md agents file and .agents/AGENTS.md source, got:\n{}",
        stdout
    );
}

#[test]
fn test_workbuddy_appears_in_platforms_all() {
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
        stdout.contains("WorkBuddy") && stdout.contains(".workbuddy"),
        "platforms list --all should include WorkBuddy, got:\n{}",
        stdout
    );
}

#[test]
fn test_workbuddy_link_creates_skill_symlink() {
    let tmp = tempfile::tempdir().unwrap();
    let workdir = tmp.path();
    let env = isolated_config_env(workdir);

    let skill_name = "workbuddy-demo-skill";
    let canonical = install_skill_to_canonical(workdir, skill_name);

    // workbuddy 为 agents_compat=false，link 应真正创建 .workbuddy/skills/<skill> 软链
    let out = run_xskill(
        workdir,
        &["link", "--skill", skill_name, "--agent", "workbuddy"],
        &as_refs(&env),
    );
    assert!(
        out.status.success(),
        "xskill link workbuddy failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // 软链存在且指向规范目录
    let link_path = workdir.join(".workbuddy").join("skills").join(skill_name);
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
        "workbuddy is agents_compat=false; should not be skipped, got:\n{}",
        stdout
    );
}

#[test]
fn test_workbuddy_link_idempotent() {
    let tmp = tempfile::tempdir().unwrap();
    let workdir = tmp.path();
    let env = isolated_config_env(workdir);

    let skill_name = "workbuddy-idempotent-skill";
    install_skill_to_canonical(workdir, skill_name);

    let args = ["link", "--skill", skill_name, "--agent", "workbuddy"];
    let first = run_xskill(workdir, &args, &as_refs(&env));
    assert!(
        first.status.success(),
        "xskill link workbuddy failed: {}",
        String::from_utf8_lossy(&first.stderr)
    );

    let link_path = workdir.join(".workbuddy").join("skills").join(skill_name);
    assert!(link_path.exists() && link_path.is_symlink());

    // 重复链接不应报错（幂等）
    let second = run_xskill(workdir, &args, &as_refs(&env));
    assert!(
        second.status.success(),
        "re-linking workbuddy should be idempotent: {}",
        String::from_utf8_lossy(&second.stderr)
    );
    assert!(link_path.exists() && link_path.is_symlink());
}
