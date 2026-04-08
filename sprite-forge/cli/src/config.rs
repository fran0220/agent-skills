use anyhow::{Context, Result, anyhow};
use std::env;

#[derive(Debug, Clone)]
pub struct SpriteForgeConfig {
    pub gemini_api_key: String,
    pub gemini_base_url: String,
    pub gemini_model: String,
    pub llm_proxy_url: String,
    pub llm_proxy_key: String,
    pub llm_model: String,
    pub output_dir: String,
    pub database_path: String,
    pub host: String,
    pub port: u16,
    pub jwt_secret: String,
}

impl SpriteForgeConfig {
    pub fn from_env() -> Result<Self> {
        Ok(Self {
            llm_proxy_url: env::var("LLM_PROXY_URL")
                .unwrap_or_else(|_| "https://api.xiaomao.chat".to_string()),
            llm_proxy_key: required_var("LLM_PROXY_KEY")?,
            gemini_api_key: env::var("GEMINI_API_KEY")
                .or_else(|_| env::var("LLM_PROXY_KEY"))
                .map_err(|_| anyhow!("Missing GEMINI_API_KEY or LLM_PROXY_KEY"))?,
            gemini_base_url: env::var("GEMINI_BASE_URL")
                .or_else(|_| env::var("LLM_PROXY_URL"))
                .unwrap_or_else(|_| "https://api.xiaomao.chat".to_string()),
            gemini_model: env::var("GEMINI_MODEL")
                .unwrap_or_else(|_| "grok-imagine-1.0".to_string()),
            llm_model: env::var("LLM_MODEL")
                .unwrap_or_else(|_| "gemini-3-flash-preview".to_string()),
            output_dir: env::var("OUTPUT_DIR").unwrap_or_else(|_| "./output".to_string()),
            database_path: env::var("DATABASE_PATH")
                .unwrap_or_else(|_| "./data/sprite-forge.db".to_string()),
            host: env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: parse_port()?,
            jwt_secret: required_var("JWT_SECRET")?,
        })
    }
}

fn required_var(name: &str) -> Result<String> {
    match env::var(name) {
        Ok(value) if !value.trim().is_empty() => Ok(value),
        _ => Err(anyhow!("Missing required environment variable: {name}")),
    }
}

fn parse_port() -> Result<u16> {
    match env::var("PORT") {
        Ok(value) => value
            .parse::<u16>()
            .with_context(|| format!("Invalid PORT value: {value}")),
        Err(_) => Ok(3100),
    }
}
