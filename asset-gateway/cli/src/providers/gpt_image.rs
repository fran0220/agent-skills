use std::time::Instant;

use crate::core::*;
use serde_json::{json, Value};

const DEFAULT_MODEL: &str = "gpt-image-1";
const SUPPORTED_MODELS: &[&str] = &["gpt-image-1", "gpt-image-1.5"];

/// GPT Image provider (OpenAI gpt-image-1 / gpt-image-1.5).
/// Supports transparency via alpha channel.
pub struct GptImageProvider {
    pub id: String,
    pub base_url: String,
    pub api_key: String,
    http: reqwest::Client,
}

impl GptImageProvider {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            id: "gpt_image".into(),
            base_url,
            api_key,
            http: reqwest::Client::new(),
        }
    }

    fn resolve_model(&self, requested: Option<&str>) -> &'static str {
        match requested {
            Some(model) if SUPPORTED_MODELS.contains(&model) => {
                if model == "gpt-image-1.5" {
                    "gpt-image-1.5"
                } else {
                    "gpt-image-1"
                }
            }
            _ => DEFAULT_MODEL,
        }
    }

    fn resolve_output_format(&self, req: &GenerateRequest) -> &'static str {
        match req
            .params
            .get("output_format")
            .and_then(Value::as_str)
            .unwrap_or("png")
            .to_ascii_lowercase()
            .as_str()
        {
            "jpeg" => "jpeg",
            "webp" => "webp",
            _ => "png",
        }
    }

    fn resolve_response_format(&self, req: &GenerateRequest) -> &'static str {
        match req
            .params
            .get("response_format")
            .and_then(Value::as_str)
            .unwrap_or("b64_json")
            .to_ascii_lowercase()
            .as_str()
        {
            "url" => "url",
            _ => "b64_json",
        }
    }
}

#[async_trait::async_trait]
impl AssetProvider for GptImageProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        "GPT Image (OpenAI)"
    }

    fn asset_types(&self) -> &[AssetType] {
        &[AssetType::Image]
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_transparency: true,
            priority: 90,
            ..Default::default()
        }
    }

    async fn generate(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        let prompt = req.prompt.as_deref().unwrap_or("");
        let model = self.resolve_model(req.model.as_deref());
        let size = req
            .params
            .get("size")
            .and_then(Value::as_str)
            .unwrap_or("1024x1024");
        let output_format = self.resolve_output_format(req);
        let response_format = self.resolve_response_format(req);
        let transparent = req.transparent();
        let start = Instant::now();

        let mut body = json!({
            "model": model,
            "prompt": prompt,
            "n": 1,
            "size": size,
            "output_format": output_format,
            "response_format": response_format,
        });

        if let Some(quality) = req.quality() {
            body["quality"] = json!(quality.as_str());
        }

        if let Some(style) = req.style() {
            body["style"] = json!(style);
        }

        if transparent {
            body["background"] = json!("transparent");
            if output_format != "png" {
                body["output_format"] = json!("png");
            }
        }

        let resp = self
            .http
            .post(format!("{}/v1/images/generations", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("GPT Image returned {}: {}", status, text);
        }

        let payload: Value = serde_json::from_str(&text)?;
        let image_data = payload["data"][0]["b64_json"].as_str().map(str::to_string);
        let image_url = payload["data"][0]["url"].as_str().map(str::to_string);

        if image_data.is_none() && image_url.is_none() {
            anyhow::bail!("GPT Image response did not include url or b64_json");
        }

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url: image_url,
            output_data: image_data,
            metadata: json!({
                "model": model,
                "size": size,
                "output_format": output_format,
                "response_format": response_format,
                "transparent": transparent,
            }),
            cost_usd: None,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();
        let resp = self
            .http
            .get(format!("{}/v1/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await;

        match resp {
            Ok(r) => Ok(HealthStatus {
                healthy: r.status().is_success(),
                latency_ms: Some(start.elapsed().as_millis() as u64),
                message: None,
            }),
            Err(e) => Ok(HealthStatus {
                healthy: false,
                latency_ms: Some(start.elapsed().as_millis() as u64),
                message: Some(e.to_string()),
            }),
        }
    }
}
