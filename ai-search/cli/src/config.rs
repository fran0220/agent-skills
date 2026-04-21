use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct ConfigFile {
    server: ServerSection,
    proxy: ProxySection,
    grok: GrokSection,
    exa: ApiKeySection,
    tavily: ApiKeySection,
    search: SearchSection,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct ServerSection {
    port: Option<u16>,
    gateway_token: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct ProxySection {
    url: Option<String>,
    key: Option<String>,
    search_model: Option<String>,
    analysis_model: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct GrokSection {
    url: Option<String>,
    key: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct ApiKeySection {
    key: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default)]
struct SearchSection {
    max_split: Option<u32>,
    timeout_secs: Option<u64>,
    default_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub api_url: String,
    pub api_key: String,
    pub search_model: String,
    pub grok_url: String,
    pub grok_key: String,
    pub analysis_model: String,
    pub max_split: u32,
    pub timeout_secs: u64,
    pub server_port: u16,
    pub gateway_token: String,
    pub exa_key: String,
    pub tavily_key: String,
    pub default_mode: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            api_url: "https://api.xiaomao.chat".to_string(),
            api_key: String::new(),
            grok_url: String::new(),
            grok_key: String::new(),
            search_model: "grok-4.20-beta".to_string(),
            analysis_model: "gemini-3-flash-preview".to_string(),
            max_split: 10,
            timeout_secs: 180,
            server_port: 6900,
            gateway_token: String::new(),
            exa_key: String::new(),
            tavily_key: String::new(),
            default_mode: "deep".to_string(),
        }
    }
}

impl AppConfig {
    pub fn load() -> Result<Self> {
        Self::load_from(Self::config_path())
    }

    pub fn load_from(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        let mut config = Self::default();

        if path.exists() {
            let content = std::fs::read_to_string(path)?;
            let file_config: ConfigFile = toml::from_str(&content)?;
            config.apply_file(file_config);
        }

        config.apply_env_overrides();
        config.default_mode = normalize_mode(&config.default_mode);

        Ok(config)
    }

    /// Effective Grok API URL (falls back to api_url).
    pub fn effective_grok_url(&self) -> &str {
        if self.grok_url.is_empty() {
            &self.api_url
        } else {
            &self.grok_url
        }
    }

    /// Effective Grok API key (falls back to api_key).
    pub fn effective_grok_key(&self) -> &str {
        if self.grok_key.is_empty() {
            &self.api_key
        } else {
            &self.grok_key
        }
    }

    pub fn apply_env_overrides(&mut self) {
        apply_string_env("AI_SEARCH_URL", &mut self.api_url);
        apply_string_env("AI_SEARCH_KEY", &mut self.api_key);
        apply_string_env("AI_SEARCH_GROK_URL", &mut self.grok_url);
        apply_string_env("AI_SEARCH_GROK_KEY", &mut self.grok_key);
        apply_string_env("AI_SEARCH_MODEL", &mut self.search_model);
        apply_string_env("AI_SEARCH_ANALYSIS_MODEL", &mut self.analysis_model);
        apply_string_env("AI_SEARCH_GATEWAY_TOKEN", &mut self.gateway_token);
        apply_string_env("EXA_KEY", &mut self.exa_key);
        apply_string_env("TAVILY_KEY", &mut self.tavily_key);
        apply_string_env("AI_SEARCH_DEFAULT_MODE", &mut self.default_mode);

        apply_parsed_env("AI_SEARCH_MAX_SPLIT", &mut self.max_split);
        apply_parsed_env("AI_SEARCH_TIMEOUT", &mut self.timeout_secs);
        apply_parsed_env("AI_SEARCH_PORT", &mut self.server_port);
    }

    pub fn config_path() -> PathBuf {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        config_path_from_home(home)
    }

    fn apply_file(&mut self, file: ConfigFile) {
        if let Some(port) = file.server.port {
            self.server_port = port;
        }
        if let Some(token) = file.server.gateway_token {
            self.gateway_token = token;
        }

        if let Some(url) = file.proxy.url {
            self.api_url = url;
        }
        if let Some(key) = file.proxy.key {
            self.api_key = key;
        }
        if let Some(model) = file.proxy.search_model {
            self.search_model = model;
        }
        if let Some(model) = file.proxy.analysis_model {
            self.analysis_model = model;
        }

        if let Some(url) = file.grok.url {
            self.grok_url = url;
        }
        if let Some(key) = file.grok.key {
            self.grok_key = key;
        }

        if let Some(key) = file.exa.key {
            self.exa_key = key;
        }
        if let Some(key) = file.tavily.key {
            self.tavily_key = key;
        }

        if let Some(max_split) = file.search.max_split {
            self.max_split = max_split;
        }
        if let Some(timeout_secs) = file.search.timeout_secs {
            self.timeout_secs = timeout_secs;
        }
        if let Some(default_mode) = file.search.default_mode {
            self.default_mode = default_mode;
        }
    }
}

fn apply_string_env(env_key: &str, target: &mut String) {
    if let Ok(value) = std::env::var(env_key) {
        if !value.trim().is_empty() {
            *target = value;
        }
    }
}

fn apply_parsed_env<T>(env_key: &str, target: &mut T)
where
    T: std::str::FromStr,
{
    if let Ok(value) = std::env::var(env_key) {
        if let Ok(parsed) = value.parse() {
            *target = parsed;
        }
    }
}

fn normalize_mode(mode: &str) -> String {
    match mode.trim().to_ascii_lowercase().as_str() {
        "fast" | "deep" | "answer" => mode.trim().to_ascii_lowercase(),
        _ => "deep".to_string(),
    }
}

fn config_path_from_home(home: Option<PathBuf>) -> PathBuf {
    home.unwrap_or_else(|| PathBuf::from("."))
        .join(".config")
        .join("ai-search")
        .join("config.toml")
}

pub const SEARCH_MODELS: &[&str] = &[
    "grok-4.20-fast",
    "grok-4.20-auto",
    "grok-4.20-expert",
    "grok-4.20-0309",
    "grok-4.3-beta",
];

pub const SEARCH_MODES: &[&str] = &["fast", "deep", "answer"];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_path_uses_home_dot_config() {
        let path = config_path_from_home(Some(PathBuf::from("/tmp/test-home")));
        assert_eq!(
            path,
            PathBuf::from("/tmp/test-home/.config/ai-search/config.toml")
        );
    }

    #[test]
    fn load_from_sectioned_toml() {
        let dir =
            std::env::temp_dir().join(format!("ai-search-config-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config.toml");
        std::fs::write(
            &path,
            r#"
[server]
port = 7777
gateway_token = "ask_test"

[proxy]
url = "https://proxy.example.com"
key = "sk-test"
search_model = "grok-4.20-beta"
analysis_model = "gemini-3-flash-preview"

[exa]
key = "exa-test"

[tavily]
key = "tvly-test"

[search]
max_split = 4
timeout_secs = 99
default_mode = "fast"
"#,
        )
        .unwrap();

        let config = AppConfig::load_from(&path).unwrap();
        assert_eq!(config.server_port, 7777);
        assert_eq!(config.gateway_token, "ask_test");
        assert_eq!(config.api_url, "https://proxy.example.com");
        assert_eq!(config.api_key, "sk-test");
        assert_eq!(config.search_model, "grok-4.20-beta");
        assert_eq!(config.analysis_model, "gemini-3-flash-preview");
        assert_eq!(config.exa_key, "exa-test");
        assert_eq!(config.tavily_key, "tvly-test");
        assert_eq!(config.max_split, 4);
        assert_eq!(config.timeout_secs, 99);
        assert_eq!(config.default_mode, "fast");

        std::fs::remove_file(path).unwrap();
        std::fs::remove_dir(dir).unwrap();
    }
}
