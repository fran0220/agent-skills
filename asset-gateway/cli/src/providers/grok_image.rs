use std::time::{Duration, Instant};

use crate::core::*;
use serde_json::{json, Value};

const IMAGE_MODEL: &str = "grok-imagine-image";
const IMAGE_EDIT_MODEL: &str = "grok-imagine-image-edit";
const VIDEO_MODEL: &str = "grok-imagine-video";

/// Grok provider — image generation, image editing, and video generation
/// via grok2api proxy (OpenAI-compatible).
///
/// - Image: `POST /v1/images/generations` (JSON body)
/// - Image edit: `POST /v1/images/edits` (multipart form)
/// - Video: `POST /v1/videos` (multipart form, async poll)
pub struct GrokImageProvider {
    pub id: String,
    base_url: String,
    api_key: String,
    http: reqwest::Client,
}

impl GrokImageProvider {
    pub fn new(base_url: String, api_key: String) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(300))
            .build()
            .expect("failed to build Grok HTTP client");
        Self {
            id: "grok_image".into(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key,
            http,
        }
    }

    /// Submit video generation (multipart form) and poll until done.
    async fn generate_video(
        &self,
        prompt: &str,
        duration: u32,
        image_url: Option<&str>,
    ) -> anyhow::Result<String> {
        let mut form = reqwest::multipart::Form::new()
            .text("model", VIDEO_MODEL.to_string())
            .text("prompt", prompt.to_string())
            .text("seconds", duration.to_string())
            .text("size", "1024x1024".to_string());

        // Attach reference image if provided
        if let Some(url) = image_url {
            let img_bytes = self
                .http
                .get(url)
                .timeout(Duration::from_secs(30))
                .send()
                .await?
                .bytes()
                .await?;
            let part = reqwest::multipart::Part::bytes(img_bytes.to_vec())
                .file_name("reference.png")
                .mime_str("image/png")?;
            form = form.part("input_reference[]", part);
        }

        let resp = self
            .http
            .post(format!("{}/v1/videos", self.base_url))
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("Grok video generation returned {}: {}", status, text);
        }

        let data: Value = serde_json::from_str(&text)?;
        let video_id = data["id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing id in video response"))?;

        // Poll until done (up to 5 minutes)
        for i in 0..60 {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let poll = self
                .http
                .get(format!("{}/v1/videos/{}", self.base_url, video_id))
                .bearer_auth(&self.api_key)
                .send()
                .await?;

            let poll_data: Value = poll.json().await?;
            let poll_status = poll_data["status"].as_str().unwrap_or("");
            tracing::debug!(attempt = i + 1, status = poll_status, "Grok video: polling");

            match poll_status {
                "completed" => {
                    // Download URL is at /v1/videos/{id}/content
                    let content_url =
                        format!("{}/v1/videos/{}/content", self.base_url, video_id);
                    return Ok(content_url);
                }
                "failed" => {
                    anyhow::bail!("Grok video generation failed: {}", poll_data);
                }
                _ => continue,
            }
        }
        anyhow::bail!("Grok video generation timed out after 5 minutes")
    }

    /// Edit an image using grok-imagine-image-edit (multipart form).
    async fn edit_image(
        &self,
        prompt: &str,
        image_url: &str,
    ) -> anyhow::Result<String> {
        // Download the source image
        let img_bytes = self
            .http
            .get(image_url)
            .timeout(Duration::from_secs(30))
            .send()
            .await?
            .bytes()
            .await?;

        let part = reqwest::multipart::Part::bytes(img_bytes.to_vec())
            .file_name("input.png")
            .mime_str("image/png")?;

        let form = reqwest::multipart::Form::new()
            .text("model", IMAGE_EDIT_MODEL.to_string())
            .text("prompt", prompt.to_string())
            .text("n", "1")
            .part("image[]", part);

        let resp = self
            .http
            .post(format!("{}/v1/images/edits", self.base_url))
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("Grok image edit returned {}: {}", status, text);
        }

        let payload: Value = serde_json::from_str(&text)?;
        payload["data"][0]["url"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("Grok image edit response missing URL"))
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
        "Grok (grok2api)"
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
                    .unwrap_or(6) as u32;
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
                // Check if this is an edit request (has input_file + edit_mode)
                let is_edit = req.input_file.is_some()
                    && req
                        .params
                        .get("edit_mode")
                        .and_then(|v| v.as_str())
                        .is_some();

                if is_edit {
                    let image_url = req.input_file.as_deref().unwrap();
                    let output_url = self.edit_image(prompt, image_url).await?;

                    return Ok(GenerateResponse {
                        provider_id: self.id.clone(),
                        output_path: None,
                        output_url: Some(output_url),
                        output_data: None,
                        metadata: json!({
                            "model": IMAGE_EDIT_MODEL,
                            "asset_type": "image",
                            "edit_mode": true,
                        }),
                        cost_usd: Some(0.02),
                        elapsed_ms: start.elapsed().as_millis() as u64,
                    });
                }

                // Standard image generation
                let model = req
                    .params
                    .get("model")
                    .and_then(|v| v.as_str())
                    .unwrap_or(IMAGE_MODEL);

                let resp = self
                    .http
                    .post(format!("{}/v1/images/generations", self.base_url))
                    .bearer_auth(&self.api_key)
                    .timeout(Duration::from_secs(120))
                    .json(&json!({
                        "model": model,
                        "prompt": prompt,
                        "n": 1,
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
                        "model": model,
                        "asset_type": "image",
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
            .get(format!("{}/v1/models", self.base_url))
            .bearer_auth(&self.api_key)
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
