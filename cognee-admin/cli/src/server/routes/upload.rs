use axum::extract::{Multipart, State};
use axum::response::Html;
use serde_json::Value;
use std::sync::Arc;

use super::{base_html, html_escape, persist_field_to_temp_file, AppState, TempUpload};
use crate::error::AppError;

pub async fn page(State(_state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let content = r##"
        <div class="mb-8">
            <h2 class="text-2xl font-bold">Upload</h2>
            <p class="text-gray-400 mt-1">Drag in source files, push them to a dataset, then kick off a background cognify run.</p>
        </div>

        <div class="card mb-6">
            <form id="upload-form"
                  hx-post="/api/upload"
                  hx-encoding="multipart/form-data"
                  hx-target="#upload-results"
                  hx-swap="innerHTML"
                  class="space-y-5">
                <div>
                    <label class="block text-sm text-gray-400 mb-1">Dataset Name</label>
                    <input type="text"
                           name="dataset_name"
                           placeholder="game-db-v3"
                           class="w-full bg-gray-900 border border-gray-700 rounded-lg px-4 py-3 text-sm focus:border-cyan-400 focus:outline-none"
                           required>
                </div>

                <div>
                    <input id="upload-files" type="file" name="files" multiple class="hidden">
                    <label id="upload-dropzone"
                           for="upload-files"
                           class="block cursor-pointer rounded-2xl border-2 border-dashed border-cyan-900/60 bg-gradient-to-br from-cyan-950/40 to-gray-900 p-10 text-center transition-colors hover:border-cyan-400/60 hover:bg-cyan-950/20">
                        <span class="text-4xl">📤</span>
                        <p class="mt-4 text-lg font-semibold">Drop files here or click to browse</p>
                        <p class="text-sm text-gray-400 mt-2">Multiple files are supported. Uploads are proxied through the server using the service JWT.</p>
                        <p id="upload-file-summary" class="text-xs text-cyan-300 mt-4">No files selected yet.</p>
                    </label>
                </div>

                <div class="flex items-center gap-3">
                    <button type="submit"
                            class="bg-cyan-600 hover:bg-cyan-700 px-6 py-2.5 rounded-lg text-sm font-medium transition-colors">
                        Upload Files
                    </button>
                    <span class="htmx-indicator text-gray-500 text-sm">Uploading...</span>
                </div>
            </form>
        </div>

        <div id="upload-results"></div>

        <script>
        (() => {
            const fileInput = document.getElementById('upload-files');
            const dropzone = document.getElementById('upload-dropzone');
            const summary = document.getElementById('upload-file-summary');

            const updateSummary = () => {
                const count = fileInput.files.length;
                if (!count) {
                    summary.textContent = 'No files selected yet.';
                    return;
                }
                const names = Array.from(fileInput.files).slice(0, 3).map(file => file.name);
                const suffix = count > 3 ? ` +${count - 3} more` : '';
                summary.textContent = `${count} file(s): ${names.join(', ')}${suffix}`;
            };

            ['dragenter', 'dragover'].forEach(eventName => {
                dropzone.addEventListener(eventName, event => {
                    event.preventDefault();
                    dropzone.classList.add('border-cyan-400', 'bg-cyan-950/30');
                });
            });

            ['dragleave', 'drop'].forEach(eventName => {
                dropzone.addEventListener(eventName, event => {
                    event.preventDefault();
                    dropzone.classList.remove('border-cyan-400', 'bg-cyan-950/30');
                });
            });

            dropzone.addEventListener('drop', event => {
                fileInput.files = event.dataTransfer.files;
                updateSummary();
            });

            fileInput.addEventListener('change', updateSummary);
        })();
        </script>
    "##;

    Ok(Html(base_html("Upload", "/upload", content)))
}

pub async fn api_upload(
    State(state): State<Arc<AppState>>,
    mut multipart: Multipart,
) -> Html<String> {
    let mut dataset_name: Option<String> = None;
    let mut uploads = Vec::new();
    let mut errors = Vec::new();

    loop {
        match multipart.next_field().await {
            Ok(Some(field)) => {
                let field_name = field.name().unwrap_or_default().to_string();
                match field_name.as_str() {
                    "dataset_name" => match field.text().await {
                        Ok(value) => dataset_name = Some(value),
                        Err(err) => errors.push(format!("Failed to read dataset name: {}", err)),
                    },
                    "files" => {
                        if field.file_name().is_none() {
                            continue;
                        }
                        match persist_field_to_temp_file(field, "upload").await {
                            Ok(upload) => uploads.push(upload),
                            Err(err) => errors.push(err.to_string()),
                        }
                    }
                    _ => {}
                }
            }
            Ok(None) => break,
            Err(err) => {
                errors.push(format!("Failed to read upload request: {}", err));
                break;
            }
        }
    }

    let html = match handle_upload(&state, dataset_name, &uploads, errors).await {
        Ok(html) => html,
        Err(err) => format!(
            r##"<div class="card border-red-900/50 text-red-400 text-sm">
                Upload failed: {}
            </div>"##,
            html_escape(&err.to_string())
        ),
    };

    cleanup_temp_uploads(&uploads).await;
    Html(html)
}

async fn handle_upload(
    state: &Arc<AppState>,
    dataset_name: Option<String>,
    uploads: &[TempUpload],
    mut errors: Vec<String>,
) -> Result<String, AppError> {
    let dataset_name = dataset_name
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .ok_or_else(|| AppError::Config("Dataset name is required".into()))?;

    if uploads.is_empty() {
        if errors.is_empty() {
            errors.push("Select at least one file to upload.".to_string());
        }
        return Ok(render_upload_result(&dataset_name, uploads, None, &errors));
    }

    let file_paths = uploads
        .iter()
        .map(|upload| upload.path.clone())
        .collect::<Vec<_>>();
    let response = match state.client.upload_files(&dataset_name, &file_paths).await {
        Ok(data) => Some(data),
        Err(err) => {
            errors.push(err.to_string());
            None
        }
    };

    Ok(render_upload_result(
        &dataset_name,
        uploads,
        response,
        &errors,
    ))
}

fn render_upload_result(
    dataset_name: &str,
    uploads: &[TempUpload],
    response: Option<Value>,
    errors: &[String],
) -> String {
    let mut sections = Vec::new();

    if response.is_some() {
        let uploaded_files = uploads
            .iter()
            .map(|upload| {
                format!(
                    r#"<li><span class="font-medium">{}</span> <span class="text-gray-500">({} bytes)</span></li>"#,
                    html_escape(&upload.filename),
                    upload.size
                )
            })
            .collect::<Vec<_>>()
            .join("\n");

        sections.push(format!(
            r##"<div class="card border-green-900/50 mb-4">
                <p class="text-green-400 font-semibold">Uploaded {count} file(s) to {dataset}</p>
                <ul class="mt-3 space-y-1 text-sm text-gray-300">{uploaded_files}</ul>
                <div class="mt-5 flex flex-wrap items-center gap-3">
                    <form hx-post="/api/cognify"
                          hx-target="#upload-results"
                          hx-swap="innerHTML"
                          class="inline-flex">
                        <input type="hidden" name="dataset_name" value="{dataset_value}">
                        <button type="submit"
                                class="bg-cyan-600 hover:bg-cyan-700 px-4 py-2 rounded-lg text-sm font-medium transition-colors">
                            Cognify Now
                        </button>
                    </form>
                    <a href="/datasets" class="text-sm text-cyan-400 hover:text-cyan-300">Review datasets →</a>
                </div>
            </div>"##,
            count = uploads.len(),
            dataset = html_escape(dataset_name),
            dataset_value = html_escape(dataset_name),
            uploaded_files = uploaded_files,
        ));
    }

    if !errors.is_empty() {
        let error_items = errors
            .iter()
            .map(|error| format!(r#"<li>{}</li>"#, html_escape(error)))
            .collect::<Vec<_>>()
            .join("\n");
        sections.push(format!(
            r##"<div class="card border-red-900/50 text-red-300 text-sm">
                <p class="font-semibold text-red-400">Upload issues</p>
                <ul class="mt-3 list-disc list-inside space-y-1">{}</ul>
            </div>"##,
            error_items
        ));
    }

    if sections.is_empty() {
        sections.push(
            r#"<div class="card border-yellow-900/50 text-yellow-300 text-sm">No files were uploaded.</div>"#
                .to_string(),
        );
    }

    sections.join("\n")
}

async fn cleanup_temp_uploads(uploads: &[TempUpload]) {
    for upload in uploads {
        let _ = tokio::fs::remove_file(&upload.path).await;
    }
}
