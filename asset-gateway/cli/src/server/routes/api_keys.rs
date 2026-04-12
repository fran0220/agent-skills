use axum::{
    extract::{Path, State},
    routing::{delete, get},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::server::routes::auth::{
    api_key_prefix, generate_api_key_value, hash_api_key, CurrentUser,
};
use crate::server::ServerState;

#[derive(Deserialize)]
struct CreateApiKeyReq {
    name: String,
    #[serde(default)]
    scopes: Vec<String>,
    expires_at: Option<String>,
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

async fn create_key(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
    Json(req): Json<CreateApiKeyReq>,
) -> AppResult<Json<Value>> {
    let name = req.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::bad_request("key name cannot be empty"));
    }

    let expires_at = parse_expires_at(req.expires_at.as_deref())?;
    let api_key = generate_api_key_value();
    let key_id = Uuid::new_v4().to_string();
    let scopes = Value::Array(req.scopes.into_iter().map(Value::String).collect());

    sqlx::query(
        "INSERT INTO api_keys (id, user_id, name, key_hash, prefix, scopes, expires_at) VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&key_id)
    .bind(&current_user.id)
    .bind(&name)
    .bind(hash_api_key(&api_key))
    .bind(api_key_prefix(&api_key))
    .bind(&scopes)
    .bind(expires_at)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(json!({
        "ok": true,
        "command": "key.create",
        "data": {
            "id": key_id,
            "name": name,
            "key": api_key,
            "prefix": api_key_prefix(&api_key),
            "scopes": scopes,
            "expires_at": expires_at.map(|value| value.to_rfc3339()),
        }
    })))
}

async fn list_keys(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
) -> AppResult<Json<Value>> {
    let rows = sqlx::query(
        "SELECT id, name, prefix, scopes, expires_at, last_used_at, created_at FROM api_keys WHERE user_id = $1 ORDER BY created_at DESC",
    )
    .bind(&current_user.id)
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    let mut keys = Vec::with_capacity(rows.len());
    for row in rows {
        keys.push(json!({
            "id": row.try_get::<String, _>("id").map_err(AppError::internal)?,
            "name": row.try_get::<String, _>("name").map_err(AppError::internal)?,
            "prefix": row.try_get::<String, _>("prefix").map_err(AppError::internal)?,
            "scopes": row.try_get::<Value, _>("scopes").map_err(AppError::internal)?,
            "expires_at": row.try_get::<Option<DateTime<Utc>>, _>("expires_at").map_err(AppError::internal)?.map(|value| value.to_rfc3339()),
            "last_used_at": row.try_get::<Option<DateTime<Utc>>, _>("last_used_at").map_err(AppError::internal)?.map(|value| value.to_rfc3339()),
            "created_at": row.try_get::<DateTime<Utc>, _>("created_at").map_err(AppError::internal)?.to_rfc3339(),
        }));
    }

    Ok(Json(json!({
        "ok": true,
        "command": "key.list",
        "data": {
            "keys": keys,
        }
    })))
}

async fn delete_key(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
    Path(id): Path<String>,
) -> AppResult<Json<Value>> {
    let result = sqlx::query("DELETE FROM api_keys WHERE id = $1 AND user_id = $2")
        .bind(&id)
        .bind(&current_user.id)
        .execute(&state.db)
        .await
        .map_err(AppError::internal)?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found(format!("api key not found: {}", id)));
    }

    Ok(Json(json!({
        "ok": true,
        "command": "key.delete",
        "data": {
            "id": id,
            "deleted": true,
        }
    })))
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/keys", get(list_keys).post(create_key))
        .route("/keys/{id}", delete(delete_key))
}
