use std::time::Instant;

use crate::core::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};

const DEFAULT_MODEL: &str = "gemini-3.1-flash-image-preview";

/// Gemini Flash Image provider (Google).
/// Cost-effective, no transparency support.
pub struct GeminiImageProvider {
    pub id: String,
    pub base_url: String,
    pub api_key: String,
    http: reqwest::Client,
}

impl GeminiImageProvider {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            id: "gemini_image".into(),
            base_url,
            api_key,
            http: reqwest::Client::new(),
        }
    }

    fn build_prompt(&self, req: &GenerateRequest) -> String {
        let mut prompt = req.prompt.clone().unwrap_or_default();

        if let Some(style) = req.style() {
            prompt.push_str("\nStyle: ");
            prompt.push_str(style);
        }

        if let Some(quality) = req.quality() {
            prompt.push_str("\nQuality: ");
            prompt.push_str(quality.as_str());
        }

        prompt
    }

    async fn fetch_image_as_base64(&self, input: &str) -> anyhow::Result<String> {
        if input.starts_with("data:") {
            let b64 = input
                .split_once(";base64,")
                .map(|(_, data)| data.to_string())
                .ok_or_else(|| anyhow::anyhow!("Invalid data URI: missing ;base64, segment"))?;
            Ok(b64)
        } else if input.starts_with("http://") || input.starts_with("https://") {
            let bytes = self.http.get(input).send().await?.bytes().await?;
            Ok(STANDARD.encode(&bytes))
        } else {
            let bytes = tokio::fs::read(input).await?;
            Ok(STANDARD.encode(&bytes))
        }
    }

    fn extract_inline_image(payload: &Value) -> Option<(String, Option<String>)> {
        let parts = payload["candidates"][0]["content"]["parts"].as_array()?;

        for part in parts {
            let data = part["inlineData"]["data"].as_str().map(str::to_string);
            if let Some(data) = data {
                let mime = part["inlineData"]["mimeType"].as_str().map(str::to_string);
                return Some((data, mime));
            }
        }

        None
    }
}

#[async_trait::async_trait]
impl AssetProvider for GeminiImageProvider {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        "Gemini Flash Image (Google)"
    }

    fn asset_types(&self) -> &[AssetType] {
        &[AssetType::Image]
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_transparency: false,
            priority: 100,
            ..Default::default()
        }
    }

    async fn generate(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        let model = req.model.as_deref().unwrap_or(DEFAULT_MODEL);
        let prompt = self.build_prompt(req);
        let start = Instant::now();

        let parts = if let Some(ref input_file) = req.input_file {
            let image_data = self.fetch_image_as_base64(input_file).await?;
            json!([
                {"inlineData": {"mimeType": "image/png", "data": image_data}},
                {"text": prompt}
            ])
        } else {
            json!([{"text": prompt}])
        };

        let body = json!({
            "contents": [{"parts": parts}],
            "generationConfig": {
                "responseModalities": ["IMAGE"],
            }
        });

        let resp = self
            .http
            .post(format!(
                "{}/v1beta/models/{}:generateContent",
                self.base_url, model
            ))
            .header("x-goog-api-key", &self.api_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("Gemini Image returned {}: {}", status, text);
        }

        let payload: Value = serde_json::from_str(&text)?;
        let (image_data, mime_type) = Self::extract_inline_image(&payload)
            .ok_or_else(|| anyhow::anyhow!("Gemini response does not contain inlineData image"))?;

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url: None,
            output_data: Some(image_data),
            metadata: json!({
                "model": model,
                "mime_type": mime_type,
                "quality": req.quality().map(|q| q.as_str()),
                "style": req.style(),
                "editing": req.input_file.is_some(),
            }),
            cost_usd: Some(if req.input_file.is_some() { 0.08 } else { 0.04 }),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();
        let resp = self
            .http
            .get(format!("{}/v1beta/models", self.base_url))
            .header("x-goog-api-key", &self.api_key)
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
