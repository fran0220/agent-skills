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
    pub xai: ProviderKeySection,
    #[serde(default)]
    pub grok2api: Grok2apiSection,
    #[serde(default)]
    pub jimeng: JimengSection,
    #[serde(default)]
    pub worldlabs: ProviderKeySection,
    #[serde(default)]
    pub vertex: VertexSection,
    #[serde(default)]
    pub autosprite: ProviderKeySection,
    #[serde(default)]
    pub chatgpt2api: Chatgpt2apiSection,
    #[serde(default)]
    pub storage: Option<StorageSection>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct StorageSection {
    pub endpoint: Option<String>,
    pub bucket: Option<String>,
    pub region: Option<String>,
    pub access_key: Option<String>,
    pub secret_key: Option<String>,
    pub public_url: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Grok2apiSection {
    pub url: Option<String>,
    pub key: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct Chatgpt2apiSection {
    pub url: Option<String>,
    pub key: Option<String>,
    pub fallback_url: Option<String>,
    pub fallback_key: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ServerSection {
    pub admin_token: Option<String>,
    pub public_url: Option<String>,
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
pub struct JimengSection {
    pub url: Option<String>,
    pub token: Option<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct VertexSection {
    pub service_account_path: Option<String>,
    pub project_id: Option<String>,
    pub location: Option<String>,
}

/// Runtime config derived from TOML file + env var overrides.
#[derive(Debug, Clone)]
pub struct AppConfig {
    pub host: String,
    pub port: u16,
    pub database_url: String,
    pub config_path: PathBuf,
    pub admin_token: String,
    pub public_url: String,

    // Shared LLM proxy (covers llm_proxy, gemini_image, gpt_image)
    pub proxy_url: String,
    pub proxy_key: String,
    pub default_model: String,

    // Vertex AI direct access (bypasses proxy for Google models)
    pub vertex_sa_path: String,
    pub vertex_project: String,
    pub vertex_location: String,

    // External provider keys (empty = disabled)
    pub elevenlabs_key: String,
    pub tripo3d_keys: Vec<String>,

    // xAI direct API — Grok image + video generation
    pub xai_key: String,

    // grok2api proxy — Grok image/edit/video via self-hosted gateway
    pub grok2api_url: String,
    pub grok2api_key: String,

    // Jimeng video+image (ByteDance) — jimeng-api gateway
    pub jimeng_url: String,
    pub jimeng_token: String,

    // WorldLabs Marble — 3D world/environment generation
    pub worldlabs_key: String,

    // AutoSprite — sprite sheet generation
    pub autosprite_key: String,

    // ChatGPT2API — ChatGPT Plus image generation gateway
    pub chatgpt2api_url: String,
    pub chatgpt2api_key: String,
    pub chatgpt2api_fallback_url: String,
    pub chatgpt2api_fallback_key: String,

    // S3-compatible object storage (Bitiful / Aliyun OSS / R2)
    pub storage: Option<StorageConfig>,
}

/// Resolved storage config (all fields required when present).
#[derive(Debug, Clone)]
pub struct StorageConfig {
    pub endpoint: String,
    pub bucket: String,
    pub region: Option<String>,
    pub access_key: String,
    pub secret_key: String,
    pub public_url: Option<String>,
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
            public_url: env_or(
                "ASSET_GATEWAY_PUBLIC_URL",
                file_cfg.server.public_url.as_deref(),
                "https://asset.origingame.dev",
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
            vertex_sa_path: env_or(
                "ASSET_GATEWAY_VERTEX_SA_PATH",
                file_cfg.vertex.service_account_path.as_deref(),
                "",
            ),
            vertex_project: env_or(
                "ASSET_GATEWAY_VERTEX_PROJECT",
                file_cfg.vertex.project_id.as_deref(),
                "",
            ),
            vertex_location: env_or(
                "ASSET_GATEWAY_VERTEX_LOCATION",
                file_cfg.vertex.location.as_deref(),
                "global",
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

            xai_key: env_or("ASSET_GATEWAY_XAI_KEY", file_cfg.xai.key.as_deref(), ""),

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

            worldlabs_key: env_or(
                "ASSET_GATEWAY_WORLDLABS_KEY",
                file_cfg.worldlabs.key.as_deref(),
                "",
            ),

            autosprite_key: env_or(
                "ASSET_GATEWAY_AUTOSPRITE_KEY",
                file_cfg.autosprite.key.as_deref(),
                "",
            ),

            chatgpt2api_url: env_or(
                "ASSET_GATEWAY_CHATGPT2API_URL",
                file_cfg.chatgpt2api.url.as_deref(),
                "",
            ),
            chatgpt2api_key: env_or(
                "ASSET_GATEWAY_CHATGPT2API_KEY",
                file_cfg.chatgpt2api.key.as_deref(),
                "",
            ),
            chatgpt2api_fallback_url: env_or(
                "ASSET_GATEWAY_CHATGPT2API_FALLBACK_URL",
                file_cfg.chatgpt2api.fallback_url.as_deref(),
                "",
            ),
            chatgpt2api_fallback_key: env_or(
                "ASSET_GATEWAY_CHATGPT2API_FALLBACK_KEY",
                file_cfg.chatgpt2api.fallback_key.as_deref(),
                "",
            ),

            storage: {
                let endpoint = env_or(
                    "ASSET_GATEWAY_STORAGE_ENDPOINT",
                    file_cfg.storage.as_ref().and_then(|s| s.endpoint.as_deref()),
                    "",
                );
                let bucket = env_or(
                    "ASSET_GATEWAY_STORAGE_BUCKET",
                    file_cfg.storage.as_ref().and_then(|s| s.bucket.as_deref()),
                    "",
                );
                let access_key = env_or(
                    "ASSET_GATEWAY_STORAGE_ACCESS_KEY",
                    file_cfg.storage.as_ref().and_then(|s| s.access_key.as_deref()),
                    "",
                );
                let secret_key = env_or(
                    "ASSET_GATEWAY_STORAGE_SECRET_KEY",
                    file_cfg.storage.as_ref().and_then(|s| s.secret_key.as_deref()),
                    "",
                );
                if !endpoint.is_empty() && !bucket.is_empty() && !access_key.is_empty() {
                    Some(StorageConfig {
                        endpoint,
                        bucket,
                        region: {
                            let r = env_or(
                                "ASSET_GATEWAY_STORAGE_REGION",
                                file_cfg.storage.as_ref().and_then(|s| s.region.as_deref()),
                                "",
                            );
                            if r.is_empty() { None } else { Some(r) }
                        },
                        access_key,
                        secret_key,
                        public_url: {
                            let u = env_or(
                                "ASSET_GATEWAY_STORAGE_PUBLIC_URL",
                                file_cfg.storage.as_ref().and_then(|s| s.public_url.as_deref()),
                                "",
                            );
                            if u.is_empty() { None } else { Some(u) }
                        },
                    })
                } else {
                    None
                }
            },
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
