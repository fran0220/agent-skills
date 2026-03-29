use std::time::Instant;

use crate::core::*;
use serde_json::{json, Value};

const DEFAULT_MODEL: &str = "grok-imagine-1.0";
const EDIT_MODEL: &str = "grok-imagine-1.0-edit";
const VIDEO_MODEL: &str = "grok-imagine-1.0-video";

/// Grok Image provider (xAI grok-imagine via OpenAI-compatible chat completions).
/// Supports image generation, image editing, and video generation.
pub struct GrokImageProvider {
    pub id: String,
    pub base_url: String,
    pub api_key: String,
    http: reqwest::Client,
}

impl GrokImageProvider {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            id: "grok_image".into(),
            base_url,
            api_key,
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl AssetProvider for GrokImageProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        "Grok Image (xAI)"
    }

    fn asset_types(&self) -> &[AssetType] {
        &[AssetType::Image, AssetType::Video]
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_transparency: false,
            priority: 70,
            ..Default::default()
        }
    }

    async fn generate(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        let prompt = req.prompt.as_deref().unwrap_or("");
        let start = Instant::now();

        let (model, messages) = match req.asset_type {
            AssetType::Video => (
                VIDEO_MODEL,
                json!([{ "role": "user", "content": prompt }]),
            ),
            _ => {
                if let Some(ref image_url) = req.input_file {
                    (
                        EDIT_MODEL,
                        json!([{
                            "role": "user",
                            "content": [
                                { "type": "image_url", "image_url": { "url": image_url } },
                                { "type": "text", "text": prompt }
                            ]
                        }]),
                    )
                } else {
                    (
                        DEFAULT_MODEL,
                        json!([{ "role": "user", "content": prompt }]),
                    )
                }
            }
        };

        let body = json!({
            "model": model,
            "messages": messages,
            "stream": false,
        });

        let resp = self
            .http
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("Grok Image returned {}: {}", status, text);
        }

        let payload: Value = serde_json::from_str(&text)?;
        let content = payload["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("");

        let output_url = match req.asset_type {
            AssetType::Video => extract_video_url(content),
            _ => extract_image_url(content),
        };

        let output_url = output_url
            .ok_or_else(|| anyhow::anyhow!("Grok Image response did not contain a valid URL"))?;

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url: Some(output_url),
            output_data: None,
            metadata: json!({
                "model": model,
                "asset_type": req.asset_type.to_string(),
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

/// Extract an image URL (http/https) from response content.
fn extract_image_url(content: &str) -> Option<String> {
    content
        .split_whitespace()
        .find(|s| s.starts_with("http://") || s.starts_with("https://"))
        .map(|s| s.trim_matches(|c: char| c == '"' || c == '\'' || c == '<' || c == '>'))
        .map(str::to_string)
}

/// Extract mp4 URL from a `<video>` HTML tag in the response content.
fn extract_video_url(content: &str) -> Option<String> {
    if let Some(src_pos) = content.find("src=\"") {
        let rest = &content[src_pos + 5..];
        if let Some(end) = rest.find('"') {
            let url = &rest[..end];
            if url.contains(".mp4") || url.starts_with("http") {
                return Some(url.to_string());
            }
        }
    }
    extract_image_url(content)
}
