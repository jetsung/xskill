use crate::config::{Config, Platform, default_config, default_platforms};
use crate::output::print_table;
use anyhow::{Result, bail};
use colored::Colorize;
use std::io::IsTerminal;

fn compat_str(agents_compat: bool) -> String {
    if agents_compat {
        "✓".green().to_string()
    } else {
        "✗".red().to_string()
    }
}

/// 路径类单元格着色：非空值显示为暗灰色（空值由 print_table 统一输出 dimmed " - "）
fn path_str(path: &str) -> String {
    if path.is_empty() {
        String::new()
    } else {
        path.dimmed().to_string()
    }
}

pub fn run(all: bool) -> Result<()> {
    let config = Config::load()?;

    if config.platforms.is_empty() {
        println!("{}", "No platforms configured".yellow());
        return Ok(());
    }

    // 默认只显示启用渠道；--all 显示全部（含禁用渠道）
    // 所有视图均输出完整列信息（NAME/KEY/PATH/SKILLS/AGENTS/COMPAT/BUILTIN/ENABLED）
    let shown: Vec<(&String, &Platform)> = config
        .platforms
        .iter()
        .filter(|(_, p)| all || p.enabled)
        .collect();
    let mut sorted = shown;
    sorted.sort_by_key(|(name, p)| (p.display_name(name).to_lowercase(), name.to_lowercase()));

    let headers = &["NAME", "KEY", "PATH", "SKILLS", "AGENTS", "COMPAT", "BUILTIN", "ENABLED"];
    let rows: Vec<Vec<String>> = sorted
        .iter()
        .map(|(name, platform)| {
            let skills_dir = platform
                .skills_dir()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            let agents_file = platform
                .agents_file()
                .map(|p| p.to_string_lossy().into_owned())
                .unwrap_or_default();
            let compat = compat_str(platform.agents_compat);
            let builtin = compat_str(platform.builtin);
            let enabled = if platform.enabled {
                "✓".green().to_string()
            } else {
                "✗".red().to_string()
            };
            vec![
                platform.display_name(name),
                name.to_string(),
                path_str(&platform.path),
                path_str(&skills_dir),
                path_str(&agents_file),
                compat,
                builtin,
                enabled,
            ]
        })
        .collect();
    print_table(headers, &rows);

    Ok(())
}

/// 重置模式
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResetMode {
    /// 完全替换：platforms 重置为默认列表（丢弃自定义）
    Replace,
    /// 覆盖：默认值覆盖内置平台，保留自定义平台
    Merge,
}

/// 返回当前配置中的自定义平台名称（不在默认列表中的，排序）
fn custom_platform_names(config: &Config) -> Vec<String> {
    let defaults = default_platforms();
    let mut names: Vec<String> = config
        .platforms
        .keys()
        .filter(|k| !defaults.contains_key(*k))
        .cloned()
        .collect();
    names.sort_unstable();
    names
}

/// 按模式应用重置，修改 config.platforms
fn apply_reset_mode(config: &mut Config, mode: ResetMode) {
    let defaults = default_platforms();
    config.platforms = match mode {
        // 完全替换：直接使用默认列表
        ResetMode::Replace => defaults,
        // 覆盖：默认列表 + 自定义平台（内置平台保持默认值，自定义平台原样保留）
        ResetMode::Merge => {
            let mut merged = defaults;
            for (name, platform) in std::mem::take(&mut config.platforms) {
                merged.entry(name).or_insert(platform);
            }
            merged
        }
    };
}

/// platforms reset 无旗标时的用法帮助（不执行任何重置）
pub fn run_reset_usage() {
    println!("Usage: xskill platforms reset (--replace | --merge)");
    println!();
    println!("Modes:");
    println!("  -r, --replace  完全恢复：重置为默认列表，丢弃自定义平台");
    println!("  -m, --merge    谨慎合并：内置平台恢复默认，保留自定义平台");
}

/// 重置 platforms（非交互，由 --replace/--merge 旗标指定模式）
pub fn run_reset(mode: ResetMode) -> Result<()> {
    // 配置文件不存在时使用与 config --init 相同的默认配置，保证生成字段一致
    let mut config = if Config::config_path().exists() {
        Config::load()?
    } else {
        default_config()
    };

    let custom = custom_platform_names(&config);
    if !custom.is_empty() {
        println!("{}: {}", "Custom platforms".yellow(), custom.join(", "));
    }

    apply_reset_mode(&mut config, mode);
    // 保存前规范化：内置渠道精简为 name/enabled/builtin，自定义渠道 builtin 恒 false
    config.normalize_platforms_for_save();
    config.save()?;

    let desc = match mode {
        ResetMode::Replace => "replaced with defaults (custom dropped)",
        ResetMode::Merge => "merged (custom kept)",
    };
    println!(
        "{}: {} platforms, {}",
        "Platforms reset".green(),
        config.platforms.len(),
        desc
    );
    Ok(())
}

/// 校验并切换指定渠道的 enabled 状态，返回已切换的 key 列表
fn apply_toggle(config: &mut Config, keys: &[String]) -> Result<Vec<String>> {
    let mut toggled: Vec<String> = Vec::new();
    for key in keys {
        let Some(platform) = config.platforms.get_mut(key) else {
            bail!(
                "Invalid platform: {}\nValid platforms: {}",
                key,
                config.platform_names().join(", ")
            );
        };
        platform.enabled = !platform.enabled;
        toggled.push(key.clone());
    }
    Ok(toggled)
}

/// 按勾选状态设置指定渠道的 enabled（set 语义，非翻转），返回状态发生变化的 key 列表
fn apply_enabled_state(config: &mut Config, states: &[(String, bool)]) -> Result<Vec<String>> {
    let mut changed: Vec<String> = Vec::new();
    for (key, enabled) in states {
        let Some(platform) = config.platforms.get_mut(key) else {
            bail!(
                "Invalid platform: {}\nValid platforms: {}",
                key,
                config.platform_names().join(", ")
            );
        };
        if platform.enabled != *enabled {
            platform.enabled = *enabled;
            changed.push(key.clone());
        }
    }
    Ok(changed)
}

/// 切换渠道启用状态：无参数时弹出 TUI 多选，指定 key 时直接切换
pub fn run_toggle(keys: &[String]) -> Result<()> {
    let mut config = Config::load()?;

    // 无参数：交互式 TUI（按当前 enabled 预勾选，确认后按勾选状态设置）
    let changed: Vec<String> = if keys.is_empty() {
        if !std::io::stdin().is_terminal() {
            bail!("'platforms toggle' requires an interactive terminal or platform keys.");
        }
        let Some(states) = select_toggle_platforms(&config)? else {
            println!("{}", "Cancelled.".yellow());
            return Ok(());
        };
        apply_enabled_state(&mut config, &states)?
    } else {
        apply_toggle(&mut config, keys)?
    };

    if changed.is_empty() {
        println!("{}", "No changes.".yellow());
        return Ok(());
    }

    // 保存前规范化（内置渠道精简保存，自定义渠道 builtin 恒 false）
    config.normalize_platforms_for_save();
    config.save()?;

    for key in &changed {
        let platform = &config.platforms[key];
        let state = if platform.enabled {
            "enabled".green().to_string()
        } else {
            "disabled".red().to_string()
        };
        println!("{}: {}", platform.display_name(key), state);
    }
    Ok(())
}

/// TUI 多选渠道：列出全部渠道并按当前 enabled 状态预勾选，确认后按勾选状态设置
///
/// 使用 crossterm 自绘而非 skim：skim 的未选中勾选位硬编码为等宽空格，
/// 无法显示 `[ ]`；自绘可完全控制勾选框、颜色与按键行为。
///
/// 返回 `None` 表示用户取消（Esc/Ctrl-C）；`Some` 为确认后的 (key, 勾选状态) 列表。
fn select_toggle_platforms(config: &Config) -> Result<Option<Vec<(String, bool)>>> {
    use crossterm::event::{self, Event, KeyCode, KeyModifiers, KeyEventKind};
    use crossterm::terminal::{self, EnterAlternateScreen, LeaveAlternateScreen};
    use crossterm::{execute, style::Print};
    use std::io::{stdout, Write};

    // 按显示名称排序的 (key, name)
    let mut entries: Vec<(String, String)> = config
        .platforms
        .iter()
        .map(|(key, p)| (key.clone(), p.display_name(key)))
        .collect();
    entries.sort_by(|a, b| a.1.to_lowercase().cmp(&b.1.to_lowercase()));
    let total = entries.len();

    terminal::enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, EnterAlternateScreen)?;

    let result = (|| -> Result<Option<Vec<(String, bool)>>> {
        // 初始勾选 = 当前 enabled 状态（与排序后的 entries 一一对应）
        let mut checked: Vec<bool> = entries
            .iter()
            .map(|(key, _)| config.platforms[key].enabled)
            .collect();
        let mut cursor = 0usize;

        let draw = |out: &mut std::io::Stdout, cursor: usize, checked: &[bool]| -> Result<()> {
            execute!(out, Print("\x1b[2J\x1b[1;1H"))?;
            // raw mode 下 \n 只换行不回车（CR），行尾必须用 \r\n，否则逐行累积缩进
            write!(out, "Toggle platforms:\r\n")?;
            // 列表完全铺满：全部渠道一次性显示，无窗口滚动
            for i in 0..total {
                let (key, name) = &entries[i];
                let checkbox = if checked[i] { "[x]" } else { "[ ]" };
                // 已勾选（启用）的行名称用绿色；未勾选用默认色
                let name_text = if checked[i] {
                    name.green().to_string()
                } else {
                    name.to_string()
                };
                let line = if i == cursor {
                    format!(
                        "❯ {} {}  [{}]",
                        checkbox,
                        name_text.bold(),
                        key.dimmed()
                    )
                } else {
                    format!("  {} {}  [{}]", checkbox, name_text, key.dimmed())
                };
                write!(out, "{}\r\n", line)?;
            }
            write!(out, "\r\n")?;
            write!(
                out,
                "{}\r\n",
                "SPACE: toggle | ↑/↓: move | enter confirm | esc cancel".dimmed()
            )?;
            out.flush()?;
            Ok(())
        };

        draw(&mut out, cursor, &checked)?;
        loop {
            let Event::Key(key) = event::read()? else {
                continue;
            };
            if key.kind != KeyEventKind::Press {
                continue;
            }
            match key.code {
                KeyCode::Char(' ') | KeyCode::Tab => {
                    checked[cursor] = !checked[cursor];
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    if cursor > 0 {
                        cursor -= 1;
                    }
                }
                KeyCode::Down | KeyCode::Char('j') => {
                    if cursor + 1 < total {
                        cursor += 1;
                    }
                }
                KeyCode::Enter => {
                    return Ok(Some(
                        entries
                            .iter()
                            .zip(checked.iter())
                            .map(|((key, _), c)| (key.clone(), *c))
                            .collect(),
                    ));
                }
                KeyCode::Esc => return Ok(None),
                KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                    return Ok(None)
                }
                _ => continue,
            }
            draw(&mut out, cursor, &checked)?;
        }
    })();

    execute!(out, LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    result
}

#[cfg(test)]
mod tests {
    use super::{ResetMode, compat_str, default_platforms};
    use crate::config::{Config, Platform, default_config};
    use colored::Colorize;

    #[test]
    fn test_compat_str() {
        assert_eq!(compat_str(true), "✓".green().to_string());
        assert_eq!(compat_str(false), "✗".red().to_string());
    }

    #[test]
    fn test_apply_enabled_state() {
        use super::apply_enabled_state;
        use crate::config::default_config;

        // 按勾选状态设置：相同状态不记录变更，不同状态被设置并记录
        let mut config = default_config();
        // claude 当前 enabled=true：设为 true 不变，设为 false 变更
        // kiro 当前 enabled=false：设为 true 变更
        let states = vec![
            ("claude".to_string(), true),
            ("claude".to_string(), false),
            ("kiro".to_string(), true),
        ];
        let changed = apply_enabled_state(&mut config, &states).unwrap();
        assert_eq!(changed, vec!["claude", "kiro"]);
        assert!(!config.platforms["claude"].enabled);
        assert!(config.platforms["kiro"].enabled);

        // 全部相同状态：无变更
        let mut config = default_config();
        let states: Vec<(String, bool)> = config
            .platforms
            .iter()
            .map(|(k, p)| (k.clone(), p.enabled))
            .collect();
        let changed = apply_enabled_state(&mut config, &states).unwrap();
        assert!(changed.is_empty());

        // 不存在的渠道报错
        let mut config = default_config();
        let err = apply_enabled_state(
            &mut config,
            &[("nonexistent".to_string(), true)],
        )
        .unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("Invalid platform: nonexistent"));
        assert!(msg.contains("Valid platforms:"));
    }

    #[test]
    fn test_apply_toggle() {
        use super::apply_toggle;

        // 切换单个渠道：enabled 翻转
        let mut config = default_config();
        assert!(config.platforms["claude"].enabled);
        let toggled = apply_toggle(&mut config, &["claude".to_string()]).unwrap();
        assert_eq!(toggled, vec!["claude"]);
        assert!(!config.platforms["claude"].enabled);

        // 再次切换恢复原状
        apply_toggle(&mut config, &["claude".to_string()]).unwrap();
        assert!(config.platforms["claude"].enabled);

        // 批量切换：禁用渠道被启用
        let mut config = default_config();
        assert!(!config.platforms["kiro"].enabled);
        assert!(!config.platforms["agentty"].enabled);
        apply_toggle(&mut config, &["kiro".to_string(), "agentty".to_string()]).unwrap();
        assert!(config.platforms["kiro"].enabled);
        assert!(config.platforms["agentty"].enabled);

        // 不存在的渠道报错并列出有效渠道
        let mut config = default_config();
        let err = apply_toggle(&mut config, &["nonexistent".to_string()]).unwrap_err();
        let msg = format!("{}", err);
        assert!(msg.contains("Invalid platform: nonexistent"));
        assert!(msg.contains("Valid platforms:"));

        // 自定义渠道也可切换
        let mut config = default_config();
        config.platforms.insert(
            "my-custom".to_string(),
            Platform { name: None, enabled: false, path: ".my-custom".to_string(), local_path: None, skills: "skills".to_string(), agents: "AGENTS.md".to_string(), source: "AGENTS.md".to_string(), agents_compat: false, builtin: false },
        );
        apply_toggle(&mut config, &["my-custom".to_string()]).unwrap();
        assert!(config.platforms["my-custom"].enabled);
    }

    #[test]
    fn test_platform_config_fields() {
        let platform = Platform { name: None, enabled: true, path: ".claude".to_string(), local_path: None, skills: "skills".to_string(), agents: "CLAUDE.md".to_string(), source: "AGENTS.md".to_string(), agents_compat: false, builtin: false };
        assert!(platform.skills_dir().is_some());
        assert!(platform.agents_file().is_some());
        assert!(!platform.agents_compat);
    }

    #[test]
    fn test_platform_agents_compat() {
        let platform = Platform { name: None, enabled: true, path: ".opencode".to_string(), local_path: None, skills: "skills".to_string(), agents: "AGENTS.md".to_string(), source: "AGENTS.md".to_string(), agents_compat: true, builtin: false };
        assert!(platform.agents_compat);
    }

    #[test]
    fn test_platform_no_skills_no_agents() {
        let platform = Platform { name: None, enabled: true, path: ".gemini".to_string(), local_path: None, skills: String::new(), agents: String::new(), source: "AGENTS.md".to_string(), agents_compat: false, builtin: false };
        assert!(platform.skills_dir().is_none());
        assert!(platform.agents_file().is_none());
        assert!(!platform.agents_compat);
    }

    fn platform(name: &str) -> Platform {
        Platform { name: None, enabled: true, path: format!(".{}", name), local_path: None, skills: "skills".to_string(), agents: "AGENTS.md".to_string(), source: "AGENTS.md".to_string(), agents_compat: true, builtin: false }
    }

    fn config_with(extra: &[(&str, Platform)]) -> Config {
        let mut config = Config {
            platforms: default_platforms(),
            ..Config::default()
        };
        for (name, p) in extra {
            config.platforms.insert(name.to_string(), p.clone());
        }
        config
    }

    #[test]
    fn test_custom_platform_names() {
        let config = config_with(&[("my-custom", platform("my-custom"))]);
        let custom = super::custom_platform_names(&config);
        assert_eq!(custom, vec!["my-custom"]);
    }

    #[test]
    fn test_custom_platform_names_empty() {
        let config = config_with(&[]);
        assert!(super::custom_platform_names(&config).is_empty());
    }

    #[test]
    fn test_apply_reset_mode_replace() {
        let mut config = config_with(&[("my-custom", platform("my-custom"))]);
        super::apply_reset_mode(&mut config, ResetMode::Replace);
        let defaults = default_platforms();
        assert_eq!(config.platforms.len(), defaults.len());
        for name in defaults.keys() {
            assert!(config.platforms.contains_key(name));
        }
        assert!(!config.platforms.contains_key("my-custom"));
    }

    #[test]
    fn test_apply_reset_mode_merge() {
        let mut config = config_with(&[("my-custom", platform("my-custom"))]);
        super::apply_reset_mode(&mut config, ResetMode::Merge);
        assert!(config.platforms.contains_key("my-custom"));
        assert_eq!(config.platforms.len(), default_platforms().len() + 1);
        // 内置平台被默认值覆盖
        let claude = &config.platforms["claude"];
        assert_eq!(claude.agents, "CLAUDE.md");
        assert_eq!(claude.path, ".claude");
    }

    /// 旗标式 run_reset 测试（--replace/--merge 非交互路径）：临时目录 + XSKILL_CONFIG 隔离。
    /// 返回测试专用环境锁的 guard，调用方持有期间断言结果，防止并行测试篡改 XSKILL_CONFIG
    fn run_reset_nontty(mode: ResetMode) -> (std::sync::MutexGuard<'static, ()>, Config) {
        // 跨模块共享锁：串行化所有依赖 XSKILL_CONFIG 的测试
        let guard = crate::config::test_env_lock().lock().unwrap_or_else(|e| e.into_inner());
        let tmp = std::env::temp_dir().join(format!(
            "xskill-test-reset-{}-{}-{:?}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos(),
            mode
        ));
        std::fs::create_dir_all(&tmp).unwrap();
        let cfg_path = tmp.join("settings.json");

        // 预置含自定义平台的配置文件（save 依赖 XSKILL_CONFIG 解析路径，先设置）
        let mut config = default_config();
        config.platforms.insert(
            "my-custom".to_string(),
            platform("my-custom"),
        );
        // SAFETY: 测试专用环境变量，测试进程内串行执行
        unsafe { std::env::set_var("XSKILL_CONFIG", &cfg_path) };
        config.save().unwrap();
        super::run_reset(mode).unwrap();

        let loaded = Config::load();
        // 清理环境变量，避免影响其他测试
        unsafe { std::env::remove_var("XSKILL_CONFIG") };
        std::fs::remove_dir_all(&tmp).unwrap();
        (guard, loaded.unwrap())
    }

    #[test]
    fn test_run_reset_replace_drops_custom() {
        let (_guard, config) = run_reset_nontty(ResetMode::Replace);
        // 自定义平台被移除，只剩内置默认列表
        assert!(!config.platforms.contains_key("my-custom"));
        assert_eq!(config.platforms.len(), default_platforms().len());
    }

    #[test]
    fn test_run_reset_merge_keeps_custom() {
        let (_guard, config) = run_reset_nontty(ResetMode::Merge);
        // 自定义平台保留，内置平台恢复默认
        assert!(config.platforms.contains_key("my-custom"));
        assert_eq!(config.platforms.len(), default_platforms().len() + 1);
    }
}
