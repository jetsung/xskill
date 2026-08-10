use crate::config::{Config, default_config};
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::Value;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

/// Open config file in editor
pub fn run_edit() -> Result<()> {
    let path = Config::config_path();
    if !path.exists() {
        // Create empty config if not exists
        let config = Config::default();
        config.save()?;
    }

    let editor = std::env::var("EDITOR").unwrap_or_else(|_| "vi".to_string());
    let status = Command::new(&editor)
        .arg(&path)
        .status()
        .with_context(|| format!("Failed to launch editor: {}", editor))?;

    if !status.success() {
        anyhow::bail!("Editor exited with non-zero status");
    }

    println!("{}: {}", "Config file saved".green(), path.display());
    Ok(())
}

/// Initialize config file with default values
pub fn run_init() -> Result<()> {
    let path = Config::config_path();

    if path.exists() {
        println!(
            "{}: {}",
            "Config file already exists".yellow(),
            path.display()
        );
        println!(
            "{}",
            "Use --edit to modify, or delete the file first.".dimmed()
        );
        return Ok(());
    }

    let config = default_config();
    config.save()?;

    println!("{}: {}", "Config file initialized".green(), path.display());
    Ok(())
}

/// Show the full configuration as pretty JSON
pub fn run_show() -> Result<()> {
    let config = Config::load()?;
    let json =
        serde_json::to_string_pretty(&config).with_context(|| "Failed to serialize config")?;
    // 直接写 stdout，避免 colored 对 JSON 着色导致不可解析
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    handle
        .write_all(json.as_bytes())
        .with_context(|| "Failed to write config")?;
    handle.write_all(b"\n").ok();
    Ok(())
}

/// Validate the configuration file against the JSON Schema.
pub fn run_validate() -> Result<()> {
    let config_path = Config::config_path();
    if !config_path.exists() {
        anyhow::bail!("Config file not found: {}", config_path.display());
    }

    // 1. 语法 + 结构校验：能被 JSON 解析且能反序列化为强类型 Config
    let content = std::fs::read_to_string(&config_path)
        .with_context(|| format!("Failed to read config: {}", config_path.display()))?;
    let instance: Value =
        serde_json::from_str(&content).with_context(|| "Config is not valid JSON".to_string())?;
    // 反序列化为强类型模型，捕获类型/必填字段错误
    if let Err(e) = serde_json::from_str::<Config>(&content) {
        anyhow::bail!("Config structure invalid: {}", e);
    }
    // 提前加载配置以应用代理环境变量（HTTPS_PROXY 等），
    // 使下方可能的云端 Schema 拉取能走代理。
    let _ = Config::load();

    // 2. JSON Schema 校验（本地优先，缺失时回退云端）
    let (schema, schema_source) = load_schema()?;

    let compiled = jsonschema::JSONSchema::compile(&schema)
        .map_err(|e| anyhow::anyhow!("Failed to compile schema: {}", e))?;

    let result = compiled.validate(&instance);
    match result {
        Ok(()) => {
            println!(
                "{} {} (schema: {})",
                "Valid".green(),
                config_path.display(),
                schema_source
            );
            Ok(())
        }
        Err(errors) => {
            let count = errors.count();
            eprintln!(
                "{} {} — {} validation error(s):",
                "Invalid".red(),
                config_path.display(),
                count
            );
            // 重新遍历以打印每条错误（上一次 .count() 已消耗迭代器）
            for err in compiled.validate(&instance).expect_err("schema errors") {
                eprintln!("  - {}: {}", err.instance_path, err);
            }
            std::process::exit(1);
        }
    }
}

/// 加载 JSON Schema：先尝试本地文件，未找到则从云端拉取。
/// 返回 (schema, 来源描述) —— 来源为本地路径或云端 URL。
fn load_schema() -> Result<(Value, String)> {
    if let Some(path) = find_local_schema() {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read schema: {}", path.display()))?;
        let schema = serde_json::from_str(&content)
            .with_context(|| "Schema is not valid JSON".to_string())?;
        return Ok((schema, path.display().to_string()));
    }

    // 本地无 Schema，回退云端拉取
    let url = crate::config::CONFIG_SCHEMA_URL;
    eprintln!(
        "{} {}",
        "Schema not found locally, fetching from".yellow(),
        url
    );
    let content = crate::skill_resolver::fetch_json(url)
        .map_err(|e| {
            anyhow::anyhow!(
                "{} Additionally, failed to fetch schema from cloud ({}). \
                 Set XSKILL_SCHEMA to a local schema path, or place it at \
                 <config_dir>/schemas/xskill.schema.json or <exe_dir>/../share/xskill/xskill.schema.json.",
                e, url
            )
        })?;
    let schema = serde_json::from_str(&content)
        .with_context(|| "Fetched schema is not valid JSON".to_string())?;
    Ok((schema, url.to_string()))
}

/// 在本地解析 JSON Schema 文件路径，按顺序查找：
/// 1. `$XSKILL_SCHEMA` 环境变量
/// 2. `<config_dir>/schemas/xskill.schema.json`
/// 3. 可执行文件所在目录向上查找 `schemas/xskill.schema.json`
/// 4. `<exe_dir>/../share/xskill/xskill.schema.json`
fn find_local_schema() -> Option<PathBuf> {
    if let Ok(p) = std::env::var("XSKILL_SCHEMA") {
        let candidate = PathBuf::from(p);
        if candidate.exists() {
            return Some(candidate);
        }
    }

    // 配置目录下的 schemas/
    let config_dir = Config::config_path().parent().map(|p| p.to_path_buf());
    if let Some(dir) = config_dir {
        let candidate = dir.join("schemas").join("xskill.schema.json");
        if candidate.exists() {
            return Some(candidate);
        }
    }

    // 可执行文件目录向上查找
    if let Ok(exe) = std::env::current_exe() {
        let mut dir = exe.parent();
        while let Some(d) = dir {
            let candidate = d.join("schemas").join("xskill.schema.json");
            if candidate.exists() {
                return Some(candidate);
            }
            // share 安装路径
            let share = d
                .join("..")
                .join("share")
                .join("xskill")
                .join("xskill.schema.json");
            if share.exists() {
                return Some(share);
            }
            dir = d.parent();
        }
    }

    None
}

/// Get config value by dot path
pub fn run_get(key: &str) -> Result<()> {
    let path = Config::config_path();
    if !path.exists() {
        anyhow::bail!("Config file not found: {}", path.display());
    }

    let content = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read config: {}", path.display()))?;
    let value: Value = serde_json::from_str(&content).context("Failed to parse config JSON")?;

    match get_nested_value(&value, key) {
        Some(v) => {
            println!("{}", format_value(v).cyan());
            Ok(())
        }
        None => {
            anyhow::bail!("Key '{}' not found in config", key);
        }
    }
}

/// Set config value by dot path
pub fn run_set(kv: &str) -> Result<()> {
    let (key, value_str) = kv.split_once('=').ok_or_else(|| {
        anyhow::anyhow!("Invalid format. Use: key=value (e.g. cache.enabled=true)")
    })?;

    let mut config = Config::load()?;

    // Convert config to JSON Value for manipulation
    let mut json: Value = serde_json::to_value(&config).context("Failed to serialize config")?;

    // Parse value with type inference
    let new_value = infer_value(value_str);

    // Set nested value
    set_nested_value(&mut json, key, new_value)?;

    // Deserialize back to Config and save
    config = serde_json::from_value(json).context("Failed to deserialize updated config")?;
    config.save()?;

    println!("{} {} = {}", "Set".green(), key, value_str);
    Ok(())
}

/// Get value from nested JSON by dot path
fn get_nested_value<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    let parts: Vec<&str> = path.split('.').collect();
    let mut current = value;

    for part in parts {
        match current.get(part) {
            Some(v) => current = v,
            None => return None,
        }
    }

    Some(current)
}

/// Set value in nested JSON by dot path
fn set_nested_value(value: &mut Value, path: &str, new_value: Value) -> Result<()> {
    let parts: Vec<&str> = path.split('.').collect();
    if parts.is_empty() {
        anyhow::bail!("Empty key path");
    }

    let mut current = value;
    for (i, part) in parts.iter().enumerate() {
        if i == parts.len() - 1 {
            // Last part: set the value
            if let Some(obj) = current.as_object_mut() {
                obj.insert(part.to_string(), new_value.clone());
            } else {
                anyhow::bail!(
                    "Cannot set value at path '{}': parent is not an object",
                    path
                );
            }
        } else {
            // Intermediate part: navigate into object
            if current.get(part).is_none() || current.get(part).map_or(false, |v| v.is_null()) {
                // Create missing intermediate object (or replace null)
                if let Some(obj) = current.as_object_mut() {
                    obj.insert(part.to_string(), Value::Object(serde_json::Map::new()));
                }
            }
            current = current
                .get_mut(part)
                .ok_or_else(|| anyhow::anyhow!("Cannot navigate path '{}'", path))?;
        }
    }

    Ok(())
}

/// Infer JSON value type from string
fn infer_value(s: &str) -> Value {
    match s {
        "true" => Value::Bool(true),
        "false" => Value::Bool(false),
        _ => {
            // Try parse as number
            if let Ok(n) = s.parse::<i64>() {
                Value::Number(n.into())
            } else if let Ok(f) = s.parse::<f64>() {
                if let Some(n) = serde_json::Number::from_f64(f) {
                    Value::Number(n)
                } else {
                    Value::String(s.to_string())
                }
            } else {
                Value::String(s.to_string())
            }
        }
    }
}

/// Format JSON value for display
fn format_value(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Null => String::new(),
        _ => serde_json::to_string_pretty(v).unwrap_or_else(|_| v.to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_serialize_config_contains_proxy() {
        // run_show 序列化的是 Config::load() 的结果，proxy 应为 Some/None 之一
        let config = crate::config::Config::load().unwrap();
        let json = serde_json::to_string_pretty(&config).unwrap();
        // 输出必须是合法 JSON
        let parsed: serde_json::Value = serde_json::from_str(&json).unwrap();
        // proxy 字段要么不存在（None），要么为字符串
        match parsed.get("proxy") {
            Some(serde_json::Value::String(_)) | None => {}
            other => panic!("unexpected proxy value: {:?}", other),
        }
    }

    #[test]
    fn test_get_nested_value() {
        let config = json!({
            "cache": {
                "enabled": true
            },
            "platforms": {
                "claude": {
                    "path": ".claude"
                }
            }
        });

        // Top level
        assert_eq!(
            get_nested_value(&config, "cache"),
            Some(&json!({"enabled": true}))
        );

        // Nested
        assert_eq!(
            get_nested_value(&config, "cache.enabled"),
            Some(&Value::Bool(true))
        );

        // Not found
        assert_eq!(get_nested_value(&config, "nonexistent"), None);
        assert_eq!(get_nested_value(&config, "cache.nonexistent"), None);
    }

    #[test]
    fn test_set_nested_value() {
        let mut config = json!({
            "cache": {
                "enabled": false
            }
        });

        set_nested_value(&mut config, "cache.enabled", Value::Bool(true)).unwrap();
        assert_eq!(config["cache"]["enabled"], Value::Bool(true));
    }

    #[test]
    fn test_set_nested_value_create_intermediate() {
        let mut config = json!({});

        set_nested_value(&mut config, "cache.enabled", Value::Bool(true)).unwrap();
        assert_eq!(config["cache"]["enabled"], Value::Bool(true));
    }

    #[test]
    fn test_infer_value() {
        assert_eq!(infer_value("true"), Value::Bool(true));
        assert_eq!(infer_value("false"), Value::Bool(false));
        assert_eq!(infer_value("42"), Value::Number(42.into()));
        assert_eq!(infer_value("hello"), Value::String("hello".to_string()));
    }

    #[test]
    fn test_format_value() {
        assert_eq!(format_value(&Value::Bool(true)), "true");
        assert_eq!(format_value(&Value::String("test".to_string())), "test");
        assert_eq!(format_value(&Value::Number(42.into())), "42");
        assert_eq!(format_value(&Value::Null), "");
    }
}
