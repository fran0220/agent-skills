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
    fallback_url: Option<String>,
    fallback_key: Option<String>,
    http: reqwest::Client,
}

impl GptImageProvider {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            id: "gpt_image".into(),
            base_url,
            api_key,
            fallback_url: None,
            fallback_key: None,
            http: reqwest::Client::new(),
        }
    }

    pub fn with_fallback(mut self, url: String, key: String) -> Self {
        self.fallback_url = Some(url);
        self.fallback_key = Some(key);
        self
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
        base_url: &str,
        api_key: &str,
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
            .post(format!("{}/v1/images/edits", base_url))
            .header("Authorization", format!("Bearer {}", api_key))
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
            // Strip data URI prefix if present (e.g. "data:image/png;base64,...")
            let raw = if let Some(pos) = b64.find(";base64,") {
                &b64[pos + 8..]
            } else {
                b64
            };
            Ok((Some(raw.to_string()), None))
        } else if let Some(url) = item["url"].as_str() {
            Ok((None, Some(url.to_string())))
        } else {
            anyhow::bail!("GPT Image response has neither b64_json nor url")
        }
    }

    async fn generate_inner(&self, req: &GenerateRequest, base_url: &str, api_key: &str) -> anyhow::Result<GenerateResponse> {
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
            return self
                .generate_edit(req, model, quality, size, transparent, base_url, api_key)
                .await;
        }

        let start = Instant::now();
        let prompt = req.prompt.as_deref().unwrap_or("");

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
            .post(format!("{}/v1/images/generations", base_url))
            .header("Authorization", format!("Bearer {}", api_key))
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
            priority: 150, // Primary image provider; +200 boost for transparent
            ..Default::default()
        }
    }

    async fn generate(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        match self.generate_inner(req, &self.base_url, &self.api_key).await {
            Ok(resp) => Ok(resp),
            Err(primary_err) => {
                if let (Some(fb_url), Some(fb_key)) = (&self.fallback_url, &self.fallback_key) {
                    tracing::warn!(
                        error = %primary_err,
                        "GPT Image primary failed, trying fallback"
                    );
                    self.generate_inner(req, fb_url, fb_key).await
                } else {
                    Err(primary_err)
                }
            }
        }
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
        assert_eq!(
            GptImageProvider::map_quality(Some(QualityLevel::Draft)),
            "low"
        );
        assert_eq!(
            GptImageProvider::map_quality(Some(QualityLevel::Standard)),
            "medium"
        );
        assert_eq!(
            GptImageProvider::map_quality(Some(QualityLevel::Hd)),
            "high"
        );
        assert_eq!(GptImageProvider::map_quality(None), "medium");
    }
}
