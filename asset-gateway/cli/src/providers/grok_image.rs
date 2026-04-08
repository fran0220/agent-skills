use std::time::{Duration, Instant};

use crate::core::*;
use serde_json::{json, Value};

const IMAGE_MODEL: &str = "grok-imagine-image";
const VIDEO_MODEL: &str = "grok-imagine-1.0-video";

/// Allowed sizes for video endpoints (grok2api-go).
const ALLOWED_VIDEO_SIZES: &[(&str, f64)] = &[
    ("1024x1024", 1.0),
    ("1280x720", 1.778),
    ("720x1280", 0.5625),
    ("1792x1024", 1.75),
    ("1024x1792", 0.571),
];

/// Grok Image provider — dual-backend:
/// - Image generation via xAI official API through proxy (`/v1/images/generations`)
/// - Video generation via grok2api-go (`/v1/video/generations`)
pub struct GrokImageProvider {
    pub id: String,
    /// Proxy URL for image generation (api.xiaomao.chat → xAI official)
    proxy_url: String,
    proxy_key: String,
    /// grok2api-go URL for video generation (grok.xiaomao.chat)
    video_url: String,
    video_key: String,
    http: reqwest::Client,
}

impl GrokImageProvider {
    pub fn new(proxy_url: String, proxy_key: String, video_url: String, video_key: String) -> Self {
        Self {
            id: "grok_image".into(),
            proxy_url,
            proxy_key,
            video_url,
            video_key,
            http: reqwest::Client::new(),
        }
    }
}

/// Map a "WxH" size string to the closest allowed grok2api-go video size.
fn map_video_size(size: &str) -> &'static str {
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
    let mut best = ALLOWED_VIDEO_SIZES[0].0;
    let mut best_diff = f64::MAX;
    for &(name, r) in ALLOWED_VIDEO_SIZES {
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

        let (url, auth_key, body, timeout, model, cost) = match req.asset_type {
            AssetType::Video => {
                let size = req
                    .params
                    .get("size")
                    .and_then(|v| v.as_str())
                    .map(map_video_size)
                    .unwrap_or("1792x1024");
                let seconds = req
                    .params
                    .get("seconds")
                    .and_then(|v| v.as_u64())
                    .map(|s| s.clamp(6, 30))
                    .unwrap_or(6);
                let quality = req
                    .params
                    .get("quality")
                    .and_then(|v| v.as_str())
                    .unwrap_or("standard");

                let mut video_body = json!({
                    "model": VIDEO_MODEL,
                    "prompt": prompt,
                    "size": size,
                    "seconds": seconds,
                    "quality": quality,
                });
                if let Some(ref image_url) = req.input_file {
                    video_body["image"] = json!(image_url);
                }

                (
                    format!("{}/v1/video/generations", self.video_url),
                    &self.video_key,
                    video_body,
                    Duration::from_secs(180),
                    VIDEO_MODEL,
                    0.10,
                )
            }
            _ => {
                // Image: use xAI official API via proxy with aspect_ratio
                let aspect_ratio = req
                    .params
                    .get("aspect_ratio")
                    .and_then(|v| v.as_str())
                    .unwrap_or("1:1");

                (
                    format!("{}/v1/images/generations", self.proxy_url),
                    &self.proxy_key,
                    json!({
                        "model": IMAGE_MODEL,
                        "prompt": prompt,
                        "n": 1,
                        "response_format": "url",
                        "aspect_ratio": aspect_ratio,
                    }),
                    Duration::from_secs(60),
                    IMAGE_MODEL,
                    0.02,
                )
            }
        };

        let resp = self
            .http
            .post(&url)
            .bearer_auth(auth_key)
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
            }),
            cost_usd: Some(cost),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();
        let resp = self
            .http
            .get(format!("{}/v1/models", self.proxy_url))
            .bearer_auth(&self.proxy_key)
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
    fn map_video_size_square() {
        assert_eq!(map_video_size("1024x1024"), "1024x1024");
        assert_eq!(map_video_size("512x512"), "1024x1024");
    }

    #[test]
    fn map_video_size_landscape() {
        assert_eq!(map_video_size("1280x720"), "1280x720");
        assert_eq!(map_video_size("1920x1080"), "1280x720");
        assert_eq!(map_video_size("1792x1024"), "1792x1024");
    }

    #[test]
    fn map_video_size_portrait() {
        assert_eq!(map_video_size("720x1280"), "720x1280");
        assert_eq!(map_video_size("1024x1792"), "1024x1792");
        assert_eq!(map_video_size("1080x1920"), "720x1280");
    }

    #[test]
    fn map_video_size_invalid() {
        assert_eq!(map_video_size("bad"), "1024x1024");
        assert_eq!(map_video_size("0x0"), "1024x1024");
        assert_eq!(map_video_size(""), "1024x1024");
    }

    #[test]
    fn map_video_size_near_ratios() {
        assert_eq!(map_video_size("800x600"), "1024x1024");
        assert_eq!(map_video_size("1680x1050"), "1792x1024");
    }
}
