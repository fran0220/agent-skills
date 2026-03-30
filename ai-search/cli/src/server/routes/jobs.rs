use std::sync::Arc;

use axum::{
    extract::{Query, State},
    routing::get,
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::server::routes::auth::GatewayAuth;
use crate::server::{ApiResult, ServerState};

#[derive(Debug, Deserialize)]
pub struct JobsQuery {
    pub limit: Option<usize>,
}

async fn list_jobs(
    State(state): State<Arc<ServerState>>,
    _auth: GatewayAuth,
    Query(query): Query<JobsQuery>,
) -> ApiResult<Json<Value>> {
    let limit = query.limit.unwrap_or(50).clamp(1, 200);
    let jobs = state.jobs.read().await;
    let items = jobs.iter().rev().take(limit).cloned().collect::<Vec<_>>();

    Ok(Json(json!({
        "ok": true,
        "command": "jobs.list",
        "data": {
            "jobs": items,
            "count": items.len(),
            "total": jobs.len(),
        }
    })))
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new().route("/jobs", get(list_jobs))
}
