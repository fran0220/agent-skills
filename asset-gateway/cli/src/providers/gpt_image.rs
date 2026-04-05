use std::time::Instant;

use crate::core::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};

const DEFAULT_MODEL: &str = "gpt-image-1.5";

/// GPT Image provider (OpenAI).
/// Native transparency support via `background: "transparent"`.
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

    /// Map our size format "WxH" to OpenAI supported sizes.
    /// GPT Image supports: 1024x1024, 1024x1536, 1536x1024, auto.
    fn map_size(size: &str) -> &str {
        let parts: Vec<&str> = size.split('x').chain(size.split('X')).take(2).collect();
        if parts.len() < 2 {
            return "auto";
        }
        let w: u32 = parts[0].trim().parse().unwrap_or(0);
        let h: u32 = parts[1].trim().parse().unwrap_or(0);
        if w == 0 || h == 0 {
            return "auto";
        }

        let ratio = w as f64 / h as f64;
        if (ratio - 1.0).abs() < 0.15 {
            "1024x1024"
        } else if ratio > 1.0 {
            "1536x1024"
        } else {
            "1024x1536"
        }
    }

    /// Map our quality level to OpenAI quality.
    fn map_quality(quality: Option<QualityLevel>) -> &'static str {
        match quality {
            Some(QualityLevel::Draft) => "low",
            Some(QualityLevel::Standard) => "medium",
            Some(QualityLevel::Hd) => "high",
            None => "medium",
        }
    }

    /// Estimate cost based on quality and size.
    fn estimate_cost(quality: &str, size: &str) -> f64 {
        match (quality, size) {
            ("low", _) => 0.02,
            ("medium", "1024x1024") => 0.04,
            ("medium", _) => 0.06,
            ("high", "1024x1024") => 0.08,
            ("high", _) => 0.10,
            _ => 0.04,
        }
    }

    /// Build the edit request with input images (multipart form).
    async fn generate_edit(
        &self,
        req: &GenerateRequest,
        model: &str,
        quality: &str,
        size: &str,
        transparent: bool,
    ) -> anyhow::Result<GenerateResponse> {
        let start = Instant::now();
        let prompt = req.prompt.as_deref().unwrap_or("");

        let mut form = reqwest::multipart::Form::new()
            .text("model", model.to_string())
            .text("prompt", prompt.to_string())
            .text("quality", quality.to_string())
            .text("size", size.to_string());

        if transparent {
            form = form.text("background", "transparent");
        }

        // Add input images
        for input in req.image_inputs() {
            let (bytes, filename) = if input.starts_with("http://") || input.starts_with("https://")
            {
                let resp = self.http.get(input).send().await?;
                let bytes = resp.bytes().await?.to_vec();
                (bytes, "input.png".to_string())
            } else if input.starts_with("data:") {
                let raw = input
                    .find(";base64,")
                    .map(|pos| &input[pos + 8..])
                    .unwrap_or(input);
                let bytes = STANDARD.decode(raw)?;
                (bytes, "input.png".to_string())
            } else {
                let bytes = tokio::fs::read(input).await?;
                let filename = std::path::Path::new(input)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("input.png")
                    .to_string();
                (bytes, filename)
            };

            let part = reqwest::multipart::Part::bytes(bytes)
                .file_name(filename)
                .mime_str("image/png")?;
            form = form.part("image[]", part);
        }

        let resp = self
            .http
            .post(format!("{}/v1/images/edits", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .multipart(form)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("GPT Image edit returned {}: {}", status, text);
        }

        let payload: Value = serde_json::from_str(&text)?;
        let (output_data, output_url) = Self::extract_output(&payload)?;

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url,
            output_data,
            metadata: json!({
                "model": model,
                "quality": quality,
                "size": size,
                "transparent": transparent,
                "editing": true,
            }),
            cost_usd: Some(Self::estimate_cost(quality, size)),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    fn extract_output(payload: &Value) -> anyhow::Result<(Option<String>, Option<String>)> {
        let item = payload["data"]
            .as_array()
            .and_then(|arr| arr.first())
            .ok_or_else(|| anyhow::anyhow!("GPT Image response missing data array"))?;

        if let Some(b64) = item["b64_json"].as_str() {
            Ok((Some(b64.to_string()), None))
        } else if let Some(url) = item["url"].as_str() {
            Ok((None, Some(url.to_string())))
        } else {
            anyhow::bail!("GPT Image response has neither b64_json nor url")
        }
    }
}

#[async_trait::async_trait]
impl AssetProvider for GptImageProvider {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

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
            priority: 50, // Lower than gemini (100) for non-transparent; dispatcher boosts for transparent
            ..Default::default()
        }
    }

    async fn generate(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        let model = req.model.as_deref().unwrap_or(DEFAULT_MODEL);
        let transparent = req.transparent();
        let quality = Self::map_quality(req.quality());
        let size_raw = req
            .params
            .get("size")
            .and_then(|v| v.as_str())
            .unwrap_or("1024x1024");
        let size = Self::map_size(size_raw);

        // If we have input images, use the edit endpoint
        if !req.image_inputs().is_empty() {
            return self.generate_edit(req, model, quality, size, transparent).await;
        }

        let start = Instant::now();
        let prompt = req.prompt.as_deref().unwrap_or("");

        // Use b64_json when transparent (to preserve alpha channel fully),
        // URL otherwise (faster, less bandwidth).
        let response_format = if transparent { "b64_json" } else { "url" };

        let mut body = json!({
            "model": model,
            "prompt": prompt,
            "size": size,
            "quality": quality,
            "response_format": response_format,
        });

        if transparent {
            body["background"] = json!("transparent");
        }

        let resp = self
            .http
            .post(format!("{}/v1/images/generations", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .timeout(std::time::Duration::from_secs(120))
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("GPT Image returned {}: {}", status, text);
        }

        let payload: Value = serde_json::from_str(&text)?;
        let (output_data, output_url) = Self::extract_output(&payload)?;

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url,
            output_data,
            metadata: json!({
                "model": model,
                "quality": quality,
                "size": size,
                "transparent": transparent,
                "mime_type": if transparent { Value::String("image/png".into()) } else { Value::Null },
            }),
            cost_usd: Some(Self::estimate_cost(quality, size)),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_size_square() {
        assert_eq!(GptImageProvider::map_size("1024x1024"), "1024x1024");
        assert_eq!(GptImageProvider::map_size("512x512"), "1024x1024");
    }

    #[test]
    fn map_size_landscape() {
        assert_eq!(GptImageProvider::map_size("1792x1024"), "1536x1024");
        assert_eq!(GptImageProvider::map_size("1536x1024"), "1536x1024");
    }

    #[test]
    fn map_size_portrait() {
        assert_eq!(GptImageProvider::map_size("1024x1792"), "1024x1536");
        assert_eq!(GptImageProvider::map_size("768x1024"), "1024x1536");
    }

    #[test]
    fn map_size_invalid() {
        assert_eq!(GptImageProvider::map_size("abc"), "auto");
        assert_eq!(GptImageProvider::map_size("0x0"), "auto");
    }

    #[test]
    fn map_quality_levels() {
        assert_eq!(GptImageProvider::map_quality(Some(QualityLevel::Draft)), "low");
        assert_eq!(GptImageProvider::map_quality(Some(QualityLevel::Standard)), "medium");
        assert_eq!(GptImageProvider::map_quality(Some(QualityLevel::Hd)), "high");
        assert_eq!(GptImageProvider::map_quality(None), "medium");
    }
}
