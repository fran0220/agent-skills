use serde_json::{json, Value};
use sqlx::PgPool;
use uuid::Uuid;

use crate::auth;
use crate::error::AppError;

pub async fn create(pool: &PgPool, name: &str, role: &str) -> Result<Value, AppError> {
    let (raw_token, token) = auth::create_token(pool, name, role, None).await?;
    Ok(json!({
        "token": raw_token,
        "id": token.id.to_string(),
        "name": token.name,
        "role": token.role,
        "created_at": token.created_at.to_rfc3339(),
        "note": "Save this token — it will not be shown again."
    }))
}

pub async fn list(pool: &PgPool) -> Result<Value, AppError> {
    let tokens = auth::list_tokens(pool).await?;
    let items: Vec<Value> = tokens
        .into_iter()
        .map(|t| {
            json!({
                "id": t.id.to_string(),
                "name": t.name,
                "role": t.role,
                "enabled": t.enabled,
                "created_at": t.created_at.to_rfc3339(),
                "expires_at": t.expires_at.map(|e| e.to_rfc3339()),
                "last_used": t.last_used.map(|l| l.to_rfc3339()),
            })
        })
        .collect();
    Ok(json!({ "tokens": items }))
}

pub async fn revoke(pool: &PgPool, id: &str) -> Result<Value, AppError> {
    let uuid = Uuid::parse_str(id)
        .map_err(|e| AppError::Config(format!("Invalid UUID: {e}")))?;
    auth::revoke_token(pool, &uuid).await?;
    Ok(json!({ "revoked": id }))
}

pub async fn delete(pool: &PgPool, id: &str) -> Result<Value, AppError> {
    let uuid = Uuid::parse_str(id)
        .map_err(|e| AppError::Config(format!("Invalid UUID: {e}")))?;
    auth::delete_token(pool, &uuid).await?;
    Ok(json!({ "deleted": id }))
}
