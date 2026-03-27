use axum::extract::State;
use axum::response::Html;
use axum::Json;
use serde_json::Value;
use std::sync::Arc;

use crate::error::AppError;
use super::AppState;
use super::base_html;

pub async fn page(State(_state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let content = r##"
        <div class="mb-8">
            <h2 class="text-2xl font-bold">Settings</h2>
            <p class="text-gray-400 mt-1">View and manage Cognee configuration</p>
        </div>

        <div id="settings-panel"
             hx-get="/api/settings"
             hx-trigger="load"
             hx-swap="innerHTML">
            <div class="text-gray-500">Loading settings...</div>
        </div>

        <div id="save-feedback" class="mt-4"></div>
    "##;

    Ok(Html(base_html("Settings", "/settings", content)))
}

pub async fn api_get_settings(State(state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let settings = state.client.get_settings().await;

    let html = match settings {
        Ok(data) => {
            let pretty = serde_json::to_string_pretty(&data).unwrap_or_else(|_| "{}".to_string());
            format!(
                r##"<div class="card">
                    <div class="flex items-center justify-between mb-4">
                        <h3 class="font-semibold">Current Configuration</h3>
                        <div class="flex gap-2">
                            <button onclick="enableEdit()"
                                    id="edit-btn"
                                    class="bg-gray-800 hover:bg-gray-700 px-4 py-1.5 rounded-lg text-sm transition-colors border border-gray-700">
                                ✏️ Edit
                            </button>
                            <button onclick="saveSettings()"
                                    id="save-btn"
                                    class="bg-cyan-600 hover:bg-cyan-700 px-4 py-1.5 rounded-lg text-sm font-medium transition-colors hidden">
                                💾 Save
                            </button>
                            <button onclick="cancelEdit()"
                                    id="cancel-btn"
                                    class="bg-gray-800 hover:bg-gray-700 px-4 py-1.5 rounded-lg text-sm transition-colors border border-gray-700 hidden">
                                Cancel
                            </button>
                        </div>
                    </div>
                    <textarea id="settings-editor"
                              class="w-full bg-gray-900 border border-gray-700 rounded-lg p-4 font-mono text-sm text-gray-300 focus:border-cyan-400 focus:outline-none"
                              rows="20"
                              readonly>{}</textarea>
                </div>

                <script>
                let originalSettings = '';

                function enableEdit() {{
                    const editor = document.getElementById('settings-editor');
                    originalSettings = editor.value;
                    editor.removeAttribute('readonly');
                    editor.classList.add('border-cyan-400/50');
                    document.getElementById('edit-btn').classList.add('hidden');
                    document.getElementById('save-btn').classList.remove('hidden');
                    document.getElementById('cancel-btn').classList.remove('hidden');
                }}

                function cancelEdit() {{
                    const editor = document.getElementById('settings-editor');
                    editor.value = originalSettings;
                    editor.setAttribute('readonly', '');
                    editor.classList.remove('border-cyan-400/50');
                    document.getElementById('edit-btn').classList.remove('hidden');
                    document.getElementById('save-btn').classList.add('hidden');
                    document.getElementById('cancel-btn').classList.add('hidden');
                    document.getElementById('save-feedback').innerHTML = '';
                }}

                async function saveSettings() {{
                    const editor = document.getElementById('settings-editor');
                    const feedback = document.getElementById('save-feedback');
                    try {{
                        const parsed = JSON.parse(editor.value);
                        const res = await fetch('/api/settings', {{
                            method: 'POST',
                            headers: {{ 'Content-Type': 'application/json' }},
                            body: JSON.stringify(parsed),
                        }});
                        const data = await res.json();
                        if (res.ok) {{
                            feedback.innerHTML = '<div class="card border-green-900/50 text-green-400 text-sm">✅ Settings saved successfully.</div>';
                            cancelEdit();
                            htmx.trigger(document.getElementById('settings-panel'), 'load');
                        }} else {{
                            feedback.innerHTML = '<div class="card border-red-900/50 text-red-400 text-sm">❌ ' + (data.error?.message || 'Save failed') + '</div>';
                        }}
                    }} catch (e) {{
                        feedback.innerHTML = '<div class="card border-red-900/50 text-red-400 text-sm">❌ Invalid JSON: ' + e.message + '</div>';
                    }}
                }}
                </script>"##,
                html_escape(&pretty)
            )
        }
        Err(e) => {
            format!(
                r##"<div class="card border-red-900">
                    <p class="text-red-400">Failed to load settings: {}</p>
                </div>"##,
                html_escape(&e.to_string())
            )
        }
    };

    Ok(Html(html))
}

pub async fn api_save_settings(
    State(state): State<Arc<AppState>>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    let result = state.client.save_settings(body).await?;
    Ok(Json(result))
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
