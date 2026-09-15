use anyhow::{Context, Result};
use regex::Regex;
use serde::{Deserialize, Deserializer, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

/// 内置注册中心默认 URL
const DEFAULT_REGISTRY_URL: &str = "https://xskill.gcli.cn/skills.json";

/// JSON Schema URL（用于 settings.json 的 $schema 字段，及云端校验回退）
pub const CONFIG_SCHEMA_URL: &str = "https://xskill.gcli.cn/xskill.schema.json";

/// 默认缓存 TTL（秒），24 小时
const DEFAULT_CACHE_TTL_SECS: u64 = 86400;

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
    /// 代理地址（如 http://127.0.0.1:7890），用于访问 GitHub 等网络受限的场景。
    /// 设置后导出 HTTP_PROXY/HTTPS_PROXY/ALL_PROXY 等环境变量，git/curl/wget 自动生效。
    #[serde(
        default,
        deserialize_with = "deserialize_null_default",
        skip_serializing_if = "Option::is_none"
    )]
    pub proxy: Option<String>,
}

/// 缓存配置
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct CacheConfig {
    /// 是否启用缓存，默认 false
    #[serde(default)]
    pub enabled: bool,
    /// 缓存 TTL（秒），默认 86400（24 小时）
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
    /// 渠道显示名称（缺失时回退到配置 key）
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// 是否启用：false（默认）时不出现在交互选择与批量操作，显式指定仍可操作
    #[serde(default)]
    pub enabled: bool,
    /// 工具配置目录（全局模式：~/<path>/skills）
    ///
    /// 精简保存的内置渠道条目可省略（加载时由内置保护逻辑恢复默认值）
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub path: String,
    /// 项目级配置目录（可选，留空时回退到 path）
    ///
    /// 少数平台的全局与项目级路径不一致（如 omp 全局 ~/.omp/agent/，项目级 .omp/），
    /// 此字段用于覆盖项目级路径；为空时回退到 `path`。
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub local_path: Option<String>,
    /// skills 子目录名（相对于 path），为空则不安装
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub skills: String,
    /// agents 配置文件名（相对于 path），为空则不安装
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub agents: String,
    /// agents 源文件名（固定 .agents/ 目录下），默认为 AGENTS.md
    #[serde(default = "default_source", skip_serializing_if = "String::is_empty")]
    pub source: String,
    /// 是否兼容 .agents/ 资源（可复用项目级 agents 配置）
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub agents_compat: bool,
    /// 是否为内置渠道（由 default_platforms 生成，用户配置中不可修改路径等字段）
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub builtin: bool,
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

/// 构建默认平台列表（来自 docs/PLATFORMS.md）
/// 元组：(key, display_name, enabled, path, local_path, skills, agents, agents_compat)
/// local_path 仅在全局与项目级路径不同时设置（如 omp 全局 .omp/agent，项目级 .omp）
pub fn default_platforms() -> HashMap<String, Platform> {
    let entries: Vec<(&str, &str, bool, &str, Option<&str>, &str, &str, bool)> = vec![
        // 常用渠道（默认启用）
        ("antigravity", "Antigravity", true, ".gemini", None, "skills", "GEMINI.md", true),
        ("claude", "Claude Code", true, ".claude", None, "skills", "CLAUDE.md", false),
        ("codebuddy", "CodeBuddy", true, ".codebuddy", None, "skills", "CODEBUDDY.md", false),
        ("codex", "Codex", true, ".codex", None, "skills", "AGENTS.md", true),
        ("commandcode", "Command Code", false, ".commandcode", None, "skills", "AGENTS.md", true),
        ("dsh", "DeepSeek Harness", true, ".dsh", None, "skills", "AGENTS.md", true),
        // omp 全局 ~/.omp/agent/，项目级 .omp/
        ("omp", "Oh My Pi", true, ".omp/agent", Some(".omp"), "skills", "AGENTS.md", true),
        ("opencode", "OpenCode", true, ".opencode", None, "skills", "AGENTS.md", true),
        // pi 全局 ~/.pi/agent/，项目级 .pi/
        ("pi", "Pi", true, ".pi/agent", Some(".pi"), "skills", "AGENTS.md", true),
        ("qoder", "Qoder", true, ".qoder", None, "skills", "AGENTS.md", true),
        ("qoder-cn", "Qoder CN", true, ".qoder-cn", None, "skills", "AGENTS.md", true),
        ("workbuddy", "WorkBuddy", true, ".workbuddy", None, "skills", "CODEBUDDY.md", false),
        ("zcode", "ZCode", true, ".zcode", None, "skills", "AGENTS.md", true),
        // 非常用渠道（默认禁用，可在 settings.json 中启用）
        ("atomcode", "AtomCode", false, ".atomcode", None, "skills", "ATOMCODE.md", true),
        ("cline", "Cline", false, ".cline", None, "skills", "CLAUDE.md", true),
        ("factory", "Factory", false, ".factory", None, "skills", "AGENTS.md", true),
        ("jcode", "JCode", false, ".jcode", None, "skills", "AGENTS.md", true),
        ("kilo", "Kilo Code", false, ".kilocode", None, "skills", "AGENTS.md", true),
        ("kiro", "Kiro", false, ".kiro", None, "skills", "AGENTS.md", false),
        ("langcli", "LangCLI", false, ".langcli", None, "skills", "LANGCLI.md", false),
        ("openclaude", "OpenClaude", false, ".openclaude", None, "skills", "CLAUDE.md", false),
        ("openinterpreter", "Open Interpreter", false, ".openinterpreter", None, "skills", "AGENTS.md", true),
        ("grok", "Grok Build CLI", false, ".grok", None, "skills", "AGENTS.md", true),
        ("qwen", "Qwen", false, ".qwen", None, "skills", "AGENTS.md", true),
        // Zoo Code 接手已停服的 Roo Code，配置目录沿用 .roo
        ("zoo", "Zoo Code", false, ".roo", None, "skills", "AGENTS.md", true),
        // MiMo Code 全局 ~/.config/mimocode/，项目级 .mimocode/
        ("mimocode", "MiMo Code", false, ".config/mimocode", Some(".mimocode"), "skills", "AGENTS.md", true),
        // agentty 兼容 .agents/ 规范目录，全局与项目级路径均为 .agentty
        ("agentty", "Agentty", false, ".agentty", None, "skills", "AGENTS.md", true),
        // hermes（Hermes Agent）兼容 .agents/ 规范目录
        ("hermes", "Hermes Agent", false, ".hermes", None, "skills", "AGENTS.md", true),
    ];

    let mut map = HashMap::new();
    for (key, name, enabled, path, local_path, skills, agents, agents_compat) in entries {
        map.insert(
            key.to_string(),
            Platform {
                name: Some(name.to_string()),
                enabled,
                path: path.to_string(),
                local_path: local_path.map(|s| s.to_string()),
                skills: skills.to_string(),
                agents: agents.to_string(),
                source: "AGENTS.md".to_string(),
                agents_compat,
                builtin: true,
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
        // init 时补全 proxy 键，值为空字符串（不生效，仅占位以便用户填写）
        proxy: Some(String::new()),
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
            proxy: None,
        }
    }
}

impl Config {
    /// 获取配置文件路径：XSKILL_CONFIG 环境变量 > 默认 ~/.xskill/settings.json
    pub fn config_path() -> PathBuf {
        if let Ok(p) = std::env::var("XSKILL_CONFIG") {
            PathBuf::from(p)
        } else {
            Self::settings_path()
        }
    }

    /// 加载配置：仅全局 settings.json
    /// 环境变量 XSKILL_CONFIG 若设置则替代默认路径
    pub fn load() -> Result<Self> {
        let path = Self::config_path();

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

        // 内置渠道保护：仅允许用户覆盖 name/enabled，其余字段恢复内置默认值
        config.enforce_builtin_platforms();

        // 导出代理环境变量，使 git/curl/wget 等子进程自动走代理
        config.apply_proxy_env();

        Ok(config)
    }

    /// 若配置了 `proxy`，导出 HTTP_PROXY/HTTPS_PROXY/ALL_PROXY 等环境变量
    /// （含小写形式）。git clone 与 curl/wget 拉取均会自动读取这些变量。
    /// 仅当配置中显式设置了代理时才覆盖已有环境变量。
    pub fn apply_proxy_env(&self) {
        let Some(proxy) = self.proxy.as_ref() else {
            return;
        };
        if proxy.is_empty() {
            return;
        }
        for var in [
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "http_proxy",
            "https_proxy",
            "all_proxy",
        ] {
            // `set_var` is unsafe in the current edition; the value is always a
            // valid String from config, so this is sound.
            unsafe {
                std::env::set_var(var, proxy);
            }
        }
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

    /// 规范化平台配置（保存前调用）：
    /// - key 在内置列表中的渠道仅保留 name/enabled/builtin 三个字段（路径等由内置保护逻辑在加载时恢复）
    /// - 自定义渠道（key 不在内置列表）builtin 强制为 false（无论用户如何设置）
    pub fn normalize_platforms_for_save(&mut self) {
        let builtin_keys: std::collections::HashSet<String> =
            default_platforms().into_keys().collect();
        for (key, platform) in self.platforms.iter_mut() {
            if builtin_keys.contains(key) {
                let name = platform.name.clone();
                let enabled = platform.enabled;
                *platform = Platform {
                    name,
                    enabled,
                    path: String::new(),
                    local_path: None,
                    skills: String::new(),
                    agents: String::new(),
                    source: String::new(),
                    agents_compat: false,
                    builtin: true,
                };
            } else {
                platform.builtin = false;
            }
        }
    }

    /// 内置渠道保护：与内置渠道同 key 的条目仅保留用户设置的 name/enabled，
    /// 其余字段（path、local_path、skills、agents、source、agents_compat）恢复内置默认值
    pub fn enforce_builtin_platforms(&mut self) {
        let defaults = default_platforms();
        for (key, default_platform) in &defaults {
            if let Some(user_platform) = self.platforms.get_mut(key) {
                let name = user_platform.name.clone();
                let enabled = user_platform.enabled;
                let mut restored = default_platform.clone();
                restored.name = name;
                restored.enabled = enabled;
                self.platforms.insert(key.clone(), restored);
            }
        }
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
        if url.is_empty() || !(url.starts_with("http://") || url.starts_with("https://")) {
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
                let last_segment = path.rsplit('/').find(|s| !s.is_empty()).unwrap_or("");
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
        let path = Self::config_path();
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create directory: {}", parent.display()))?;
        }
        let json = serde_json::to_string_pretty(self).context("Failed to serialize config")?;
        fs::write(&path, json)
            .with_context(|| format!("Failed to write config file: {}", path.display()))?;
        Ok(())
    }
}

/// 校验源名称格式：空值合法（表示清空），非空时仅允许字母、数字、下划线、连字符、斜杠（支持 user/repo 格式）
pub fn validate_source_name(name: &str) -> Result<()> {
    if name.is_empty() {
        return Ok(());
    }
    let re = Regex::new(r"^[a-zA-Z0-9_/-]+$").unwrap();
    if re.is_match(name) {
        Ok(())
    } else {
        anyhow::bail!(
            "Invalid source name '{}'. Only letters, digits, hyphens, underscores and slashes are allowed.",
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
    /// 渠道显示名称：优先 name，缺失时回退到配置 key
    pub fn display_name(&self, key: &str) -> String {
        self.name.clone().unwrap_or_else(|| key.to_string())
    }

    /// 解析路径字符串：支持相对路径、绝对路径和 ~ 开头的路径
    fn resolve_str(path: &str) -> PathBuf {
        if path.starts_with('/') {
            // 绝对路径，直接使用
            PathBuf::from(path)
        } else if path.starts_with('~') {
            // ~ 开头的路径，替换为 home_dir
            let stripped = path.trim_start_matches("~/").trim_start_matches('~');
            dirs::home_dir()
                .unwrap_or_else(|| PathBuf::from("~"))
                .join(stripped)
        } else {
            // 相对路径，直接使用
            PathBuf::from(path)
        }
    }

    /// 解析平台路径：支持相对路径、绝对路径和 ~ 开头的路径
    fn resolve_path(&self) -> PathBuf {
        Self::resolve_str(&self.path)
    }

    /// 根据模式返回有效的路径字符串
    ///
    /// - 全局模式（`global=true`）：使用 `path`（如 `.omp/agent`）
    /// - 项目级模式（`global=false`）：优先使用 `local_path`，未设置时回退到 `path`
    pub fn effective_path(&self, global: bool) -> &str {
        if !global {
            if let Some(local) = &self.local_path {
                return local;
            }
        }
        &self.path
    }

    /// 获取 skills 安装目录（相对于 resolve_path）
    pub fn skills_dir(&self) -> Option<PathBuf> {
        if self.skills.is_empty() {
            None
        } else {
            Some(self.resolve_path().join(&self.skills))
        }
    }

    /// 获取 skills 安装目录（基于指定 base_dir，用于 global/project 切换）
    pub fn skills_dir_with_base(&self, base_dir: &Path, global: bool) -> Option<PathBuf> {
        if self.skills.is_empty() {
            None
        } else {
            Some(base_dir.join(self.effective_path(global)).join(&self.skills))
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
}

#[cfg(test)]
mod tests {
    use super::*;

    fn make_platform(path: &str, skills: &str, agents: &str) -> Platform {
        Platform {
            name: None,
            enabled: true,
            path: path.to_string(),
            local_path: None,
            skills: skills.to_string(),
            agents: agents.to_string(),
            source: "AGENTS.md".to_string(),
            agents_compat: false,
            builtin: false,
        }
    }

    #[test]
    fn test_relative_path_resolve() {
        let p = make_platform(".claude", "skills", "CLAUDE.md");
        assert_eq!(p.resolve_path(), PathBuf::from(".claude"));
        assert_eq!(p.skills_dir(), Some(PathBuf::from(".claude/skills")));
        assert_eq!(p.agents_file(), Some(PathBuf::from(".claude/CLAUDE.md")));
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
            proxy: None,
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
        assert_eq!(platforms.len(), 28);
        assert!(platforms.contains_key("antigravity"));
        assert!(platforms.contains_key("claude"));
        assert!(platforms.contains_key("cline"));
        // gemini 已合并入 antigravity（同一平台）
        assert!(!platforms.contains_key("gemini"));
        assert!(platforms.contains_key("jcode"));
        assert!(platforms.contains_key("kiro"));
        assert!(platforms.contains_key("omp"));
        assert!(platforms.contains_key("pi"));
        assert!(platforms.contains_key("zcode"));
        // roo 已停服，由 zoo（Zoo Code）接手，配置目录沿用 .roo
        assert!(!platforms.contains_key("roo"));
        assert!(platforms.contains_key("zoo"));

        // 默认启用 12 个常用渠道
        let enabled: Vec<_> = platforms
            .iter()
            .filter(|(_, p)| p.enabled)
            .map(|(k, _)| k.as_str())
            .collect();
        assert_eq!(enabled.len(), 12);
        for key in ["claude", "codex", "antigravity", "zcode", "opencode", "codebuddy", "qoder", "qoder-cn", "pi", "omp", "dsh", "workbuddy"] {
            assert!(
                platforms[key].enabled,
                "expected {} to be enabled by default",
                key
            );
        }

        let antigravity = &platforms["antigravity"];
        assert_eq!(antigravity.name.as_deref(), Some("Antigravity"));
        assert_eq!(antigravity.path, ".gemini");
        assert_eq!(antigravity.skills, "skills");
        assert_eq!(antigravity.agents, "GEMINI.md");
        assert!(antigravity.agents_compat);

        let claude = &platforms["claude"];
        assert_eq!(claude.name.as_deref(), Some("Claude Code"));
        assert_eq!(claude.path, ".claude");
        assert_eq!(claude.skills, "skills");
        assert_eq!(claude.agents, "CLAUDE.md");

        // kiro 渠道默认禁用且不支持 agents 兼容
        let kiro = &platforms["kiro"];
        assert_eq!(kiro.name.as_deref(), Some("Kiro"));
        assert!(!kiro.enabled);
        assert_eq!(kiro.path, ".kiro");
        assert_eq!(kiro.skills, "skills");
        assert_eq!(kiro.agents, "AGENTS.md");
        assert!(!kiro.agents_compat);

        // omp 渠道默认路径为 .omp/agent（全局），项目级为 .omp
        let omp = &platforms["omp"];
        assert_eq!(omp.name.as_deref(), Some("Oh My Pi"));
        assert_eq!(omp.path, ".omp/agent");
        assert_eq!(omp.local_path.as_deref(), Some(".omp"));
        assert_eq!(omp.effective_path(true), ".omp/agent");
        assert_eq!(omp.effective_path(false), ".omp");
        assert_eq!(omp.skills, "skills");
        assert_eq!(omp.agents, "AGENTS.md");
        assert!(omp.agents_compat);

        // pi 渠道默认路径为 .pi/agent（全局），项目级为 .pi
        let pi = &platforms["pi"];
        assert_eq!(pi.name.as_deref(), Some("Pi"));
        assert_eq!(pi.path, ".pi/agent");
        assert_eq!(pi.local_path.as_deref(), Some(".pi"));
        assert_eq!(pi.effective_path(true), ".pi/agent");
        assert_eq!(pi.effective_path(false), ".pi");
        assert_eq!(pi.skills, "skills");
        assert_eq!(pi.agents, "AGENTS.md");

        // zoo 渠道沿用 roo 的配置目录
        let zoo = &platforms["zoo"];
        assert_eq!(zoo.name.as_deref(), Some("Zoo Code"));
        assert_eq!(zoo.path, ".roo");
        assert_eq!(zoo.skills, "skills");
        assert!(zoo.agents_compat);

        // dsh（DeepSeek Harness）渠道：AGENTS.md 兼容，默认启用
        let dsh = &platforms["dsh"];
        assert_eq!(dsh.name.as_deref(), Some("DeepSeek Harness"));
        assert_eq!(dsh.path, ".dsh");
        assert_eq!(dsh.skills, "skills");
        assert_eq!(dsh.agents, "AGENTS.md");
        assert_eq!(dsh.source, "AGENTS.md");
        assert!(dsh.agents_compat);
        assert!(dsh.enabled);

        // workbuddy（WorkBuddy）渠道：沿用 codebuddy 约定，agents 文件为 CODEBUDDY.md，默认启用
        let workbuddy = &platforms["workbuddy"];
        assert_eq!(workbuddy.name.as_deref(), Some("WorkBuddy"));
        assert_eq!(workbuddy.path, ".workbuddy");
        assert_eq!(workbuddy.skills, "skills");
        assert_eq!(workbuddy.agents, "CODEBUDDY.md");
        assert_eq!(workbuddy.source, "AGENTS.md");
        assert!(!workbuddy.agents_compat);
        assert!(workbuddy.enabled);

        // qoder-cn（Qoder 中国版）渠道：与 qoder 国际版配置一致，仅路径为 .qoder-cn，默认启用
        let qoder_cn = &platforms["qoder-cn"];
        assert_eq!(qoder_cn.name.as_deref(), Some("Qoder CN"));
        assert_eq!(qoder_cn.path, ".qoder-cn");
        assert_eq!(qoder_cn.skills, "skills");
        assert_eq!(qoder_cn.agents, "AGENTS.md");
        assert_eq!(qoder_cn.source, "AGENTS.md");
        assert!(qoder_cn.agents_compat);
        assert!(qoder_cn.enabled);

        // commandcode 渠道：AGENTS.md 兼容，默认禁用
        let commandcode = &platforms["commandcode"];
        assert_eq!(commandcode.name.as_deref(), Some("Command Code"));
        assert_eq!(commandcode.path, ".commandcode");
        assert_eq!(commandcode.skills, "skills");
        assert_eq!(commandcode.agents, "AGENTS.md");
        assert!(commandcode.agents_compat);
        assert!(!commandcode.enabled);

        // 兼容性标记与 docs/PLATFORMS.md 一致：atomcode ✓、cline ✅、claude ❌
        assert!(platforms["atomcode"].agents_compat);
        assert!(platforms["cline"].agents_compat);
        assert!(!platforms["claude"].agents_compat);
        assert!(!platforms["codebuddy"].agents_compat);

        // mimocode 渠道：全局 .config/mimocode，项目级 .mimocode
        let mimocode = &platforms["mimocode"];
        assert_eq!(mimocode.name.as_deref(), Some("MiMo Code"));
        assert_eq!(mimocode.path, ".config/mimocode");
        assert_eq!(mimocode.local_path.as_deref(), Some(".mimocode"));
        assert_eq!(mimocode.effective_path(true), ".config/mimocode");
        assert_eq!(mimocode.effective_path(false), ".mimocode");
        assert_eq!(mimocode.skills, "skills");
        assert_eq!(mimocode.agents, "AGENTS.md");
        assert!(mimocode.agents_compat);
        assert!(!mimocode.enabled);

        // agentty 渠道：全局与项目级路径相同，均为 .agentty
        let agentty = &platforms["agentty"];
        assert_eq!(agentty.name.as_deref(), Some("Agentty"));
        assert_eq!(agentty.path, ".agentty");
        assert_eq!(agentty.local_path, None);
        assert_eq!(agentty.effective_path(true), ".agentty");
        assert_eq!(agentty.effective_path(false), ".agentty");
        assert_eq!(agentty.skills, "skills");
        assert_eq!(agentty.agents, "AGENTS.md");
        assert!(agentty.agents_compat);
        assert!(!agentty.enabled);

        // hermes 渠道（Hermes Agent）：全局与项目级路径相同，均为 .hermes
        let hermes = &platforms["hermes"];
        assert_eq!(hermes.name.as_deref(), Some("Hermes Agent"));
        assert_eq!(hermes.path, ".hermes");
        assert_eq!(hermes.local_path, None);
        assert_eq!(hermes.effective_path(true), ".hermes");
        assert_eq!(hermes.effective_path(false), ".hermes");
        assert_eq!(hermes.skills, "skills");
        assert_eq!(hermes.agents, "AGENTS.md");
        assert!(hermes.agents_compat);
        assert!(!hermes.enabled);

        // local_path 未设置时，effective_path 在两种模式下都回退到 path
        let claude = &platforms["claude"];
        assert_eq!(claude.effective_path(true), ".claude");
        assert_eq!(claude.effective_path(false), ".claude");
    }

    #[test]
    fn test_normalize_platforms_for_save() {
        let mut config = default_config();

        // 内置渠道：篡改 path/skills 后规范化，应精简为 name/enabled/builtin 三字段
        {
            let claude = config.platforms.get_mut("claude").unwrap();
            claude.name = Some("My Claude".to_string());
            claude.enabled = false;
            claude.path = ".tampered".to_string();
            claude.skills = "tampered".to_string();
        }

        // 自定义渠道：builtin 被误设为 true，规范化后应强制为 false
        let mut custom = make_platform(".my-custom", "skills", "CUSTOM.md");
        custom.builtin = true;
        config.platforms.insert("my-custom".to_string(), custom);

        config.normalize_platforms_for_save();

        // 内置渠道：仅保留 name/enabled/builtin，其余字段清空（序列化时跳过）
        let claude = &config.platforms["claude"];
        assert_eq!(claude.name.as_deref(), Some("My Claude"));
        assert!(!claude.enabled);
        assert!(claude.builtin);
        assert_eq!(claude.path, "");
        assert_eq!(claude.skills, "");
        assert_eq!(claude.agents, "");
        assert_eq!(claude.source, "");
        assert!(claude.local_path.is_none());
        assert!(!claude.agents_compat);

        // 精简条目序列化后仅含 name/enabled/builtin 三个字段（顺序无关）
        let json = serde_json::to_string_pretty(&config.platforms["claude"]).unwrap();
        let value: serde_json::Value = serde_json::from_str(&json).unwrap();
        let obj = value.as_object().unwrap();
        let mut keys: Vec<_> = obj.keys().cloned().collect();
        keys.sort();
        assert_eq!(keys, vec!["builtin", "enabled", "name"]);

        // 自定义渠道：builtin 强制 false，其余字段原样保留
        let custom = &config.platforms["my-custom"];
        assert!(!custom.builtin);
        assert_eq!(custom.path, ".my-custom");
        assert_eq!(custom.skills, "skills");
        assert_eq!(custom.agents, "CUSTOM.md");
    }

    #[test]
    fn test_enforce_builtin_platforms() {
        let mut config = default_config();

        // 用户篡改内置渠道的字段（path/skills/agents_compat），仅 name/enabled 允许覆盖
        {
            let claude = config.platforms.get_mut("claude").unwrap();
            claude.name = Some("My Claude".to_string());
            claude.enabled = false;
            claude.path = ".hacked".to_string();
            claude.skills = "hacked".to_string();
            claude.agents_compat = true;
        }

        config.enforce_builtin_platforms();

        let claude = &config.platforms["claude"];
        // name/enabled 保留用户设置
        assert_eq!(claude.name.as_deref(), Some("My Claude"));
        assert!(!claude.enabled);
        // 其余字段恢复内置默认值
        assert_eq!(claude.path, ".claude");
        assert_eq!(claude.skills, "skills");
        assert!(!claude.agents_compat);
        assert!(claude.builtin);

        // 自定义渠道不受影响
        config.platforms.insert(
            "my-custom".to_string(),
            make_platform(".my-custom", "skills", "CUSTOM.md"),
        );
        config.enforce_builtin_platforms();
        let custom = &config.platforms["my-custom"];
        assert_eq!(custom.path, ".my-custom");
        assert!(!custom.builtin);
    }

    #[test]
    fn test_platform_display_name() {
        let mut platform = default_platforms()["claude"].clone();
        assert_eq!(platform.display_name("claude"), "Claude Code");

        // name 缺失时回退到 key
        platform.name = None;
        assert_eq!(platform.display_name("claude"), "claude");
    }

    #[test]
    fn test_default_config() {
        let config = default_config();
        assert!(!config.cache.enabled);
        assert_eq!(config.cache.ttl, 86400);
        assert!(!config.registry.enabled);
        assert_eq!(config.registry.url, DEFAULT_REGISTRY_URL);
        assert_eq!(config.platforms.len(), 28);
        // init 时 proxy 键占位为空字符串（不生效，供用户填写）
        assert_eq!(config.proxy, Some(String::new()));
    }

    #[test]
    fn test_apply_proxy_env() {
        // Save & restore existing env to avoid leaking into other tests.
        let saved: Vec<(String, Option<String>)> = [
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "http_proxy",
            "https_proxy",
            "all_proxy",
        ]
        .iter()
        .map(|v| (v.to_string(), std::env::var(v).ok()))
        .collect();

        let proxy = "http://127.0.0.1:7890";
        let config = Config {
            proxy: Some(proxy.to_string()),
            ..default_config()
        };
        config.apply_proxy_env();

        for var in [
            "HTTP_PROXY",
            "HTTPS_PROXY",
            "ALL_PROXY",
            "http_proxy",
            "https_proxy",
            "all_proxy",
        ] {
            assert_eq!(std::env::var(var).as_deref(), Ok(proxy));
        }

        // None proxy → no env change beyond what was set before
        let config2 = Config {
            proxy: None,
            ..default_config()
        };
        config2.apply_proxy_env();
        assert_eq!(std::env::var("HTTPS_PROXY").as_deref(), Ok(proxy));

        // Restore
        for (var, val) in saved {
            match val {
                Some(v) => unsafe { std::env::set_var(var, v) },
                None => unsafe { std::env::remove_var(var) },
            }
        }
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
        assert!(
            !json.contains("null"),
            "Config JSON should not contain null values"
        );
    }
}
