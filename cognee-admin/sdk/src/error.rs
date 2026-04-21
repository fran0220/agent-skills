use thiserror::Error;

/// Unified error type for all Origin SDK operations.
#[derive(Debug, Error)]
pub enum OriginError {
    /// HTTP transport error (connection, timeout, TLS, etc.).
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    /// The API returned a non-success response.
    #[error("API error ({status}): {message}")]
    Api {
        status: u16,
        code: Option<String>,
        message: String,
        command: Option<String>,
    },

    /// JSON serialization / deserialization error.
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// File I/O error (e.g. reading a file for upload).
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Configuration error (missing key, invalid URL, etc.).
    #[error("{0}")]
    Config(String),
}

impl OriginError {
    pub fn api(status: u16, message: impl Into<String>) -> Self {
        Self::Api {
            status,
            code: None,
            message: message.into(),
            command: None,
        }
    }

    pub fn api_full(
        status: u16,
        code: Option<String>,
        message: impl Into<String>,
        command: Option<String>,
    ) -> Self {
        Self::Api {
            status,
            code,
            message: message.into(),
            command,
        }
    }

    pub fn config(message: impl Into<String>) -> Self {
        Self::Config(message.into())
    }
}

/// Convenience alias used throughout the SDK.
pub type Result<T> = std::result::Result<T, OriginError>;
