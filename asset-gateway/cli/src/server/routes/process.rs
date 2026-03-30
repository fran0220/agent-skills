use axum::{extract::State, routing::post, Json, Router};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::core::pipeline::{Pipeline, ProcessRequest};
use crate::error::{AppError, AppResult};
use crate::server::routes::auth::CurrentUser;
use crate::server::ServerState;

async fn process(
    State(_state): State<Arc<ServerState>>,
    _current_user: CurrentUser,
    Json(req): Json<ProcessRequest>,
) -> AppResult<Json<Value>> {
    if req.operations.is_empty() {
        return Err(AppError::bad_request("operations array must not be empty"));
    }

    let result = Pipeline::run(&req)
        .await
        .map_err(|e| AppError::provider(e.to_string()))?;

    Ok(Json(json!({
        "ok": true,
        "command": "process",
        "data": result,
    })))
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new().route("/process", post(process))
}
