use axum::extract::State;
use axum::response::Html;
use std::sync::Arc;

use crate::error::AppError;
use super::AppState;
use super::base_html;

pub async fn page(State(_state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let content = r##"
        <div class="mb-8">
            <h2 class="text-2xl font-bold">Pipelines</h2>
            <p class="text-gray-400 mt-1">Pipeline execution history and status</p>
        </div>

        <div class="flex items-center gap-4 mb-6">
            <button hx-get="/api/pipelines" hx-target="#pipelines-panel" hx-swap="innerHTML"
                    class="bg-gray-800 hover:bg-gray-700 px-4 py-2 rounded-lg text-sm transition-colors border border-gray-700">
                Refresh
            </button>
        </div>

        <div id="pipelines-panel"
             hx-get="/api/pipelines"
             hx-trigger="load"
             hx-swap="innerHTML">
            <div class="text-gray-500">Loading pipeline runs...</div>
        </div>
    "##;

    Ok(Html(base_html("Pipelines", "/pipelines", content)))
}

pub async fn api_pipelines(State(state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let rows = sqlx::query_as::<_, PipelineRow>(
        "SELECT id, status, pipeline_name, created_at, updated_at, duration_ms, error_message \
         FROM public.pipeline_runs ORDER BY created_at DESC LIMIT 50"
    )
    .fetch_all(&state.pool)
    .await;

    let html = match rows {
        Ok(rows) if !rows.is_empty() => {
            let mut row_html = String::new();
            for r in &rows {
                let status_color = match r.status.as_str() {
                    "completed" | "success" => "green",
                    "running" | "processing" => "yellow",
                    "failed" | "error" => "red",
                    _ => "gray",
                };
                let name = r.pipeline_name.as_deref().unwrap_or("unnamed");
                let created = r.created_at.format("%Y-%m-%d %H:%M:%S").to_string();
                let duration = r.duration_ms
                    .map(|d| format!("{:.1}s", d as f64 / 1000.0))
                    .unwrap_or_else(|| "—".to_string());
                let error = r.error_message.as_deref().unwrap_or("");

                row_html.push_str(&format!(
                    r##"<tr class="border-b border-gray-800 hover:bg-gray-800/50">
                        <td class="py-3 px-4 font-medium">{name}</td>
                        <td class="py-3 px-4">
                            <span class="flex items-center gap-2">
                                <span class="status-dot {status_color}"></span>
                                {}
                            </span>
                        </td>
                        <td class="py-3 px-4 text-sm text-gray-400 font-mono">{created}</td>
                        <td class="py-3 px-4 text-sm">{duration}</td>
                        <td class="py-3 px-4 text-sm text-red-400 max-w-xs truncate">{}</td>
                    </tr>"##,
                    r.status,
                    html_escape(error),
                ));
            }

            format!(
                r##"<div class="card p-0 overflow-hidden">
                    <table class="w-full text-sm">
                        <thead>
                            <tr class="bg-gray-800/50 text-left text-xs uppercase text-gray-500">
                                <th class="py-3 px-4">Pipeline</th>
                                <th class="py-3 px-4">Status</th>
                                <th class="py-3 px-4">Started</th>
                                <th class="py-3 px-4">Duration</th>
                                <th class="py-3 px-4">Error</th>
                            </tr>
                        </thead>
                        <tbody>{row_html}</tbody>
                    </table>
                </div>
                <div class="mt-2 text-xs text-gray-500">{} pipeline run(s)</div>"##,
                rows.len()
            )
        }
        Ok(_) => {
            r##"<div class="card text-gray-500 text-center py-8">No pipeline runs found.</div>"##.to_string()
        }
        Err(_) => {
            r##"<div class="card border-yellow-900/50">
                <div class="flex items-center gap-3">
                    <span class="text-yellow-400 text-lg">⚠️</span>
                    <div>
                        <p class="text-yellow-400 font-medium">Pipeline table not available</p>
                        <p class="text-sm text-gray-400 mt-1">The <code class="bg-gray-800 px-1 rounded">public.pipeline_runs</code> table was not found. This is expected if Cognee has not been configured with pipeline tracking.</p>
                    </div>
                </div>
            </div>"##.to_string()
        }
    };

    Ok(Html(html))
}

#[derive(Debug, sqlx::FromRow)]
struct PipelineRow {
    #[allow(dead_code)]
    id: i64,
    status: String,
    pipeline_name: Option<String>,
    created_at: chrono::DateTime<chrono::Utc>,
    #[allow(dead_code)]
    updated_at: Option<chrono::DateTime<chrono::Utc>>,
    duration_ms: Option<i64>,
    error_message: Option<String>,
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
