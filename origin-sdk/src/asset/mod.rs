pub mod types;
pub use types::*;

use reqwest::Method;
use serde::Deserialize;
use serde_json::{json, Map, Value};

use crate::error::Result;
use crate::transport::HttpTransport;

#[derive(Debug, Clone)]
pub struct AssetClient {
    pub(crate) transport: HttpTransport,
}

impl AssetClient {
    pub fn new(transport: HttpTransport) -> Self {
        Self { transport }
    }

    pub async fn generate(&self, request: &GenerateRequest) -> Result<GenerateResponse> {
        let body = json!({
            "asset_type": request.asset_type,
            "prompt": request.prompt,
            "model": request.model,
            "input_file": request.input_file,
            "provider": request.provider,
            "size": request.size,
            "transparent": request.transparent,
            "reference_images": request.reference_images,
            "edit_mode": request.edit_mode,
            "session_id": request.session_id,
            "params": request.params,
        });

        self.transport.post("/api/generate", &body).await
    }

    pub async fn generate_image(
        &self,
        prompt: impl Into<String>,
        options: Option<ImageOptions>,
    ) -> Result<GenerateResponse> {
        let options = options.unwrap_or_default();
        let request = GenerateRequest {
            asset_type: AssetType::Image,
            prompt: Some(prompt.into()),
            model: options.model,
            input_file: options.input,
            provider: options.provider,
            size: options.size,
            transparent: options.transparent,
            reference_images: options.reference_images,
            edit_mode: options.edit_mode,
            session_id: options.session_id,
            params: options.params,
        };

        self.generate(&request).await
    }

    pub async fn generate_video(
        &self,
        prompt: impl Into<String>,
        options: Option<VideoOptions>,
    ) -> Result<GenerateResponse> {
        let options = options.unwrap_or_default();
        let request = GenerateRequest {
            asset_type: AssetType::Video,
            prompt: Some(prompt.into()),
            model: options.model,
            input_file: options.input,
            provider: options.provider,
            size: options.size,
            transparent: None,
            reference_images: Vec::new(),
            edit_mode: None,
            session_id: None,
            params: options.params,
        };

        self.generate(&request).await
    }

    pub async fn generate_audio(
        &self,
        prompt: impl Into<String>,
        options: Option<AudioOptions>,
    ) -> Result<GenerateResponse> {
        let options = options.unwrap_or_default();
        let request = GenerateRequest {
            asset_type: AssetType::Audio,
            prompt: Some(prompt.into()),
            model: options.model,
            input_file: None,
            provider: options.provider,
            size: None,
            transparent: None,
            reference_images: Vec::new(),
            edit_mode: None,
            session_id: None,
            params: merge_params(
                options.params,
                [
                    option_entry("audio_type", options.audio_type),
                    option_entry("duration_seconds", options.duration.map(Value::from)),
                ],
            ),
        };

        self.generate(&request).await
    }

    pub async fn generate_tts(
        &self,
        prompt: impl Into<String>,
        options: Option<TtsOptions>,
    ) -> Result<GenerateResponse> {
        let options = options.unwrap_or_default();
        let request = GenerateRequest {
            asset_type: AssetType::Tts,
            prompt: Some(prompt.into()),
            model: options.model,
            input_file: None,
            provider: options.provider,
            size: None,
            transparent: None,
            reference_images: Vec::new(),
            edit_mode: None,
            session_id: None,
            params: merge_params(
                options.params,
                [
                    option_entry("voice", options.voice),
                    option_entry("voice_id", options.voice_id),
                    option_entry("language_type", options.language),
                    option_entry("instructions", options.instructions),
                ],
            ),
        };

        self.generate(&request).await
    }

    pub async fn generate_music(
        &self,
        prompt: impl Into<String>,
        options: Option<MusicOptions>,
    ) -> Result<GenerateResponse> {
        let options = options.unwrap_or_default();
        let request = GenerateRequest {
            asset_type: AssetType::Music,
            prompt: Some(prompt.into()),
            model: options.model,
            input_file: None,
            provider: options.provider,
            size: None,
            transparent: None,
            reference_images: Vec::new(),
            edit_mode: None,
            session_id: None,
            params: merge_params(
                options.params,
                [
                    option_entry("duration_seconds", options.duration.map(Value::from)),
                    bool_entry("force_instrumental", options.force_instrumental),
                    option_entry("output_format", options.output_format),
                ],
            ),
        };

        self.generate(&request).await
    }

    pub async fn generate_model3d(
        &self,
        prompt: impl Into<String>,
        options: Option<Model3dOptions>,
    ) -> Result<GenerateResponse> {
        let options = options.unwrap_or_default();
        let request = GenerateRequest {
            asset_type: AssetType::Model3d,
            prompt: Some(prompt.into()),
            model: options.model,
            input_file: options.input,
            provider: options.provider,
            size: None,
            transparent: None,
            reference_images: Vec::new(),
            edit_mode: None,
            session_id: None,
            params: merge_params(
                options.params,
                [
                    option_entry("model_version", options.model_version),
                    option_entry("face_limit", options.face_limit.map(Value::from)),
                    bool_entry("pbr", options.pbr),
                    option_entry("texture_quality", options.texture_quality),
                    bool_entry("auto_size", options.auto_size),
                    option_entry("negative_prompt", options.negative_prompt),
                    option_entry(
                        "multiview",
                        (!options.multiview.is_empty()).then(|| {
                            Value::Array(options.multiview.into_iter().map(Value::from).collect())
                        }),
                    ),
                    option_entry("style", options.style),
                ],
            ),
        };

        self.generate(&request).await
    }

    pub async fn generate_sprite(
        &self,
        prompt: impl Into<String>,
        options: Option<SpriteOptions>,
    ) -> Result<GenerateResponse> {
        let options = options.unwrap_or_default();
        let request = GenerateRequest {
            asset_type: AssetType::Sprite,
            prompt: Some(prompt.into()),
            model: options.model,
            input_file: options.input,
            provider: options.provider,
            size: None,
            transparent: None,
            reference_images: Vec::new(),
            edit_mode: None,
            session_id: None,
            params: merge_params(
                options.params,
                [
                    option_entry(
                        "animation_type",
                        Some(Value::String(
                            options.animation_type.unwrap_or_else(|| "walk".to_string()),
                        )),
                    ),
                    option_entry(
                        "direction",
                        Some(Value::String(
                            options.direction.unwrap_or_else(|| "right".to_string()),
                        )),
                    ),
                    option_entry("duration", Some(Value::from(options.duration.unwrap_or(2)))),
                    option_entry(
                        "output_format",
                        Some(Value::String(
                            options
                                .output_format
                                .unwrap_or_else(|| "spritesheet".to_string()),
                        )),
                    ),
                    option_entry("fps", options.fps.map(Value::from)),
                    option_entry("style", options.style),
                ],
            ),
        };

        self.generate(&request).await
    }

    pub async fn process(&self, request: &ProcessRequest) -> Result<ProcessResponse> {
        let body = json!({
            "input": request.input,
            "inputs": request.inputs,
            "operations": request.operations,
        });

        self.transport.post("/api/process", &body).await
    }

    pub async fn jobs(&self, status: Option<&str>, limit: Option<u32>) -> Result<Vec<JobSummary>> {
        let mut path = String::from("/api/jobs");
        let mut query = Vec::new();

        if let Some(status) = status {
            query.push(format!("status={status}"));
        }
        if let Some(limit) = limit {
            query.push(format!("limit={limit}"));
        }
        if !query.is_empty() {
            path.push('?');
            path.push_str(&query.join("&"));
        }

        let response: JobListResponse = self.transport.get(&path).await?;
        Ok(response.jobs)
    }

    pub async fn job_status(&self, job_id: &str) -> Result<JobSummary> {
        self.transport.get(&format!("/api/jobs/{job_id}")).await
    }

    pub async fn providers(&self) -> Result<Vec<ProviderInfo>> {
        let response: ProviderListResponse = self.transport.get("/api/providers").await?;
        Ok(response.providers)
    }

    pub async fn health(&self) -> Result<bool> {
        let response: HealthResponse = self.transport.request(Method::GET, "/health", None).await?;
        Ok(response.status == "healthy")
    }
}

#[derive(Debug, Deserialize)]
struct JobListResponse {
    jobs: Vec<JobSummary>,
}

#[derive(Debug, Deserialize)]
struct ProviderListResponse {
    providers: Vec<ProviderInfo>,
}

#[derive(Debug, Deserialize)]
struct HealthResponse {
    status: String,
}

fn merge_params<const N: usize>(base: Value, entries: [(String, Option<Value>); N]) -> Value {
    let mut params = match base {
        Value::Object(map) => map,
        _ => Map::new(),
    };

    for (key, value) in entries {
        if let Some(value) = value {
            params.insert(key, value);
        }
    }

    if params.is_empty() {
        Value::Null
    } else {
        Value::Object(params)
    }
}

fn option_entry<T>(key: &str, value: Option<T>) -> (String, Option<Value>)
where
    T: Into<Value>,
{
    (key.to_string(), value.map(Into::into))
}

fn bool_entry(key: &str, value: Option<bool>) -> (String, Option<Value>) {
    (
        key.to_string(),
        value.and_then(|enabled| enabled.then_some(Value::Bool(true))),
    )
}
