use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct AppConfig {
    pub cognee_url: String,
    pub database_url: String,
    pub host: String,
    pub port: u16,
}

impl AppConfig {
    pub fn from_env() -> Self {
        Self {
            cognee_url: std::env::var("COGNEE_URL")
                .unwrap_or_else(|_| "https://cogneeapi.xiaomao.chat".to_string()),
            database_url: std::env::var("COGNEE_ADMIN_DB")
                .unwrap_or_else(|_| "postgres://cognee:cognee@localhost:5433/cognee_db".to_string()),
            host: std::env::var("COGNEE_ADMIN_HOST")
                .unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: std::env::var("COGNEE_ADMIN_PORT")
                .ok()
                .and_then(|p| p.parse().ok())
                .unwrap_or(9847),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthConfig {
    pub token: Option<String>,
    pub cognee_url: Option<String>,
}

impl AuthConfig {
    pub fn config_dir() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("cognee-admin")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("auth.json")
    }

    pub fn load() -> Self {
        let path = Self::config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            serde_json::from_str(&content).unwrap_or(Self { token: None, cognee_url: None })
        } else {
            Self { token: None, cognee_url: None }
        }
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let dir = Self::config_dir();
        std::fs::create_dir_all(&dir)?;
        let content = serde_json::to_string_pretty(self)?;
        std::fs::write(Self::config_path(), content)?;
        Ok(())
    }
}
