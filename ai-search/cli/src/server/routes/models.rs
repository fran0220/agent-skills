use std::sync::Arc;

use axum::{extract::State, routing::get, Json, Router};
use serde_json::{json, Value};

use crate::config::{SEARCH_MODELS, SEARCH_MODES};
use crate::server::routes::auth::GatewayAuth;
use crate::server::{ApiResult, ServerState};

async fn list_models(
    State(state): State<Arc<ServerState>>,
    _auth: GatewayAuth,
) -> ApiResult<Json<Value>> {
    let config = state.config.read().await;

    Ok(Json(json!({
        "ok": true,
        "command": "models",
        "data": {
            "models": SEARCH_MODELS,
            "modes": SEARCH_MODES,
            "current": {
                "search_model": config.search_model,
                "analysis_model": config.analysis_model,
                "default_mode": config.default_mode,
            }
        }
    })))
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new().route("/models", get(list_models))
}
