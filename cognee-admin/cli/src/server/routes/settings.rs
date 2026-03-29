use axum::extract::State;
use axum::response::Html;
use axum::Extension;
use axum::Json;
use serde_json::Value;
use std::sync::Arc;

use super::{base_html, html_escape, AppState};
use crate::auth::ApiToken;
use crate::error::AppError;

const MODEL_PRESETS: &[&str] = &[
    "claude-sonnet-4-6",
    "claude-opus-4-6",
    "claude-haiku-4-5",
    "gpt-5.4",
    "gpt-5.3-codex",
    "gemini-3.1-pro-preview",
    "grok-4.1-fast",
    "glm-5",
];

pub async fn page(
    State(_state): State<Arc<AppState>>,
    Extension(current_user): Extension<ApiToken>,
) -> Result<Html<String>, AppError> {
    if current_user.role != "admin" {
        return Ok(Html(base_html(
            "Access Denied",
            "/settings",
            &access_denied_card(),
        )));
    }

    let content = r##"
        <div class="mb-8">
            <h2 class="text-2xl font-bold">Settings</h2>
            <p class="text-gray-400 mt-1">Manage LLM provider settings and inspect the raw Cognee configuration</p>
        </div>

        <div id="settings-panel"
             hx-get="/api/settings"
             hx-trigger="load"
             hx-swap="innerHTML">
            <div class="text-gray-500">Loading settings...</div>
        </div>
    "##;

    Ok(Html(base_html("Settings", "/settings", content)))
}

pub async fn api_get_settings(
    State(state): State<Arc<AppState>>,
    Extension(current_user): Extension<ApiToken>,
) -> Result<Html<String>, AppError> {
    if current_user.role != "admin" {
        return Ok(Html(access_denied_card()));
    }

    let settings = state.client.get_settings().await;

    let html = match settings {
        Ok(data) => {
            let pretty = serde_json::to_string_pretty(&data).unwrap_or_else(|_| "{}".to_string());
            let compact = json_for_script(&data);

            let llm = data.get("llm").and_then(|value| value.as_object());
            let provider = llm
                .and_then(|value| value.get("provider"))
                .and_then(|value| value.as_str())
                .unwrap_or("openai");
            let model = llm
                .and_then(|value| value.get("model"))
                .and_then(|value| value.as_str())
                .unwrap_or("");
            let api_key = llm
                .and_then(|value| value.get("api_key"))
                .and_then(|value| value.as_str())
                .unwrap_or("");
            let masked_api_key = if api_key.is_empty() {
                "Not set".to_string()
            } else {
                mask_secret(api_key)
            };
            let endpoint = std::env::var("LLM_ENDPOINT").unwrap_or_else(|_| {
                "Configured on the Cognee server via LLM_ENDPOINT; not writable through the REST API"
                    .to_string()
            });
            let model_options = MODEL_PRESETS
                .iter()
                .map(|model| format!(r#"<option value="{}"></option>"#, html_escape(model)))
                .collect::<Vec<_>>()
                .join("\n");

            format!(
                r##"<div class="grid grid-cols-1 xl:grid-cols-[1.25fr,0.75fr] gap-6">
                    <div class="card">
                        <div class="flex items-start justify-between gap-4 mb-6">
                            <div>
                                <h3 class="text-lg font-semibold">LLM Configuration</h3>
                                <p class="text-sm text-gray-400 mt-1">Update the upstream provider, model, and API key used by Cognee.</p>
                            </div>
                            <span class="text-xs uppercase tracking-[0.24em] text-cyan-400 bg-cyan-950/50 border border-cyan-900/60 rounded-full px-3 py-1">v0.2</span>
                        </div>

                        <div class="grid grid-cols-1 md:grid-cols-2 gap-4">
                            <div>
                                <label class="block text-sm text-gray-400 mb-1">Provider</label>
                                <select id="llm-provider"
                                        class="w-full bg-gray-900 border border-gray-700 rounded-lg px-4 py-2.5 text-sm focus:border-cyan-400 focus:outline-none">
                                    <option value="openai" {openai_selected}>OpenAI compatible</option>
                                    <option value="anthropic" {anthropic_selected}>Anthropic</option>
                                    <option value="ollama" {ollama_selected}>Ollama</option>
                                    <option value="gemini" {gemini_selected}>Google Gemini</option>
                                    <option value="mistral" {mistral_selected}>Mistral</option>
                                </select>
                            </div>
                            <div>
                                <label class="block text-sm text-gray-400 mb-1">Model</label>
                                <input id="llm-model"
                                       list="llm-model-presets"
                                       value="{model}"
                                       placeholder="claude-sonnet-4-6"
                                       class="w-full bg-gray-900 border border-gray-700 rounded-lg px-4 py-2.5 text-sm focus:border-cyan-400 focus:outline-none">
                                <datalist id="llm-model-presets">{model_options}</datalist>
                            </div>
                        </div>

                        <div class="mt-4">
                            <label class="block text-sm text-gray-400 mb-1">API Key</label>
                            <input id="llm-api-key"
                                   type="password"
                                   placeholder="Leave blank to keep the current key"
                                   class="w-full bg-gray-900 border border-gray-700 rounded-lg px-4 py-2.5 text-sm focus:border-cyan-400 focus:outline-none">
                            <p class="text-xs text-gray-500 mt-2">Current key: <span class="font-mono text-gray-300">{masked_api_key}</span></p>
                        </div>

                        <div class="mt-4 rounded-xl border border-gray-800 bg-gray-900/60 p-4">
                            <div class="flex items-start justify-between gap-4">
                                <div>
                                    <p class="text-sm font-medium text-gray-200">LLM Endpoint</p>
                                    <p class="text-xs text-gray-500 mt-1">Cognee does not expose endpoint changes via REST. Update <code>LLM_ENDPOINT</code> on the Cognee server if you need to switch gateways.</p>
                                </div>
                                <span class="text-xs text-gray-500">read-only</span>
                            </div>
                            <input value="{endpoint}"
                                   readonly
                                   class="mt-3 w-full bg-gray-950 border border-gray-800 rounded-lg px-4 py-2.5 text-sm text-gray-300 cursor-not-allowed">
                        </div>

                        <div class="mt-6 flex flex-wrap items-center gap-3">
                            <button type="button"
                                    onclick="saveLlmSettings()"
                                    class="bg-cyan-600 hover:bg-cyan-700 px-5 py-2.5 rounded-lg text-sm font-medium transition-colors">
                                Save LLM Settings
                            </button>
                            <p class="text-sm text-gray-500">Structured edits only touch the <code>llm</code> section.</p>
                        </div>
                    </div>

                    <div class="card">
                        <h3 class="text-lg font-semibold">Proxy Presets</h3>
                        <p class="text-sm text-gray-400 mt-1">Suggested models available on your LLM proxy.</p>
                        <div class="mt-4 flex flex-wrap gap-2">
                            <span class="text-xs px-3 py-1 rounded-full bg-gray-900 border border-gray-800 text-cyan-300">claude-sonnet-4-6</span>
                            <span class="text-xs px-3 py-1 rounded-full bg-gray-900 border border-gray-800 text-cyan-300">claude-opus-4-6</span>
                            <span class="text-xs px-3 py-1 rounded-full bg-gray-900 border border-gray-800 text-cyan-300">claude-haiku-4-5</span>
                            <span class="text-xs px-3 py-1 rounded-full bg-gray-900 border border-gray-800 text-cyan-300">gpt-5.4</span>
                            <span class="text-xs px-3 py-1 rounded-full bg-gray-900 border border-gray-800 text-cyan-300">gpt-5.3-codex</span>
                            <span class="text-xs px-3 py-1 rounded-full bg-gray-900 border border-gray-800 text-cyan-300">gemini-3.1-pro-preview</span>
                            <span class="text-xs px-3 py-1 rounded-full bg-gray-900 border border-gray-800 text-cyan-300">grok-4.1-fast</span>
                            <span class="text-xs px-3 py-1 rounded-full bg-gray-900 border border-gray-800 text-cyan-300">glm-5</span>
                        </div>
                        <div class="mt-6 rounded-xl border border-gray-800 bg-gray-900/60 p-4 text-sm text-gray-400">
                            <p class="font-medium text-gray-200">REST payload shape</p>
                            <pre class="mt-3 text-xs text-cyan-300 overflow-x-auto">{{
  "llm": {{
    "provider": "openai",
    "model": "claude-sonnet-4-6",
    "api_key": "..."
  }}
}}</pre>
                        </div>
                    </div>
                </div>

                <div id="save-feedback" class="mt-6"></div>

                <details class="card mt-6 group">
                    <summary class="cursor-pointer list-none flex items-center justify-between gap-4">
                        <div>
                            <h3 class="font-semibold">Advanced JSON Editor</h3>
                            <p class="text-sm text-gray-400 mt-1">Direct access to the full settings document returned by Cognee.</p>
                        </div>
                        <span class="text-xs text-gray-500 group-open:hidden">Expand</span>
                        <span class="text-xs text-gray-500 hidden group-open:inline">Collapse</span>
                    </summary>
                    <div class="mt-5">
                        <textarea id="settings-editor"
                                  class="w-full bg-gray-950 border border-gray-800 rounded-lg p-4 font-mono text-sm text-gray-300 focus:border-cyan-400 focus:outline-none"
                                  rows="18">{pretty}</textarea>
                        <div class="mt-4 flex items-center gap-3">
                            <button type="button"
                                    onclick="saveRawSettings()"
                                    class="bg-gray-800 hover:bg-gray-700 px-4 py-2 rounded-lg text-sm border border-gray-700 transition-colors">
                                Save Raw JSON
                            </button>
                            <p class="text-xs text-gray-500">Raw mode posts the full JSON document back to <code>/api/settings</code>.</p>
                        </div>
                    </div>
                </details>

                <script type="application/json" id="settings-current-json">{compact}</script>
                <script>
                function settingsFeedback(kind, message) {{
                    const palette = kind === 'success'
                        ? 'border-green-900/50 text-green-400'
                        : 'border-red-900/50 text-red-400';
                    return `<div class="card ${{palette}} text-sm">${{message}}</div>`;
                }}

                function currentSettings() {{
                    const source = document.getElementById('settings-current-json');
                    try {{
                        return JSON.parse(source.textContent);
                    }} catch (_) {{
                        return {{}};
                    }}
                }}

                async function postSettings(payload) {{
                    const feedback = document.getElementById('save-feedback');

                    try {{
                        const res = await fetch('/api/settings', {{
                            method: 'POST',
                            headers: {{ 'Content-Type': 'application/json' }},
                            body: JSON.stringify(payload),
                        }});
                        const data = await res.json();

                        if (res.ok) {{
                            feedback.innerHTML = settingsFeedback('success', 'Settings saved successfully.');
                            htmx.ajax('GET', '/api/settings', {{ target: '#settings-panel', swap: 'innerHTML' }});
                        }} else {{
                            feedback.innerHTML = settingsFeedback('error', data.error?.message || 'Save failed');
                        }}
                    }} catch (err) {{
                        feedback.innerHTML = settingsFeedback('error', err.message || 'Save failed');
                    }}
                }}

                async function saveLlmSettings() {{
                    const current = currentSettings();
                    const llm = current.llm || {{}};
                    const apiKeyInput = document.getElementById('llm-api-key');
                    const existingKey = llm.apiKey || llm.api_key || '';

                    await postSettings({{
                        llm: {{
                            provider: document.getElementById('llm-provider').value,
                            model: document.getElementById('llm-model').value.trim(),
                            api_key: apiKeyInput.value || existingKey,
                        }},
                    }});
                }}

                async function saveRawSettings() {{
                    const editor = document.getElementById('settings-editor');

                    try {{
                        const parsed = JSON.parse(editor.value);
                        await postSettings(parsed);
                    }} catch (err) {{
                        document.getElementById('save-feedback').innerHTML = settingsFeedback('error', 'Invalid JSON: ' + err.message);
                    }}
                }}
                </script>"##,
                openai_selected = selected_attr(provider, "openai"),
                anthropic_selected = selected_attr(provider, "anthropic"),
                ollama_selected = selected_attr(provider, "ollama"),
                gemini_selected = selected_attr(provider, "gemini"),
                mistral_selected = selected_attr(provider, "mistral"),
                model = html_escape(model),
                masked_api_key = html_escape(&masked_api_key),
                endpoint = html_escape(&endpoint),
                model_options = model_options,
                pretty = html_escape(&pretty),
                compact = compact,
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
    Extension(current_user): Extension<ApiToken>,
    Json(body): Json<Value>,
) -> Result<Json<Value>, AppError> {
    if current_user.role != "admin" {
        return Err(AppError::Config(
            "settings management requires admin privileges".into(),
        ));
    }

    let result = state.client.save_settings(body).await?;
    Ok(Json(result))
}

fn access_denied_card() -> String {
    r#"<div class="card border-red-900/50">
        <p class="text-red-400 font-semibold">403 — Access Denied</p>
        <p class="text-gray-400 text-sm mt-2">Settings management requires admin privileges.</p>
    </div>"#
        .to_string()
}

fn json_for_script(value: &Value) -> String {
    serde_json::to_string(value)
        .unwrap_or_else(|_| "{}".to_string())
        .replace("</", "<\\/")
}

fn mask_secret(secret: &str) -> String {
    if secret.len() <= 4 {
        return "•".repeat(secret.len());
    }

    format!(
        "{}{}",
        "•".repeat(secret.len() - 4),
        &secret[secret.len() - 4..]
    )
}

fn selected_attr(current: &str, candidate: &str) -> &'static str {
    if current.eq_ignore_ascii_case(candidate) {
        "selected"
    } else {
        ""
    }
}
