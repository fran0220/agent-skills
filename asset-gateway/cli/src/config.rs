use std::path::{Path, PathBuf};

use serde::Deserialize;

/// On-disk TOML structure. All fields optional — missing = disabled.
#[derive(Debug, Clone, Default, Deserialize)]
pub struct ConfigFile {
    #[serde(default)]
    pub server: ServerSection,
    #[serde(default)]
    pub proxy: ProxySection,
    #[serde(default)]
    pub elevenlabs: ProviderKeySection,
    #[serde(default)]
    pub tripo3d: ProviderKeySection,
    #[serde(default)]
    pub dashscope: DashscopeSection,
    #[serde(default)]
    pub grok2api: Grok2apiSection,
    #[serde(default)]
    pub jimeng: JimengSection,
    #[serde(default)]
    pub pixelengine: ProviderKeySection,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ServerSection {
    pub admin_token: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProxySection {
    pub url: Option<String>,
    pub key: Option<String>,
    pub default_model: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ProviderKeySection {
    pub key: Option<String>,
    pub keys: Option<Vec<String>>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DashscopeSection {
    pub url: Option<String>,
    pub key: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Grok2apiSection {
    pub url: Option<String>,
    pub key: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JimengSection {
    pub url: Option<String>,
    pub token: Option<String>,
}

/// Runtime config derived from TOML file + env var overrides.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub config_path: PathBuf,
    pub admin_token: String,

    // Shared LLM proxy (covers llm_proxy, gemini_image, gpt_image)
    pub proxy_url: String,
    pub proxy_key: String,
    pub default_model: String,

    // External provider keys (empty = disabled)
    pub elevenlabs_key: String,
    pub tripo3d_keys: Vec<String>,

    // DashScope Qwen3-TTS (International Singapore region)
    pub dashscope_url: String,
    pub dashscope_key: String,

    // Grok2API (direct connection to grok2api-go reverse proxy)
    pub grok2api_url: String,
    pub grok2api_key: String,

    // Jimeng video+image (ByteDance) — jimeng-api gateway
    pub jimeng_url: String,
    pub jimeng_token: String,

    // PixelEngine — sprite animation generation
    pub pixelengine_key: String,
}

impl AppConfig {
    /// Load config: TOML file first, env vars override.
    pub fn load(
        host: String,
        port: u16,
        database_url: String,
        config_path: &Path,
    ) -> anyhow::Result<Self> {
        let file_cfg = if config_path.exists() {
            let content = std::fs::read_to_string(config_path)?;
            toml::from_str::<ConfigFile>(&content)?
        } else {
            tracing::warn!(path = %config_path.display(), "config file not found, using env vars only");
            ConfigFile::default()
        };

        let database_url = if database_url.trim().is_empty() {
            std::env::var("ASSET_GATEWAY_DATABASE_URL")
                .unwrap_or_else(|_| "postgres://localhost/asset_gateway".into())
        } else {
            database_url
        };

        Ok(Self {
            host,
            port,
            database_url,
            config_path: config_path.to_path_buf(),

            admin_token: env_or(
                "ASSET_GATEWAY_ADMIN_TOKEN",
                file_cfg.server.admin_token.as_deref(),
                "",
            ),

            proxy_url: env_or(
                "ASSET_GATEWAY_PROXY_URL",
                file_cfg.proxy.url.as_deref(),
                "https://api.xiaomao.chat",
            ),
            proxy_key: env_or("ASSET_GATEWAY_PROXY_KEY", file_cfg.proxy.key.as_deref(), ""),
            default_model: env_or(
                "ASSET_GATEWAY_DEFAULT_MODEL",
                file_cfg.proxy.default_model.as_deref(),
                "claude-sonnet-4-6",
            ),

            elevenlabs_key: env_or(
                "ASSET_GATEWAY_ELEVENLABS_KEY",
                file_cfg.elevenlabs.key.as_deref(),
                "",
            ),
            tripo3d_keys: {
                let env_keys = std::env::var("ASSET_GATEWAY_TRIPO3D_KEY").unwrap_or_default();
                if !env_keys.trim().is_empty() {
                    env_keys
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect()
                } else if let Some(keys) = &file_cfg.tripo3d.keys {
                    keys.iter()
                        .filter(|s| !s.trim().is_empty())
                        .cloned()
                        .collect()
                } else if let Some(key) = &file_cfg.tripo3d.key {
                    if key.trim().is_empty() {
                        vec![]
                    } else {
                        vec![key.clone()]
                    }
                } else {
                    vec![]
                }
            },

            dashscope_url: env_or(
                "ASSET_GATEWAY_DASHSCOPE_URL",
                file_cfg.dashscope.url.as_deref(),
                "https://dashscope-intl.aliyuncs.com",
            ),
            dashscope_key: env_or(
                "ASSET_GATEWAY_DASHSCOPE_KEY",
                file_cfg.dashscope.key.as_deref(),
                "",
            ),

            grok2api_url: env_or(
                "ASSET_GATEWAY_GROK2API_URL",
                file_cfg.grok2api.url.as_deref(),
                "",
            ),
            grok2api_key: env_or(
                "ASSET_GATEWAY_GROK2API_KEY",
                file_cfg.grok2api.key.as_deref(),
                "",
            ),

            jimeng_url: env_or(
                "ASSET_GATEWAY_JIMENG_URL",
                file_cfg.jimeng.url.as_deref(),
                "",
            ),
            jimeng_token: env_or(
                "ASSET_GATEWAY_JIMENG_TOKEN",
                file_cfg.jimeng.token.as_deref(),
                "",
            ),

            pixelengine_key: env_or(
                "ASSET_GATEWAY_PIXELENGINE_KEY",
                file_cfg.pixelengine.key.as_deref(),
                "",
            ),
        })
    }

    /// Re-read config file for hot reload (keeps host/port/database_url unchanged).
    pub fn reload(&self) -> anyhow::Result<Self> {
        Self::load(
            self.host.clone(),
            self.port,
            self.database_url.clone(),
            &self.config_path,
        )
    }
}

/// Env var wins > TOML value > default.
fn env_or(env_key: &str, file_value: Option<&str>, default: &str) -> String {
    if let Ok(val) = std::env::var(env_key) {
        if !val.trim().is_empty() {
            return val;
        }
    }
    if let Some(val) = file_value {
        if !val.trim().is_empty() {
            return val.to_string();
        }
    }
    default.to_string()
}
