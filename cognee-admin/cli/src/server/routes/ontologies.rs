use axum::extract::{Multipart, State};
use axum::response::Html;
use serde_json::Value;
use std::sync::Arc;

use super::{base_html, html_escape, persist_field_to_temp_file, AppState, TempUpload};
use crate::error::AppError;

pub async fn page(State(_state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let content = r##"
        <div class="mb-8">
            <h2 class="text-2xl font-bold">Ontologies</h2>
            <p class="text-gray-400 mt-1">Register OWL ontologies for structured graph construction and prompt grounding.</p>
        </div>

        <div id="ontology-feedback" class="mb-4"></div>

        <div class="card mb-6">
            <form hx-post="/api/ontologies"
                  hx-encoding="multipart/form-data"
                  hx-target="#ontology-feedback"
                  hx-swap="innerHTML"
                  class="grid grid-cols-1 lg:grid-cols-[1fr,1fr,auto] gap-4 items-end">
                <div>
                    <label class="block text-sm text-gray-400 mb-1">Ontology Key</label>
                    <input type="text"
                           name="ontology_key"
                           placeholder="games-taxonomy"
                           class="w-full bg-gray-900 border border-gray-700 rounded-lg px-4 py-2.5 text-sm focus:border-cyan-400 focus:outline-none"
                           required>
                </div>
                <div>
                    <label class="block text-sm text-gray-400 mb-1">OWL File</label>
                    <input type="file"
                           name="ontology_file"
                           accept=".owl,.rdf,.xml"
                           class="w-full bg-gray-900 border border-gray-700 rounded-lg px-4 py-2 text-sm file:mr-4 file:border-0 file:bg-cyan-600 file:px-3 file:py-2 file:text-sm file:font-medium file:text-white hover:file:bg-cyan-700"
                           required>
                </div>
                <button type="submit"
                        class="bg-cyan-600 hover:bg-cyan-700 px-5 py-2.5 rounded-lg text-sm font-medium transition-colors">
                    Upload Ontology
                </button>
            </form>
        </div>

        <div id="ontology-panel"
             hx-get="/api/ontologies"
             hx-trigger="load"
             hx-swap="innerHTML">
            <div class="text-gray-500">Loading ontologies...</div>
        </div>
    "##;

    Ok(Html(base_html("Ontologies", "/ontologies", content)))
}

pub async fn api_list(State(state): State<Arc<AppState>>) -> Html<String> {
    let html = match state.client.list_ontologies().await {
        Ok(data) => render_ontology_list(&data),
        Err(err) => format!(
            r##"<div class="card border-red-900/50 text-red-400 text-sm">
                Failed to load ontologies: {}
            </div>"##,
            html_escape(&err.to_string())
        ),
    };

    Html(html)
}

pub async fn api_upload(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Html<String> {
    let mut ontology_key: Option<String> = None;
    let mut upload: Option<TempUpload> = None;
    let mut errors = Vec::new();

    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                let field_name = field.name().unwrap_or_default().to_string();
                match field_name.as_str() {
                    "ontology_key" => match field.text().await {
                        Ok(value) => ontology_key = Some(value),
                        Err(err) => errors.push(format!("Failed to read ontology key: {}", err)),
                    },
                    "ontology_file" => match persist_field_to_temp_file(field, "ontology").await {
                        Ok(saved_upload) => upload = Some(saved_upload),
                        Err(err) => errors.push(err.to_string()),
                    },
                    _ => {}
                }
            }
            Ok(None) => break,
            Err(err) => {
                errors.push(format!("Failed to read ontology upload: {}", err));
                break;
            }
        }
    }

    let html = match handle_upload(&state, ontology_key, upload.as_ref(), errors).await {
        Ok(html) => html,
        Err(err) => format!(
            r##"<div class="card border-red-900/50 text-red-400 text-sm">
                Ontology upload failed: {}
            </div>"##,
            html_escape(&err.to_string())
        ),
    };

    if let Some(upload) = upload {
        let _ = tokio::fs::remove_file(upload.path).await;
    }

    Html(html)
}

async fn handle_upload(
    state: &Arc<AppState>,
    ontology_key: Option<String>,
    upload: Option<&TempUpload>,
    mut errors: Vec<String>,
) -> Result<String, AppError> {
    let ontology_key = ontology_key
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Config("Ontology key is required".into()))?;

    let Some(upload) = upload else {
        if errors.is_empty() {
            errors.push("Select an ontology file to upload.".to_string());
        }
        return Ok(render_feedback(None, &errors));
    };

    let upload_result = state
        .client
        .upload_ontology(&ontology_key, &upload.path)
        .await;
    match upload_result {
        Ok(_) => Ok(render_feedback(
            Some((&ontology_key, &upload.filename)),
            &errors,
        )),
        Err(err) => {
            errors.push(err.to_string());
            Ok(render_feedback(None, &errors))
        }
    }
}

fn render_feedback(success: Option<(&str, &str)>, errors: &[String]) -> String {
    let mut blocks = Vec::new();

    if let Some((ontology_key, filename)) = success {
        blocks.push(format!(
            r##"<div class="card border-green-900/50 text-green-300 text-sm mb-4">
                Uploaded ontology <span class="font-medium text-green-400">{}</span> from <span class="font-medium text-gray-200">{}</span>.
            </div>
            <script>
                htmx.ajax('GET', '/api/ontologies', {{ target: '#ontology-panel', swap: 'innerHTML' }});
            </script>"##,
            html_escape(ontology_key),
            html_escape(filename)
        ));
    }

    if !errors.is_empty() {
        let items = errors
            .iter()
            .map(|error| format!(r#"<li>{}</li>"#, html_escape(error)))
            .collect::<Vec<_>>()
            .join("\n");
        blocks.push(format!(
            r##"<div class="card border-red-900/50 text-red-300 text-sm">
                <p class="font-semibold text-red-400">Ontology upload issues</p>
                <ul class="mt-3 list-disc list-inside space-y-1">{}</ul>
            </div>"##,
            items
        ));
    }

    if blocks.is_empty() {
        blocks.push(
            r#"<div class="card border-yellow-900/50 text-yellow-300 text-sm">No ontology was uploaded.</div>"#
                .to_string(),
        );
    }

    blocks.join("\n")
}

fn render_ontology_list(data: &Value) -> String {
    let items = data
        .as_array()
        .cloned()
        .or_else(|| {
            data.get("ontologies")
                .and_then(|value| value.as_array())
                .cloned()
        })
        .unwrap_or_default();

    if items.is_empty() {
        return r##"<div class="card text-gray-500 text-center py-10">No ontologies uploaded yet.</div>"##
            .to_string();
    }

    let rows = items
        .iter()
        .map(|item| {
            if let Some(key) = item.as_str() {
                return format!(
                    r##"<tr class="border-b border-gray-800 hover:bg-gray-800/50">
                        <td class="py-3 px-4 font-medium">{}</td>
                        <td class="py-3 px-4 text-sm text-gray-500">—</td>
                        <td class="py-3 px-4 text-sm text-gray-500">Available</td>
                    </tr>"##,
                    html_escape(key)
                );
            }

            let key = item
                .get("key")
                .or_else(|| item.get("name"))
                .or_else(|| item.get("ontology_key"))
                .and_then(|value| value.as_str())
                .unwrap_or("unknown");
            let source = item
                .get("filename")
                .or_else(|| item.get("file_name"))
                .or_else(|| item.get("source"))
                .and_then(|value| value.as_str())
                .unwrap_or("—");
            let status = item
                .get("status")
                .or_else(|| item.get("created_at"))
                .and_then(|value| value.as_str())
                .unwrap_or("Available");

            format!(
                r##"<tr class="border-b border-gray-800 hover:bg-gray-800/50">
                    <td class="py-3 px-4 font-medium">{}</td>
                    <td class="py-3 px-4 text-sm text-gray-500">{}</td>
                    <td class="py-3 px-4 text-sm text-gray-400">{}</td>
                </tr>"##,
                html_escape(key),
                html_escape(source),
                html_escape(status)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");

    format!(
        r##"<div class="card p-0 overflow-hidden">
            <table class="w-full text-sm">
                <thead>
                    <tr class="bg-gray-800/50 text-left text-xs uppercase text-gray-500">
                        <th class="py-3 px-4">Key</th>
                        <th class="py-3 px-4">Source</th>
                        <th class="py-3 px-4">Status</th>
                    </tr>
                </thead>
                <tbody>{}</tbody>
            </table>
        </div>"##,
        rows
    )
}
