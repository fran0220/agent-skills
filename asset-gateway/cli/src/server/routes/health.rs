use crate::server::ServerState;
use axum::{extract::State, Json};
use serde_json::{json, Value};
use std::sync::Arc;

pub async fn health_check(State(_state): State<Arc<ServerState>>) -> Json<Value> {
    Json(json!({
        "ok": true,
        "command": "health",
        "data": {
            "status": "healthy",
            "version": env!("CARGO_PKG_VERSION"),
        }
    }))
}
