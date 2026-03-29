use axum::{
    extract::{Multipart, Path, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::server::routes::auth::{require_admin, CurrentUser};
use crate::server::ServerState;

const UPLOAD_DIR: &str = "uploads";
const MAX_FILE_SIZE: usize = 100 * 1024 * 1024; // 100MB

fn is_allowed_content_type(ct: &str) -> bool {
    ct.starts_with("image/")
        || ct.starts_with("video/")
        || ct.starts_with("audio/")
        || ct.starts_with("model/")
        || ct == "application/octet-stream"
}

async fn upload(
    State(_state): State<Arc<ServerState>>,
    _user: CurrentUser,
    mut multipart: Multipart,
) -> AppResult<Json<Value>> {
    while let Some(field) = multipart
        .next_field()
        .await
        .map_err(|e| AppError::bad_request(format!("invalid multipart data: {e}")))?
    {
        let name = field.name().unwrap_or_default().to_string();
        if name != "file" {
            continue;
        }

        let original_name = field
            .file_name()
            .unwrap_or("upload")
            .to_string();

        let content_type = field
            .content_type()
            .unwrap_or("application/octet-stream")
            .to_string();

        if !is_allowed_content_type(&content_type) {
            return Err(AppError::bad_request(format!(
                "unsupported content type: {content_type}"
            )));
        }

        let data = field
            .bytes()
            .await
            .map_err(|e| AppError::bad_request(format!("failed to read file: {e}")))?;

        if data.len() > MAX_FILE_SIZE {
            return Err(AppError::bad_request(format!(
                "file too large: {} bytes (max {})",
                data.len(),
                MAX_FILE_SIZE
            )));
        }

        let extension = std::path::Path::new(&original_name)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("bin");

        let filename = format!("{}.{}", uuid::Uuid::new_v4(), extension);
        let filepath = std::path::Path::new(UPLOAD_DIR).join(&filename);

        tokio::fs::create_dir_all(UPLOAD_DIR)
            .await
            .map_err(|e| AppError::internal(format!("failed to create upload dir: {e}")))?;

        let size = data.len();
        tokio::fs::write(&filepath, &data)
            .await
            .map_err(|e| AppError::internal(format!("failed to write file: {e}")))?;

        return Ok(Json(json!({
            "ok": true,
            "command": "asset.upload",
            "data": {
                "filename": filename,
                "url": format!("/uploads/{filename}"),
                "size": size,
                "content_type": content_type,
            }
        })));
    }

    Err(AppError::bad_request("no 'file' field found in multipart data"))
}

async fn delete_file(
    State(_state): State<Arc<ServerState>>,
    user: CurrentUser,
    Path(filename): Path<String>,
) -> AppResult<Json<Value>> {
    require_admin(&user)?;

    let filepath = std::path::Path::new(UPLOAD_DIR).join(&filename);
    if !filepath.exists() {
        return Err(AppError::not_found(format!("file not found: {filename}")));
    }

    tokio::fs::remove_file(&filepath)
        .await
        .map_err(|e| AppError::internal(format!("failed to delete file: {e}")))?;

    Ok(Json(json!({
        "ok": true,
        "command": "asset.delete",
        "data": { "filename": filename }
    })))
}

async fn list_files(
    State(_state): State<Arc<ServerState>>,
    _user: CurrentUser,
) -> AppResult<Json<Value>> {
    let dir = std::path::Path::new(UPLOAD_DIR);
    if !dir.exists() {
        return Ok(Json(json!({
            "ok": true,
            "command": "asset.list",
            "data": { "files": [] }
        })));
    }

    let mut files = Vec::new();
    let mut entries = tokio::fs::read_dir(dir)
        .await
        .map_err(|e| AppError::internal(format!("failed to read upload dir: {e}")))?;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| AppError::internal(format!("failed to read entry: {e}")))?
    {
        let metadata = entry
            .metadata()
            .await
            .map_err(|e| AppError::internal(format!("failed to read metadata: {e}")))?;

        if metadata.is_file() {
            let name = entry.file_name().to_string_lossy().to_string();
            files.push(json!({
                "filename": name,
                "url": format!("/uploads/{name}"),
                "size": metadata.len(),
            }));
        }
    }

    Ok(Json(json!({
        "ok": true,
        "command": "asset.list",
        "data": { "files": files }
    })))
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/assets/upload", post(upload))
        .route("/assets/{filename}", delete(delete_file))
        .route("/assets", get(list_files))
}
