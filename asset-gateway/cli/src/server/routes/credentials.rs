use axum::{
    extract::{Path, State},
    routing::{delete, get},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::collections::BTreeSet;
use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::server::routes::auth::{require_admin, CurrentUser};
use crate::server::{list_enabled_provider_ids, reload_provider_by_id, ServerState};

async fn list_credentials(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
) -> AppResult<Json<Value>> {
    require_admin(&current_user)?;

    let rows = sqlx::query(
        "SELECT key, encrypted_value, nonce, provider_id, description FROM credentials ORDER BY key ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    let mut credentials = Vec::with_capacity(rows.len());
    for row in rows {
        let key: String = row.try_get("key").map_err(AppError::internal)?;
        let encrypted_value: Vec<u8> =
            row.try_get("encrypted_value").map_err(AppError::internal)?;
        let nonce: Vec<u8> = row.try_get("nonce").map_err(AppError::internal)?;
        let provider_id: Option<String> = row.try_get("provider_id").map_err(AppError::internal)?;
        let description: Option<String> = row.try_get("description").map_err(AppError::internal)?;

        let decrypted = state
            .vault
            .decrypt(&encrypted_value, &nonce)
            .map_err(AppError::internal)?;

        credentials.push(json!({
            "key": key,
            "provider_id": provider_id,
            "description": description,
            "value_masked": mask_secret(&decrypted),
        }));
    }

    Ok(Json(json!({
        "ok": true,
        "command": "credential.list",
        "data": { "credentials": credentials }
    })))
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

#[derive(Deserialize)]
pub struct SetCredentialReq {
    pub key: String,
    pub value: String,
    pub provider_id: Option<String>,
    pub description: Option<String>,
}

fn normalize_optional_string(input: Option<String>) -> Option<String> {
    input
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
}

async fn affected_provider_ids(
    db: &sqlx::PgPool,
    previous_provider_id: Option<&str>,
    updated_provider_id: Option<&str>,
) -> AppResult<Vec<String>> {
    // Global credentials can impact every enabled provider, so reload all.
    if previous_provider_id.is_none() || updated_provider_id.is_none() {
        return list_enabled_provider_ids(db)
            .await
            .map_err(AppError::internal);
    }

    let mut ids = BTreeSet::new();
    if let Some(provider_id) = previous_provider_id {
        ids.insert(provider_id.to_string());
    }
    if let Some(provider_id) = updated_provider_id {
        ids.insert(provider_id.to_string());
    }
    Ok(ids.into_iter().collect())
}

async fn set_credential(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
    Json(req): Json<SetCredentialReq>,
) -> AppResult<Json<Value>> {
    require_admin(&current_user)?;

    let key = req.key.trim();
    if key.is_empty() {
        return Err(AppError::bad_request("credential key cannot be empty"));
    }
    if req.value.is_empty() {
        return Err(AppError::bad_request("credential value cannot be empty"));
    }

    let provider_id = normalize_optional_string(req.provider_id);
    let description = normalize_optional_string(req.description);

    let previous_provider_id: Option<String> =
        sqlx::query_scalar("SELECT provider_id FROM credentials WHERE key = $1")
            .bind(key)
            .fetch_optional(&state.db)
            .await
            .map_err(AppError::internal)?
            .flatten();

    let (encrypted_value, nonce) = state
        .vault
        .encrypt(&req.value)
        .map_err(AppError::internal)?;

    sqlx::query(
        "INSERT INTO credentials (key, encrypted_value, nonce, provider_id, description) VALUES ($1, $2, $3, $4, $5) \
         ON CONFLICT(key) DO UPDATE SET encrypted_value = excluded.encrypted_value, nonce = excluded.nonce, provider_id = excluded.provider_id, description = excluded.description, updated_at = now()",
    )
    .bind(key)
    .bind(encrypted_value)
    .bind(nonce)
    .bind(provider_id.as_deref())
    .bind(description.as_deref())
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    let impacted_provider_ids = affected_provider_ids(
        &state.db,
        previous_provider_id.as_deref(),
        provider_id.as_deref(),
    )
    .await?;

    let mut reloaded = Vec::new();
    let mut reload_failures = Vec::new();
    for impacted_id in &impacted_provider_ids {
        match reload_provider_by_id(&state.db, &state.vault, &state.registry, impacted_id).await {
            Ok(()) => reloaded.push(impacted_id.clone()),
            Err(error) => {
                reload_failures.push(json!({
                    "provider_id": impacted_id,
                    "error": error.to_string(),
                }));
            }
        }
    }

    Ok(Json(json!({
        "ok": true,
        "command": "credential.set",
        "data": {
            "key": key,
            "provider_id": provider_id,
            "message": "credential stored",
            "reloaded_providers": reloaded,
            "reload_failures": reload_failures,
        }
    })))
}

async fn delete_credential(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
    Path(key): Path<String>,
) -> AppResult<Json<Value>> {
    require_admin(&current_user)?;

    let result = sqlx::query("DELETE FROM credentials WHERE key = $1")
        .bind(&key)
        .execute(&state.db)
        .await
        .map_err(AppError::internal)?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found(format!(
            "credential not found: {}",
            key
        )));
    }

    Ok(Json(json!({
        "ok": true,
        "command": "credential.delete",
        "data": {
            "key": key,
            "message": "credential deleted",
        }
    })))
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/credentials", get(list_credentials).put(set_credential))
        .route("/credentials/{key}", delete(delete_credential))
}
