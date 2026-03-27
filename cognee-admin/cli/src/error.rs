use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::json;

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("Cognee API error: {0}")]
    CogneeApi(String),

    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("HTTP client error: {0}")]
    HttpClient(#[from] reqwest::Error),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Not found: {0}")]
    NotFound(String),

    #[error("{0}")]
    Internal(#[from] anyhow::Error),
}

impl AppError {
    pub fn error_code(&self) -> &'static str {
        match self {
            AppError::CogneeApi(_) => "COGNEE_API_ERROR",
            AppError::Database(_) => "DATABASE_ERROR",
            AppError::HttpClient(_) => "HTTP_CLIENT_ERROR",
            AppError::Config(_) => "CONFIG_ERROR",
            AppError::NotFound(_) => "NOT_FOUND",
            AppError::Internal(_) => "INTERNAL_ERROR",
        }
    }

    pub fn exit_code(&self) -> i32 {
        match self {
            AppError::Config(_) | AppError::NotFound(_) => 1,
            AppError::Database(_) | AppError::Internal(_) => 2,
            AppError::CogneeApi(_) | AppError::HttpClient(_) => 3,
        }
    }

    pub fn suggestion(&self) -> Option<&'static str> {
        match self {
            AppError::CogneeApi(_) => Some("Check if Cognee is running: cognee-admin health"),
            AppError::Database(_) => Some("Check COGNEE_ADMIN_DB connection string"),
            AppError::HttpClient(_) => Some("Check network connectivity to Cognee API"),
            AppError::Config(_) => Some("Run cognee-admin health to verify configuration"),
            _ => None,
        }
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let status = match &self {
            AppError::NotFound(_) => StatusCode::NOT_FOUND,
            AppError::Config(_) => StatusCode::BAD_REQUEST,
            AppError::CogneeApi(_) | AppError::HttpClient(_) => StatusCode::BAD_GATEWAY,
            _ => StatusCode::INTERNAL_SERVER_ERROR,
        };

        let body = json!({
            "ok": false,
            "error": {
                "code": self.error_code(),
                "message": self.to_string(),
                "suggestion": self.suggestion(),
            }
        });

        (status, axum::Json(body)).into_response()
    }
}
