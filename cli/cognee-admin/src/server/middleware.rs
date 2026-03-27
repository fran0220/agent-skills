use axum::{
    extract::{Request, State},
    http::{header, StatusCode},
    middleware::Next,
    response::Response,
};
use std::sync::Arc;

use super::AppState;
use crate::auth;

const PUBLIC_PATHS: &[&str] = &["/auth/validate", "/auth/login", "/login", "/health"];

pub async fn auth_middleware(
    State(state): State<Arc<AppState>>,
    request: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    let path = request.uri().path().to_string();

    // Public paths — no auth needed
    if PUBLIC_PATHS.iter().any(|p| path.starts_with(p)) {
        return Ok(next.run(request).await);
    }

    // Static assets
    if path.starts_with("/static/") {
        return Ok(next.run(request).await);
    }

    // Try Bearer token from Authorization header
    let bearer = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    // Try cookie fallback
    let cookie_token = if bearer.is_none() {
        request
            .headers()
            .get(header::COOKIE)
            .and_then(|v| v.to_str().ok())
            .and_then(|cookies| {
                cookies
                    .split(';')
                    .find_map(|c| c.trim().strip_prefix("ca_token="))
            })
            .map(|s| s.to_string())
    } else {
        None
    };

    let raw_token = bearer.or(cookie_token.as_deref());

    let is_browser = is_browser_request(request.headers());

    match raw_token {
        Some(t) => match auth::validate_token(&state.pool, t).await {
            Ok(api_token) => {
                let mut request = request;
                request.extensions_mut().insert(api_token);
                Ok(next.run(request).await)
            }
            Err(_) => {
                if is_browser {
                    Ok(redirect_to_login())
                } else {
                    Err(StatusCode::UNAUTHORIZED)
                }
            }
        },
        None => {
            if is_browser {
                Ok(redirect_to_login())
            } else {
                Err(StatusCode::UNAUTHORIZED)
            }
        }
    }
}

fn is_browser_request(headers: &axum::http::HeaderMap) -> bool {
    headers
        .get(header::ACCEPT)
        .and_then(|v| v.to_str().ok())
        .map(|v| v.contains("text/html"))
        .unwrap_or(false)
}

fn redirect_to_login() -> Response {
    Response::builder()
        .status(StatusCode::SEE_OTHER)
        .header("Location", "/login")
        .body(axum::body::Body::empty())
        .unwrap()
}
