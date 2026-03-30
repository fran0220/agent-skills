use axum::{extract::State, routing::get, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::server::routes::auth::{require_admin, CurrentUser};
use crate::server::{load_providers, ServerState};

#[derive(Deserialize)]
struct UpdateConfigBody {
    config: String,
}

async fn get_config(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
) -> AppResult<Json<Value>> {
    require_admin(&current_user)?;

    let config = state.config.read().await;
    let path = config.config_path.display().to_string();
    let toml_str = std::fs::read_to_string(&config.config_path).unwrap_or_default();

    Ok(Json(json!({
        "ok": true,
        "command": "config.get",
        "data": {
            "config": toml_str,
            "path": path,
        }
    })))
}

async fn update_config(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
    Json(body): Json<UpdateConfigBody>,
) -> AppResult<Json<Value>> {
    require_admin(&current_user)?;

    toml::from_str::<crate::config::ConfigFile>(&body.config)
        .map_err(|e| AppError::bad_request(format!("invalid TOML: {e}")))?;

    let config_path = {
        let current = state.config.read().await;
        current.config_path.clone()
    };

    std::fs::write(&config_path, &body.config)
        .map_err(|e| AppError::internal(format!("failed to write config: {e}")))?;

    let new_config = {
        let current = state.config.read().await;
        current.reload().map_err(AppError::internal)?
    };

    let loaded = load_providers(&new_config, &state.registry).await;
    tracing::info!(
        loaded_provider_count = loaded,
        config_path = %new_config.config_path.display(),
        "config updated and providers reloaded"
    );

    *state.config.write().await = new_config;

    let providers = state.registry.list().await;
    let ids: Vec<&str> = providers.iter().map(|p| p.id()).collect();

    Ok(Json(json!({
        "ok": true,
        "command": "config.update",
        "data": {
            "loaded": loaded,
            "providers": ids,
        }
    })))
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new().route("/config", get(get_config).put(update_config))
}
