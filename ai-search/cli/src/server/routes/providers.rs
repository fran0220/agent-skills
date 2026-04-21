use std::sync::Arc;

use axum::{extract::State, routing::get, Json, Router};
use serde::Serialize;
use serde_json::{json, Value};

use crate::config::AppConfig;
use crate::providers::exa::ExaSearchProvider;
use crate::providers::grok::GrokSearchProvider;
use crate::providers::tavily::TavilySearchProvider;
use crate::providers::SearchProvider;
use crate::server::routes::auth::GatewayAuth;
use crate::server::{ApiResult, ServerState};

#[derive(Debug, Clone, Serialize)]
pub(crate) struct ProviderStatus {
    pub id: &'static str,
    pub name: &'static str,
    pub configured: bool,
    pub healthy: bool,
    pub details: String,
}

pub(crate) fn provider_statuses(config: &AppConfig) -> Vec<ProviderStatus> {
    let grok_ready = !config.effective_grok_key().is_empty();
    let exa_ready = !config.exa_key.is_empty();
    let tavily_ready = !config.tavily_key.is_empty();

    vec![
        ProviderStatus {
            id: "grok",
            name: "Grok",
            configured: grok_ready,
            healthy: grok_ready,
            details: if grok_ready {
                format!("endpoint: {}", config.effective_grok_url())
            } else {
                "missing grok key".to_string()
            },
        },
        ProviderStatus {
            id: "exa",
            name: "Exa",
            configured: exa_ready,
            healthy: exa_ready,
            details: if exa_ready {
                "ready".to_string()
            } else {
                "missing exa key".to_string()
            },
        },
        ProviderStatus {
            id: "tavily",
            name: "Tavily",
            configured: tavily_ready,
            healthy: tavily_ready,
            details: if tavily_ready {
                "ready".to_string()
            } else {
                "missing tavily key".to_string()
            },
        },
    ]
}

async fn list_providers(
    State(state): State<Arc<ServerState>>,
    _auth: GatewayAuth,
) -> ApiResult<Json<Value>> {
    let config = state.config.read().await;
    let providers = provider_statuses(&config);

    Ok(Json(json!({
        "ok": true,
        "command": "providers.list",
        "data": { "providers": providers }
    })))
}

async fn provider_health(
    State(state): State<Arc<ServerState>>,
    _auth: GatewayAuth,
) -> ApiResult<Json<Value>> {
    let config = state.config.read().await.clone();
    let providers = check_provider_health(&config).await;

    Ok(Json(json!({
        "ok": true,
        "command": "providers.health",
        "data": {
            "checked_at": chrono::Utc::now().to_rfc3339(),
            "providers": providers,
        }
    })))
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/providers", get(list_providers))
        .route("/providers/health", get(provider_health))
}

async fn check_provider_health(config: &AppConfig) -> Vec<ProviderStatus> {
    let mut statuses = provider_statuses(config);

    for status in &mut statuses {
        let result = match status.id {
            "grok" if status.configured => match GrokSearchProvider::new(
                config.api_url.clone(),
                config.api_key.clone(),
                config.search_model.clone(),
                config.timeout_secs,
            ) {
                Ok(provider) => provider.health_check().await,
                Err(error) => Err(error),
            },
            "exa" if status.configured => {
                match ExaSearchProvider::new(config.exa_key.clone(), config.timeout_secs) {
                    Ok(provider) => provider.health_check().await,
                    Err(error) => Err(error),
                }
            }
            "tavily" if status.configured => {
                match TavilySearchProvider::new(config.tavily_key.clone(), config.timeout_secs) {
                    Ok(provider) => provider.health_check().await,
                    Err(error) => Err(error),
                }
            }
            _ => Ok(false),
        };

        match result {
            Ok(healthy) => status.healthy = healthy,
            Err(error) => {
                status.healthy = false;
                status.details = error.to_string();
            }
        }
    }

    statuses
}
