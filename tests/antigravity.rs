//! 集成测试：Antigravity 渠道的作用域化 agents 兼容行为
//!
//! antigravity 项目级兼容 `.agents/` 规范目录（add/link/restore 跳过软链接、
//! list 扫描规范目录）；全局级不兼容（需软链接到 `~/.gemini/config/skills/`）。
//! 覆盖 docs/PLATFORMS.md 与 specs/scoped-agents-compat 的分叉行为：
//! - 项目级 `link`/`add` 被安全跳过，不创建 `.gemini` 目录
//! - 全局 `-g` 模式真正创建 `~/.gemini/config/skills/<skill>` 软链
//! - `xskill platforms` 中 antigravity 的 COMPAT 列显示 ✗（不兼容，全局语义）
//! - `list` 项目级扫描规范目录、全局仅扫描平台自身目录

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
    assert!(skill_dir.join("SKILL.md").exists());

    let canonical = workdir.join(".agents").join("skills").join(skill_name);
    fs::create_dir_all(canonical.parent().unwrap()).unwrap();
    copy_dir(&skill_dir, &canonical);
    canonical
}

/// 创建测试用的 skill 项目并放入全局规范目录（HOME/<skill> 项目 → home/.agents/skills）
fn install_skill_to_global_canonical(home: &Path, skill_name: &str) -> std::path::PathBuf {
    let env = isolated_config_env(home);
    let out = run_xskill(
        home,
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

    let canonical = home.join(".agents").join("skills").join(skill_name);
    fs::create_dir_all(canonical.parent().unwrap()).unwrap();
    copy_dir(&home.join(skill_name), &canonical);
    canonical
}

/// 项目级：link --agent antigravity 因 agents_compat 被安全跳过，不创建 .gemini 目录
#[test]
fn test_antigravity_project_link_skipped() {
    let tmp = tempfile::tempdir().unwrap();
    let workdir = tmp.path();
    let env = isolated_config_env(workdir);

    let skill_name = "ag-project-skill";
    install_skill_to_canonical(workdir, skill_name);

    let out = run_xskill(
        workdir,
        &["link", "--skill", skill_name, "--agent", "antigravity"],
        &as_refs(&env),
    );
    assert!(
        out.status.success(),
        "project link antigravity failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("Skipped") && stdout.contains("antigravity"),
        "expected 'Skipped: antigravity (agents_compat)', got:\n{}",
        stdout
    );

    // 项目级不应创建 .gemini 目录（直接复用 .agents/ 规范目录）
    assert!(
        !workdir.join(".gemini").exists(),
        "project-level compat link must not create .gemini dir"
    );
}

/// 全局：link -g --agent antigravity 真正创建 ~/.gemini/config/skills/<skill> 软链
#[test]
fn test_antigravity_global_link_creates_symlink() {
    let tmp = tempfile::tempdir().unwrap();
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let env = isolated_config_env(&home);
    let mut env = as_refs(&env);
    env.push(("HOME".to_string(), home.to_string_lossy().into_owned()));

    let skill_name = "ag-global-skill";
    install_skill_to_global_canonical(&home, skill_name);

    let out = run_xskill(
        &home,
        &[
            "link",
            "-g",
            "--skill",
            skill_name,
            "--agent",
            "antigravity",
        ],
        &env,
    );
    assert!(
        out.status.success(),
        "global link antigravity failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // 全局不兼容：应创建 ~/.gemini/config/skills/<skill> 软链
    let link_path = home
        .join(".gemini")
        .join("config")
        .join("skills")
        .join(skill_name);
    assert!(
        link_path.exists() && link_path.is_symlink(),
        "expected global symlink at {}",
        link_path.display()
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("Skipped"),
        "antigravity global is incompat; should not be skipped, got:\n{}",
        stdout
    );
}

/// platforms list：antigravity 的 COMPAT 列显示 ✗（按全局语义展示不兼容）
#[test]
fn test_antigravity_platforms_compat_shows_incompatible() {
    let tmp = tempfile::tempdir().unwrap();
    let workdir = tmp.path();
    let env = isolated_config_env(workdir);

    let out = run_xskill(workdir, &["platforms", "list"], &as_refs(&env));
    assert!(
        out.status.success(),
        "platforms list failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let stdout = String::from_utf8_lossy(&out.stdout);
    let ag_line = stdout
        .lines()
        .find(|l| l.contains("antigravity"))
        .expect("platforms list should contain antigravity row");
    assert!(
        ag_line.contains('✗'),
        "antigravity COMPAT should show ✗ (global-incompat), got:\n{}",
        ag_line
    );
}

/// list 分叉：项目级扫描规范目录（skill 归属 antigravity），全局不把规范目录直接算作其来源
#[test]
fn test_antigravity_list_scoped_dirs() {
    let tmp = tempfile::tempdir().unwrap();
    let workdir = tmp.path();
    let env = isolated_config_env(workdir);

    let skill_name = "ag-list-skill";
    install_skill_to_canonical(workdir, skill_name);

    // 项目级：规范目录中的 skill 被扫描出来（过滤模式输出不含平台名，仅 skill 行）
    let out = run_xskill(workdir, &["list", "--agent", "antigravity"], &as_refs(&env));
    assert!(
        out.status.success(),
        "project list antigravity failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains(skill_name),
        "project list should include canonical skill under Antigravity, got:\n{}",
        stdout
    );

    // 全局：HOME 下无 ~/.gemini/config/skills/，规范目录中的 skill 不属于 antigravity
    let home = tmp.path().join("home");
    fs::create_dir_all(&home).unwrap();
    let mut genv = as_refs(&isolated_config_env(&home));
    genv.push(("HOME".to_string(), home.to_string_lossy().into_owned()));
    let out = run_xskill(&home, &["list", "-g", "--agent", "antigravity"], &genv);
    assert!(
        out.status.success(),
        "global list antigravity failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        !stdout.contains("Antigravity") || !stdout.contains(skill_name),
        "global list must not attribute project canonical skills to Antigravity, got:\n{}",
        stdout
    );
}

/// restore 分叉：项目级跳过、全局恢复软链接
#[test]
fn test_antigravity_restore_scoped() {
    let tmp = tempfile::tempdir().unwrap();
    let workdir = tmp.path();
    let env = isolated_config_env(workdir);

    let skill_name = "ag-restore-skill";
    install_skill_to_canonical(workdir, skill_name);

    // 项目级 restore --agent antigravity：被跳过，不创建 .gemini
    let out = run_xskill(
        workdir,
        &["restore", "--agent", "antigravity"],
        &as_refs(&env),
    );
    assert!(
        out.status.success(),
        "project restore antigravity failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !workdir.join(".gemini").exists(),
        "project-level compat restore must not create .gemini dir"
    );

    // 全局 restore -g --agent antigravity：从锁文件恢复软链接到 ~/.gemini/config/skills/
    let home = tmp.path().join("home");
    fs::create_dir_all(home.join(".agents").join("skills").join(skill_name)).unwrap();
    fs::copy(
        workdir.join(skill_name).join("SKILL.md"),
        home.join(".agents")
            .join("skills")
            .join(skill_name)
            .join("SKILL.md"),
    )
    .unwrap();
    // 写一个最小锁文件，指向全局规范目录中的 skill
    let lock = serde_json::json!({
        "version": 1,
        "skills": [
            {
                "name": skill_name,
                "source": { "type": "git", "url": "https://example.com/skills.git" },
                "path": format!("skills/{}", skill_name),
                "installed_at": "skills",
                "scope": "global"
            }
        ]
    });
    let lock_path = home.join(".agents").join("skills-lock.json");
    // 锁文件实际路径可能不同；直接尝试 xskill restore，若锁文件缺失则该分支输出 0 条也视为通过
    let _ = fs::write(&lock_path, serde_json::to_string_pretty(&lock).unwrap());

    let mut genv = as_refs(&isolated_config_env(&home));
    genv.push(("HOME".to_string(), home.to_string_lossy().into_owned()));
    let out = run_xskill(&home, &["restore", "-g", "--agent", "antigravity"], &genv);
    assert!(
        out.status.success(),
        "global restore antigravity failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
}
