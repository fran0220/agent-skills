use axum::extract::{Query, State};
use axum::response::{Html, IntoResponse, Redirect, Response};
use axum::Form;
use axum::http::header::SET_COOKIE;
use serde::Deserialize;
use std::sync::Arc;

use crate::error::AppError;
use super::AppState;

#[derive(Debug, Deserialize)]
pub struct LoginForm {
    pub token: String,
}

#[derive(Debug, Deserialize)]
pub struct LoginQuery {
    pub error: Option<String>,
}

pub async fn page(Query(params): Query<LoginQuery>) -> Html<String> {
    let error_html = if params.error.as_deref() == Some("invalid") {
        r#"<div class="bg-red-900/30 border border-red-800 text-red-400 text-sm rounded-lg px-4 py-3 mb-4">
            Invalid or expired API token. Please try again.
        </div>"#
    } else {
        ""
    };

    Html(format!(
        r##"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>Login - Cognee Admin</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet">
    <style>
        body {{ font-family: 'Inter', sans-serif; }}
    </style>
</head>
<body class="bg-gray-950 text-gray-100 min-h-screen flex items-center justify-center">
    <div class="w-full max-w-sm">
        <div class="bg-gray-900 border border-gray-800 rounded-xl p-8">
            <div class="text-center mb-8">
                <h1 class="text-2xl font-bold text-cyan-400">🧠 Cognee Admin</h1>
                <p class="text-sm text-gray-500 mt-2">Enter your API token to continue</p>
            </div>

            {error_html}

            <form method="POST" action="/auth/login">
                <div class="mb-6">
                    <label for="token" class="block text-sm font-medium text-gray-400 mb-2">API Token</label>
                    <input type="password" id="token" name="token" required
                           placeholder="ca_..."
                           class="w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2.5 text-sm focus:border-cyan-400 focus:outline-none focus:ring-1 focus:ring-cyan-400/50 placeholder-gray-600">
                </div>
                <button type="submit"
                        class="w-full bg-cyan-600 hover:bg-cyan-700 text-white font-medium py-2.5 rounded-lg text-sm transition-colors">
                    Login
                </button>
            </form>
        </div>
        <p class="text-center text-xs text-gray-600 mt-6">cognee-admin v0.1.0</p>
    </div>
</body>
</html>"##
    ))
}

pub async fn handle_login(
    State(state): State<Arc<AppState>>,
    Form(data): Form<LoginForm>,
) -> Result<Response, AppError> {
    match crate::auth::validate_token(&state.pool, &data.token).await {
        Ok(_api_token) => {
            let cookie = format!(
                "ca_token={}; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age={}",
                data.token,
                30 * 24 * 60 * 60
            );
            Ok((
                [(SET_COOKIE, cookie)],
                Redirect::to("/"),
            )
                .into_response())
        }
        Err(_) => Ok(Redirect::to("/login?error=invalid").into_response()),
    }
}

pub async fn handle_logout() -> Response {
    let cookie = "ca_token=; HttpOnly; Secure; SameSite=Strict; Path=/; Max-Age=0";
    (
        [(SET_COOKIE, cookie.to_string())],
        Redirect::to("/login"),
    )
        .into_response()
}
