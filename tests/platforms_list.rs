//! 集成测试：验证 `xskill platforms list` 的 `--enabled` 过滤参数
//!
//! 覆盖：
//! - `platforms list --enabled` 以详细视图（含 SKILLS/AGENTS/SOURCE/ENABLED 列）只显示启用渠道
//! - 默认禁用渠道（如 kiro、commandcode）不出现
//! - `--all` 仍显示全部渠道（含禁用）

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

#[test]
fn test_platforms_list_enabled_shows_only_enabled_detailed() {
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
    // 详细视图：应包含详细表头
    assert!(
        stdout.contains("SKILLS") && stdout.contains("ENABLED"),
        "platforms list --enabled should use detailed view, got:\n{}",
        stdout
    );
    // 启用渠道应出现
    assert!(
        stdout.contains("Claude Code") && stdout.contains(".claude"),
        "enabled platform (claude) should appear, got:\n{}",
        stdout
    );
    // 默认禁用渠道不应出现
    for disabled in ["Kiro", "Command Code", "Kilo Code", "LangCLI"] {
        assert!(
            !stdout.contains(disabled),
            "disabled platform ({}) should NOT appear in `platforms list --enabled`, got:\n{}",
            disabled,
            stdout
        );
    }
}

#[test]
fn test_platforms_list_all_still_shows_disabled() {
    let tmp = tempfile::tempdir().unwrap();
    let workdir = tmp.path();
    let env = isolated_config_env(workdir);

    let out = run_xskill(workdir, &["platforms", "list", "--all"], &as_refs(&env));
    assert!(out.status.success());

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Kiro"),
        "--all should still list disabled platform (kiro), got:\n{}",
        stdout
    );
}
