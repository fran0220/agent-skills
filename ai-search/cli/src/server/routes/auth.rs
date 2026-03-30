use std::sync::Arc;

use axum::extract::FromRequestParts;
use axum::http::{header::AUTHORIZATION, request::Parts};

use crate::server::{ApiError, ServerState};

#[derive(Debug, Clone, Copy)]
pub struct GatewayAuth;

impl FromRequestParts<Arc<ServerState>> for GatewayAuth {
    type Rejection = ApiError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        let expected_token = state.config.read().await.gateway_token.clone();
        if expected_token.is_empty() {
            return Ok(Self);
        }

        let provided = bearer_token(parts).or_else(|| query_token(parts));
        let token = provided.ok_or_else(|| {
            ApiError::unauthorized("auth", "AUTH_REQUIRED", "missing gateway token")
        })?;

        if token == expected_token {
            return Ok(Self);
        }

        Err(ApiError::forbidden(
            "auth",
            "INVALID_TOKEN",
            "invalid gateway token",
        ))
    }
}

pub async fn auth_required(state: &Arc<ServerState>) -> bool {
    !state.config.read().await.gateway_token.is_empty()
}

pub fn ensure_non_empty_query(query: &str) -> Result<&str, ApiError> {
    let trimmed = query.trim();
    if trimmed.is_empty() {
        return Err(ApiError::bad_request(
            "search",
            "NO_QUERY",
            "query cannot be empty",
        ));
    }
    Ok(trimmed)
}

fn bearer_token(parts: &Parts) -> Option<String> {
    let raw = parts.headers.get(AUTHORIZATION)?.to_str().ok()?;
    let token = raw.strip_prefix("Bearer ")?.trim();
    if token.is_empty() {
        return None;
    }
    Some(token.to_string())
}

fn query_token(parts: &Parts) -> Option<String> {
    let query = parts.uri.query()?;
    query.split('&').find_map(|segment| {
        let mut pieces = segment.splitn(2, '=');
        let key = pieces.next()?;
        let value = pieces.next().unwrap_or_default();
        if key == "token" && !value.is_empty() {
            return Some(value.to_string());
        }
        None
    })
}
