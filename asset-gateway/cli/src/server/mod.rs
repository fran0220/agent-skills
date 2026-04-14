pub mod routes;
pub mod ws;

use std::path::Path;
use std::sync::Arc;

use axum::extract::DefaultBodyLimit;
use axum::Router;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;

use crate::config::AppConfig;
use crate::core::dispatcher::Dispatcher;
use crate::core::registry::ProviderRegistry;
use crate::core::AssetProvider;
use crate::frontend;
use crate::server::ws::JobEventHub;

pub struct ServerState {
    pub db: sqlx::PgPool,
    pub config: RwLock<AppConfig>,
    pub registry: Arc<ProviderRegistry>,
    pub dispatcher: Dispatcher,
    pub job_events: JobEventHub,
}

fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") {
        if let Ok(home) = std::env::var("HOME") {
            return format!("{}{}", home, &path[1..]);
        }
    }
    path.to_string()
}

/// Build all providers from config. A provider is enabled when its key is present.
pub async fn build_providers_from_config(config: &AppConfig) -> Vec<Arc<dyn AssetProvider>> {
    let mut providers: Vec<Arc<dyn AssetProvider>> = Vec::new();

    let vertex_auth: Option<crate::vertex_auth::VertexAuth> = if !config.vertex_sa_path.is_empty()
        && !config.vertex_project.is_empty()
        && !config.vertex_location.is_empty()
    {
        let sa_path = expand_tilde(&config.vertex_sa_path);

        match crate::vertex_auth::VertexAuth::from_file(&sa_path) {
            Ok(auth) => {
                tracing::info!(
                    project = %config.vertex_project,
                    location = %config.vertex_location,
                    "Vertex AI auth initialized"
                );
                Some(auth)
            }
            Err(error) => {
                tracing::warn!(
                    error = %error,
                    "Failed to init Vertex AI auth, falling back to proxy"
                );
                None
            }
        }
    } else {
        None
    };

    if !config.proxy_key.is_empty() {
        // LLM text
        let mut llm = crate::providers::llm_proxy::LlmProxyProvider::new(
            config.proxy_url.clone(),
            config.proxy_key.clone(),
            config.default_model.clone(),
        );
        llm.id = "llm_proxy".into();
        providers.push(Arc::new(llm));

        // Gemini Image
        let mut gemini = crate::providers::gemini_image::GeminiImageProvider::new(
            config.proxy_url.clone(),
            config.proxy_key.clone(),
        );
        if let Some(ref auth) = vertex_auth {
            gemini = gemini.with_vertex(
                auth.clone(),
                config.vertex_project.clone(),
                config.vertex_location.clone(),
            );
        }
        gemini.id = "gemini_image".into();
        providers.push(Arc::new(gemini));

        // GPT Image (transparent-capable)
        let mut gpt = crate::providers::gpt_image::GptImageProvider::new(
            config.proxy_url.clone(),
            config.proxy_key.clone(),
        );
        gpt.id = "gpt_image".into();
        providers.push(Arc::new(gpt));

        // Lyria — music/audio generation (Google, replaces ElevenLabs for BGM)
        let mut lyria = crate::providers::lyria::LyriaProvider::new(
            config.proxy_url.clone(),
            config.proxy_key.clone(),
        );
        if let Some(ref auth) = vertex_auth {
            lyria = lyria.with_vertex(
                auth.clone(),
                config.vertex_project.clone(),
                config.vertex_location.clone(),
            );
        }
        providers.push(Arc::new(lyria));
    }

    // Veo — general video generation via Vertex AI
    if let Some(ref auth) = vertex_auth {
        let veo =
            crate::providers::veo::VeoProvider::new(auth.clone(), config.vertex_project.clone());
        providers.push(Arc::new(veo));
    }

    // AutoSprite — dedicated sprite sheet generation
    if !config.autosprite_key.is_empty() {
        let autosprite =
            crate::providers::autosprite::AutoSpriteProvider::new(config.autosprite_key.clone());
        providers.push(Arc::new(autosprite));
    }

    if !config.jimeng_token.is_empty() && !config.jimeng_url.is_empty() {
        let mut jimeng = crate::providers::jimeng::JimengProvider::new(
            config.jimeng_url.clone(),
            config.jimeng_token.clone(),
        );
        jimeng.id = "jimeng".into();
        providers.push(Arc::new(jimeng));
    }

    // Grok — image + video via xAI direct API
    if !config.xai_key.is_empty() {
        let mut grok = crate::providers::grok_image::GrokImageProvider::new(config.xai_key.clone());
        grok.id = "grok_image".into();
        providers.push(Arc::new(grok));
    }

    if !config.elevenlabs_key.is_empty() {
        let mut el =
            crate::providers::elevenlabs::ElevenLabsProvider::new(config.elevenlabs_key.clone());
        el.id = "elevenlabs".into();
        providers.push(Arc::new(el));
    }

    if !config.tripo3d_keys.is_empty() {
        let mut tripo =
            crate::providers::tripo3d::Tripo3dProvider::new(config.tripo3d_keys.clone());
        tripo.id = "tripo3d".into();
        providers.push(Arc::new(tripo));
    }

    if !config.worldlabs_key.is_empty() {
        let mut wl =
            crate::providers::worldlabs::WorldLabsProvider::new(config.worldlabs_key.clone());
        wl.id = "worldlabs".into();
        providers.push(Arc::new(wl));
    }

    if !config.dashscope_key.is_empty() {
        let mut qwen = crate::providers::qwen_tts::QwenTtsProvider::new(
            config.dashscope_url.clone(),
            config.dashscope_key.clone(),
        );
        qwen.id = "qwen_tts".into();
        providers.push(Arc::new(qwen));
    }

    if !config.voicebox_url.is_empty() {
        let mut vb = crate::providers::voicebox::VoiceBoxProvider::new(config.voicebox_url.clone());
        vb.id = "voicebox".into();
        providers.push(Arc::new(vb));
    }

    providers
}

/// Register all config-driven providers into the registry, replacing any existing ones.
pub async fn load_providers(config: &AppConfig, registry: &Arc<ProviderRegistry>) -> usize {
    registry.clear().await;
    let providers = build_providers_from_config(config).await;
    let count = providers.len();
    for provider in providers {
        registry.register(provider).await;
    }
    count
}

pub async fn run(
    host: String,
    port: u16,
    database_url: String,
    config_path: &Path,
) -> anyhow::Result<()> {
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
        job_events: JobEventHub::default(),
    });

    // Ensure uploads directory exists
    tokio::fs::create_dir_all("uploads").await?;

    let app = Router::new()
        .nest("/api", routes::api_router())
        .nest("/auth", routes::auth_router())
        .merge(ws::router())
        .nest_service("/uploads", tower_http::services::ServeDir::new("uploads"))
        .merge(frontend::router())
        .layer(DefaultBodyLimit::max(500 * 1024 * 1024)) // 500MB
        .layer(CorsLayer::permissive())
        .with_state(state);

    let addr = format!("{}:{}", host, port);
    tracing::info!("asset-gateway listening on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}
