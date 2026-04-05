use std::time::{Duration, Instant};

use crate::core::*;
use serde_json::{json, Value};

const IMAGE_MODEL: &str = "grok-imagine-1.0";
const VIDEO_MODEL: &str = "grok-imagine-1.0-video";

/// Allowed sizes for grok2api-go image/video endpoints.
const ALLOWED_SIZES: &[(&str, f64)] = &[
    ("1024x1024", 1.0),
    ("1280x720", 1.778),
    ("720x1280", 0.5625),
    ("1792x1024", 1.75),
    ("1024x1792", 0.571),
];

/// Grok Image provider — connects directly to grok2api-go native REST APIs.
/// Supports image generation (`/v1/images/generations`) and video generation
/// (`/v1/video/generations`).
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

/// Map a "WxH" size string to the closest allowed grok2api-go size.
fn map_size(size: &str) -> &'static str {
    let parts: Vec<&str> = size.split('x').chain(size.split('X')).take(2).collect();
    if parts.len() < 2 {
        return "1024x1024";
    }
    let w: f64 = parts[0].trim().parse().unwrap_or(0.0);
    let h: f64 = parts[1].trim().parse().unwrap_or(0.0);
    if w <= 0.0 || h <= 0.0 {
        return "1024x1024";
    }

    let ratio = w / h;
    let mut best = ALLOWED_SIZES[0].0;
    let mut best_diff = f64::MAX;
    for &(name, r) in ALLOWED_SIZES {
        let diff = (ratio - r).abs();
        if diff < best_diff {
            best_diff = diff;
            best = name;
        }
    }
    best
}

#[async_trait::async_trait]
impl AssetProvider for GrokImageProvider {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

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
            priority: 80,
            ..Default::default()
        }
    }

    async fn generate(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        let prompt = req.prompt.as_deref().unwrap_or("");
        let start = Instant::now();

        let size = req
            .params
            .get("size")
            .and_then(|v| v.as_str())
            .map(map_size)
            .unwrap_or("1024x1024");

        let (url, body, timeout, model, cost) = match req.asset_type {
            AssetType::Video => {
                let seconds = req
                    .params
                    .get("seconds")
                    .and_then(|v| v.as_u64())
                    .map(|s| s.clamp(6, 30))
                    .unwrap_or(6);

                (
                    format!("{}/v1/video/generations", self.base_url),
                    json!({
                        "model": VIDEO_MODEL,
                        "prompt": prompt,
                        "size": size,
                        "seconds": seconds,
                        "quality": "standard",
                    }),
                    Duration::from_secs(180),
                    VIDEO_MODEL,
                    0.10,
                )
            }
            _ => (
                format!("{}/v1/images/generations", self.base_url),
                json!({
                    "model": IMAGE_MODEL,
                    "prompt": prompt,
                    "size": size,
                    "response_format": "url",
                }),
                Duration::from_secs(60),
                IMAGE_MODEL,
                0.07,
            ),
        };

        let resp = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .timeout(timeout)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("Grok Image returned {}: {}", status, text);
        }

        let payload: Value = serde_json::from_str(&text)?;

        let output_url = match req.asset_type {
            AssetType::Video => payload["url"].as_str().map(String::from),
            _ => payload["data"][0]["url"].as_str().map(String::from),
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
                "size": size,
            }),
            cost_usd: Some(cost),
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
        assert_eq!(map_size("1024x1024"), "1024x1024");
        assert_eq!(map_size("512x512"), "1024x1024");
    }

    #[test]
    fn map_size_landscape() {
        assert_eq!(map_size("1280x720"), "1280x720");
        // 1920x1080 = 1.778 ratio, same as 1280x720
        assert_eq!(map_size("1920x1080"), "1280x720");
        assert_eq!(map_size("1792x1024"), "1792x1024");
    }

    #[test]
    fn map_size_portrait() {
        assert_eq!(map_size("720x1280"), "720x1280");
        assert_eq!(map_size("1024x1792"), "1024x1792");
        // 1080x1920 = 0.5625 ratio, same as 720x1280
        assert_eq!(map_size("1080x1920"), "720x1280");
    }

    #[test]
    fn map_size_invalid() {
        assert_eq!(map_size("bad"), "1024x1024");
        assert_eq!(map_size("0x0"), "1024x1024");
        assert_eq!(map_size(""), "1024x1024");
    }

    #[test]
    fn map_size_near_ratios() {
        // 4:3 ≈ 1.333 — closest to 1280x720 (1.778) vs 1024x1024 (1.0)
        // diff to 1.0 = 0.333, diff to 1.778 = 0.445, diff to 1.75 = 0.417
        // → closest is 1024x1024
        assert_eq!(map_size("800x600"), "1024x1024");
        // 16:10 = 1.6 — closest to 1792x1024 (1.75)
        assert_eq!(map_size("1680x1050"), "1792x1024");
    }
}
