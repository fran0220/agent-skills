pub mod routes;

use std::path::Path;
use std::sync::Arc;

use axum::Router;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

use crate::config::AppConfig;
use crate::core::dispatcher::Dispatcher;
use crate::core::registry::ProviderRegistry;
use crate::core::AssetProvider;
use crate::frontend;

pub struct ServerState {
    pub db: sqlx::PgPool,
    pub config: RwLock<AppConfig>,
    pub registry: Arc<ProviderRegistry>,
    pub dispatcher: Dispatcher,
}

/// Build all providers from config. A provider is enabled when its key is present.
pub fn build_providers_from_config(config: &AppConfig) -> Vec<Arc<dyn AssetProvider>> {
    let mut providers: Vec<Arc<dyn AssetProvider>> = Vec::new();

    if !config.proxy_key.is_empty() {
        // LLM text
        let mut llm = crate::providers::llm_proxy::LlmProxyProvider::new(
            config.proxy_url.clone(),
            config.proxy_key.clone(),
            config.default_model.clone(),
        );
        llm.id = "llm_proxy".into();
        providers.push(Arc::new(llm));

        // GPT Image
        let mut gpt = crate::providers::gpt_image::GptImageProvider::new(
            config.proxy_url.clone(),
            config.proxy_key.clone(),
        );
        gpt.id = "gpt_image".into();
        providers.push(Arc::new(gpt));

        // Gemini Image
        let mut gemini = crate::providers::gemini_image::GeminiImageProvider::new(
            config.proxy_url.clone(),
            config.proxy_key.clone(),
        );
        gemini.id = "gemini_image".into();
        providers.push(Arc::new(gemini));

        // Grok Image/Video
        let mut grok = crate::providers::grok_image::GrokImageProvider::new(
            config.proxy_url.clone(),
            config.proxy_key.clone(),
        );
        grok.id = "grok_image".into();
        providers.push(Arc::new(grok));
    }

    if !config.elevenlabs_key.is_empty() {
        let mut el = crate::providers::elevenlabs::ElevenLabsProvider::new(
            config.elevenlabs_key.clone(),
        );
        el.id = "elevenlabs".into();
        providers.push(Arc::new(el));
    }

    if !config.tripo3d_key.is_empty() {
        let mut tripo = crate::providers::tripo3d::Tripo3dProvider::new(
            config.tripo3d_key.clone(),
        );
        tripo.id = "tripo3d".into();
        providers.push(Arc::new(tripo));
    }

    if !config.jimeng_key.is_empty() && !config.jimeng_url.is_empty() {
        let mut jimeng = crate::providers::jimeng::JimengProvider::new(
            config.jimeng_url.clone(),
            config.jimeng_key.clone(),
        );
        jimeng.id = "jimeng".into();
        providers.push(Arc::new(jimeng));
    }

    providers
}

/// Register all config-driven providers into the registry, replacing any existing ones.
pub async fn load_providers(config: &AppConfig, registry: &Arc<ProviderRegistry>) -> usize {
    registry.clear().await;
    let providers = build_providers_from_config(config);
    let count = providers.len();
    for provider in providers {
        registry.register(provider).await;
    }
    count
}

pub async fn run(host: String, port: u16, database_url: String, config_path: &Path) -> anyhow::Result<()> {
    let config = AppConfig::load(host.clone(), port, database_url.clone(), config_path)?;
    let db = crate::db::connect(&database_url).await?;

    let registry = Arc::new(ProviderRegistry::new());
    let dispatcher = Dispatcher::new(registry.clone());

    let loaded = load_providers(&config, &registry).await;
    tracing::info!(loaded_provider_count = loaded, config_path = %config_path.display(), "providers loaded from config");

    let state = Arc::new(ServerState {
        db,
        config: RwLock::new(config),
        registry,
        dispatcher,
    });

    // Ensure uploads directory exists
    tokio::fs::create_dir_all("uploads").await?;

    let app = Router::new()
        .nest("/api", routes::api_router())
        .nest("/auth", routes::auth_router())
        .nest_service("/uploads", tower_http::services::ServeDir::new("uploads"))
        .merge(frontend::router())
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("{}:{}", host, port);
    tracing::info!("asset-gateway listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
