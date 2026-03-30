pub mod routes;

use std::path::PathBuf;
use std::sync::Arc;

use anyhow::Result;
use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use axum::{Json, Router};
use serde::Serialize;
use serde_json::Value;
use tokio::sync::RwLock;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::config::AppConfig;
use crate::frontend;
use crate::output;
use crate::search::SearchEngine;

const MAX_JOB_RECORDS: usize = 1000;

pub struct ServerState {
    pub engine: RwLock<SearchEngine>,
    pub config: RwLock<AppConfig>,
    pub config_path: PathBuf,
    pub jobs: RwLock<Vec<JobRecord>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct JobRecord {
    pub id: String,
    pub query: String,
    pub mode: String,
    pub model: String,
    pub sources_used: Vec<String>,
    pub tokens: u32,
    pub duration_ms: u64,
    pub result_count: usize,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error_message: Option<String>,
    pub created_at: String,
}

impl ServerState {
    pub async fn push_job(&self, job: JobRecord) {
        let mut jobs = self.jobs.write().await;
        jobs.push(job);
        if jobs.len() > MAX_JOB_RECORDS {
            let excess = jobs.len() - MAX_JOB_RECORDS;
            jobs.drain(0..excess);
        }
    }
}

pub type ApiResult<T> = Result<T, ApiError>;

#[derive(Debug)]
pub struct ApiError {
    status: StatusCode,
    command: String,
    code: &'static str,
    message: String,
}

impl ApiError {
    pub fn bad_request(
        command: impl Into<String>,
        code: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            status: StatusCode::BAD_REQUEST,
            command: command.into(),
            code,
            message: message.into(),
        }
    }

    pub fn unauthorized(
        command: impl Into<String>,
        code: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            status: StatusCode::UNAUTHORIZED,
            command: command.into(),
            code,
            message: message.into(),
        }
    }

    pub fn forbidden(
        command: impl Into<String>,
        code: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            status: StatusCode::FORBIDDEN,
            command: command.into(),
            code,
            message: message.into(),
        }
    }

    pub fn bad_gateway(
        command: impl Into<String>,
        code: &'static str,
        message: impl Into<String>,
    ) -> Self {
        Self {
            status: StatusCode::BAD_GATEWAY,
            command: command.into(),
            code,
            message: message.into(),
        }
    }

    pub fn internal(command: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            command: command.into(),
            code: "INTERNAL_ERROR",
            message: message.into(),
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let body: Value = output::error(&self.command, self.code, &self.message);
        (self.status, Json(body)).into_response()
    }
}

pub async fn run(config: AppConfig, port: u16, config_path: PathBuf) -> Result<()> {
    let state = Arc::new(ServerState {
        engine: RwLock::new(SearchEngine::new(config.clone())?),
        config: RwLock::new(config),
        config_path,
        jobs: RwLock::new(Vec::new()),
    });

    let app = Router::new()
        .nest("/api", routes::api_router())
        .route("/health", axum::routing::get(routes::health::health_check))
        .merge(frontend::router())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(("0.0.0.0", port)).await?;
    tracing::info!("ai-search HTTP server listening on 0.0.0.0:{port}");
    axum::serve(listener, app).await?;
    Ok(())
}
