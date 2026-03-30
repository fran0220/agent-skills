use std::sync::Arc;

use axum::body::Bytes;
use axum::{extract::State, routing::get, Json, Router};
use serde_json::{json, Value};

use crate::config::AppConfig;
use crate::search::SearchEngine;
use crate::server::routes::auth::GatewayAuth;
use crate::server::{ApiError, ApiResult, ServerState};

async fn get_config(
    State(state): State<Arc<ServerState>>,
    _auth: GatewayAuth,
) -> ApiResult<Json<Value>> {
    let config = state.config.read().await.clone();
    let raw = std::fs::read_to_string(&state.config_path)
        .map_err(|error| ApiError::internal("config.get", error.to_string()))?;

    Ok(Json(json!({
        "ok": true,
        "command": "config.get",
        "data": {
            "path": state.config_path.display().to_string(),
            "config": raw,
            "summary": summarize_config(&config),
        }
    })))
}

async fn update_config(
    State(state): State<Arc<ServerState>>,
    _auth: GatewayAuth,
    body: Bytes,
) -> ApiResult<Json<Value>> {
    let raw = decode_config_body(&body)?;
    toml::from_str::<toml::Value>(&raw).map_err(|error| {
        ApiError::bad_request(
            "config.update",
            "INVALID_TOML",
            format!("invalid TOML: {error}"),
        )
    })?;

    std::fs::write(&state.config_path, &raw)
        .map_err(|error| ApiError::internal("config.update", error.to_string()))?;

    let new_config = AppConfig::load_from(&state.config_path)
        .map_err(|error| ApiError::internal("config.update", error.to_string()))?;

    {
        let mut config = state.config.write().await;
        *config = new_config.clone();
    }

    {
        let mut engine = state.engine.write().await;
        *engine = SearchEngine::new(new_config.clone())
            .map_err(|error| ApiError::internal("config.update", error.to_string()))?;
    }

    Ok(Json(json!({
        "ok": true,
        "command": "config.update",
        "data": {
            "path": state.config_path.display().to_string(),
            "summary": summarize_config(&new_config),
        }
    })))
}

fn decode_config_body(body: &Bytes) -> ApiResult<String> {
    let text = std::str::from_utf8(body)
        .map_err(|error| ApiError::bad_request("config.update", "INVALID_BODY", error.to_string()))?
        .trim()
        .to_string();

    if text.is_empty() {
        return Err(ApiError::bad_request(
            "config.update",
            "EMPTY_CONFIG",
            "request body cannot be empty",
        ));
    }

    if text.starts_with('{') {
        let payload: Value = serde_json::from_str(&text).map_err(|error| {
            ApiError::bad_request("config.update", "INVALID_JSON", error.to_string())
        })?;
        let raw = payload
            .get("config")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                ApiError::bad_request(
                    "config.update",
                    "MISSING_CONFIG",
                    "JSON body must contain a string field named config",
                )
            })?;
        return Ok(raw.to_string());
    }

    Ok(text)
}

fn summarize_config(config: &AppConfig) -> Value {
    json!({
        "proxy_url": config.api_url,
        "search_model": config.search_model,
        "analysis_model": config.analysis_model,
        "default_mode": config.default_mode,
        "max_split": config.max_split,
        "timeout_secs": config.timeout_secs,
        "server_port": config.server_port,
        "proxy_key_configured": !config.api_key.is_empty(),
        "exa_key_configured": !config.exa_key.is_empty(),
        "tavily_key_configured": !config.tavily_key.is_empty(),
        "gateway_token_configured": !config.gateway_token.is_empty(),
    })
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new().route("/config", get(get_config).put(update_config))
}
