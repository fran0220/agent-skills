use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::server::routes::auth::CurrentUser;
use crate::server::ServerState;

#[derive(Deserialize)]
pub struct SearchQuery {
    pub q: Option<String>,
    #[serde(rename = "type")]
    pub asset_type: Option<String>,
    pub tags: Option<String>,
    pub source: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Deserialize)]
pub struct CreateLibraryItem {
    pub asset_type: String,
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub file_url: String,
    pub file_path: Option<String>,
    pub file_size: Option<i64>,
    pub duration_seconds: Option<f64>,
    pub source: Option<String>,
    pub source_job_id: Option<String>,
    #[serde(default)]
    pub metadata: Value,
}

async fn search(
    State(state): State<Arc<ServerState>>,
    _user: CurrentUser,
    Query(params): Query<SearchQuery>,
) -> AppResult<Json<Value>> {
    let limit = params.limit.unwrap_or(20).min(100);
    let offset = params.offset.unwrap_or(0);

    let mut conditions = Vec::new();
    let mut bind_idx = 0usize;
    let mut query_parts = Vec::new();

    // Build dynamic WHERE clauses
    if let Some(ref q) = params.q {
        if !q.trim().is_empty() {
            bind_idx += 1;
            conditions.push(format!(
                "search_vector @@ plainto_tsquery('english', ${bind_idx})"
            ));
            query_parts.push(q.trim().to_string());
        }
    }

    if let Some(ref asset_type) = params.asset_type {
        if !asset_type.trim().is_empty() {
            bind_idx += 1;
            conditions.push(format!("asset_type = ${bind_idx}"));
            query_parts.push(asset_type.trim().to_string());
        }
    }

    if let Some(ref tags) = params.tags {
        if !tags.trim().is_empty() {
            let tag_list: Vec<&str> = tags
                .split(',')
                .map(str::trim)
                .filter(|t| !t.is_empty())
                .collect();
            if !tag_list.is_empty() {
                bind_idx += 1;
                conditions.push(format!("tags && ${bind_idx}"));
                query_parts.push(tag_list.join(","));
            }
        }
    }

    if let Some(ref source) = params.source {
        if !source.trim().is_empty() {
            bind_idx += 1;
            conditions.push(format!("source = ${bind_idx}"));
            query_parts.push(source.trim().to_string());
        }
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", conditions.join(" AND "))
    };

    // Build rank expression for ordering
    let order_clause = if params.q.as_ref().map_or(true, |q| q.trim().is_empty()) {
        "created_at DESC".to_string()
    } else {
        format!("ts_rank(search_vector, plainto_tsquery('english', $1)) DESC, created_at DESC")
    };

    let sql = format!(
        r#"SELECT id, asset_type, name, description, tags, file_path, file_url,
                  file_size, duration_seconds, source, source_job_id, metadata, created_at
           FROM asset_library
           {where_clause}
           ORDER BY {order_clause}
           LIMIT {limit} OFFSET {offset}"#
    );

    // Use raw query with dynamic binds
    let mut q = sqlx::query_as::<_, LibraryRow>(&sql);
    for part in &query_parts {
        if part.contains(',') && conditions.iter().any(|c| c.contains("tags &&")) {
            let tags: Vec<String> = part.split(',').map(|s| s.trim().to_string()).collect();
            q = q.bind(tags);
        } else {
            q = q.bind(part);
        }
    }

    let rows = q.fetch_all(&state.db).await.map_err(AppError::internal)?;

    // Count total
    let count_sql = format!("SELECT COUNT(*) as cnt FROM asset_library {where_clause}");
    let mut count_q = sqlx::query_scalar::<_, i64>(&count_sql);
    for part in &query_parts {
        if part.contains(',') && conditions.iter().any(|c| c.contains("tags &&")) {
            let tags: Vec<String> = part.split(',').map(|s| s.trim().to_string()).collect();
            count_q = count_q.bind(tags);
        } else {
            count_q = count_q.bind(part);
        }
    }
    let total = count_q
        .fetch_one(&state.db)
        .await
        .map_err(AppError::internal)?;

    let items: Vec<Value> = rows.into_iter().map(|r| r.to_json()).collect();

    Ok(Json(json!({
        "ok": true,
        "command": "library.search",
        "data": {
            "items": items,
            "total": total,
            "limit": limit,
            "offset": offset,
        }
    })))
}

async fn create(
    State(state): State<Arc<ServerState>>,
    _user: CurrentUser,
    Json(req): Json<CreateLibraryItem>,
) -> AppResult<Json<Value>> {
    let id = Uuid::new_v4().to_string();
    let source = req.source.as_deref().unwrap_or("manual");
    let file_path = req.file_path.clone().unwrap_or_default();
    let metadata = if req.metadata.is_null() {
        json!({})
    } else {
        req.metadata
    };

    sqlx::query(
        r#"INSERT INTO asset_library (id, asset_type, name, description, tags, file_path, file_url, file_size, duration_seconds, source, source_job_id, metadata)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)"#,
    )
    .bind(&id)
    .bind(&req.asset_type)
    .bind(&req.name)
    .bind(req.description.as_deref().unwrap_or(""))
    .bind(&req.tags)
    .bind(&file_path)
    .bind(&req.file_url)
    .bind(req.file_size.unwrap_or(0))
    .bind(req.duration_seconds)
    .bind(source)
    .bind(req.source_job_id.as_deref())
    .bind(&metadata)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(json!({
        "ok": true,
        "command": "library.create",
        "data": {
            "id": id,
            "asset_type": req.asset_type,
            "name": req.name,
            "file_url": req.file_url,
        }
    })))
}

async fn get_item(
    State(state): State<Arc<ServerState>>,
    _user: CurrentUser,
    Path(id): Path<String>,
) -> AppResult<Json<Value>> {
    let row = sqlx::query_as::<_, LibraryRow>(
        r#"SELECT id, asset_type, name, description, tags, file_path, file_url,
                  file_size, duration_seconds, source, source_job_id, metadata, created_at
           FROM asset_library WHERE id = $1"#,
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or_else(|| AppError::not_found(format!("library item not found: {id}")))?;

    Ok(Json(json!({
        "ok": true,
        "command": "library.get",
        "data": row.to_json(),
    })))
}

async fn delete_item(
    State(state): State<Arc<ServerState>>,
    user: CurrentUser,
    Path(id): Path<String>,
) -> AppResult<Json<Value>> {
    crate::server::routes::auth::require_admin(&user)?;

    let result = sqlx::query("DELETE FROM asset_library WHERE id = $1")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(AppError::internal)?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found(format!("library item not found: {id}")));
    }

    Ok(Json(json!({
        "ok": true,
        "command": "library.delete",
        "data": { "id": id }
    })))
}

/// Catalog completed jobs into the asset library.
/// POST /api/library/catalog-jobs
#[derive(Deserialize)]
pub struct CatalogJobsReq {
    /// Filter by asset_type (e.g. "audio", "music")
    pub asset_type: Option<String>,
    /// Only catalog jobs after this date
    pub after: Option<String>,
    /// Filter by provider_id
    pub provider_id: Option<String>,
    /// Dry run — just count how many would be cataloged
    #[serde(default)]
    pub dry_run: bool,
}

async fn catalog_jobs(
    State(state): State<Arc<ServerState>>,
    user: CurrentUser,
    Json(req): Json<CatalogJobsReq>,
) -> AppResult<Json<Value>> {
    crate::server::routes::auth::require_admin(&user)?;

    let mut conditions = vec!["j.status = 'completed'".to_string()];
    let mut binds: Vec<String> = Vec::new();

    if let Some(ref at) = req.asset_type {
        binds.push(at.clone());
        conditions.push(format!("j.asset_type = ${}", binds.len()));
    }
    if let Some(ref pid) = req.provider_id {
        binds.push(pid.clone());
        conditions.push(format!("j.provider_id = ${}", binds.len()));
    }
    if let Some(ref after) = req.after {
        binds.push(after.clone());
        conditions.push(format!("j.created_at >= ${}::timestamptz", binds.len()));
    }

    // Exclude already-cataloged jobs
    conditions.push(
        "NOT EXISTS (SELECT 1 FROM asset_library al WHERE al.source_job_id = j.id)".to_string(),
    );
    // Only jobs that have output_data
    conditions.push("j.response IS NOT NULL".to_string());

    let where_clause = conditions.join(" AND ");
    let count_sql = format!("SELECT COUNT(*) FROM jobs j WHERE {where_clause}");

    let mut q = sqlx::query_scalar::<_, i64>(&count_sql);
    for b in &binds {
        q = q.bind(b);
    }
    let total = q.fetch_one(&state.db).await.map_err(AppError::internal)?;

    if req.dry_run {
        return Ok(Json(json!({
            "ok": true,
            "command": "library.catalog_jobs",
            "data": { "dry_run": true, "would_catalog": total }
        })));
    }

    // Fetch jobs and catalog them
    let select_sql = format!(
        "SELECT j.id, j.asset_type, j.provider_id, j.request, j.response, j.created_at FROM jobs j WHERE {where_clause} ORDER BY j.created_at"
    );
    let mut select_q = sqlx::query_as::<_, JobRow>(&select_sql);
    for b in &binds {
        select_q = select_q.bind(b);
    }
    let jobs = select_q
        .fetch_all(&state.db)
        .await
        .map_err(AppError::internal)?;

    let public_url = state
        .config
        .read()
        .await
        .public_url
        .trim_end_matches('/')
        .to_string();
    let mut cataloged = 0u64;
    let mut errors = Vec::new();

    for job in &jobs {
        match catalog_single_job(job, &public_url, &state.db).await {
            Ok(_) => cataloged += 1,
            Err(e) => errors.push(format!("{}: {}", job.id, e)),
        }
    }

    Ok(Json(json!({
        "ok": true,
        "command": "library.catalog_jobs",
        "data": {
            "cataloged": cataloged,
            "errors": errors.len(),
            "error_details": errors,
        }
    })))
}

async fn catalog_single_job(
    job: &JobRow,
    public_url: &str,
    db: &sqlx::PgPool,
) -> anyhow::Result<()> {
    let response: Value = serde_json::from_str(&job.response)?;
    let request: Value = serde_json::from_str(&job.request)?;

    let output_data = response.get("output_data").and_then(|v| v.as_str());
    let output_url = response.get("output_url").and_then(|v| v.as_str());

    // Determine file extension
    let content_type = response
        .get("metadata")
        .and_then(|m| m.get("content_type"))
        .and_then(|v| v.as_str())
        .unwrap_or("audio/mpeg");
    let ext = match content_type {
        ct if ct.contains("wav") => "wav",
        ct if ct.contains("ogg") => "ogg",
        ct if ct.contains("mp4") => "mp4",
        _ => "mp3",
    };

    let file_url;
    let file_path;
    let file_size: i64;

    if let Some(b64) = output_data {
        // Decode base64 and save to uploads/
        use base64::Engine;
        let bytes = base64::engine::general_purpose::STANDARD.decode(b64)?;
        file_size = bytes.len() as i64;

        let filename = format!("lib_{}.{}", Uuid::new_v4(), ext);
        let path = std::path::Path::new("uploads").join(&filename);
        tokio::fs::create_dir_all("uploads").await?;
        tokio::fs::write(&path, &bytes).await?;

        file_path = filename.clone();
        file_url = format!("{}/uploads/{}", public_url, filename);
    } else if let Some(url) = output_url {
        file_url = url.to_string();
        file_path = String::new();
        file_size = 0;
    } else {
        anyhow::bail!("no output_data or output_url in job response");
    }

    let prompt = request.get("prompt").and_then(|v| v.as_str()).unwrap_or("");
    let name = truncate_name(prompt);
    let duration = response
        .get("metadata")
        .and_then(|m| m.get("duration_seconds"))
        .and_then(|v| v.as_f64());

    let id = Uuid::new_v4().to_string();
    sqlx::query(
        r#"INSERT INTO asset_library (id, asset_type, name, description, tags, file_path, file_url, file_size, duration_seconds, source, source_job_id, metadata)
           VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, 'generated', $10, $11)"#,
    )
    .bind(&id)
    .bind(&job.asset_type)
    .bind(&name)
    .bind(prompt)
    .bind(&Vec::<String>::new()) // tags — can be enriched later
    .bind(&file_path)
    .bind(&file_url)
    .bind(file_size)
    .bind(duration)
    .bind(&job.id)
    .bind(json!({ "provider_id": job.provider_id }))
    .execute(db)
    .await?;

    Ok(())
}

fn truncate_name(prompt: &str) -> String {
    let trimmed = prompt.trim();
    if trimmed.len() <= 80 {
        trimmed.to_string()
    } else {
        format!("{}…", &trimmed[..77])
    }
}

// -- Row types for sqlx --

#[derive(sqlx::FromRow)]
struct LibraryRow {
    id: String,
    asset_type: String,
    name: String,
    description: String,
    tags: Vec<String>,
    file_path: String,
    file_url: String,
    file_size: i64,
    duration_seconds: Option<f64>,
    source: String,
    source_job_id: Option<String>,
    metadata: Value,
    created_at: chrono::DateTime<chrono::Utc>,
}

impl LibraryRow {
    fn to_json(&self) -> Value {
        json!({
            "id": self.id,
            "asset_type": self.asset_type,
            "name": self.name,
            "description": self.description,
            "tags": self.tags,
            "file_path": self.file_path,
            "file_url": self.file_url,
            "file_size": self.file_size,
            "duration_seconds": self.duration_seconds,
            "source": self.source,
            "source_job_id": self.source_job_id,
            "metadata": self.metadata,
            "created_at": self.created_at.to_rfc3339(),
        })
    }
}

#[derive(sqlx::FromRow)]
#[allow(dead_code)]
struct JobRow {
    id: String,
    asset_type: String,
    provider_id: String,
    request: String,
    response: String,
    created_at: chrono::DateTime<chrono::Utc>,
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/library/search", get(search))
        .route("/library/catalog-jobs", post(catalog_jobs))
        .route("/library", post(create))
        .route("/library/{id}", get(get_item))
        .route("/library/{id}", delete(delete_item))
}
