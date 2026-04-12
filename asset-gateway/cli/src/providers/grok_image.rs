use std::time::{Duration, Instant};

use crate::core::*;
use serde_json::{json, Value};

const XAI_BASE: &str = "https://api.x.ai";
const IMAGE_MODEL: &str = "grok-imagine-image";
const VIDEO_MODEL: &str = "grok-imagine-video";

/// Grok provider — image and video generation via xAI direct API.
///
/// - Image: `POST /v1/images/generations` (grok-imagine-image)
/// - Video: `POST /v1/videos/generations` (grok-imagine-video, async poll)
pub struct GrokImageProvider {
    pub id: String,
    xai_key: String,
    http: reqwest::Client,
}

impl GrokImageProvider {
    pub fn new(xai_key: String) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(300))
            .build()
            .expect("failed to build Grok HTTP client");
        Self {
            id: "grok_image".into(),
            xai_key,
            http,
        }
    }

    /// Submit video generation and poll until done. Returns video URL.
    async fn generate_video(
        &self,
        prompt: &str,
        duration: u32,
        image_url: Option<&str>,
    ) -> anyhow::Result<String> {
        let mut body = json!({
            "model": VIDEO_MODEL,
            "prompt": prompt,
            "duration": duration,
        });
        if let Some(url) = image_url {
            body["image_url"] = json!(url);
        }

        let resp = self
            .http
            .post(format!("{}/v1/videos/generations", XAI_BASE))
            .bearer_auth(&self.xai_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("Grok video generation returned {}: {}", status, text);
        }

        let data: Value = serde_json::from_str(&text)?;
        let request_id = data["request_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing request_id in video response"))?;

        // Poll until done (up to 5 minutes)
        for i in 0..60 {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let poll = self
                .http
                .get(format!("{}/v1/videos/{}", XAI_BASE, request_id))
                .bearer_auth(&self.xai_key)
                .send()
                .await?;

            let poll_data: Value = poll.json().await?;
            let poll_status = poll_data["status"].as_str().unwrap_or("");
            tracing::debug!(attempt = i + 1, status = poll_status, "Grok video: polling");

            match poll_status {
                "done" => {
                    let url = poll_data["video"]["url"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("missing video url in done response"))?;
                    return Ok(url.to_string());
                }
                "expired" | "failed" => {
                    anyhow::bail!("Grok video generation {}: {}", poll_status, poll_data);
                }
                _ => continue,
            }
        }
        anyhow::bail!("Grok video generation timed out after 5 minutes")
    }
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
        "Grok (xAI)"
    }

    fn asset_types(&self) -> &[AssetType] {
        &[AssetType::Image, AssetType::Video]
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_transparency: false,
            priority: 95,
            ..Default::default()
        }
    }

    async fn generate(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        let prompt = req.prompt.as_deref().unwrap_or("");
        let start = Instant::now();

        match req.asset_type {
            AssetType::Video => {
                let duration = req
                    .params
                    .get("duration")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(5) as u32;
                let image_url = req.input_file.as_deref();

                let video_url = self.generate_video(prompt, duration, image_url).await?;
                let cost = 0.05 * duration as f64;

                Ok(GenerateResponse {
                    provider_id: self.id.clone(),
                    output_path: None,
                    output_url: Some(video_url),
                    output_data: None,
                    metadata: json!({
                        "model": VIDEO_MODEL,
                        "asset_type": "video",
                        "duration": duration,
                        "has_reference_image": image_url.is_some(),
                    }),
                    cost_usd: Some(cost),
                    elapsed_ms: start.elapsed().as_millis() as u64,
                })
            }
            _ => {
                // Image generation
                let aspect_ratio = req
                    .params
                    .get("aspect_ratio")
                    .and_then(|v| v.as_str())
                    .unwrap_or("1:1");

                let resp = self
                    .http
                    .post(format!("{}/v1/images/generations", XAI_BASE))
                    .bearer_auth(&self.xai_key)
                    .timeout(Duration::from_secs(60))
                    .json(&json!({
                        "model": IMAGE_MODEL,
                        "prompt": prompt,
                        "n": 1,
                        "response_format": "url",
                        "aspect_ratio": aspect_ratio,
                    }))
                    .send()
                    .await?;

                let status = resp.status();
                let text = resp.text().await?;
                if !status.is_success() {
                    anyhow::bail!("Grok Image returned {}: {}", status, text);
                }

                let payload: Value = serde_json::from_str(&text)?;
                let output_url = payload["data"][0]["url"]
                    .as_str()
                    .map(String::from)
                    .ok_or_else(|| {
                        anyhow::anyhow!("Grok Image response did not contain a valid URL")
                    })?;

                Ok(GenerateResponse {
                    provider_id: self.id.clone(),
                    output_path: None,
                    output_url: Some(output_url),
                    output_data: None,
                    metadata: json!({
                        "model": IMAGE_MODEL,
                        "asset_type": "image",
                        "aspect_ratio": aspect_ratio,
                    }),
                    cost_usd: Some(0.02),
                    elapsed_ms: start.elapsed().as_millis() as u64,
                })
            }
        }
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();
        let resp = self
            .http
            .get(format!("{}/v1/models", XAI_BASE))
            .bearer_auth(&self.xai_key)
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
