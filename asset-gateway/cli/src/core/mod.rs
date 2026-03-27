pub mod dispatcher;
pub mod queue;
pub mod registry;
pub mod vault;

use serde::{Deserialize, Serialize};

// -- Asset types --

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetType {
    Text,
    Image,
    Video,
    Audio,
    Model3d,
}

impl AssetType {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Text => "text",
            Self::Image => "image",
            Self::Video => "video",
            Self::Audio => "audio",
            Self::Model3d => "model3d",
        }
    }
}

impl std::fmt::Display for AssetType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QualityLevel {
    Draft,
    Standard,
    Hd,
}

impl QualityLevel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Draft => "draft",
            Self::Standard => "standard",
            Self::Hd => "hd",
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "draft" => Some(Self::Draft),
            "standard" => Some(Self::Standard),
            "hd" => Some(Self::Hd),
            _ => None,
        }
    }
}

// -- Provider capabilities --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub supports_transparency: bool,
    pub supports_streaming: bool,
    pub max_concurrent: u32,
    pub rate_limit_rpm: Option<u32>,
    pub priority: i32,
}

impl Default for ProviderCapabilities {
    fn default() -> Self {
        Self {
            supports_transparency: false,
            supports_streaming: false,
            max_concurrent: 5,
            rate_limit_rpm: None,
            priority: 100,
        }
    }
}

// -- Generate request / response --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    pub asset_type: AssetType,
    pub prompt: Option<String>,
    pub model: Option<String>,
    pub input_file: Option<String>,
    pub params: serde_json::Value,
}

impl GenerateRequest {
    pub fn transparent(&self) -> bool {
        self.params
            .get("transparent")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    pub fn quality(&self) -> Option<QualityLevel> {
        self.params
            .get("quality")
            .and_then(|v| v.as_str())
            .and_then(QualityLevel::parse)
    }

    pub fn style(&self) -> Option<&str> {
        self.params
            .get("style")
            .and_then(|v| v.as_str())
            .map(str::trim)
            .filter(|s| !s.is_empty())
    }

    pub fn wants_streaming(&self) -> bool {
        self.params
            .get("stream")
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateResponse {
    pub provider_id: String,
    pub output_path: Option<String>,
    pub output_url: Option<String>,
    pub output_data: Option<String>,
    pub metadata: serde_json::Value,
    pub cost_usd: Option<f64>,
    pub elapsed_ms: u64,
}

// -- Health --

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthStatus {
    pub healthy: bool,
    pub latency_ms: Option<u64>,
    pub message: Option<String>,
}

// -- Provider trait --

#[async_trait::async_trait]
pub trait AssetProvider: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    fn asset_types(&self) -> &[AssetType];
    fn capabilities(&self) -> ProviderCapabilities;

    async fn generate(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse>;
    async fn health_check(&self) -> anyhow::Result<HealthStatus>;
}
