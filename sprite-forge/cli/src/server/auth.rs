use crate::db;
use crate::server::middleware::CurrentUser;
use crate::server::state::AppState;
use crate::types::{AuthClaims, AuthResponse, LoginRequest, MeResponse, RegisterRequest, UserRole};
use axum::{Json, extract::State, http::StatusCode};
use bcrypt::{DEFAULT_COST, hash, verify};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use std::sync::Arc;
use uuid::Uuid;

pub type ApiResult<T> = Result<T, (StatusCode, String)>;

pub async fn register(
    State(state): State<Arc<AppState>>,
    Json(request): Json<RegisterRequest>,
) -> ApiResult<Json<AuthResponse>> {
    validate_register(&request)?;

    let email = request.email.trim().to_lowercase();
    if db::get_user_by_email(&state.db, &email).await?.is_some() {
        return Err((
            StatusCode::CONFLICT,
            "Email has already been registered".to_string(),
        ));
    }

    let password = request.password.clone();
    let password_hash = tokio::task::spawn_blocking(move || hash(&password, DEFAULT_COST))
        .await
        .map_err(|err| internal_error(format!("Hash task failed: {err}")))?
        .map_err(|err| internal_error(format!("Failed to hash password: {err}")))?;

    let user = db::create_user(
        &state.db,
        &Uuid::new_v4().to_string(),
        &email,
        &password_hash,
        request.name.trim(),
        UserRole::User,
        5,
    )
    .await?;

    let token = issue_token(&state.config.jwt_secret, &user)?;
    Ok(Json(AuthResponse { token, user }))
}

pub async fn login(
    State(state): State<Arc<AppState>>,
    Json(request): Json<LoginRequest>,
) -> ApiResult<Json<AuthResponse>> {
    if request.email.trim().is_empty() || request.password.is_empty() {
        return Err((
            StatusCode::BAD_REQUEST,
            "Email and password are required".to_string(),
        ));
    }

    let email = request.email.trim().to_lowercase();
    let user_record = db::get_user_by_email(&state.db, &email)
        .await?
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()))?;

    if user_record.user.is_banned {
        return Err((StatusCode::FORBIDDEN, "User account is banned".to_string()));
    }

    let password = request.password.clone();
    let stored_hash = user_record.password_hash.clone();
    let verified = tokio::task::spawn_blocking(move || verify(&password, &stored_hash))
        .await
        .map_err(|err| internal_error(format!("Verify task failed: {err}")))?
        .map_err(|err| internal_error(format!("Failed to verify password: {err}")))?;
    if !verified {
        return Err((StatusCode::UNAUTHORIZED, "Invalid credentials".to_string()));
    }

    let token = issue_token(&state.config.jwt_secret, &user_record.user)?;
    Ok(Json(AuthResponse {
        token,
        user: user_record.user,
    }))
}

pub async fn me(
    State(state): State<Arc<AppState>>,
    current_user: CurrentUser,
) -> ApiResult<Json<MeResponse>> {
    let user = db::get_user_by_id(&state.db, &current_user.claims.sub)
        .await?
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "User not found".to_string()))?;

    Ok(Json(MeResponse { user }))
}

pub fn issue_token(secret: &str, user: &crate::types::User) -> ApiResult<String> {
    let claims = AuthClaims::new(user);
    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(secret.as_bytes()),
    )
    .map_err(|err| internal_error(format!("Failed to encode token: {err}")))
}

pub fn decode_token(secret: &str, token: &str) -> ApiResult<AuthClaims> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    decode::<AuthClaims>(
        token,
        &DecodingKey::from_secret(secret.as_bytes()),
        &validation,
    )
    .map(|data| data.claims)
    .map_err(|_| (StatusCode::UNAUTHORIZED, "Invalid token".to_string()))
}

fn validate_register(request: &RegisterRequest) -> ApiResult<()> {
    if request.email.trim().is_empty()
        || request.password.is_empty()
        || request.name.trim().is_empty()
    {
        return Err((
            StatusCode::BAD_REQUEST,
            "Email, password and name are required".to_string(),
        ));
    }

    if !request.email.contains('@') {
        return Err((
            StatusCode::BAD_REQUEST,
            "Email format is invalid".to_string(),
        ));
    }

    if request.password.len() < 6 {
        return Err((
            StatusCode::BAD_REQUEST,
            "Password must be at least 6 characters".to_string(),
        ));
    }

    Ok(())
}

fn internal_error(message: String) -> (StatusCode, String) {
    (StatusCode::INTERNAL_SERVER_ERROR, message)
}
