use axum::http::StatusCode;
use axum::response::{IntoResponse, Response};
use serde_json::{json, Value};

#[derive(Debug, thiserror::Error)]
pub enum AppError {
    #[error("bad request: {message}")]
    BadRequest {
        message: String,
        suggestion: Option<String>,
    },

    #[error("unauthorized: {message}")]
    Unauthorized {
        message: String,
        suggestion: Option<String>,
    },

    #[error("forbidden: {message}")]
    Forbidden {
        message: String,
        suggestion: Option<String>,
    },

    #[error("not found: {message}")]
    NotFound {
        message: String,
        suggestion: Option<String>,
    },

    #[error("conflict: {message}")]
    Conflict {
        message: String,
        suggestion: Option<String>,
    },

    #[error("provider error: {message}")]
    Provider {
        message: String,
        suggestion: Option<String>,
    },

    #[error("internal: {message}")]
    Internal {
        message: String,
        suggestion: Option<String>,
    },
}

impl AppError {
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::BadRequest {
            message: message.into(),
            suggestion: None,
        }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::Unauthorized {
            message: message.into(),
            suggestion: None,
        }
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        Self::Forbidden {
            message: message.into(),
            suggestion: None,
        }
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::NotFound {
            message: message.into(),
            suggestion: None,
        }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        Self::Conflict {
            message: message.into(),
            suggestion: None,
        }
    }

    pub fn provider(message: impl Into<String>) -> Self {
        Self::Provider {
            message: message.into(),
            suggestion: None,
        }
    }

    pub fn internal(e: impl std::fmt::Display) -> Self {
        Self::Internal {
            message: e.to_string(),
            suggestion: None,
        }
    }

    pub fn with_suggestion(mut self, suggestion: impl Into<String>) -> Self {
        let value = Some(suggestion.into());
        match &mut self {
            Self::BadRequest { suggestion, .. }
            | Self::Unauthorized { suggestion, .. }
            | Self::Forbidden { suggestion, .. }
            | Self::NotFound { suggestion, .. }
            | Self::Conflict { suggestion, .. }
            | Self::Provider { suggestion, .. }
            | Self::Internal { suggestion, .. } => {
                *suggestion = value;
            }
        }
        self
    }

    pub fn code(&self) -> &'static str {
        match self {
            Self::BadRequest { .. } => "BAD_REQUEST",
            Self::Unauthorized { .. } => "UNAUTHORIZED",
            Self::Forbidden { .. } => "FORBIDDEN",
            Self::NotFound { .. } => "NOT_FOUND",
            Self::Conflict { .. } => "CONFLICT",
            Self::Provider { .. } => "PROVIDER_ERROR",
            Self::Internal { .. } => "INTERNAL_ERROR",
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            Self::BadRequest { .. } => StatusCode::BAD_REQUEST,
            Self::Unauthorized { .. } => StatusCode::UNAUTHORIZED,
            Self::Forbidden { .. } => StatusCode::FORBIDDEN,
            Self::NotFound { .. } => StatusCode::NOT_FOUND,
            Self::Conflict { .. } => StatusCode::CONFLICT,
            Self::Provider { .. } => StatusCode::BAD_GATEWAY,
            Self::Internal { .. } => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }

    fn message(&self) -> &str {
        match self {
            Self::BadRequest { message, .. }
            | Self::Unauthorized { message, .. }
            | Self::Forbidden { message, .. }
            | Self::NotFound { message, .. }
            | Self::Conflict { message, .. }
            | Self::Provider { message, .. }
            | Self::Internal { message, .. } => message,
        }
    }

    pub fn suggestion(&self) -> Option<String> {
        self.explicit_suggestion()
            .or_else(|| self.infer_suggestion())
    }

    fn explicit_suggestion(&self) -> Option<String> {
        match self {
            Self::BadRequest { suggestion, .. }
            | Self::Unauthorized { suggestion, .. }
            | Self::Forbidden { suggestion, .. }
            | Self::NotFound { suggestion, .. }
            | Self::Conflict { suggestion, .. }
            | Self::Provider { suggestion, .. }
            | Self::Internal { suggestion, .. } => suggestion.clone(),
        }
    }

    fn infer_suggestion(&self) -> Option<String> {
        let message = self.message().to_ascii_lowercase();

        if matches!(self, Self::Unauthorized { .. }) {
            return Some(
                "Run `asset-gateway auth login` and retry, or pass a valid `api_key` query parameter."
                    .into(),
            );
        }

        if matches!(self, Self::Forbidden { .. }) {
            return Some("Use an admin account for this operation, then retry.".into());
        }

        if message.contains("provider not found") || message.contains("provider not loaded") {
            return Some(
                "Run `asset-gateway provider list` to verify provider IDs and enabled state."
                    .into(),
            );
        }

        if message.contains("missing credential") {
            return Some(
                "Run `asset-gateway credential set <key> <value> --provider <provider-id>` and retry provider registration."
                    .into(),
            );
        }

        if message.contains("credential not found") {
            return Some(
                "Run `asset-gateway credential list` to inspect available keys first.".into(),
            );
        }

        None
    }
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let mut error = json!({
            "code": self.code(),
            "message": self.to_string(),
        });
        if let Some(suggestion) = self.suggestion() {
            error["suggestion"] = Value::String(suggestion);
        }

        let body = json!({
            "ok": false,
            "error": error
        });
        (self.status_code(), axum::Json(body)).into_response()
    }
}

pub type AppResult<T> = Result<T, AppError>;
