pub mod routes;

use std::collections::HashMap;
use std::sync::Arc;

use axum::Router;
use serde_json::Value;
use sqlx::Row;
use tower_http::cors::CorsLayer;

use crate::config::AppConfig;
use crate::core::dispatcher::Dispatcher;
use crate::core::registry::ProviderRegistry;
use crate::core::vault::Vault;
use crate::core::AssetProvider;
use crate::frontend;

pub struct ServerState {
    pub db: sqlx::PgPool,
    pub config: AppConfig,
    pub vault: Vault,
    pub registry: Arc<ProviderRegistry>,
    pub dispatcher: Dispatcher,
}

fn credential_candidates(adapter: &str) -> &'static [&'static str] {
    match adapter {
        "llm_proxy" => &[
            "api_key",
            "llm_proxy_api_key",
            "llm_proxy_key",
            "LLM_PROXY_KEY",
        ],
        "gpt_image" => &["api_key", "openai_api_key", "OPENAI_API_KEY"],
        "gemini_image" => &["api_key", "gemini_api_key", "GOOGLE_API_KEY"],
        "jimeng" => &["api_key", "jimeng_api_key", "JIMENG_API_KEY"],
        "elevenlabs" => &["api_key", "elevenlabs_api_key", "ELEVENLABS_API_KEY"],
        "tripo3d" => &["api_key", "tripo3d_api_key", "TRIPO3D_API_KEY"],
        _ => &["api_key"],
    }
}

fn resolve_credential(
    provider_id: &str,
    adapter: &str,
    scoped: &HashMap<String, HashMap<String, String>>,
    global: &HashMap<String, String>,
) -> Option<String> {
    for key in credential_candidates(adapter) {
        if let Some(provider_map) = scoped.get(provider_id) {
            if let Some(value) = provider_map.get(*key) {
                return Some(value.clone());
            }
        }
        if let Some(value) = global.get(*key) {
            return Some(value.clone());
        }
    }
    None
}

fn config_string(config: &Value, key: &str) -> Option<String> {
    config
        .get(key)
        .and_then(|value| value.as_str())
        .map(ToString::to_string)
}

fn build_provider(
    provider_id: &str,
    adapter: &str,
    config: &Value,
    scoped_credentials: &HashMap<String, HashMap<String, String>>,
    global_credentials: &HashMap<String, String>,
) -> anyhow::Result<Arc<dyn AssetProvider>> {
    let api_key = resolve_credential(provider_id, adapter, scoped_credentials, global_credentials)
        .ok_or_else(|| anyhow::anyhow!("missing credential for adapter {}", adapter))?;

    match adapter {
        "llm_proxy" => {
            let base_url = config_string(config, "base_url")
                .unwrap_or_else(|| "http://67.230.182.59:8317".into());
            let default_model = config_string(config, "default_model")
                .unwrap_or_else(|| "claude-sonnet-4-6".into());
            let mut provider = crate::providers::llm_proxy::LlmProxyProvider::new(
                base_url,
                api_key,
                default_model,
            );
            provider.id = provider_id.to_string();
            Ok(Arc::new(provider))
        }
        "gpt_image" => {
            let base_url = config_string(config, "base_url")
                .unwrap_or_else(|| "https://api.openai.com".into());
            let mut provider =
                crate::providers::gpt_image::GptImageProvider::new(base_url, api_key);
            provider.id = provider_id.to_string();
            Ok(Arc::new(provider))
        }
        "gemini_image" => {
            let base_url = config_string(config, "base_url")
                .unwrap_or_else(|| "https://generativelanguage.googleapis.com".into());
            let mut provider =
                crate::providers::gemini_image::GeminiImageProvider::new(base_url, api_key);
            provider.id = provider_id.to_string();
            Ok(Arc::new(provider))
        }
        "jimeng" => {
            let base_url =
                config_string(config, "base_url").unwrap_or_else(|| "https://api.jimeng.ai".into());
            let mut provider = crate::providers::jimeng::JimengProvider::new(base_url, api_key);
            provider.id = provider_id.to_string();
            Ok(Arc::new(provider))
        }
        "elevenlabs" => {
            let mut provider = crate::providers::elevenlabs::ElevenLabsProvider::new(api_key);
            provider.id = provider_id.to_string();
            Ok(Arc::new(provider))
        }
        "tripo3d" => {
            let mut provider = crate::providers::tripo3d::Tripo3dProvider::new(api_key);
            if let Some(url) = config_string(config, "base_url") {
                provider = provider.with_base_url(url);
            }
            provider.id = provider_id.to_string();
            Ok(Arc::new(provider))
        }
        _ => Err(anyhow::anyhow!("unsupported provider adapter: {}", adapter)),
    }
}

async fn load_credentials(
    db: &sqlx::PgPool,
    vault: &Vault,
) -> anyhow::Result<(
    HashMap<String, String>,
    HashMap<String, HashMap<String, String>>,
)> {
    let rows = sqlx::query("SELECT key, encrypted_value, nonce, provider_id FROM credentials")
        .fetch_all(db)
        .await?;

    let mut global = HashMap::new();
    let mut scoped: HashMap<String, HashMap<String, String>> = HashMap::new();

    for row in rows {
        let key: String = row.try_get("key")?;
        let encrypted_value: Vec<u8> = row.try_get("encrypted_value")?;
        let nonce: Vec<u8> = row.try_get("nonce")?;
        let provider_id: Option<String> = row.try_get("provider_id")?;

        let value = match vault.decrypt(&encrypted_value, &nonce) {
            Ok(v) => v,
            Err(error) => {
                tracing::warn!(credential_key = %key, error = %error, "skip invalid encrypted credential");
                continue;
            }
        };

        if let Some(pid) = provider_id {
            scoped.entry(pid).or_default().insert(key, value);
        } else {
            global.insert(key, value);
        }
    }

    Ok((global, scoped))
}

pub async fn register_provider_by_id(
    db: &sqlx::PgPool,
    vault: &Vault,
    registry: &Arc<ProviderRegistry>,
    provider_id: &str,
) -> anyhow::Result<()> {
    let row = sqlx::query("SELECT id, adapter, config, enabled FROM providers WHERE id = $1")
        .bind(provider_id)
        .fetch_optional(db)
        .await?
        .ok_or_else(|| anyhow::anyhow!("provider not found: {}", provider_id))?;

    let enabled: bool = row.try_get("enabled")?;
    if !enabled {
        anyhow::bail!("provider is disabled: {}", provider_id);
    }

    let adapter: String = row.try_get("adapter")?;
    let config_raw: String = row.try_get("config")?;
    let config =
        serde_json::from_str::<Value>(&config_raw).unwrap_or(Value::Object(Default::default()));

    let (global_credentials, scoped_credentials) = load_credentials(db, vault).await?;
    let provider = build_provider(
        provider_id,
        &adapter,
        &config,
        &scoped_credentials,
        &global_credentials,
    )?;

    registry.register(provider).await;
    Ok(())
}

pub async fn reload_provider_by_id(
    db: &sqlx::PgPool,
    vault: &Vault,
    registry: &Arc<ProviderRegistry>,
    provider_id: &str,
) -> anyhow::Result<()> {
    match register_provider_by_id(db, vault, registry, provider_id).await {
        Ok(()) => Ok(()),
        Err(error) => {
            // Ensure stale runtime providers are removed when rebuild fails.
            registry.unregister(provider_id).await;
            Err(error)
        }
    }
}

pub async fn list_enabled_provider_ids(db: &sqlx::PgPool) -> anyhow::Result<Vec<String>> {
    let provider_ids = sqlx::query_scalar(
        "SELECT id FROM providers WHERE enabled = TRUE ORDER BY priority DESC, id ASC",
    )
    .fetch_all(db)
    .await?;
    Ok(provider_ids)
}

pub async fn load_enabled_providers(
    db: &sqlx::PgPool,
    vault: &Vault,
    registry: &Arc<ProviderRegistry>,
) -> anyhow::Result<usize> {
    let rows = sqlx::query(
        "SELECT id, adapter, config FROM providers WHERE enabled = TRUE ORDER BY priority DESC, id ASC",
    )
        .fetch_all(db)
        .await?;

    let (global_credentials, scoped_credentials) = load_credentials(db, vault).await?;
    let mut loaded = 0usize;

    for row in rows {
        let provider_id: String = row.try_get("id")?;
        let adapter: String = row.try_get("adapter")?;
        let config_raw: String = row.try_get("config")?;
        let config =
            serde_json::from_str::<Value>(&config_raw).unwrap_or(Value::Object(Default::default()));

        match build_provider(
            &provider_id,
            &adapter,
            &config,
            &scoped_credentials,
            &global_credentials,
        ) {
            Ok(provider) => {
                registry.register(provider).await;
                loaded += 1;
            }
            Err(error) => {
                tracing::warn!(
                    provider_id = %provider_id,
                    adapter = %adapter,
                    error = %error,
                    "skip provider during boot load"
                );
            }
        }
    }

    Ok(loaded)
}

pub async fn run(host: String, port: u16, database_url: String) -> anyhow::Result<()> {
    let config = AppConfig::load(host.clone(), port, database_url.clone())?;
    let db = crate::db::connect(&database_url).await?;

    let vault = Vault::new(&config.vault_key);
    let registry = Arc::new(ProviderRegistry::new());
    let dispatcher = Dispatcher::new(registry.clone());

    let loaded = load_enabled_providers(&db, &vault, &registry).await?;
    tracing::info!(
        loaded_provider_count = loaded,
        "providers loaded at startup"
    );

    let state = Arc::new(ServerState {
        db,
        config,
        vault,
        registry,
        dispatcher,
    });

    let app = Router::new()
        .nest("/api", routes::api_router())
        .nest("/auth", routes::auth_router())
        .merge(frontend::router())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("{}:{}", host, port);
    tracing::info!("asset-gateway listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
