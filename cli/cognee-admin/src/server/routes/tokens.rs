use axum::extract::{Path, State};
use axum::response::Html;
use axum::Extension;
use axum::Form;
use serde::Deserialize;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::{self, ApiToken};
use crate::error::AppError;
use super::AppState;
use super::base_html;

#[derive(Debug, Deserialize)]
pub struct CreateTokenForm {
    pub name: String,
    pub role: String,
}

pub async fn page(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<ApiToken>,
) -> Result<Html<String>, AppError> {
    if current_user.role != "admin" {
        return Ok(Html(base_html(
            "Access Denied",
            "/tokens",
            r#"<div class="card border-red-900/50">
                <p class="text-red-400 font-semibold">403 — Access Denied</p>
                <p class="text-gray-400 text-sm mt-2">Token management requires admin privileges.</p>
            </div>"#,
        )));
    }

    let tokens = auth::list_tokens(&state.pool).await?;
    let table_body = render_token_rows(&tokens);

    let content = format!(
        r##"
        <div class="mb-8">
            <h2 class="text-2xl font-bold">Token Management</h2>
            <p class="text-gray-400 mt-1">Create and manage API tokens</p>
        </div>

        <div id="token-feedback" class="mb-4"></div>

        <div class="card mb-6">
            <h3 class="font-semibold mb-4">Create New Token</h3>
            <form hx-post="/api/tokens" hx-target="#token-feedback" hx-swap="innerHTML"
                  class="flex items-end gap-4">
                <div class="flex-1">
                    <label class="block text-sm text-gray-400 mb-1">Name</label>
                    <input type="text" name="name" required placeholder="e.g. CI Pipeline"
                           class="w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm focus:border-cyan-400 focus:outline-none">
                </div>
                <div class="w-40">
                    <label class="block text-sm text-gray-400 mb-1">Role</label>
                    <select name="role"
                            class="w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm focus:border-cyan-400 focus:outline-none">
                        <option value="user">User</option>
                        <option value="admin">Admin</option>
                    </select>
                </div>
                <button type="submit"
                        class="bg-cyan-600 hover:bg-cyan-700 px-6 py-2 rounded-lg text-sm font-medium transition-colors whitespace-nowrap">
                    + Create Token
                </button>
            </form>
        </div>

        <div class="card p-0 overflow-hidden">
            <table class="w-full text-sm">
                <thead>
                    <tr class="bg-gray-800/50 text-left text-xs uppercase text-gray-500">
                        <th class="py-2.5 px-4">Name</th>
                        <th class="py-2.5 px-4">Role</th>
                        <th class="py-2.5 px-4">Status</th>
                        <th class="py-2.5 px-4">Created</th>
                        <th class="py-2.5 px-4">Last Used</th>
                        <th class="py-2.5 px-4">Expires</th>
                        <th class="py-2.5 px-4">Actions</th>
                    </tr>
                </thead>
                <tbody id="tokens-table-body">
                    {table_body}
                </tbody>
            </table>
        </div>
    "##
    );

    Ok(Html(base_html("Token Management", "/tokens", &content)))
}

pub async fn api_list_tokens(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<ApiToken>,
) -> Result<Html<String>, AppError> {
    if current_user.role != "admin" {
        return Ok(Html(
            r#"<tr><td colspan="7" class="py-4 px-4 text-center text-red-400">Access denied</td></tr>"#
                .to_string(),
        ));
    }

    let tokens = auth::list_tokens(&state.pool).await?;
    Ok(Html(render_token_rows(&tokens)))
}

pub async fn api_create_token(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<ApiToken>,
    Form(data): Form<CreateTokenForm>,
) -> Result<Html<String>, AppError> {
    if current_user.role != "admin" {
        return Ok(Html(
            r#"<div class="card border-red-900/50 text-red-400 text-sm">Access denied</div>"#
                .to_string(),
        ));
    }

    let (raw_token, _api_token) =
        auth::create_token(&state.pool, &data.name, &data.role, None).await?;

    let tokens = auth::list_tokens(&state.pool).await?;
    let table_body = render_token_rows(&tokens);

    let html = format!(
        r##"<div class="card border-yellow-800/50 mb-4">
            <div class="flex items-start gap-3">
                <span class="text-yellow-400 text-xl">⚠️</span>
                <div class="flex-1">
                    <p class="text-yellow-400 font-semibold text-sm">Save this token now. It won't be shown again.</p>
                    <div class="mt-3 flex items-center gap-2">
                        <code id="new-token-value"
                              class="flex-1 bg-gray-950 border border-gray-700 rounded px-3 py-2 text-sm font-mono text-cyan-300 select-all break-all">{raw_token}</code>
                        <button onclick="copyToken()"
                                class="bg-gray-800 hover:bg-gray-700 px-3 py-2 rounded text-sm border border-gray-700 transition-colors whitespace-nowrap"
                                id="copy-token-btn">📋 Copy</button>
                    </div>
                    <p class="text-xs text-gray-500 mt-2" id="copy-status"></p>
                </div>
            </div>
        </div>
        <script>
        function copyToken() {{
            const val = document.getElementById('new-token-value').textContent;
            navigator.clipboard.writeText(val).then(() => {{
                document.getElementById('copy-status').textContent = '✓ Copied to clipboard';
                document.getElementById('copy-token-btn').textContent = '✅ Copied';
            }}).catch(() => {{
                document.getElementById('copy-status').textContent = 'Failed to copy. Please select and copy manually.';
            }});
        }}
        </script>
        <script>
            document.getElementById('tokens-table-body').innerHTML = `{}`
        </script>"##,
        table_body.replace('`', "\\`").replace("${", "\\${")
    );

    Ok(Html(html))
}

pub async fn api_revoke_token(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<ApiToken>,
    Path(id): Path<Uuid>,
) -> Result<Html<String>, AppError> {
    if current_user.role != "admin" {
        return Ok(Html(
            r#"<div class="text-red-400 text-sm">Access denied</div>"#.to_string(),
        ));
    }

    auth::revoke_token(&state.pool, &id).await?;

    let tokens = auth::list_tokens(&state.pool).await?;
    Ok(Html(render_token_rows(&tokens)))
}

pub async fn api_delete_token(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<ApiToken>,
    Path(id): Path<Uuid>,
) -> Result<Html<String>, AppError> {
    if current_user.role != "admin" {
        return Ok(Html(
            r#"<div class="text-red-400 text-sm">Access denied</div>"#.to_string(),
        ));
    }

    auth::delete_token(&state.pool, &id).await?;

    let tokens = auth::list_tokens(&state.pool).await?;
    Ok(Html(render_token_rows(&tokens)))
}

fn render_token_rows(tokens: &[ApiToken]) -> String {
    if tokens.is_empty() {
        return r#"<tr><td colspan="7" class="py-8 px-4 text-center text-gray-500">No tokens yet. Create one above.</td></tr>"#.to_string();
    }

    let mut html = String::new();
    for t in tokens {
        let role_badge = if t.role == "admin" {
            r#"<span class="bg-rose-900/50 text-rose-400 text-xs px-2 py-0.5 rounded">admin</span>"#
        } else {
            r#"<span class="bg-cyan-900/50 text-cyan-400 text-xs px-2 py-0.5 rounded">user</span>"#
        };

        let status_dot = if t.enabled {
            r#"<span class="status-dot green"></span> <span class="text-xs text-gray-400">Active</span>"#
        } else {
            r#"<span class="status-dot red"></span> <span class="text-xs text-gray-400">Revoked</span>"#
        };

        let created = t.created_at.format("%Y-%m-%d %H:%M").to_string();
        let last_used = t
            .last_used
            .map(|ts| ts.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "Never".to_string());
        let expires = t
            .expires_at
            .map(|ts| ts.format("%Y-%m-%d %H:%M").to_string())
            .unwrap_or_else(|| "Never".to_string());

        let id = t.id;
        let mut actions = String::new();
        if t.enabled {
            actions.push_str(&format!(
                r##"<button hx-post="/api/tokens/{id}/revoke"
                           hx-target="#tokens-table-body"
                           hx-swap="innerHTML"
                           hx-confirm="Revoke token '{name}'? It will no longer authenticate."
                           class="bg-yellow-900/30 hover:bg-yellow-900/50 text-yellow-400 text-xs px-2.5 py-1 rounded transition-colors">Revoke</button>"##,
                name = html_escape(&t.name)
            ));
        }
        actions.push_str(&format!(
            r##" <button hx-delete="/api/tokens/{id}"
                        hx-target="#tokens-table-body"
                        hx-swap="innerHTML"
                        hx-confirm="Permanently delete token '{name}'? This cannot be undone."
                        class="bg-red-900/30 hover:bg-red-900/50 text-red-400 text-xs px-2.5 py-1 rounded transition-colors">Delete</button>"##,
            name = html_escape(&t.name)
        ));

        html.push_str(&format!(
            r##"<tr class="border-b border-gray-800 hover:bg-gray-800/50">
                <td class="py-2.5 px-4 text-sm font-medium">{name}</td>
                <td class="py-2.5 px-4">{role_badge}</td>
                <td class="py-2.5 px-4 flex items-center gap-2">{status_dot}</td>
                <td class="py-2.5 px-4 text-xs text-gray-500 font-mono">{created}</td>
                <td class="py-2.5 px-4 text-xs text-gray-500">{last_used}</td>
                <td class="py-2.5 px-4 text-xs text-gray-500">{expires}</td>
                <td class="py-2.5 px-4">{actions}</td>
            </tr>"##,
            name = html_escape(&t.name)
        ));
    }
    html
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}
