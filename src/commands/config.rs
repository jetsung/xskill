use crate::config::{default_config, Config};
use anyhow::{Context, Result};
use colored::Colorize;
use serde_json::Value;
use std::process::Command;

/// Open config file in editor
pub fn run_edit() -> Result<()> {
    let path = Config::settings_path();
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
    let path = Config::settings_path();

    if path.exists() {
        println!("{}: {}", "Config file already exists".yellow(), path.display());
        println!("{}", "Use --edit to modify, or delete the file first.".dimmed());
        return Ok(());
    }

    let config = default_config();
    config.save()?;

    println!("{}: {}", "Config file initialized".green(), path.display());
    Ok(())
}

/// Get config value by dot path
pub fn run_get(key: &str) -> Result<()> {
    let path = Config::settings_path();
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
    let mut json: Value =
        serde_json::to_value(&config).context("Failed to serialize config")?;

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
            if current.get(part).is_none()
                || current.get(part).map_or(false, |v| v.is_null())
            {
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
        assert_eq!(
            format_value(&Value::String("test".to_string())),
            "test"
        );
        assert_eq!(format_value(&Value::Number(42.into())), "42");
        assert_eq!(format_value(&Value::Null), "");
    }
}
