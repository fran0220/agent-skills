use std::str::FromStr;
use std::sync::Arc;
use std::time::Instant;

use axum::{extract::State, routing::post, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use uuid::Uuid;

use crate::search::SearchMode;
use crate::server::routes::auth::{ensure_non_empty_query, GatewayAuth};
use crate::server::{ApiError, ApiResult, JobRecord, ServerState};

#[derive(Debug, Deserialize)]
pub struct SearchRequest {
    pub query: String,
    pub mode: Option<String>,
    pub model: Option<String>,
    pub split: Option<u32>,
    #[allow(dead_code)]
    pub num: Option<u32>,
}

async fn search(
    State(state): State<Arc<ServerState>>,
    _auth: GatewayAuth,
    Json(req): Json<SearchRequest>,
) -> ApiResult<Json<Value>> {
    let query = ensure_non_empty_query(&req.query)?;

    let config = state.config.read().await.clone();
    let requested_mode = req
        .mode
        .clone()
        .unwrap_or_else(|| config.default_mode.clone());
    let mode = parse_mode(&requested_mode)?;
    let split = req.split.unwrap_or(config.max_split).max(1);
    let started_at = chrono::Utc::now().to_rfc3339();
    let timer = Instant::now();

    let result = {
        let engine = state.engine.read().await;
        engine
            .search(query, mode, req.model.as_deref(), split)
            .await
    };

    match result {
        Ok(response) => {
            state
                .push_job(JobRecord {
                    id: Uuid::new_v4().to_string(),
                    query: query.to_string(),
                    mode: response.mode.to_string(),
                    model: response.model.clone(),
                    sources_used: response.providers.clone(),
                    tokens: response.tokens,
                    duration_ms: timer.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                    result_count: response.results.len(),
                    status: "completed".to_string(),
                    error_message: None,
                    created_at: started_at,
                })
                .await;

            let data = serde_json::to_value(&response)
                .map_err(|error| ApiError::internal("search", error.to_string()))?;

            Ok(Json(json!({
                "ok": true,
                "command": "search",
                "data": data,
            })))
        }
        Err(error) => {
            state
                .push_job(JobRecord {
                    id: Uuid::new_v4().to_string(),
                    query: query.to_string(),
                    mode: requested_mode,
                    model: req
                        .model
                        .clone()
                        .unwrap_or_else(|| config.search_model.clone()),
                    sources_used: Vec::new(),
                    tokens: 0,
                    duration_ms: timer.elapsed().as_millis().min(u128::from(u64::MAX)) as u64,
                    result_count: 0,
                    status: "failed".to_string(),
                    error_message: Some(error.to_string()),
                    created_at: started_at,
                })
                .await;

            Err(ApiError::bad_gateway(
                "search",
                "SEARCH_FAILED",
                error.to_string(),
            ))
        }
    }
}

fn parse_mode(mode: &str) -> ApiResult<SearchMode> {
    SearchMode::from_str(mode)
        .map_err(|error| ApiError::bad_request("search", "INVALID_MODE", error.to_string()))
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new().route("/search", post(search))
}
