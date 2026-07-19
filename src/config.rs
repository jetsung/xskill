use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// 内置注册中心默认 URL
const DEFAULT_REGISTRY_URL: &str = "https://xskill.gcli.cn/skills.json";

/// JSON Schema URL（用于 settings.json 的 $schema 字段）
const CONFIG_SCHEMA_URL: &str = "https://xskill.gcli.cn/xskill.schema.json";

/// 默认缓存 TTL（秒），10 分钟
const DEFAULT_CACHE_TTL_SECS: u64 = 600;

/// 反序列化辅助：null 值视为默认值
fn deserialize_null_default<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + Deserialize<'de>,
{
    let opt = Option::<T>::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

/// 顶层配置结构
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Config {
    #[serde(rename = "$schema", default = "default_schema_url", skip_deserializing)]
    pub schema: String,
    #[serde(default)]
    pub platforms: HashMap<String, Platform>,
    #[serde(default)]
    pub sources: Vec<Source>,
    #[serde(default)]
    pub recommended: Vec<RecommendedSource>,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub cache: CacheConfig,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub registry: RegistryConfig,
}

/// 缓存配置
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CacheConfig {
    /// 是否启用缓存，默认 false
    #[serde(default)]
    pub enabled: bool,
    /// 缓存 TTL（秒），默认 600（10 分钟）
    #[serde(default = "default_cache_ttl")]
    pub ttl: u64,
}

/// 注册中心配置
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RegistryConfig {
    /// 是否启用注册中心，默认 false
    #[serde(default)]
    pub enabled: bool,
    /// 注册中心 URL，默认 "https://xskill.gcli.cn/skills.json"
    #[serde(default = "default_registry_url")]
    pub url: String,
}

/// 平台配置
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Platform {
    /// 工具配置目录
    pub path: String,
    /// skills 子目录名（相对于 path），为空则不安装
    #[serde(default)]
    pub skills: String,
    /// agents 配置文件名（相对于 path），为空则不安装
    #[serde(default)]
    pub agents: String,
    /// agents 源文件名（固定 .agents/ 目录下），默认为 AGENTS.md
    #[serde(default = "default_source")]
    pub source: String,
    /// 是否兼容 .agents/ 资源（可复用项目级 agents 配置）
    #[serde(default)]
    pub agents_compat: bool,
}

/// 源配置
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Source {
    /// 源名称（可选，留空或无效时使用 url 作为名称）
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub name: String,
    /// 源类型：git 或 api，默认为 git
    #[serde(default = "default_source_type", rename = "type")]
    pub source_type: String,
    /// 源地址（必填）
    pub url: String,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct RecommendedSource {
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub name: String,
    #[serde(default, deserialize_with = "deserialize_null_default")]
    pub url: String,
    pub skills: Vec<String>,
}

fn default_source() -> String {
    "AGENTS.md".to_string()
}

fn default_source_type() -> String {
    "git".to_string()
}

fn default_cache_ttl() -> u64 {
    DEFAULT_CACHE_TTL_SECS
}

fn default_registry_url() -> String {
    DEFAULT_REGISTRY_URL.to_string()
}

fn default_schema_url() -> String {
    CONFIG_SCHEMA_URL.to_string()
}

/// 构建默认平台列表（来自 platforms.json）
pub fn default_platforms() -> HashMap<String, Platform> {
    // (name, path, skills, agents, agents_compat)
    let entries: Vec<(&str, &str, &str, &str, bool)> = vec![
        ("atomcode", ".atomcode", "skills", "ATOMCODE.md", false),
        ("claude", ".claude", "skills", "CLAUDE.md", false),
        ("cline", ".cline", "skills", "CLAUDE.md", false),
        ("codebuddy", ".codebuddy", "skills", "CODEBUDDY.md", false),
        ("codex", ".codex", "skills", "AGENTS.md", true),
        ("factory", ".factory", "skills", "AGENTS.md", true),
        ("gemini", ".gemini", "skills", "GEMINI.md", false),
        ("jcode", ".jcode", "skills", "AGENTS.md", true),
        ("kilo", ".kilocode", "skills", "AGENTS.md", true),
        ("langcli", ".langcli", "skills", "LANGCLI.md", false),
        ("opencode", ".opencode", "skills", "AGENTS.md", true),
        ("openclaude", ".openclaude", "skills", "CLAUDE.md", false),
        ("openinterpreter", ".openinterpreter", "skills", "AGENTS.md", true),
        ("qoder", ".qoder", "skills", "AGENTS.md", true),
        ("qwen", ".qwen", "skills", "AGENTS.md", true),
        ("roo", ".roo", "skills", "AGENTS.md", true),
    ];

    let mut map = HashMap::new();
    for (name, path, skills, agents, agents_compat) in entries {
        map.insert(
            name.to_string(),
            Platform {
                path: path.to_string(),
                skills: skills.to_string(),
                agents: agents.to_string(),
                source: "AGENTS.md".to_string(),
                agents_compat,
            },
        );
    }
    map
}

/// 构建含默认值的完整配置（用于 init）
pub fn default_config() -> Config {
    Config {
        schema: default_schema_url(),
        platforms: default_platforms(),
        sources: vec![],
        recommended: vec![],
        cache: CacheConfig {
            enabled: false,
            ttl: DEFAULT_CACHE_TTL_SECS,
        },
        registry: RegistryConfig {
            enabled: false,
            url: DEFAULT_REGISTRY_URL.to_string(),
        },
    }
}

impl Default for CacheConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            ttl: DEFAULT_CACHE_TTL_SECS,
        }
    }
}

impl Default for RegistryConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            url: DEFAULT_REGISTRY_URL.to_string(),
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            schema: default_schema_url(),
            platforms: HashMap::new(),
            sources: vec![],
            recommended: vec![],
            cache: CacheConfig::default(),
            registry: RegistryConfig::default(),
        }
    }
}

impl Config {
    /// 加载配置：仅全局 settings.json
    /// 环境变量 XSKILL_CONFIG 若设置则替代默认路径
    pub fn load() -> Result<Self> {
        let path = if let Ok(p) = std::env::var("XSKILL_CONFIG") {
            PathBuf::from(p)
        } else {
            Self::settings_path()
        };

        let mut config = if path.exists() {
            Self::load_from_file(&path)
                .with_context(|| format!("Failed to load config file: {}", path.display()))?
        } else {
            Self::default()
        };

        // 若配置中无平台，使用默认平台
        if config.platforms.is_empty() {
            config.platforms = default_platforms();
        }

        Ok(config)
    }

    /// 全局配置路径：~/.xskill/settings.json
    pub fn settings_path() -> PathBuf {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("~"))
            .join(".xskill")
            .join("settings.json")
    }

    /// 从指定文件加载配置
    fn load_from_file(path: &Path) -> Result<Self> {
        let content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read config file: {}", path.display()))?;
        let config: Self = serde_json::from_str(&content)
            .with_context(|| format!("Invalid config file format: {}", path.display()))?;
        Ok(config)
    }

    /// 获取指定平台配置
    pub fn get_platform(&self, name: &str) -> Option<&Platform> {
        self.platforms.get(name)
    }

    /// 获取所有平台名称列表（按字母排序）
    pub fn platform_names(&self) -> Vec<&str> {
        let mut names: Vec<&str> = self.platforms.keys().map(|s| s.as_str()).collect();
        names.sort_unstable();
        names
    }

    /// 获取指定源配置（优先按 effective_name 匹配，其次按 url 匹配）
    pub fn get_source(&self, name: &str) -> Option<&Source> {
        self.sources
            .iter()
            .find(|s| s.effective_name() == name)
            .or_else(|| self.sources.iter().find(|s| s.url == name))
    }

    /// 检查缓存是否启用
    pub fn is_cache_enabled(&self) -> bool {
        self.cache.enabled
    }

    /// 检查注册中心是否启用
    pub fn is_registry_enabled(&self) -> bool {
        self.registry.enabled
    }

    /// 获取有效的注册中心 URL，支持裸域名 / 目录路径 / 完整文件路径
    /// - 空值或无效协议 → 回退内置默认
    /// - 末尾为文件名（含 `.`）→ 原样使用
    /// - 末尾为 `/` 或无路径 → 自动补全 `/skills.json`
    pub fn effective_registry_url(&self) -> String {
        let url = self.registry.url.trim();

        // 空值或无效协议，回退默认
        if url.is_empty()
            || !(url.starts_with("http://") || url.starts_with("https://"))
        {
            return DEFAULT_REGISTRY_URL.to_string();
        }

        // 提取 host 之后的路径部分
        let after_proto = url.splitn(2, "://").nth(1).unwrap_or("");

        match after_proto.find('/') {
            // 无路径（纯域名如 https://example.com）
            None => format!("{}/skills.json", url),
            Some(pos) => {
                let path = &after_proto[pos..];

                // 路径仅为 "/"（如 https://example.com/）
                if path.len() <= 1 {
                    let base = url.trim_end_matches('/');
                    return format!("{}/skills.json", base);
                }

                // 取路径最后一个非空段，判断是否为文件名
                let last_segment =
                    path.rsplit('/').find(|s| !s.is_empty()).unwrap_or("");
                if last_segment.contains('.') {
                    url.to_string()
                } else {
                    format!("{}/skills.json", url.trim_end_matches('/'))
                }
            }
        }
    }

    /// 保存配置到 settings.json
    pub fn save(&self) -> Result<()> {
        let path = Self::settings_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }
        let json =
            serde_json::to_string_pretty(self).context("Failed to serialize config")?;
        fs::write(&path, json)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;
        Ok(())
    }
}

/// 校验源名称格式：空值合法（表示清空），非空时仅允许字母、数字、下划线、连字符
pub fn validate_source_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Ok(());
    }
    let re = Regex::new(r"^[a-zA-Z0-9_-]+$").unwrap();
    if re.is_match(name) {
        Ok(())
    } else {
        anyhow::bail!(
            "Invalid source name '{}'. Only letters, digits, hyphens and underscores are allowed.",
            name
        )
    }
}

/// 校验源地址格式：必须以 http:// 或 https:// 开头
pub fn validate_source_url(url: &str) -> Result<()> {
    if url.starts_with("http://") || url.starts_with("https://") {
        Ok(())
    } else {
        anyhow::bail!(
            "Invalid URL '{}'. Must start with http:// or https://.",
            url
        )
    }
}

impl Source {
    /// 获取有效的源名称：name 非空且格式有效时使用 name，否则回退为 url
    pub fn effective_name(&self) -> String {
        if !self.name.is_empty() && validate_source_name(&self.name).is_ok() {
            self.name.clone()
        } else {
            self.url.clone()
        }
    }

    /// 获取有效的源类型（确保为 git 或 api）
    pub fn effective_type(&self) -> String {
        match self.source_type.as_str() {
            "git" | "api" => self.source_type.clone(),
            _ => "git".to_string(),
        }
    }
}

impl Platform {
    /// 解析平台路径：支持相对路径、绝对路径和 ~ 开头的路径
    fn resolve_path(&self) -> PathBuf {
        if self.path.starts_with('/') {
            // 绝对路径，直接使用
            PathBuf::from(&self.path)
        } else if self.path.starts_with('~') {
            // ~ 开头的路径，替换为 home_dir
            let stripped = self.path.trim_start_matches("~/").trim_start_matches('~');
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("~"))
                .join(stripped)
        } else {
            // 相对路径，直接使用
            PathBuf::from(&self.path)
        }
    }

    /// 获取 skills 安装目录
    pub fn skills_dir(&self) -> Option<PathBuf> {
        if self.skills.is_empty() {
            None
        } else {
            Some(self.resolve_path().join(&self.skills))
        }
    }

    /// 获取 agents 配置文件路径
    pub fn agents_file(&self) -> Option<PathBuf> {
        if self.agents.is_empty() {
            None
        } else {
            Some(self.resolve_path().join(&self.agents))
        }
    }

    /// 获取 agents 源文件路径（固定 .agents/<source>）
    pub fn source_file(&self) -> PathBuf {
        PathBuf::from(".agents").join(&self.source)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_platform(path: &str, skills: &str, agents: &str) -> Platform {
        Platform {
            path: path.to_string(),
            skills: skills.to_string(),
            agents: agents.to_string(),
            source: "AGENTS.md".to_string(),
            agents_compat: false,
        }
    }

    #[test]
    fn test_relative_path_resolve() {
        let p = make_platform(".claude", "skills", "CLAUDE.md");
        assert_eq!(p.resolve_path(), PathBuf::from(".claude"));
        assert_eq!(p.skills_dir(), Some(PathBuf::from(".claude/skills")));
        assert_eq!(p.agents_file(), Some(PathBuf::from(".claude/CLAUDE.md")));
        assert_eq!(p.source_file(), PathBuf::from(".agents/AGENTS.md"));
    }

    #[test]
    fn test_absolute_path_resolve() {
        let p = make_platform("/usr/local/config", "skills", "AGENTS.md");
        assert_eq!(p.resolve_path(), PathBuf::from("/usr/local/config"));
        assert_eq!(
            p.skills_dir(),
            Some(PathBuf::from("/usr/local/config/skills"))
        );
        assert_eq!(
            p.agents_file(),
            Some(PathBuf::from("/usr/local/config/AGENTS.md"))
        );
    }

    #[test]
    fn test_tilde_path_resolve() {
        let p = make_platform("~/config", "skills", "AGENTS.md");
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        assert_eq!(p.resolve_path(), home.join("config"));
        assert_eq!(p.skills_dir(), Some(home.join("config/skills")));
        assert_eq!(p.agents_file(), Some(home.join("config/AGENTS.md")));
    }

    #[test]
    fn test_tilde_slash_path_resolve() {
        let p = make_platform("~/.claude", "skills", "");
        let home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("~"));
        assert_eq!(p.resolve_path(), home.join(".claude"));
        assert_eq!(p.skills_dir(), Some(home.join(".claude/skills")));
        assert_eq!(p.agents_file(), None);
    }

    #[test]
    fn test_empty_skills_and_agents() {
        let p = make_platform(".gemini", "", "");
        assert_eq!(p.skills_dir(), None);
        assert_eq!(p.agents_file(), None);
    }

    #[test]
    fn test_source_effective_type() {
        let s = Source {
            name: "test".to_string(),
            source_type: "git".to_string(),
            url: "https://example.com".to_string(),
        };
        assert_eq!(s.effective_type(), "git");

        let s = Source {
            name: "test".to_string(),
            source_type: "api".to_string(),
            url: "https://example.com".to_string(),
        };
        assert_eq!(s.effective_type(), "api");

        let s = Source {
            name: "test".to_string(),
            source_type: "invalid".to_string(),
            url: "https://example.com".to_string(),
        };
        assert_eq!(s.effective_type(), "git");
    }

    #[test]
    fn test_source_effective_name() {
        // 有效 name → 使用 name
        let s = Source {
            name: "antfu".to_string(),
            source_type: "git".to_string(),
            url: "https://github.com/antfu/skills".to_string(),
        };
        assert_eq!(s.effective_name(), "antfu");

        // name 为空 → 回退为 url
        let s = Source {
            name: "".to_string(),
            source_type: "git".to_string(),
            url: "https://github.com/antfu/skills".to_string(),
        };
        assert_eq!(s.effective_name(), "https://github.com/antfu/skills");

        // name 含无效字符 → 回退为 url
        let s = Source {
            name: "invalid name!".to_string(),
            source_type: "git".to_string(),
            url: "https://example.com/repo".to_string(),
        };
        assert_eq!(s.effective_name(), "https://example.com/repo");
    }

    #[test]
    fn test_source_deserialize_empty_name() {
        let json = r#"{"url": "https://example.com/repo"}"#;
        let source: Source = serde_json::from_str(json).unwrap();
        assert_eq!(source.name, "");
        assert_eq!(source.source_type, "git");
        assert_eq!(source.url, "https://example.com/repo");
        assert_eq!(source.effective_name(), "https://example.com/repo");
    }

    #[test]
    fn test_source_deserialize_null_name() {
        let json = r#"{"name": null, "url": "https://example.com/repo"}"#;
        let source: Source = serde_json::from_str(json).unwrap();
        assert_eq!(source.name, "");
        assert_eq!(source.effective_name(), "https://example.com/repo");
    }

    #[test]
    fn test_validate_source_name() {
        assert!(validate_source_name("antfu").is_ok());
        assert!(validate_source_name("my-repo").is_ok());
        assert!(validate_source_name("repo_123").is_ok());
        assert!(validate_source_name("").is_ok());
        assert!(validate_source_name("my repo").is_err());
        assert!(validate_source_name("repo@name").is_err());
    }

    #[test]
    fn test_validate_source_url() {
        assert!(validate_source_url("https://github.com/example/skills.git").is_ok());
        assert!(validate_source_url("http://example.com/skills").is_ok());
        assert!(validate_source_url("ftp://example.com").is_err());
        assert!(validate_source_url("github.com/example").is_err());
        assert!(validate_source_url("").is_err());
    }

    #[test]
    fn test_save_and_load() {
        let dir = tempfile::tempdir().unwrap();
        let config = Config {
            schema: default_schema_url(),
            platforms: HashMap::new(),
            sources: vec![Source {
                name: "test".to_string(),
                source_type: "git".to_string(),
                url: "https://example.com".to_string(),
            }],
            recommended: vec![],
            cache: CacheConfig::default(),
            registry: RegistryConfig::default(),
        };

        // Save
        let json = serde_json::to_string_pretty(&config).unwrap();
        let path = dir.path().join("settings.json");
        fs::write(&path, &json).unwrap();

        // Load and verify
        let content = fs::read_to_string(&path).unwrap();
        let loaded: Config = serde_json::from_str(&content).unwrap();
        assert_eq!(loaded.sources.len(), 1);
        assert_eq!(loaded.sources[0].name, "test");
    }

    #[test]
    fn test_default_platforms() {
        let platforms = default_platforms();
        assert_eq!(platforms.len(), 16);
        assert!(platforms.contains_key("claude"));
        assert!(platforms.contains_key("cline"));
        assert!(platforms.contains_key("gemini"));
        assert!(platforms.contains_key("jcode"));

        let claude = &platforms["claude"];
        assert_eq!(claude.path, ".claude");
        assert_eq!(claude.skills, "skills");
        assert_eq!(claude.agents, "CLAUDE.md");
    }

    #[test]
    fn test_default_config() {
        let config = default_config();
        assert!(!config.cache.enabled);
        assert_eq!(config.cache.ttl, 600);
        assert!(!config.registry.enabled);
        assert_eq!(config.registry.url, DEFAULT_REGISTRY_URL);
        assert_eq!(config.platforms.len(), 16);
    }

    #[test]
    fn test_effective_registry_url() {
        let mut config = default_config();

        // 默认 URL
        assert_eq!(
            config.effective_registry_url(),
            "https://xskill.gcli.cn/skills.json"
        );

        // 完整文件路径 → 原样使用
        config.registry.url = "https://example.com/api/skills.json".to_string();
        assert_eq!(
            config.effective_registry_url(),
            "https://example.com/api/skills.json"
        );

        // 裸域名 → 补全 /skills.json
        config.registry.url = "https://xskill.gcli.cn".to_string();
        assert_eq!(
            config.effective_registry_url(),
            "https://xskill.gcli.cn/skills.json"
        );

        // 域名 + 尾部斜杠 → 补全 skills.json
        config.registry.url = "https://xskill.gcli.cn/".to_string();
        assert_eq!(
            config.effective_registry_url(),
            "https://xskill.gcli.cn/skills.json"
        );

        // 目录路径 → 补全 /skills.json
        config.registry.url = "https://example.com/api/v1".to_string();
        assert_eq!(
            config.effective_registry_url(),
            "https://example.com/api/v1/skills.json"
        );

        // 目录路径 + 尾部斜杠 → 补全 skills.json
        config.registry.url = "https://example.com/api/v1/".to_string();
        assert_eq!(
            config.effective_registry_url(),
            "https://example.com/api/v1/skills.json"
        );

        // 空 URL 回退默认
        config.registry.url = "".to_string();
        assert_eq!(
            config.effective_registry_url(),
            "https://xskill.gcli.cn/skills.json"
        );

        // 无效 URL 回退默认
        config.registry.url = "not-a-url".to_string();
        assert_eq!(
            config.effective_registry_url(),
            "https://xskill.gcli.cn/skills.json"
        );
    }

    #[test]
    fn test_no_null_values_in_serialized_config() {
        let config = default_config();
        let json = serde_json::to_string_pretty(&config).unwrap();
        assert!(!json.contains("null"), "Config JSON should not contain null values");
    }
}
