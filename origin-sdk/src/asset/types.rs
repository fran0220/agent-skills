use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AssetType {
    Image,
    Video,
    Audio,
    Music,
    Tts,
    Model3d,
    Sprite,
    World,
    Text,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ImageEditMode {
    Edit,
    Inpaint,
    Restyle,
    Expand,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateRequest {
    pub asset_type: AssetType,
    pub prompt: Option<String>,
    pub model: Option<String>,
    pub input_file: Option<String>,
    pub provider: Option<String>,
    pub size: Option<String>,
    pub transparent: Option<bool>,
    #[serde(default)]
    pub reference_images: Vec<String>,
    pub edit_mode: Option<ImageEditMode>,
    pub session_id: Option<String>,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenerateResponse {
    pub provider_id: String,
    pub output_path: Option<String>,
    pub output_url: Option<String>,
    pub output_data: Option<String>,
    #[serde(default)]
    pub metadata: Value,
    pub cost_usd: Option<f64>,
    pub elapsed_ms: Option<u64>,
    pub job_id: Option<String>,
    pub user_id: Option<String>,
    pub session_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ImageOptions {
    pub model: Option<String>,
    pub provider: Option<String>,
    pub size: Option<String>,
    pub transparent: Option<bool>,
    pub input: Option<String>,
    #[serde(default)]
    pub reference_images: Vec<String>,
    pub edit_mode: Option<ImageEditMode>,
    pub session_id: Option<String>,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct VideoOptions {
    pub model: Option<String>,
    pub provider: Option<String>,
    pub input: Option<String>,
    pub size: Option<String>,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AudioOptions {
    pub model: Option<String>,
    pub provider: Option<String>,
    pub audio_type: Option<String>,
    pub duration: Option<u32>,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TtsOptions {
    pub model: Option<String>,
    pub provider: Option<String>,
    pub voice: Option<String>,
    pub voice_id: Option<String>,
    pub language: Option<String>,
    pub instructions: Option<String>,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct MusicOptions {
    pub model: Option<String>,
    pub provider: Option<String>,
    pub duration: Option<u32>,
    pub force_instrumental: Option<bool>,
    pub output_format: Option<String>,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Model3dOptions {
    pub model: Option<String>,
    pub provider: Option<String>,
    pub input: Option<String>,
    pub model_version: Option<String>,
    pub face_limit: Option<u32>,
    pub pbr: Option<bool>,
    pub texture_quality: Option<String>,
    pub auto_size: Option<bool>,
    pub negative_prompt: Option<String>,
    #[serde(default)]
    pub multiview: Vec<String>,
    pub style: Option<String>,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SpriteOptions {
    pub model: Option<String>,
    pub provider: Option<String>,
    pub input: Option<String>,
    pub animation_type: Option<String>,
    pub direction: Option<String>,
    pub duration: Option<u32>,
    pub style: Option<String>,
    pub output_format: Option<String>,
    pub fps: Option<u32>,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CropMode {
    Tightest,
    PowerOf2,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComposeDirection {
    Horizontal,
    Vertical,
    Grid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "op", rename_all = "snake_case")]
pub enum ProcessOperation {
    SmartCrop {
        mode: Option<CropMode>,
    },
    Resize {
        width: u32,
        height: u32,
    },
    Compose {
        direction: ComposeDirection,
        columns: Option<u32>,
        padding: Option<u32>,
        frame_width: Option<u32>,
        frame_height: Option<u32>,
    },
    ExtractFrames {
        count: Option<u32>,
    },
    RemoveBg {
        bg_color: Option<String>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessRequest {
    pub input: Option<String>,
    #[serde(default)]
    pub inputs: Vec<String>,
    pub operations: Vec<ProcessOperation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessOutputItem {
    pub output_data: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessResponse {
    pub output_data: String,
    pub width: u32,
    pub height: u32,
    #[serde(default)]
    pub outputs: Vec<ProcessOutputItem>,
    #[serde(default)]
    pub operations_applied: Vec<String>,
    pub elapsed_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobSummary {
    pub id: String,
    pub user_id: Option<String>,
    pub asset_type: String,
    pub provider_id: String,
    pub status: String,
    pub error_message: Option<String>,
    pub output_path: Option<String>,
    pub cost_usd: Option<f64>,
    pub created_at: String,
    pub started_at: Option<String>,
    pub completed_at: Option<String>,
    pub duration_ms: Option<i64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCapabilities {
    pub supports_transparency: bool,
    pub supports_streaming: bool,
    pub max_concurrent: u32,
    pub rate_limit_rpm: Option<u32>,
    pub priority: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderInfo {
    pub id: String,
    pub display_name: String,
    pub asset_types: Vec<AssetType>,
    pub capabilities: ProviderCapabilities,
}
