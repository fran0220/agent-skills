use anyhow::Context as _;
use reqwest::{
    multipart::{Form, Part},
    Body, Client, Method, RequestBuilder,
};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::{
    path::{Path, PathBuf},
    time::Instant,
};
use tokio::fs::File;
use tokio_util::io::ReaderStream;

use crate::error::AppError;

struct LoggedResponse {
    response: reqwest::Response,
    log_id: Option<i64>,
}

struct UploadLogEntry {
    filename: String,
    size: u64,
}

pub struct CogneeClient {
    client: Client,
    base_url: String,
    cognee_jwt: Option<String>,
    pool: Option<PgPool>,
}

impl CogneeClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            cognee_jwt: None,
            pool: None,
        }
    }

    pub fn with_token(mut self, cognee_jwt: String) -> Self {
        self.cognee_jwt = Some(cognee_jwt);
        self
    }

    pub fn with_pool(mut self, pool: PgPool) -> Self {
        self.pool = Some(pool);
        self
    }

    async fn request(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<LoggedResponse, AppError> {
        let url = format!("{}{}", self.base_url, path);

        let mut builder: RequestBuilder = self.client.request(method.clone(), &url);

        if let Some(cognee_jwt) = &self.cognee_jwt {
            builder = builder.bearer_auth(cognee_jwt);
        }

        if let Some(body) = body {
            builder = builder.json(body);
        }

        let request_body_str = body.map(|b| {
            let s = serde_json::to_string(b).unwrap_or_default();
            truncate_string(&s, 4096)
        });

        let start = Instant::now();
        let response = builder.send().await?;
        let latency_ms = start.elapsed().as_millis() as i32;
        let status_code = response.status().as_u16() as i32;

        let log_id = if let Some(pool) = &self.pool {
            sqlx::query_scalar::<_, i64>(
                r#"INSERT INTO cognee_admin.request_logs
                   (method, endpoint, status_code, latency_ms, request_body, response_preview, source)
                   VALUES ($1, $2, $3, $4, $5, NULL, 'cli')
                   RETURNING id"#,
            )
            .bind(method.as_str())
            .bind(path)
            .bind(status_code)
            .bind(latency_ms)
            .bind(request_body_str.as_deref())
            .fetch_optional(pool)
            .await
            .ok()
            .flatten()
        } else {
            None
        };

        Ok(LoggedResponse { response, log_id })
    }

    async fn request_json(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, AppError> {
        let LoggedResponse { response, log_id } = self.request(method, path, body).await?;
        let status = response.status();
        let text = response.text().await?;

        let response_preview = truncate_string(&text, 1024);

        if let (Some(pool), Some(log_id)) = (&self.pool, log_id) {
            let _ = sqlx::query(
                r#"UPDATE cognee_admin.request_logs
                   SET response_preview = $1
                   WHERE id = $2"#,
            )
            .bind(&response_preview)
            .bind(log_id)
            .execute(pool)
            .await;
        }

        if status.is_success() {
            let value: Value = serde_json::from_str(&text).unwrap_or(Value::String(text));
            Ok(value)
        } else {
            Err(AppError::CogneeApi(format!(
                "HTTP {} — {}",
                status.as_u16(),
                truncate_string(&text, 512)
            )))
        }
    }

    async fn multipart_request_json(
        &self,
        method: Method,
        path: &str,
        query_params: Option<&[(&str, &str)]>,
        form: Form,
        request_summary: Value,
    ) -> Result<Value, AppError> {
        let url = format!("{}{}", self.base_url, path);
        let mut builder = self.client.request(method.clone(), &url);

        if let Some(cognee_jwt) = &self.cognee_jwt {
            builder = builder.bearer_auth(cognee_jwt);
        }

        if let Some(query_params) = query_params {
            builder = builder.query(query_params);
        }

        let start = Instant::now();
        let response = builder.multipart(form).send().await?;
        let latency_ms = start.elapsed().as_millis() as i32;
        let status_code = response.status().as_u16() as i32;
        let status = response.status();
        let text = response.text().await?;
        let response_preview = truncate_string(&text, 1024);

        self.log_request(
            method.as_str(),
            &format_endpoint(path, query_params),
            status_code,
            latency_ms,
            Some(&truncate_string(
                &serde_json::to_string(&request_summary).unwrap_or_default(),
                4096,
            )),
            Some(&response_preview),
        )
        .await;

        if status.is_success() {
            let value: Value = serde_json::from_str(&text).unwrap_or(Value::String(text));
            Ok(value)
        } else {
            Err(AppError::CogneeApi(format!(
                "HTTP {} — {}",
                status.as_u16(),
                truncate_string(&text, 512)
            )))
        }
    }

    async fn log_request(
        &self,
        method: &str,
        endpoint: &str,
        status_code: i32,
        latency_ms: i32,
        request_body: Option<&str>,
        response_preview: Option<&str>,
    ) {
        if let Some(pool) = &self.pool {
            let _ = sqlx::query(
                r#"INSERT INTO cognee_admin.request_logs
                   (method, endpoint, status_code, latency_ms, request_body, response_preview, source)
                   VALUES ($1, $2, $3, $4, $5, $6, 'cli')"#,
            )
            .bind(method)
            .bind(endpoint)
            .bind(status_code)
            .bind(latency_ms)
            .bind(request_body)
            .bind(response_preview)
            .execute(pool)
            .await;
        }
    }

    async fn build_upload_form(
        &self,
        files: &[PathBuf],
    ) -> Result<(Form, Vec<UploadLogEntry>), AppError> {
        let mut form = Form::new();
        let mut file_logs = Vec::with_capacity(files.len());

        for file_path in files {
            let (part, log_entry) = upload_part_from_path(file_path).await?;
            form = form.part("data", part);
            file_logs.push(log_entry);
        }

        Ok((form, file_logs))
    }

    // ── Public API methods ──────────────────────────────────────────

    pub async fn health(&self) -> Result<Value, AppError> {
        self.request_json(Method::GET, "/health", None).await
    }

    pub async fn health_detailed(&self) -> Result<Value, AppError> {
        self.request_json(Method::GET, "/health/detailed", None)
            .await
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<Value, AppError> {
        let url = format!("{}/api/v1/auth/login", self.base_url);

        let start = Instant::now();
        let mut builder = self
            .client
            .post(&url)
            .form(&[("username", username), ("password", password)]);

        if let Some(cognee_jwt) = &self.cognee_jwt {
            builder = builder.bearer_auth(cognee_jwt);
        }

        let response = builder.send().await?;
        let latency_ms = start.elapsed().as_millis() as i32;
        let status_code = response.status().as_u16() as i32;
        let status = response.status();
        let text = response.text().await?;
        let response_preview = truncate_string(&text, 1024);

        if let Some(pool) = &self.pool {
            let _ = sqlx::query(
                r#"INSERT INTO cognee_admin.request_logs
                   (method, endpoint, status_code, latency_ms, request_body, response_preview, source)
                   VALUES ($1, $2, $3, $4, $5, $6, 'cli')"#,
            )
            .bind("POST")
            .bind("/api/v1/auth/login")
            .bind(status_code)
            .bind(latency_ms)
            .bind("[form-encoded credentials]")
            .bind(&response_preview)
            .execute(pool)
            .await;
        }

        if status.is_success() {
            let value: Value = serde_json::from_str(&text).unwrap_or(Value::String(text));
            Ok(value)
        } else {
            Err(AppError::CogneeApi(format!(
                "Login failed: HTTP {} — {}",
                status.as_u16(),
                truncate_string(&text, 512)
            )))
        }
    }

    pub async fn get_me(&self) -> Result<Value, AppError> {
        self.request_json(Method::GET, "/api/v1/auth/me", None)
            .await
    }

    pub async fn datasets(&self) -> Result<Value, AppError> {
        self.request_json(Method::GET, "/api/v1/datasets", None)
            .await
    }

    pub async fn create_dataset(&self, name: &str) -> Result<Value, AppError> {
        let body = json!({ "name": name });
        self.request_json(Method::POST, "/api/v1/datasets", Some(&body))
            .await
    }

    pub async fn delete_dataset(&self, id: &str) -> Result<Value, AppError> {
        let path = format!("/api/v1/datasets/{}", id);
        self.request_json(Method::DELETE, &path, None).await
    }

    pub async fn delete_all_datasets(&self) -> Result<Value, AppError> {
        self.request_json(Method::DELETE, "/api/v1/datasets", None)
            .await
    }

    pub async fn dataset_status(&self) -> Result<Value, AppError> {
        self.request_json(Method::GET, "/api/v1/datasets/status", None)
            .await
    }

    pub async fn dataset_graph(&self, id: &str) -> Result<Value, AppError> {
        let path = format!("/api/v1/datasets/{}/graph", id);
        self.request_json(Method::GET, &path, None).await
    }

    pub async fn dataset_data(&self, id: &str) -> Result<Value, AppError> {
        let path = format!("/api/v1/datasets/{}/data", id);
        self.request_json(Method::GET, &path, None).await
    }

    pub async fn delete_data(&self, dataset_id: &str, data_id: &str) -> Result<Value, AppError> {
        let path = format!("/api/v1/datasets/{}/data/{}", dataset_id, data_id);
        self.request_json(Method::DELETE, &path, None).await
    }

    pub async fn raw_data(&self, dataset_id: &str, data_id: &str) -> Result<Value, AppError> {
        let path = format!("/api/v1/datasets/{}/data/{}/raw", dataset_id, data_id);
        self.request_json(Method::GET, &path, None).await
    }

    pub async fn add_data(&self, dataset_name: &str, data: &str) -> Result<Value, AppError> {
        let part = Part::text(data.to_owned())
            .file_name("inline.txt")
            .mime_str("text/plain")
            .context("Failed to set inline upload MIME type")?;
        let form = Form::new()
            .text("datasetName", dataset_name.to_owned())
            .part("data", part);

        self.multipart_request_json(
            Method::POST,
            "/api/v1/add",
            None,
            form,
            json!({
                "datasetName": dataset_name,
                "files": [{
                    "filename": "inline.txt",
                    "size": data.len(),
                }],
            }),
        )
        .await
    }

    pub async fn upload_file(
        &self,
        dataset_name: &str,
        file_path: &Path,
    ) -> Result<Value, AppError> {
        self.upload_files(dataset_name, &[file_path.to_path_buf()])
            .await
    }

    pub async fn upload_files(
        &self,
        dataset_name: &str,
        files: &[PathBuf],
    ) -> Result<Value, AppError> {
        let (form, file_logs) = self.build_upload_form(files).await?;
        let form = form.text("datasetName", dataset_name.to_owned());

        self.multipart_request_json(
            Method::POST,
            "/api/v1/add",
            None,
            form,
            json!({
                "datasetName": dataset_name,
                "files": file_logs_to_json(&file_logs),
            }),
        )
        .await
    }

    pub async fn update_data(
        &self,
        dataset_id: &str,
        data_id: &str,
        file_path: &Path,
    ) -> Result<Value, AppError> {
        let files = vec![file_path.to_path_buf()];
        let (form, file_logs) = self.build_upload_form(&files).await?;
        let query_params = [("dataset_id", dataset_id), ("data_id", data_id)];

        self.multipart_request_json(
            Method::PATCH,
            "/api/v1/update",
            Some(&query_params),
            form,
            json!({
                "dataset_id": dataset_id,
                "data_id": data_id,
                "files": file_logs_to_json(&file_logs),
            }),
        )
        .await
    }

    pub async fn cognify(&self, payload: Value) -> Result<Value, AppError> {
        self.request_json(Method::POST, "/api/v1/cognify", Some(&payload))
            .await
    }

    pub async fn search(
        &self,
        query: &str,
        search_type: Option<&str>,
        top_k: Option<u32>,
        datasets: Option<&[String]>,
    ) -> Result<Value, AppError> {
        let mut body = serde_json::Map::new();
        body.insert("query".into(), Value::String(query.to_string()));
        body.insert(
            "search_type".into(),
            Value::String(search_type.unwrap_or("INSIGHTS").to_string()),
        );
        body.insert("top_k".into(), Value::Number(top_k.unwrap_or(5).into()));

        if let Some(datasets) = datasets.filter(|datasets| !datasets.is_empty()) {
            body.insert("datasets".into(), json!(datasets));
        }

        let body = Value::Object(body);

        self.request_json(Method::POST, "/api/v1/search", Some(&body))
            .await
    }

    pub async fn search_history(&self) -> Result<Value, AppError> {
        self.request_json(Method::GET, "/api/v1/search", None).await
    }

    pub async fn get_settings(&self) -> Result<Value, AppError> {
        self.request_json(Method::GET, "/api/v1/settings", None)
            .await
    }

    pub async fn save_settings(&self, settings: Value) -> Result<Value, AppError> {
        self.request_json(Method::POST, "/api/v1/settings", Some(&settings))
            .await
    }

    pub async fn visualize(&self) -> Result<Value, AppError> {
        self.request_json(Method::GET, "/api/v1/visualize", None)
            .await
    }

    pub async fn list_ontologies(&self) -> Result<Value, AppError> {
        self.request_json(Method::GET, "/api/v1/ontologies", None)
            .await
    }

    pub async fn upload_ontology(
        &self,
        ontology_key: &str,
        file_path: &Path,
    ) -> Result<Value, AppError> {
        let (part, log_entry) = upload_part_from_path(file_path).await?;
        let form = Form::new()
            .text("ontology_key", ontology_key.to_owned())
            .part("ontology_file", part);

        self.multipart_request_json(
            Method::POST,
            "/api/v1/ontologies",
            None,
            form,
            json!({
                "ontology_key": ontology_key,
                "ontology_file": log_entry.filename,
                "size": log_entry.size,
            }),
        )
        .await
    }

    pub async fn record_health_snapshot(&self, health: &Value) {
        let Some(pool) = &self.pool else {
            return;
        };

        let status = health
            .get("status")
            .and_then(|value| value.as_str())
            .unwrap_or("unknown");
        let components = health
            .get("components")
            .cloned()
            .unwrap_or_else(|| json!({}));
        let uptime = health
            .get("uptime")
            .and_then(|value| value.as_i64())
            .unwrap_or(0);

        if let Err(err) = sqlx::query(
            "INSERT INTO cognee_admin.health_snapshots (status, components, uptime) VALUES ($1, $2, $3)",
        )
        .bind(status)
        .bind(&components)
        .bind(uptime as i32)
        .execute(pool)
        .await
        {
            tracing::warn!("failed to record health snapshot: {}", err);
        }
    }
}

fn truncate_string(s: &str, max_bytes: usize) -> String {
    if s.len() <= max_bytes {
        s.to_string()
    } else {
        let truncated = &s[..s.floor_char_boundary(max_bytes)];
        format!("{}…[truncated]", truncated)
    }
}

fn file_logs_to_json(file_logs: &[UploadLogEntry]) -> Vec<Value> {
    file_logs
        .iter()
        .map(|entry| {
            json!({
                "filename": entry.filename,
                "size": entry.size,
            })
        })
        .collect()
}

fn format_endpoint(path: &str, query_params: Option<&[(&str, &str)]>) -> String {
    match query_params {
        Some(params) if !params.is_empty() => format!(
            "{}?{}",
            path,
            params
                .iter()
                .map(|(key, value)| format!("{key}={value}"))
                .collect::<Vec<_>>()
                .join("&")
        ),
        _ => path.to_string(),
    }
}

async fn upload_part_from_path(file_path: &Path) -> Result<(Part, UploadLogEntry), AppError> {
    let metadata = tokio::fs::metadata(file_path)
        .await
        .with_context(|| format!("Failed to read metadata for {}", file_path.display()))?;
    let filename = file_path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            AppError::Config(format!("Invalid upload file name: {}", file_path.display()))
        })?
        .to_string();
    let file = File::open(file_path)
        .await
        .with_context(|| format!("Failed to open upload file {}", file_path.display()))?;
    let stream = ReaderStream::new(file);
    let part = Part::stream_with_length(Body::wrap_stream(stream), metadata.len())
        .file_name(filename.clone());

    Ok((
        part,
        UploadLogEntry {
            filename,
            size: metadata.len(),
        },
    ))
}
