use reqwest::{Client, Method, RequestBuilder};
use serde_json::{json, Value};
use sqlx::PgPool;
use std::time::Instant;

use crate::error::AppError;

pub struct CogneeClient {
    client: Client,
    base_url: String,
    token: Option<String>,
    pool: Option<PgPool>,
}

impl CogneeClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            token: None,
            pool: None,
        }
    }

    pub fn with_token(mut self, token: String) -> Self {
        self.token = Some(token);
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
    ) -> Result<reqwest::Response, AppError> {
        let url = format!("{}{}", self.base_url, path);

        let mut builder: RequestBuilder = self.client.request(method.clone(), &url);

        if let Some(token) = &self.token {
            builder = builder.bearer_auth(token);
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

        if let Some(pool) = &self.pool {
            let _ = sqlx::query(
                r#"INSERT INTO cognee_admin.request_logs
                   (method, endpoint, status_code, latency_ms, request_body, response_preview, source)
                   VALUES ($1, $2, $3, $4, $5, NULL, 'cli')"#,
            )
            .bind(method.as_str())
            .bind(path)
            .bind(status_code)
            .bind(latency_ms)
            .bind(request_body_str.as_deref())
            .execute(pool)
            .await;
        }

        Ok(response)
    }

    async fn request_json(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<Value, AppError> {
        let response = self.request(method, path, body).await?;
        let status = response.status();
        let text = response.text().await?;

        let response_preview = truncate_string(&text, 1024);

        // Update response_preview in the most recent log entry
        if let Some(pool) = &self.pool {
            let _ = sqlx::query(
                r#"UPDATE cognee_admin.request_logs
                   SET response_preview = $1
                   WHERE id = (SELECT MAX(id) FROM cognee_admin.request_logs WHERE source = 'cli')"#,
            )
            .bind(&response_preview)
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

    // ── Public API methods ──────────────────────────────────────────

    pub async fn health(&self) -> Result<Value, AppError> {
        self.request_json(Method::GET, "/health", None).await
    }

    pub async fn health_detailed(&self) -> Result<Value, AppError> {
        self.request_json(Method::GET, "/health/detailed", None).await
    }

    pub async fn login(&self, username: &str, password: &str) -> Result<Value, AppError> {
        let url = format!("{}/api/v1/auth/login", self.base_url);

        let start = Instant::now();
        let mut builder = self.client.post(&url)
            .form(&[("username", username), ("password", password)]);

        if let Some(token) = &self.token {
            builder = builder.bearer_auth(token);
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
        self.request_json(Method::GET, "/api/v1/auth/me", None).await
    }

    pub async fn datasets(&self) -> Result<Value, AppError> {
        self.request_json(Method::GET, "/api/v1/datasets", None).await
    }

    pub async fn create_dataset(&self, name: &str) -> Result<Value, AppError> {
        let body = json!({ "name": name });
        self.request_json(Method::POST, "/api/v1/datasets", Some(&body)).await
    }

    pub async fn delete_dataset(&self, id: &str) -> Result<Value, AppError> {
        let path = format!("/api/v1/datasets/{}", id);
        self.request_json(Method::DELETE, &path, None).await
    }

    pub async fn dataset_status(&self) -> Result<Value, AppError> {
        self.request_json(Method::GET, "/api/v1/datasets/status", None).await
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

    pub async fn add_data(&self, dataset_name: &str, data: Value) -> Result<Value, AppError> {
        let body = json!({
            "dataset_name": dataset_name,
            "data": data,
        });
        self.request_json(Method::POST, "/api/v1/add", Some(&body)).await
    }

    pub async fn cognify(&self, payload: Value) -> Result<Value, AppError> {
        self.request_json(Method::POST, "/api/v1/cognify", Some(&payload)).await
    }

    pub async fn search(
        &self,
        query: &str,
        search_type: Option<&str>,
        top_k: Option<u32>,
    ) -> Result<Value, AppError> {
        let body = json!({
            "query": query,
            "search_type": search_type.unwrap_or("INSIGHTS"),
            "top_k": top_k.unwrap_or(5),
        });
        self.request_json(Method::POST, "/api/v1/search", Some(&body)).await
    }

    pub async fn get_settings(&self) -> Result<Value, AppError> {
        self.request_json(Method::GET, "/api/v1/settings", None).await
    }

    pub async fn save_settings(&self, settings: Value) -> Result<Value, AppError> {
        self.request_json(Method::POST, "/api/v1/settings", Some(&settings)).await
    }

    pub async fn visualize(&self) -> Result<Value, AppError> {
        self.request_json(Method::GET, "/api/v1/visualize", None).await
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
