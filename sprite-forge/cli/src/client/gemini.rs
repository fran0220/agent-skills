use std::time::Instant;

use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use tracing::{error, info, warn};

pub struct ImageClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl ImageClient {
    pub fn new(base_url: &str, api_key: &str, model: &str) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
        }
    }

    pub async fn generate_image(
        &self,
        prompt: &str,
        reference_image: Option<&[u8]>,
    ) -> Result<Vec<u8>> {
        if reference_image.is_some() {
            warn!("Reference image provided but Grok image API does not support image input; using text prompt only");
        }

        let body = json!({
            "model": self.model,
            "prompt": prompt,
            "n": 1,
            "size": "1024x1024",
            "response_format": "url"
        });

        let url = format!("{}/v1/images/generations", self.base_url);
        let started_at = Instant::now();

        let response = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await
            .map_err(|err| anyhow!("Image generation request failed: {err}"))?;

        let status = response.status();
        let payload = response
            .text()
            .await
            .map_err(|err| anyhow!("Failed to read image generation response body: {err}"))?;
        let elapsed = started_at.elapsed();

        if !status.is_success() {
            error!(
                status = %status,
                elapsed_ms = elapsed.as_millis(),
                body = %payload,
                "Image generation failed"
            );
            return Err(anyhow!("Image API returned {}: {}", status, payload));
        }

        let value: Value = serde_json::from_str(&payload).map_err(|err| {
            anyhow!("Failed to parse image generation response JSON: {err}; body: {payload}")
        })?;

        let image_bytes = self.extract_image(&value).await?;

        info!(
            model = %self.model,
            elapsed_ms = elapsed.as_millis(),
            bytes = image_bytes.len(),
            has_reference = reference_image.is_some(),
            "Image generation completed"
        );

        Ok(image_bytes)
    }

    pub async fn generate_image_from_url(&self, prompt: &str, image_url: &str) -> Result<Vec<u8>> {
        warn!("Reference image URL provided but Grok image API does not support image input; using text prompt only. URL: {image_url}");
        self.generate_image(prompt, None).await
    }

    async fn extract_image(&self, payload: &Value) -> Result<Vec<u8>> {
        let data = payload["data"]
            .as_array()
            .ok_or_else(|| anyhow!("Image API response missing 'data' array"))?;

        let item = data.first().ok_or_else(|| anyhow!("Image API response 'data' array is empty"))?;

        // Try URL first
        if let Some(url) = item["url"].as_str() {
            let resp = self
                .http
                .get(url)
                .send()
                .await
                .map_err(|err| anyhow!("Failed to download generated image from URL: {err}"))?;
            if !resp.status().is_success() {
                return Err(anyhow!("Failed to download image: HTTP {}", resp.status()));
            }
            let bytes = resp.bytes().await
                .map_err(|err| anyhow!("Failed to read downloaded image bytes: {err}"))?;
            return Ok(bytes.to_vec());
        }

        // Try b64_json
        if let Some(b64) = item["b64_json"].as_str() {
            let bytes = STANDARD
                .decode(b64)
                .map_err(|err| anyhow!("Failed to decode base64 image: {err}"))?;
            return Ok(bytes);
        }

        Err(anyhow!("Image API response does not contain 'url' or 'b64_json'"))
    }

    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/v1/models", self.base_url);
        let response = self
            .http
            .get(url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await
            .map_err(|err| anyhow!("Image API health check request failed: {err}"))?;

        Ok(response.status().is_success())
    }
}
