use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use serde_json::json;
use serde_json::Value;
use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::server::routes::auth::{require_admin, CurrentUser};
use crate::server::{load_providers, ServerState};

async fn list_providers(
    State(state): State<Arc<ServerState>>,
    _current_user: CurrentUser,
) -> AppResult<Json<Value>> {
    let providers = state.registry.list().await;
    let mut items = Vec::with_capacity(providers.len());
    for provider in &providers {
        items.push(json!({
            "id": provider.id(),
            "display_name": provider.display_name(),
            "asset_types": provider.asset_types().iter().map(|t| t.as_str()).collect::<Vec<_>>(),
            "capabilities": provider.capabilities(),
        }));
    }

    Ok(Json(json!({
        "ok": true,
        "command": "provider.list",
        "data": { "providers": items }
    })))
}

async fn reload_providers(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
) -> AppResult<Json<Value>> {
    require_admin(&current_user)?;

    let new_config = {
        let current = state.config.read().await;
        current.reload().map_err(AppError::internal)?
    };

    let loaded = load_providers(&new_config, &state.registry).await;
    tracing::info!(
        loaded_provider_count = loaded,
        config_path = %new_config.config_path.display(),
        "providers reloaded from config file"
    );

    *state.config.write().await = new_config;

    let providers = state.registry.list().await;
    let ids: Vec<&str> = providers.iter().map(|p| p.id()).collect();

    Ok(Json(json!({
        "ok": true,
        "command": "provider.reload",
        "data": {
            "loaded": loaded,
            "providers": ids,
        }
    })))
}

async fn provider_health(
    State(state): State<Arc<ServerState>>,
    _current_user: CurrentUser,
    Path(id): Path<String>,
) -> AppResult<Json<Value>> {
    let provider = state.registry.get(&id).await.ok_or_else(|| {
        AppError::not_found(format!("provider not loaded: {}", id))
    })?;

    let health = provider
        .health_check()
        .await
        .map_err(|error| AppError::provider(error.to_string()))?;

    Ok(Json(json!({
        "ok": true,
        "command": "provider.health",
        "data": {
            "id": id,
            "health": health,
        }
    })))
}

async fn all_provider_health(
    State(state): State<Arc<ServerState>>,
    _current_user: CurrentUser,
) -> AppResult<Json<Value>> {
    let providers = state.registry.list().await;
    let mut health_items = Vec::with_capacity(providers.len());
    for provider in providers {
        let health = provider
            .health_check()
            .await
            .map_err(|error| AppError::provider(error.to_string()))?;
        health_items.push(json!({
            "id": provider.id(),
            "health": health,
        }));
    }

    Ok(Json(json!({
        "ok": true,
        "command": "provider.health.list",
        "data": {
            "providers": health_items,
        }
    })))
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/providers", get(list_providers))
        .route("/providers/reload", post(reload_providers))
        .route("/providers/health", get(all_provider_health))
        .route("/providers/{id}/health", get(provider_health))
}
