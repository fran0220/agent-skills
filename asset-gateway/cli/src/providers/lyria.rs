use std::time::{Duration, Instant};

use crate::core::*;
use serde_json::{json, Value};

const DEFAULT_MODEL: &str = "lyria-3-clip-preview";

/// Lyria 3 music generation provider (Google).
/// Generates 30-second high-quality music clips via Gemini API.
/// Replaces ElevenLabs for music/BGM generation.
pub struct LyriaProvider {
    pub id: String,
    proxy_url: String,
    proxy_key: String,
    http: reqwest::Client,
}

impl LyriaProvider {
    pub fn new(proxy_url: String, proxy_key: String) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
            .expect("failed to build Lyria HTTP client");
        Self {
            id: "lyria".into(),
            proxy_url,
            proxy_key,
            http,
        }
    }

    fn build_prompt(req: &GenerateRequest) -> String {
        let mut prompt = req.prompt.clone().unwrap_or_default();
        if let Some(style) = req.style() {
            prompt.push_str(&format!("\nStyle: {style}"));
        }
        prompt
    }
}

#[async_trait::async_trait]
impl AssetProvider for LyriaProvider {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        "Lyria 3 (Google Music Generation)"
    }

    fn asset_types(&self) -> &[AssetType] {
        &[AssetType::Music, AssetType::Audio]
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            max_concurrent: 3,
            priority: 110,
            ..Default::default()
        }
    }

    async fn generate(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        let prompt = req.prompt.as_deref().unwrap_or("").trim();
        if prompt.is_empty() {
            anyhow::bail!("Lyria requires a non-empty prompt");
        }

        let model = req.model.as_deref().unwrap_or(DEFAULT_MODEL);
        let start = Instant::now();

        let full_prompt = Self::build_prompt(req);

        let body = json!({
            "contents": [{"parts": [{"text": full_prompt}]}],
            "generationConfig": {
                "responseModalities": ["AUDIO", "TEXT"]
            }
        });

        let resp = self
            .http
            .post(format!(
                "{}/v1beta/models/{}:generateContent",
                self.proxy_url, model
            ))
            .header("x-goog-api-key", &self.proxy_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("Lyria returned {}: {}", status, text);
        }

        let payload: Value = serde_json::from_str(&text)?;
        let parts = payload["candidates"][0]["content"]["parts"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Lyria response missing parts"))?;

        let mut audio_data = None;
        let mut mime_type = None;
        let mut caption = None;

        for part in parts {
            if let Some(data) = part["inlineData"]["data"].as_str() {
                audio_data = Some(data.to_string());
                mime_type = part["inlineData"]["mimeType"].as_str().map(str::to_string);
            }
            if let Some(text) = part["text"].as_str() {
                let trimmed = text.trim();
                if !trimmed.is_empty() {
                    caption = Some(trimmed.to_string());
                }
            }
        }

        let audio_b64 =
            audio_data.ok_or_else(|| anyhow::anyhow!("Lyria response contains no audio data"))?;

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url: None,
            output_data: Some(audio_b64),
            metadata: json!({
                "model": model,
                "content_type": mime_type.unwrap_or_else(|| "audio/mpeg".to_string()),
                "caption": caption,
                "style": req.style(),
            }),
            cost_usd: Some(0.04),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();
        let resp = self
            .http
            .get(format!("{}/v1beta/models", self.proxy_url))
            .header("x-goog-api-key", &self.proxy_key)
            .timeout(Duration::from_secs(10))
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
