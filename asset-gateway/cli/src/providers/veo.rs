use std::time::{Duration, Instant};

use crate::core::*;
use serde_json::{json, Value};

const DEFAULT_MODEL: &str = "veo3.1";
const POLL_INTERVAL: Duration = Duration::from_secs(5);
const POLL_TIMEOUT: Duration = Duration::from_secs(300);

/// Veo video provider (Google) — async task-based video generation via the shared LLM proxy.
///
/// Supports text-to-video and image-to-video. Uses the same proxy URL and API key
/// as Gemini Image (configured under `[proxy]` in config.toml).
///
/// Models:
/// - `veo3.1`        — fast mode, auto audio, supports first/last frame images (default)
/// - `veo3.1-pro`    — high quality, expensive
/// - `veo3.1-components` — multi-image reference (1-3 images)
pub struct VeoProvider {
    pub id: String,
    pub base_url: String,
    pub api_key: String,
    http: reqwest::Client,
}

impl VeoProvider {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            id: "veo".into(),
            base_url,
            api_key,
            http: reqwest::Client::new(),
        }
    }

    /// Map a "WxH" size string to a Veo aspect_ratio ("16:9" or "9:16").
    fn map_aspect_ratio(size: &str) -> Option<&'static str> {
        let (w, h) = size.split_once('x').or_else(|| size.split_once('X'))?;
        let w: f64 = w.trim().parse().ok()?;
        let h: f64 = h.trim().parse().ok()?;
        if w <= 0.0 || h <= 0.0 {
            return None;
        }
        if w > h {
            Some("16:9")
        } else if h > w {
            Some("9:16")
        } else {
            None // square — let API auto-decide
        }
    }

    /// Submit a generation job and return the task_id.
    async fn submit(&self, body: &Value) -> anyhow::Result<String> {
        let resp = self
            .http
            .post(format!("{}/v2/videos/generations", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("Veo submit returned {}: {}", status, text);
        }

        let payload: Value = serde_json::from_str(&text)?;
        payload["task_id"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("Veo submit response missing task_id: {}", text))
    }

    /// Poll a task until SUCCESS or FAILURE (or timeout).
    async fn poll(&self, task_id: &str) -> anyhow::Result<Value> {
        let deadline = Instant::now() + POLL_TIMEOUT;

        loop {
            if Instant::now() > deadline {
                anyhow::bail!(
                    "Veo task {} timed out after {}s",
                    task_id,
                    POLL_TIMEOUT.as_secs()
                );
            }

            tokio::time::sleep(POLL_INTERVAL).await;

            let resp = self
                .http
                .get(format!(
                    "{}/v2/videos/generations/{}",
                    self.base_url, task_id
                ))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .send()
                .await?;

            let status = resp.status();
            let text = resp.text().await?;

            if !status.is_success() {
                anyhow::bail!("Veo poll returned {}: {}", status, text);
            }

            let payload: Value = serde_json::from_str(&text)?;
            let task_status = payload["status"].as_str().unwrap_or("");

            match task_status {
                "SUCCESS" => return Ok(payload),
                "FAILURE" => {
                    let reason = payload["fail_reason"].as_str().unwrap_or("unknown");
                    anyhow::bail!("Veo task {} failed: {}", task_id, reason);
                }
                _ => continue, // NOT_START, IN_PROGRESS
            }
        }
    }
}

#[async_trait::async_trait]
impl AssetProvider for VeoProvider {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        "Veo (Google)"
    }

    fn asset_types(&self) -> &[AssetType] {
        &[AssetType::Video]
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_transparency: false,
            priority: 100,
            ..Default::default()
        }
    }

    async fn generate(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        let prompt = req.prompt.as_deref().unwrap_or("");
        let model = req.model.as_deref().unwrap_or(DEFAULT_MODEL);
        let start = Instant::now();

        // Collect images: input_file first, then reference_images
        let image_inputs = req.image_inputs();
        let images: Vec<&str> = image_inputs.into_iter().collect();

        let mut body = json!({
            "prompt": prompt,
            "model": model,
        });

        if !images.is_empty() {
            body["images"] = json!(images);
        }

        // Map size to aspect_ratio if provided
        if let Some(size_str) = req.params.get("size").and_then(|v| v.as_str()) {
            if let Some(ar) = Self::map_aspect_ratio(size_str) {
                body["aspect_ratio"] = json!(ar);
            }
        }

        // Pass through aspect_ratio if explicitly set
        if let Some(ar) = req.params.get("aspect_ratio").and_then(|v| v.as_str()) {
            body["aspect_ratio"] = json!(ar);
        }

        if let Some(enhance) = req.params.get("enhance_prompt").and_then(|v| v.as_bool()) {
            body["enhance_prompt"] = json!(enhance);
        }

        // Submit → poll → extract output URL
        let task_id = self.submit(&body).await?;
        let result = self.poll(&task_id).await?;

        let output_url = result["data"]["output"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| {
                anyhow::anyhow!("Veo task {} completed but no output URL found", task_id)
            })?;

        let cost = match model {
            "veo3.1-pro" => 0.50,
            "veo3.1-components" => 0.10,
            _ => 0.08, // veo3.1 fast
        };

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url: Some(output_url),
            output_data: None,
            metadata: json!({
                "model": model,
                "asset_type": "video",
                "task_id": task_id,
                "has_reference_images": !images.is_empty(),
                "image_count": images.len(),
            }),
            cost_usd: Some(cost),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();
        // Use the models endpoint to verify connectivity
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
    fn map_aspect_ratio_landscape() {
        assert_eq!(VeoProvider::map_aspect_ratio("1920x1080"), Some("16:9"));
        assert_eq!(VeoProvider::map_aspect_ratio("1280x720"), Some("16:9"));
    }

    #[test]
    fn map_aspect_ratio_portrait() {
        assert_eq!(VeoProvider::map_aspect_ratio("720x1280"), Some("9:16"));
        assert_eq!(VeoProvider::map_aspect_ratio("1080x1920"), Some("9:16"));
    }

    #[test]
    fn map_aspect_ratio_square() {
        assert_eq!(VeoProvider::map_aspect_ratio("1024x1024"), None);
    }

    #[test]
    fn map_aspect_ratio_invalid() {
        assert_eq!(VeoProvider::map_aspect_ratio("bad"), None);
        assert_eq!(VeoProvider::map_aspect_ratio("0x0"), None);
    }
}
