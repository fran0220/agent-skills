use chrono::{DateTime, Utc};
use rand::Rng;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use sqlx::PgPool;
use uuid::Uuid;

use crate::error::AppError;

#[derive(Debug, Clone, Serialize, Deserialize, sqlx::FromRow)]
pub struct ApiToken {
    pub id: Uuid,
    pub token_hash: String,
    pub name: String,
    pub role: String,
    pub created_at: DateTime<Utc>,
    pub expires_at: Option<DateTime<Utc>>,
    pub enabled: bool,
    pub last_used: Option<DateTime<Utc>>,
}

/// Generate a random 32-byte token, return (raw_token, sha256_hex_hash)
pub fn generate_token() -> (String, String) {
    let mut rng = rand::thread_rng();
    let bytes: [u8; 32] = rng.gen();
    let raw = format!("ca_{}", hex::encode(bytes));
    let hash = sha256_hex(&raw);
    (raw, hash)
}

pub fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

/// Create a new token, returns the raw token (only shown once)
pub async fn create_token(
    pool: &PgPool,
    name: &str,
    role: &str,
    expires_at: Option<DateTime<Utc>>,
) -> Result<(String, ApiToken), AppError> {
    let (raw_token, hash) = generate_token();
    let token = sqlx::query_as::<_, ApiToken>(
        "INSERT INTO cognee_admin.api_tokens (token_hash, name, role, expires_at) \
         VALUES ($1, $2, $3, $4) RETURNING *",
    )
    .bind(&hash)
    .bind(name)
    .bind(role)
    .bind(expires_at)
    .fetch_one(pool)
    .await?;
    Ok((raw_token, token))
}

/// Validate a raw token, returns the ApiToken if valid
pub async fn validate_token(pool: &PgPool, raw_token: &str) -> Result<ApiToken, AppError> {
    let hash = sha256_hex(raw_token);
    let token = sqlx::query_as::<_, ApiToken>(
        "SELECT * FROM cognee_admin.api_tokens WHERE token_hash = $1 AND enabled = true",
    )
    .bind(&hash)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::Config("Invalid or disabled token".to_string()))?;

    if let Some(exp) = token.expires_at {
        if Utc::now() > exp {
            return Err(AppError::Config("Token expired".to_string()));
        }
    }

    // Update last_used (fire and forget)
    let _ = sqlx::query("UPDATE cognee_admin.api_tokens SET last_used = now() WHERE id = $1")
        .bind(token.id)
        .execute(pool)
        .await;

    Ok(token)
}

/// List all tokens (does not expose hashes)
pub async fn list_tokens(pool: &PgPool) -> Result<Vec<ApiToken>, AppError> {
    let tokens = sqlx::query_as::<_, ApiToken>(
        "SELECT * FROM cognee_admin.api_tokens ORDER BY created_at DESC",
    )
    .fetch_all(pool)
    .await?;
    Ok(tokens)
}

/// Revoke a token by ID
pub async fn revoke_token(pool: &PgPool, id: &Uuid) -> Result<(), AppError> {
    let result = sqlx::query("UPDATE cognee_admin.api_tokens SET enabled = false WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Token not found".to_string()));
    }
    Ok(())
}

/// Delete a token by ID
pub async fn delete_token(pool: &PgPool, id: &Uuid) -> Result<(), AppError> {
    let result = sqlx::query("DELETE FROM cognee_admin.api_tokens WHERE id = $1")
        .bind(id)
        .execute(pool)
        .await?;
    if result.rows_affected() == 0 {
        return Err(AppError::NotFound("Token not found".to_string()));
    }
    Ok(())
}
