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
    pub jimeng: JimengSection,
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
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct JimengSection {
    pub url: Option<String>,
    pub key: Option<String>,
}

/// Runtime config derived from TOML file + env var overrides.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub config_path: PathBuf,
    pub admin_token: String,

    // Shared LLM proxy (covers llm_proxy, gpt_image, gemini_image, grok_image)
    pub proxy_url: String,
    pub proxy_key: String,
    pub default_model: String,

    // External provider keys (empty = disabled)
    pub elevenlabs_key: String,
    pub tripo3d_key: String,
    pub jimeng_url: String,
    pub jimeng_key: String,
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
            proxy_key: env_or(
                "ASSET_GATEWAY_PROXY_KEY",
                file_cfg.proxy.key.as_deref(),
                "",
            ),
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
            tripo3d_key: env_or(
                "ASSET_GATEWAY_TRIPO3D_KEY",
                file_cfg.tripo3d.key.as_deref(),
                "",
            ),
            jimeng_url: env_or(
                "ASSET_GATEWAY_JIMENG_URL",
                file_cfg.jimeng.url.as_deref(),
                "",
            ),
            jimeng_key: env_or(
                "ASSET_GATEWAY_JIMENG_KEY",
                file_cfg.jimeng.key.as_deref(),
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
