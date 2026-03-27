use axum::extract::State;
use axum::response::Html;
use std::sync::Arc;

use crate::error::AppError;
use super::AppState;
use super::base_html;

pub async fn page(State(_state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let content = r##"
        <div class="mb-8">
            <h2 class="text-2xl font-bold">Datasets</h2>
            <p class="text-gray-400 mt-1">Manage knowledge datasets and their processing status</p>
        </div>

        <div id="datasets-panel"
             hx-get="/api/datasets"
             hx-trigger="load"
             hx-swap="innerHTML">
            <div class="text-gray-500">Loading datasets...</div>
        </div>
    "##;

    Ok(Html(base_html("Datasets", "/datasets", content)))
}

pub async fn api_datasets(State(state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let datasets = state.client.datasets().await;

    let html = match datasets {
        Ok(data) => {
            let arr = data.as_array().cloned().unwrap_or_default();
            if arr.is_empty() {
                return Ok(Html(r##"<div class="card text-gray-500 text-center py-8">No datasets found. Add data to Cognee to get started.</div>"##.to_string()));
            }

            let mut rows = String::new();
            for ds in &arr {
                let id = ds.get("id").and_then(|v| v.as_str()).unwrap_or("—");
                let name = ds.get("name").and_then(|v| v.as_str()).unwrap_or("Unnamed");
                let status = ds.get("status").and_then(|v| v.as_str()).unwrap_or("unknown");
                let doc_count = ds.get("document_count")
                    .or_else(|| ds.get("num_documents"))
                    .and_then(|v| v.as_i64())
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| "—".to_string());
                let created = ds.get("created_at")
                    .and_then(|v| v.as_str())
                    .unwrap_or("—");

                let status_color = match status {
                    "processed" | "completed" | "ready" => "green",
                    "processing" | "pending" => "yellow",
                    _ => "red",
                };

                rows.push_str(&format!(
                    r##"<tr class="border-b border-gray-800 hover:bg-gray-800/50">
                        <td class="py-3 px-4 font-medium">{name}</td>
                        <td class="py-3 px-4 text-xs text-gray-500 font-mono">{id}</td>
                        <td class="py-3 px-4">
                            <span class="flex items-center gap-2">
                                <span class="status-dot {status_color}"></span>
                                {status}
                            </span>
                        </td>
                        <td class="py-3 px-4">{doc_count}</td>
                        <td class="py-3 px-4 text-sm text-gray-500">{created}</td>
                        <td class="py-3 px-4">
                            <a href="/graph" onclick="localStorage.setItem('graph-dataset','{id}')"
                               class="text-cyan-400 hover:text-cyan-300 text-sm">View Graph →</a>
                        </td>
                    </tr>"##
                ));
            }

            format!(
                r##"<div class="card p-0 overflow-hidden">
                    <table class="w-full text-sm">
                        <thead>
                            <tr class="bg-gray-800/50 text-left text-xs uppercase text-gray-500">
                                <th class="py-3 px-4">Name</th>
                                <th class="py-3 px-4">ID</th>
                                <th class="py-3 px-4">Status</th>
                                <th class="py-3 px-4">Documents</th>
                                <th class="py-3 px-4">Created</th>
                                <th class="py-3 px-4">Actions</th>
                            </tr>
                        </thead>
                        <tbody>{rows}</tbody>
                    </table>
                </div>
                <div class="mt-4 text-sm text-gray-500">{} dataset(s) total</div>"##,
                arr.len()
            )
        }
        Err(e) => {
            format!(
                r##"<div class="card border-red-900">
                    <p class="text-red-400">Failed to load datasets: {}</p>
                </div>"##,
                html_escape(&e.to_string())
            )
        }
    };

    Ok(Html(html))
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
