use axum::extract::{Form, Query, State};
use axum::response::Html;
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;

use super::{base_html, html_escape, AppState};
use crate::error::AppError;

#[derive(Debug, Deserialize)]
pub struct DatasetViewParams {
    pub view: Option<String>,
    pub selected: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CognifyForm {
    pub dataset_name: String,
}

struct DatasetEntry {
    id: String,
    name: String,
    status: String,
    doc_count: String,
    created: String,
}

pub async fn page(State(_state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let content = r##"
        <div class="mb-8">
            <h2 class="text-2xl font-bold">Datasets</h2>
            <p class="text-gray-400 mt-1">Manage knowledge datasets and their processing status</p>
        </div>

        <div id="dataset-feedback" class="mb-4"></div>

        <div id="datasets-panel"
             hx-get="/api/datasets"
             hx-trigger="load"
             hx-swap="innerHTML">
            <div class="text-gray-500">Loading datasets...</div>
        </div>

        <div id="delete-all-modal" class="fixed inset-0 bg-gray-950/80 backdrop-blur-sm hidden items-center justify-center z-50 p-4">
            <div class="w-full max-w-lg rounded-2xl border border-red-900/50 bg-gray-900 p-6 shadow-2xl">
                <div class="flex items-start justify-between gap-4">
                    <div>
                        <h3 class="text-xl font-semibold text-red-400">Delete all datasets?</h3>
                        <p class="text-sm text-gray-400 mt-2">This will remove every dataset from Cognee. Make sure you really want a full reset before confirming.</p>
                    </div>
                    <button type="button" onclick="closeDeleteAllModal()" class="text-gray-500 hover:text-gray-300 text-xl leading-none">×</button>
                </div>
                <div class="mt-6 flex flex-wrap justify-end gap-3">
                    <button type="button"
                            onclick="closeDeleteAllModal()"
                            class="bg-gray-800 hover:bg-gray-700 px-4 py-2 rounded-lg text-sm border border-gray-700 transition-colors">
                        Cancel
                    </button>
                    <button type="button"
                            onclick="closeDeleteAllModal()"
                            hx-delete="/api/datasets/all"
                            hx-target="#dataset-feedback"
                            hx-swap="innerHTML"
                            class="bg-red-600 hover:bg-red-700 px-4 py-2 rounded-lg text-sm font-medium transition-colors">
                        Delete Everything
                    </button>
                </div>
            </div>
        </div>

        <script>
        function openDeleteAllModal() {
            const modal = document.getElementById('delete-all-modal');
            modal.classList.remove('hidden');
            modal.classList.add('flex');
        }

        function closeDeleteAllModal() {
            const modal = document.getElementById('delete-all-modal');
            modal.classList.add('hidden');
            modal.classList.remove('flex');
        }
        </script>
    "##;

    Ok(Html(base_html("Datasets", "/datasets", content)))
}

pub async fn api_datasets(
    State(state): State<Arc<AppState>>,
    Query(params): Query<DatasetViewParams>,
) -> Html<String> {
    let datasets = state.client.datasets().await;

    let html = match datasets {
        Ok(data) => {
            let entries = parse_dataset_entries(&data);
            if params.view.as_deref() == Some("options") {
                render_dataset_options(&entries, params.selected.as_deref())
            } else {
                render_dataset_panel(&entries)
            }
        }
        Err(e) => {
            if params.view.as_deref() == Some("options") {
                format!(
                    r##"<div>
                        <label class="block text-sm text-gray-400 mb-1">Dataset Filter</label>
                        <select name="dataset" disabled
                                class="w-full bg-gray-900 border border-red-900/50 rounded-lg px-4 py-2 text-sm text-red-300">
                            <option>Dataset list unavailable</option>
                        </select>
                        <p class="text-xs text-red-400 mt-2">{}</p>
                    </div>"##,
                    html_escape(&e.to_string())
                )
            } else {
                format!(
                    r##"<div class="card border-red-900">
                        <p class="text-red-400">Failed to load datasets: {}</p>
                    </div>"##,
                    html_escape(&e.to_string())
                )
            }
        }
    };

    Html(html)
}

pub async fn api_delete_all_datasets(State(state): State<Arc<AppState>>) -> Html<String> {
    let html = match state.client.delete_all_datasets().await {
        Ok(_) => r##"<div class="card border-yellow-900/50 text-yellow-300 text-sm">
                All datasets were deleted successfully.
            </div>
            <script>
                htmx.ajax('GET', '/api/datasets', { target: '#datasets-panel', swap: 'innerHTML' });
            </script>"##
            .to_string(),
        Err(err) => format!(
            r##"<div class="card border-red-900/50 text-red-400 text-sm">
                Failed to delete all datasets: {}
            </div>"##,
            html_escape(&err.to_string())
        ),
    };

    Html(html)
}

pub async fn api_cognify(
    State(state): State<Arc<AppState>>,
    Form(form): Form<CognifyForm>,
) -> Html<String> {
    let dataset_name = form.dataset_name.trim().to_string();
    if dataset_name.is_empty() {
        return Html(
            r#"<div class="card border-red-900/50 text-red-400 text-sm">Dataset name is required.</div>"#
                .to_string(),
        );
    }

    let payload = json!({
        "datasets": [dataset_name],
        "run_in_background": true,
    });

    let html = match state.client.cognify(payload).await {
        Ok(result) => {
            let run_id = result
                .get("run_id")
                .or_else(|| result.get("id"))
                .and_then(|value| value.as_str())
                .map(|value| {
                    format!(
                        r#"<p class="text-xs text-gray-500 mt-2">Run ID: <code>{}</code></p>"#,
                        html_escape(value)
                    )
                })
                .unwrap_or_default();
            format!(
                r##"<div class="card border-cyan-900/50 text-cyan-200 text-sm">
                    <p class="font-medium text-cyan-300">Cognify started in background.</p>
                    <p class="mt-1 text-gray-300">Dataset: <span class="font-medium">{}</span></p>
                    {}
                </div>"##,
                html_escape(&form.dataset_name),
                run_id
            )
        }
        Err(err) => format!(
            r##"<div class="card border-red-900/50 text-red-400 text-sm">
                Failed to start cognify for <span class="font-medium">{}</span>: {}
            </div>"##,
            html_escape(&form.dataset_name),
            html_escape(&err.to_string())
        ),
    };

    Html(html)
}

fn parse_dataset_entries(data: &Value) -> Vec<DatasetEntry> {
    data.as_array()
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .map(|dataset| DatasetEntry {
            id: dataset
                .get("id")
                .and_then(|value| value.as_str())
                .unwrap_or("—")
                .to_string(),
            name: dataset
                .get("name")
                .and_then(|value| value.as_str())
                .unwrap_or("Unnamed")
                .to_string(),
            status: dataset
                .get("status")
                .and_then(|value| value.as_str())
                .unwrap_or("unknown")
                .to_string(),
            doc_count: dataset
                .get("document_count")
                .or_else(|| dataset.get("num_documents"))
                .and_then(|value| value.as_i64())
                .map(|value| value.to_string())
                .unwrap_or_else(|| "—".to_string()),
            created: dataset
                .get("created_at")
                .and_then(|value| value.as_str())
                .unwrap_or("—")
                .to_string(),
        })
        .collect()
}

fn render_dataset_panel(entries: &[DatasetEntry]) -> String {
    let body = if entries.is_empty() {
        r##"<div class="card text-gray-500 text-center py-10">No datasets found. Upload files or add data to Cognee to get started.</div>"##
            .to_string()
    } else {
        let rows = entries
            .iter()
            .map(|dataset| {
                let status_color = match dataset.status.as_str() {
                    "processed" | "completed" | "ready" => "green",
                    "processing" | "pending" => "yellow",
                    _ => "red",
                };
                format!(
                    r##"<tr class="border-b border-gray-800 hover:bg-gray-800/50 align-top">
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
                            <div class="flex flex-wrap items-center gap-2">
                                <form hx-post="/api/cognify"
                                      hx-target="#dataset-feedback"
                                      hx-swap="innerHTML"
                                      class="inline-flex">
                                    <input type="hidden" name="dataset_name" value="{dataset_name}">
                                    <button type="submit"
                                            class="bg-cyan-600/20 hover:bg-cyan-600/30 text-cyan-300 text-xs px-3 py-1.5 rounded-md border border-cyan-900/50 transition-colors">
                                        Cognify
                                    </button>
                                </form>
                                <a href="/graph"
                                   onclick="localStorage.setItem('graph-dataset','{id_raw}')"
                                   class="text-cyan-400 hover:text-cyan-300 text-sm">View Graph →</a>
                            </div>
                        </td>
                    </tr>"##,
                    name = html_escape(&dataset.name),
                    id = html_escape(&dataset.id),
                    status = html_escape(&dataset.status),
                    doc_count = html_escape(&dataset.doc_count),
                    created = html_escape(&dataset.created),
                    dataset_name = html_escape(&dataset.name),
                    id_raw = html_escape(&dataset.id),
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

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
            </div>"##,
            rows = rows
        )
    };

    format!(
        r##"<div class="flex flex-col lg:flex-row lg:items-center lg:justify-between gap-4 mb-4">
            <div>
                <h3 class="text-lg font-semibold">Dataset Inventory</h3>
                <p class="text-sm text-gray-400 mt-1">Trigger background cognify runs or wipe the instance before a fresh import.</p>
            </div>
            <button type="button"
                    onclick="openDeleteAllModal()"
                    class="bg-red-950/40 hover:bg-red-900/50 text-red-300 px-4 py-2.5 rounded-lg text-sm font-medium border border-red-900/50 transition-colors">
                Delete All Datasets
            </button>
        </div>
        {body}
        <div class="mt-4 text-sm text-gray-500">{count} dataset(s) total</div>"##,
        body = body,
        count = entries.len()
    )
}

fn render_dataset_options(entries: &[DatasetEntry], selected: Option<&str>) -> String {
    let options = entries
        .iter()
        .map(|dataset| {
            let is_selected = selected
                .map(|value| value.eq_ignore_ascii_case(&dataset.name))
                .unwrap_or(false);
            let selected_attr = if is_selected { "selected" } else { "" };
            format!(
                r#"<option value="{}" {}>{}</option>"#,
                html_escape(&dataset.name),
                selected_attr,
                html_escape(&dataset.name)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r##"<div id="search-dataset-filter" class="flex-1">
            <label class="block text-sm text-gray-400 mb-1">Dataset Filter</label>
            <select name="dataset"
                    class="w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm focus:border-cyan-400 focus:outline-none">
                <option value="">All datasets</option>
                {options}
            </select>
        </div>"##,
        options = options
    )
}
