use axum::{
    extract::{FromRequestParts, State},
    http::{header::AUTHORIZATION, request::Parts},
    routing::post,
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::server::ServerState;

#[derive(Debug, Clone, serde::Serialize)]
pub struct CurrentUser {
    pub id: String,
    pub username: String,
    pub role: String,
    pub status: String,
}

impl CurrentUser {
    pub fn is_admin(&self) -> bool {
        self.role.eq_ignore_ascii_case("admin")
    }
}

pub fn require_admin(user: &CurrentUser) -> AppResult<()> {
    if user.is_admin() {
        return Ok(());
    }
    Err(AppError::forbidden(
        "admin role is required for this operation",
    ))
}

fn ensure_active_status(status: &str) -> AppResult<()> {
    if status.eq_ignore_ascii_case("active") {
        return Ok(());
    }

    let message = if status.eq_ignore_ascii_case("pending") {
        "account is pending admin approval"
    } else if status.eq_ignore_ascii_case("rejected") {
        "account registration was rejected"
    } else {
        "account is not active"
    };

    Err(AppError::forbidden(message))
}

pub(crate) async fn authenticate_token(
    token: &str,
    state: &Arc<ServerState>,
) -> AppResult<CurrentUser> {
    let admin_token = state.config.read().await.admin_token.clone();
    if !admin_token.is_empty() && token == admin_token {
        return Ok(CurrentUser {
            id: "admin".into(),
            username: "admin".into(),
            role: "admin".into(),
            status: "active".into(),
        });
    }
    load_user_by_api_key(token, state).await
}

impl FromRequestParts<Arc<ServerState>> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        if let Some(token) = bearer_token(parts) {
            return authenticate_token(&token, state).await;
        }

        if let Some(api_key) = query_api_key(parts) {
            return authenticate_token(&api_key, state).await;
        }

        Err(AppError::unauthorized("missing authentication token").with_suggestion(
            "Provide a Bearer token in Authorization header, or api_key query parameter.",
        ))
    }
}

fn bearer_token(parts: &Parts) -> Option<String> {
    let value = parts.headers.get(AUTHORIZATION)?.to_str().ok()?;
    let token = value.strip_prefix("Bearer ")?;
    if token.is_empty() {
        return None;
    }
    Some(token.to_string())
}

fn query_api_key(parts: &Parts) -> Option<String> {
    let query = parts.uri.query()?;
    query.split('&').find_map(|segment| {
        let mut pieces = segment.splitn(2, '=');
        let key = pieces.next()?;
        let value = pieces.next().unwrap_or_default();
        if key == "api_key" && !value.is_empty() {
            return Some(value.to_string());
        }
        None
    })
}

async fn load_user_by_api_key(api_key: &str, state: &Arc<ServerState>) -> AppResult<CurrentUser> {
    let row = sqlx::query(
        "SELECT id, username, role, status, api_key_expires_at, api_key_quota, api_key_quota_used FROM users WHERE api_key = $1",
    )
    .bind(api_key)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or_else(|| AppError::unauthorized("invalid api_key"))?;

    let status: String = row.try_get("status").map_err(AppError::internal)?;
    ensure_active_status(&status)?;

    let api_key_expires_at: Option<DateTime<Utc>> = row
        .try_get("api_key_expires_at")
        .map_err(AppError::internal)?;
    if let Some(expires_at) = api_key_expires_at {
        if expires_at < Utc::now() {
            return Err(AppError::unauthorized("api_key is expired"));
        }
    }

    let quota_limit: Option<i64> = row.try_get("api_key_quota").map_err(AppError::internal)?;
    let quota_used: i64 = row
        .try_get("api_key_quota_used")
        .map_err(AppError::internal)?;
    if let Some(limit) = quota_limit {
        if quota_used >= limit {
            return Err(AppError::forbidden("api_key quota exceeded"));
        }
    }

    Ok(CurrentUser {
        id: row.try_get("id").map_err(AppError::internal)?,
        username: row.try_get("username").map_err(AppError::internal)?,
        role: row.try_get("role").map_err(AppError::internal)?,
        status,
    })
}

#[derive(Deserialize)]
pub struct TokenLoginReq {
    pub token: String,
}

async fn login(
    State(state): State<Arc<ServerState>>,
    Json(req): Json<TokenLoginReq>,
) -> AppResult<Json<Value>> {
    let token = req.token.trim().to_string();
    if token.is_empty() {
        return Err(AppError::bad_request("token cannot be empty"));
    }

    let user = authenticate_token(&token, &state).await?;

    let (api_key_expires_at, api_key_quota, api_key_quota_used) = if user.id != "admin" {
        let row = sqlx::query(
            "SELECT api_key_expires_at, api_key_quota, api_key_quota_used FROM users WHERE id = $1",
        )
        .bind(&user.id)
        .fetch_optional(&state.db)
        .await
        .map_err(AppError::internal)?;

        match row {
            Some(r) => {
                let expires: Option<DateTime<Utc>> =
                    r.try_get("api_key_expires_at").map_err(AppError::internal)?;
                let quota: Option<i64> =
                    r.try_get("api_key_quota").map_err(AppError::internal)?;
                let used: i64 = r.try_get("api_key_quota_used").map_err(AppError::internal)?;
                (expires.map(|v| v.to_rfc3339()), quota, used)
            }
            None => (None, None, 0),
        }
    } else {
        (None, None, 0)
    };

    Ok(Json(json!({
        "ok": true,
        "command": "auth.login",
        "data": {
            "token": token,
            "token_type": "Bearer",
            "expires_at": api_key_expires_at,
            "role": user.role,
            "status": user.status,
            "api_key_quota": api_key_quota,
            "api_key_quota_used": api_key_quota_used,
            "user": {
                "id": user.id,
                "username": user.username,
                "role": user.role,
                "status": user.status,
            }
        }
    })))
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new().route("/login", post(login))
}
