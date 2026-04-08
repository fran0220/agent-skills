use std::time::Instant;

use anyhow::{Result, anyhow};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use serde_json::{Value, json};
use tracing::{error, info};

pub struct GeminiClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl GeminiClient {
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
        let mut parts = Vec::with_capacity(if reference_image.is_some() { 2 } else { 1 });

        if let Some(bytes) = reference_image {
            parts.push(json!({
                "inlineData": {
                    "mimeType": "image/png",
                    "data": STANDARD.encode(bytes),
                }
            }));
        }

        parts.push(json!({ "text": prompt }));

        let body = json!({
            "contents": [{
                "role": "user",
                "parts": parts,
            }],
            "generationConfig": {
                "responseModalities": ["IMAGE", "TEXT"]
            }
        });

        let url = format!(
            "{}/v1beta/models/{}:generateContent",
            self.base_url, self.model
        );
        let started_at = Instant::now();

        let response = self
            .http
            .post(&url)
            .header("x-goog-api-key", &self.api_key)
            .json(&body)
            .send()
            .await
            .map_err(|err| anyhow!("Gemini request failed: {err}"))?;

        let status = response.status();
        let payload = response
            .text()
            .await
            .map_err(|err| anyhow!("Failed to read Gemini response body: {err}"))?;
        let elapsed = started_at.elapsed();

        if !status.is_success() {
            error!(
                status = %status,
                elapsed_ms = elapsed.as_millis(),
                body = %payload,
                "Gemini image generation failed"
            );
            return Err(anyhow!("Gemini image API returned {}: {}", status, payload));
        }

        let value: Value = serde_json::from_str(&payload).map_err(|err| {
            anyhow!("Failed to parse Gemini response JSON: {err}; body: {payload}")
        })?;
        let (image, mime_type) = Self::extract_image(&value)?;

        info!(
            model = %self.model,
            elapsed_ms = elapsed.as_millis(),
            mime_type = %mime_type,
            bytes = image.len(),
            has_reference = reference_image.is_some(),
            "Gemini image generation completed"
        );

        Ok(image)
    }

    pub async fn generate_image_from_url(&self, prompt: &str, image_url: &str) -> Result<Vec<u8>> {
        let response = self
            .http
            .get(image_url)
            .send()
            .await
            .map_err(|err| anyhow!("Failed to download reference image from URL: {err}"))?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!(
                "Failed to download reference image from URL, status {}: {}",
                status,
                body
            ));
        }

        let bytes = response
            .bytes()
            .await
            .map_err(|err| anyhow!("Failed to read downloaded reference image bytes: {err}"))?;

        self.generate_image(prompt, Some(bytes.as_ref())).await
    }

    fn extract_image(payload: &Value) -> Result<(Vec<u8>, String)> {
        let parts = payload["candidates"][0]["content"]["parts"]
            .as_array()
            .ok_or_else(|| anyhow!("Gemini response missing candidates[0].content.parts"))?;

        for part in parts {
            let Some(inline_data) = part.get("inlineData") else {
                continue;
            };

            let mime_type = inline_data["mimeType"]
                .as_str()
                .ok_or_else(|| anyhow!("Gemini image part missing inlineData.mimeType"))?
                .to_string();
            let encoded = inline_data["data"]
                .as_str()
                .ok_or_else(|| anyhow!("Gemini image part missing inlineData.data"))?;
            let bytes = STANDARD
                .decode(encoded)
                .map_err(|err| anyhow!("Failed to decode Gemini image base64 payload: {err}"))?;

            return Ok((bytes, mime_type));
        }

        Err(anyhow!(
            "Gemini response did not contain any inlineData image parts"
        ))
    }

    pub async fn health_check(&self) -> Result<bool> {
        let url = format!("{}/v1beta/models", self.base_url);
        let response = self
            .http
            .get(url)
            .header("x-goog-api-key", &self.api_key)
            .send()
            .await
            .map_err(|err| anyhow!("Gemini health check request failed: {err}"))?;

        Ok(response.status().is_success())
    }
}
