use argon2::{
    password_hash::{rand_core::OsRng, PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};
use axum::{
    extract::{FromRequestParts, State},
    http::{header::AUTHORIZATION, request::Parts},
    routing::post,
    Json, Router,
};
use chrono::{DateTime, Duration, Utc};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::server::ServerState;

#[derive(Deserialize)]
pub struct RegisterReq {
    pub username: String,
    pub password: String,
}

#[derive(Deserialize)]
pub struct LoginReq {
    pub username: String,
    pub password: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Claims {
    sub: String,
    username: String,
    role: String,
    exp: usize,
    iat: usize,
}

#[derive(Debug, Clone, Serialize)]
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

impl FromRequestParts<Arc<ServerState>> for CurrentUser {
    type Rejection = AppError;

    async fn from_request_parts(
        parts: &mut Parts,
        state: &Arc<ServerState>,
    ) -> Result<Self, Self::Rejection> {
        if let Some(token) = bearer_token(parts) {
            return decode_jwt_user(&token, state).await;
        }

        if let Some(api_key) = query_api_key(parts) {
            return load_user_by_api_key(&api_key, state).await;
        }

        Err(AppError::unauthorized("missing authentication token or api_key").with_suggestion(
            "Run `asset-gateway auth login` to obtain a token, or provide `api_key` in the request query.",
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

async fn decode_jwt_user(token: &str, state: &Arc<ServerState>) -> AppResult<CurrentUser> {
    let token_data = decode::<Claims>(
        token,
        &DecodingKey::from_secret(state.config.jwt_secret.as_bytes()),
        &Validation::default(),
    )
    .map_err(|_| AppError::unauthorized("invalid or expired token"))?;

    let row = sqlx::query("SELECT id, username, role, status FROM users WHERE id = $1")
        .bind(&token_data.claims.sub)
        .fetch_optional(&state.db)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::unauthorized("user no longer exists"))?;

    let status: String = row.try_get("status").map_err(AppError::internal)?;
    ensure_active_status(&status)?;

    Ok(CurrentUser {
        id: row.try_get("id").map_err(AppError::internal)?,
        username: row.try_get("username").map_err(AppError::internal)?,
        role: row.try_get("role").map_err(AppError::internal)?,
        status,
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

async fn register(
    State(state): State<Arc<ServerState>>,
    Json(req): Json<RegisterReq>,
) -> AppResult<Json<Value>> {
    let username = req.username.trim().to_string();
    if username.is_empty() {
        return Err(AppError::bad_request("username cannot be empty"));
    }
    if req.password.len() < 8 {
        return Err(AppError::bad_request(
            "password must be at least 8 characters",
        ));
    }

    let user_count: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM users")
        .fetch_one(&state.db)
        .await
        .map_err(AppError::internal)?;

    let first_user = user_count == 0;
    let role = if first_user { "admin" } else { "user" };
    let status = if first_user { "active" } else { "pending" };
    let user_id = Uuid::new_v4().to_string();
    let api_key = if first_user {
        Some(format!("agk_{}", Uuid::new_v4().simple()))
    } else {
        None
    };

    let salt = SaltString::generate(&mut OsRng);
    let password_hash = Argon2::default()
        .hash_password(req.password.as_bytes(), &salt)
        .map_err(AppError::internal)?
        .to_string();

    let insert_result = sqlx::query(
        "INSERT INTO users (id, username, password_hash, role, status, api_key, approved_at, approved_by) VALUES ($1, $2, $3, $4, $5, $6, $7, $8)",
    )
    .bind(&user_id)
    .bind(&username)
    .bind(&password_hash)
    .bind(role)
    .bind(status)
    .bind(api_key.as_deref())
    .bind(if first_user { Some(Utc::now()) } else { None })
    .bind(if first_user { Some("system-bootstrap") } else { None })
    .execute(&state.db)
    .await;

    if let Err(err) = insert_result {
        if let sqlx::Error::Database(db_err) = &err {
            if db_err.is_unique_violation() {
                return Err(AppError::conflict("username already exists"));
            }
        }
        return Err(AppError::internal(err));
    }

    Ok(Json(json!({
        "ok": true,
        "command": "auth.register",
        "data": {
            "id": user_id,
            "username": username,
            "role": role,
            "status": status,
            "api_key": api_key,
            "requires_approval": !first_user,
        }
    })))
}

async fn login(
    State(state): State<Arc<ServerState>>,
    Json(req): Json<LoginReq>,
) -> AppResult<Json<Value>> {
    let username = req.username.trim().to_string();
    if username.is_empty() {
        return Err(AppError::bad_request("username cannot be empty"));
    }

    let row = sqlx::query(
        "SELECT id, username, password_hash, role, status, api_key, api_key_expires_at, api_key_quota, api_key_quota_used FROM users WHERE username = $1",
    )
    .bind(&username)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or_else(|| AppError::unauthorized("invalid username or password"))?;

    let user_id: String = row.try_get("id").map_err(AppError::internal)?;
    let user_name: String = row.try_get("username").map_err(AppError::internal)?;
    let stored_hash: String = row.try_get("password_hash").map_err(AppError::internal)?;
    let role: String = row.try_get("role").map_err(AppError::internal)?;
    let status: String = row.try_get("status").map_err(AppError::internal)?;
    let api_key: Option<String> = row.try_get("api_key").map_err(AppError::internal)?;
    let api_key_expires_at: Option<DateTime<Utc>> = row
        .try_get("api_key_expires_at")
        .map_err(AppError::internal)?;
    let api_key_quota: Option<i64> = row.try_get("api_key_quota").map_err(AppError::internal)?;
    let api_key_quota_used: i64 = row
        .try_get("api_key_quota_used")
        .map_err(AppError::internal)?;

    let parsed_hash = PasswordHash::new(&stored_hash).map_err(AppError::internal)?;
    Argon2::default()
        .verify_password(req.password.as_bytes(), &parsed_hash)
        .map_err(|_| AppError::unauthorized("invalid username or password"))?;

    ensure_active_status(&status)?;

    let now = Utc::now();
    let expires_at = now + Duration::hours(24);
    let claims = Claims {
        sub: user_id.clone(),
        username: user_name.clone(),
        role: role.clone(),
        exp: expires_at.timestamp() as usize,
        iat: now.timestamp() as usize,
    };

    let token = encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(state.config.jwt_secret.as_bytes()),
    )
    .map_err(AppError::internal)?;

    Ok(Json(json!({
        "ok": true,
        "command": "auth.login",
        "data": {
            "token": token,
            "token_type": "Bearer",
            "expires_at": expires_at.to_rfc3339(),
            "role": role.clone(),
            "status": status.clone(),
            "api_key": api_key,
            "api_key_expires_at": api_key_expires_at.map(|v: DateTime<Utc>| v.to_rfc3339()),
            "api_key_quota": api_key_quota,
            "api_key_quota_used": api_key_quota_used,
            "user": {
                "id": user_id,
                "username": user_name,
                "role": role,
                "status": status,
            }
        }
    })))
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
}
