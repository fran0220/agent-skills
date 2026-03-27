use axum::{
    extract::{Path, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::server::routes::auth::{require_admin, CurrentUser};
use crate::server::ServerState;

#[derive(Deserialize, Default)]
struct RejectUserReq {
    reason: Option<String>,
}

#[derive(Deserialize, Default)]
struct ApiKeyConfigReq {
    quota_limit: Option<i64>,
    expires_at: Option<String>,
}

fn mask_secret(secret: &str) -> String {
    let chars: Vec<char> = secret.chars().collect();
    let len = chars.len();
    if len <= 8 {
        return "*".repeat(len.max(1));
    }

    let prefix: String = chars.iter().take(4).collect();
    let suffix: String = chars.iter().skip(len - 4).collect();
    format!("{}{}{}", prefix, "*".repeat(len - 8), suffix)
}

fn parse_expires_at(raw: Option<&str>) -> AppResult<Option<DateTime<Utc>>> {
    match raw {
        None => Ok(None),
        Some(value) => {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                return Ok(None);
            }

            DateTime::parse_from_rfc3339(trimmed)
                .map(|dt| dt.with_timezone(&Utc))
                .map(Some)
                .map_err(|_| {
                    AppError::bad_request("expires_at must be RFC3339, e.g. 2026-03-25T00:00:00Z")
                })
        }
    }
}

fn validate_quota(quota_limit: Option<i64>) -> AppResult<Option<i64>> {
    if let Some(limit) = quota_limit {
        if limit < 0 {
            return Err(AppError::bad_request("quota_limit must be >= 0"));
        }
    }
    Ok(quota_limit)
}

async fn list_users(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
) -> AppResult<Json<Value>> {
    require_admin(&current_user)?;

    let rows = sqlx::query(
        "SELECT id, username, role, status, api_key, api_key_expires_at, api_key_quota, api_key_quota_used, approved_at, approved_by, rejected_at, rejected_by, rejected_reason, created_at, updated_at FROM users ORDER BY created_at DESC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    let mut users = Vec::with_capacity(rows.len());
    for row in rows {
        let api_key: Option<String> = row.try_get("api_key").map_err(AppError::internal)?;
        users.push(json!({
            "id": row.try_get::<String, _>("id").map_err(AppError::internal)?,
            "username": row.try_get::<String, _>("username").map_err(AppError::internal)?,
            "role": row.try_get::<String, _>("role").map_err(AppError::internal)?,
            "status": row.try_get::<String, _>("status").map_err(AppError::internal)?,
            "api_key_masked": api_key.as_deref().map(mask_secret),
            "has_api_key": api_key.is_some(),
            "api_key_expires_at": row.try_get::<Option<DateTime<Utc>>, _>("api_key_expires_at").map_err(AppError::internal)?.map(|v| v.to_rfc3339()),
            "api_key_quota": row.try_get::<Option<i64>, _>("api_key_quota").map_err(AppError::internal)?,
            "api_key_quota_used": row.try_get::<i64, _>("api_key_quota_used").map_err(AppError::internal)?,
            "approved_at": row.try_get::<Option<DateTime<Utc>>, _>("approved_at").map_err(AppError::internal)?.map(|v| v.to_rfc3339()),
            "approved_by": row.try_get::<Option<String>, _>("approved_by").map_err(AppError::internal)?,
            "rejected_at": row.try_get::<Option<DateTime<Utc>>, _>("rejected_at").map_err(AppError::internal)?.map(|v| v.to_rfc3339()),
            "rejected_by": row.try_get::<Option<String>, _>("rejected_by").map_err(AppError::internal)?,
            "rejected_reason": row.try_get::<Option<String>, _>("rejected_reason").map_err(AppError::internal)?,
            "created_at": row.try_get::<DateTime<Utc>, _>("created_at").map_err(AppError::internal)?.to_rfc3339(),
            "updated_at": row.try_get::<DateTime<Utc>, _>("updated_at").map_err(AppError::internal)?.to_rfc3339(),
        }));
    }

    Ok(Json(json!({
        "ok": true,
        "command": "user.list",
        "data": {
            "users": users,
        }
    })))
}

async fn approve_user(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
    Path(id): Path<String>,
) -> AppResult<Json<Value>> {
    require_admin(&current_user)?;

    let result = sqlx::query(
        "UPDATE users SET status = 'active', approved_at = now(), approved_by = $1, rejected_at = NULL, rejected_by = NULL, rejected_reason = NULL, updated_at = now() WHERE id = $2",
    )
    .bind(&current_user.username)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found(format!("user not found: {}", id)));
    }

    Ok(Json(json!({
        "ok": true,
        "command": "user.approve",
        "data": {
            "id": id,
            "status": "active",
        }
    })))
}

async fn reject_user(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
    Path(id): Path<String>,
    Json(req): Json<RejectUserReq>,
) -> AppResult<Json<Value>> {
    require_admin(&current_user)?;

    let reason = req
        .reason
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("rejected by admin")
        .to_string();

    let result = sqlx::query(
        "UPDATE users SET status = 'rejected', api_key = NULL, api_key_expires_at = NULL, api_key_quota = NULL, api_key_quota_used = 0, rejected_at = now(), rejected_by = $1, rejected_reason = $2, updated_at = now() WHERE id = $3",
    )
    .bind(&current_user.username)
    .bind(&reason)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found(format!("user not found: {}", id)));
    }

    Ok(Json(json!({
        "ok": true,
        "command": "user.reject",
        "data": {
            "id": id,
            "status": "rejected",
            "reason": reason,
        }
    })))
}

async fn generate_api_key(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
    Path(id): Path<String>,
    Json(req): Json<ApiKeyConfigReq>,
) -> AppResult<Json<Value>> {
    require_admin(&current_user)?;

    let quota_limit = validate_quota(req.quota_limit)?;
    let expires_at = parse_expires_at(req.expires_at.as_deref())?;

    let status_row = sqlx::query("SELECT status FROM users WHERE id = $1")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::not_found(format!("user not found: {}", id)))?;

    let status: String = status_row.try_get("status").map_err(AppError::internal)?;
    if !status.eq_ignore_ascii_case("active") {
        return Err(AppError::conflict(
            "api_key can only be generated for active users",
        ));
    }

    let api_key = format!("agk_{}", Uuid::new_v4().simple());

    sqlx::query(
        "UPDATE users SET api_key = $1, api_key_expires_at = $2, api_key_quota = $3, api_key_quota_used = 0, updated_at = now() WHERE id = $4",
    )
    .bind(&api_key)
    .bind(expires_at)
    .bind(quota_limit)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(json!({
        "ok": true,
        "command": "user.api_key.generate",
        "data": {
            "id": id,
            "api_key": api_key,
            "api_key_masked": mask_secret(&api_key),
            "api_key_expires_at": expires_at.map(|v| v.to_rfc3339()),
            "api_key_quota": quota_limit,
            "api_key_quota_used": 0,
        }
    })))
}

async fn update_api_key_policy(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
    Path(id): Path<String>,
    Json(req): Json<ApiKeyConfigReq>,
) -> AppResult<Json<Value>> {
    require_admin(&current_user)?;

    let quota_limit = validate_quota(req.quota_limit)?;
    let expires_at = parse_expires_at(req.expires_at.as_deref())?;

    let key_row = sqlx::query("SELECT api_key FROM users WHERE id = $1")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::not_found(format!("user not found: {}", id)))?;

    let has_key: Option<String> = key_row.try_get("api_key").map_err(AppError::internal)?;
    if has_key.is_none() {
        return Err(AppError::conflict("user has no api_key to update"));
    }

    sqlx::query(
        "UPDATE users SET api_key_expires_at = $1, api_key_quota = $2, updated_at = now() WHERE id = $3",
    )
    .bind(expires_at)
    .bind(quota_limit)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(json!({
        "ok": true,
        "command": "user.api_key.update",
        "data": {
            "id": id,
            "api_key_expires_at": expires_at.map(|v| v.to_rfc3339()),
            "api_key_quota": quota_limit,
        }
    })))
}

async fn revoke_api_key(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
    Path(id): Path<String>,
) -> AppResult<Json<Value>> {
    require_admin(&current_user)?;

    let key_row = sqlx::query("SELECT api_key FROM users WHERE id = $1")
        .bind(&id)
        .fetch_optional(&state.db)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::not_found(format!("user not found: {}", id)))?;

    let had_key: Option<String> = key_row.try_get("api_key").map_err(AppError::internal)?;

    sqlx::query(
        "UPDATE users SET api_key = NULL, api_key_expires_at = NULL, api_key_quota = NULL, api_key_quota_used = 0, updated_at = now() WHERE id = $1",
    )
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(json!({
        "ok": true,
        "command": "user.api_key.revoke",
        "data": {
            "id": id,
            "revoked": had_key.is_some(),
        }
    })))
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/users", get(list_users))
        .route("/users/{id}/approve", post(approve_user))
        .route("/users/{id}/reject", post(reject_user))
        .route(
            "/users/{id}/api-key",
            post(generate_api_key)
                .put(update_api_key_policy)
                .delete(revoke_api_key),
        )
}
