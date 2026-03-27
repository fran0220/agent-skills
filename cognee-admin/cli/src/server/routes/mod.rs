use axum::extract::{Request, State};
use axum::http::{header, StatusCode};
use axum::routing::{delete, get, post};
use axum::Router;
use std::sync::Arc;

use super::AppState;
use crate::auth;

pub mod api;
pub mod dashboard;
pub mod datasets;
pub mod graph;
pub mod login;
pub mod logs;
pub mod pipelines;
pub mod search;
pub mod settings;
pub mod tokens;

pub fn ui_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(dashboard::page))
        .route("/graph", get(graph::page))
        .route("/datasets", get(datasets::page))
        .route("/search", get(search::page))
        .route("/logs", get(logs::page))
        .route("/pipelines", get(pipelines::page))
        .route("/settings", get(settings::page))
        .route("/login", get(login::page))
        .route("/auth/login", post(login::handle_login))
        .route("/auth/logout", post(login::handle_logout))
        .route("/tokens", get(tokens::page))
}

pub fn api_routes() -> Router<Arc<AppState>> {
    Router::new()
        .route("/auth/validate", get(validate_token))
        .route("/api/health", get(dashboard::api_health))
        .route("/api/datasets", get(datasets::api_datasets))
        .route("/api/graph/{dataset_id}", get(graph::api_graph))
        .route("/api/search", get(search::api_search).post(search::api_search_post))
        .route("/api/logs", get(logs::api_logs))
        .route("/api/pipelines", get(pipelines::api_pipelines))
        .route("/api/settings", get(settings::api_get_settings).post(settings::api_save_settings))
        .route("/api/tokens", get(tokens::api_list_tokens).post(tokens::api_create_token))
        .route("/api/tokens/{id}/revoke", post(tokens::api_revoke_token))
        .route("/api/tokens/{id}", delete(tokens::api_delete_token))
}

/// GET /auth/validate — returns 200 if valid token, 401 if not (for Nginx auth_request)
async fn validate_token(State(state): State<Arc<AppState>>, request: Request) -> StatusCode {
    let token = request
        .headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "));

    match token {
        Some(t) => match auth::validate_token(&state.pool, t).await {
            Ok(_) => StatusCode::OK,
            Err(_) => StatusCode::UNAUTHORIZED,
        },
        None => StatusCode::UNAUTHORIZED,
    }
}

struct NavItem {
    path: &'static str,
    label: &'static str,
    icon: &'static str,
}

const NAV_ITEMS: &[NavItem] = &[
    NavItem { path: "/", label: "Dashboard", icon: "📊" },
    NavItem { path: "/graph", label: "Knowledge Graph", icon: "🕸️" },
    NavItem { path: "/datasets", label: "Datasets", icon: "📁" },
    NavItem { path: "/search", label: "Search", icon: "🔍" },
    NavItem { path: "/logs", label: "Request Logs", icon: "📋" },
    NavItem { path: "/pipelines", label: "Pipelines", icon: "⚙️" },
    NavItem { path: "/settings", label: "Settings", icon: "🔧" },
    NavItem { path: "/tokens", label: "Tokens", icon: "🔑" },
];

pub fn base_html(title: &str, active: &str, content: &str) -> String {
    let nav_html: String = NAV_ITEMS
        .iter()
        .map(|item| {
            let active_class = if item.path == active {
                "bg-gray-800 text-cyan-400 border-l-2 border-cyan-400"
            } else {
                "text-gray-400 hover:text-gray-200 hover:bg-gray-800/50 border-l-2 border-transparent"
            };
            format!(
                r#"<a href="{}" class="flex items-center gap-3 px-4 py-2.5 rounded-r-lg text-sm font-medium transition-all {}">{} {}</a>"#,
                item.path, active_class, item.icon, item.label
            )
        })
        .collect::<Vec<_>>()
        .join("\n            ");

    format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title} - Cognee Admin</title>
    <script src="https://cdn.tailwindcss.com"></script>
    <script src="https://unpkg.com/htmx.org@2.0.4"></script>
    <script src="https://cdn.jsdelivr.net/npm/chart.js@4"></script>
    <link rel="preconnect" href="https://fonts.googleapis.com">
    <link href="https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap" rel="stylesheet">
    <style>
        body {{ font-family: 'Inter', sans-serif; }}
        .status-green {{ color: #22c55e; }}
        .status-red {{ color: #ef4444; }}
        .status-yellow {{ color: #eab308; }}
        .status-dot {{ width: 8px; height: 8px; border-radius: 50%; display: inline-block; }}
        .status-dot.green {{ background: #22c55e; box-shadow: 0 0 6px #22c55e; }}
        .status-dot.red {{ background: #ef4444; box-shadow: 0 0 6px #ef4444; }}
        .status-dot.yellow {{ background: #eab308; box-shadow: 0 0 6px #eab308; }}
        .card {{ background: #111827; border: 1px solid #1f2937; border-radius: 0.75rem; padding: 1.5rem; }}
        .card:hover {{ border-color: #374151; }}
        .htmx-indicator {{ display: none; }}
        .htmx-request .htmx-indicator {{ display: inline-block; }}
        .htmx-request.htmx-indicator {{ display: inline-block; }}
    </style>
</head>
<body class="bg-gray-950 text-gray-100 min-h-screen">
    <div class="flex">
        <nav class="w-64 bg-gray-900 min-h-screen p-4 border-r border-gray-800 flex-shrink-0 sticky top-0 h-screen overflow-y-auto">
            <div class="mb-8 px-4">
                <h1 class="text-xl font-bold text-cyan-400">🧠 Cognee Admin</h1>
                <p class="text-xs text-gray-500 mt-1">Knowledge Engine Manager</p>
            </div>
            <div class="space-y-1">
            {nav_html}
            </div>
            <div class="mt-8 px-4 pt-4 border-t border-gray-800">
                <form method="POST" action="/auth/logout" class="mb-3">
                    <button type="submit" class="text-xs text-gray-500 hover:text-gray-300 transition-colors">🚪 Logout</button>
                </form>
                <p class="text-xs text-gray-600">cognee-admin v0.1.0</p>
            </div>
        </nav>
        <main class="flex-1 p-8 min-w-0">
            {content}
        </main>
    </div>
</body>
</html>"#
    )
}
