use std::sync::Arc;

use axum::{extract::State, Json};
use serde_json::{json, Value};

use crate::server::routes::auth::auth_required;
use crate::server::routes::providers::provider_statuses;
use crate::server::ServerState;

pub async fn health_check(State(state): State<Arc<ServerState>>) -> Json<Value> {
    let config = state.config.read().await.clone();
    let providers = provider_statuses(&config);

    Json(json!({
        "ok": true,
        "command": "health",
        "data": {
            "status": "ok",
            "service": "ai-search",
            "version": env!("CARGO_PKG_VERSION"),
            "auth_required": auth_required(&state).await,
            "providers_configured": providers.iter().filter(|provider| provider.configured).count(),
        }
    }))
}
