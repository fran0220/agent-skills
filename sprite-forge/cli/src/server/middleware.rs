use crate::server::auth;
use crate::server::state::AppState;
use crate::types::AuthClaims;
use axum::{
    extract::{FromRequestParts, Request, State},
    http::{StatusCode, header::AUTHORIZATION, request::Parts},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub claims: AuthClaims,
}

pub async fn require_auth(
    State(state): State<Arc<AppState>>,
    mut request: Request,
    next: Next,
) -> Result<Response, (StatusCode, String)> {
    let token = request
        .headers()
        .get(AUTHORIZATION)
        .and_then(|value| value.to_str().ok())
        .and_then(parse_bearer_token)
        .ok_or_else(|| {
            (
                StatusCode::UNAUTHORIZED,
                "Missing or invalid Authorization header".to_string(),
            )
        })?;

    let claims = auth::decode_token(&state.config.jwt_secret, token)?;
    request.extensions_mut().insert(CurrentUser { claims });
    Ok(next.run(request).await)
}

pub fn parse_bearer_token(header_value: &str) -> Option<&str> {
    let (scheme, token) = header_value.split_once(' ')?;
    if !scheme.eq_ignore_ascii_case("bearer") || token.trim().is_empty() {
        return None;
    }
    Some(token.trim())
}

pub fn ensure_admin(current_user: &CurrentUser) -> Result<(), (StatusCode, String)> {
    if current_user.claims.is_admin() {
        Ok(())
    } else {
        Err((StatusCode::FORBIDDEN, "Admin access required".to_string()))
    }
}

impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
{
    type Rejection = (StatusCode, String);

    async fn from_request_parts(parts: &mut Parts, _state: &S) -> Result<Self, Self::Rejection> {
        parts
            .extensions
            .get::<CurrentUser>()
            .cloned()
            .ok_or_else(|| (StatusCode::UNAUTHORIZED, "Unauthorized".to_string()))
    }
}
