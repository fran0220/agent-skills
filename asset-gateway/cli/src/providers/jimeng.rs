use std::time::{Duration, Instant};

use crate::core::*;
use serde_json::{json, Value};

const DEFAULT_IMAGE_MODEL: &str = "jimeng-5.0";
const DEFAULT_VIDEO_MODEL: &str = "seedance-2.0-fast-vip";
const VIDEO_TIMEOUT: Duration = Duration::from_secs(600);

/// Jimeng provider (ByteDance) — image and video generation via jimeng-api.
///
/// Models:
/// - Image: `jimeng-5.0` (default)
/// - Video: `seedance-2.0-fast-vip` (default), `seedance-2.0-vip`
pub struct JimengProvider {
    pub id: String,
    pub base_url: String,
    pub token: String,
    http: reqwest::Client,
}

impl JimengProvider {
    pub fn new(base_url: String, token: String) -> Self {
        Self {
            id: "jimeng".into(),
            base_url,
            token,
            http: reqwest::Client::new(),
        }
    }

    async fn generate_image(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        let prompt = req.prompt.as_deref().unwrap_or("");
        let model = req.model.as_deref().unwrap_or(DEFAULT_IMAGE_MODEL);
        let start = Instant::now();

        let ratio = req
            .params
            .get("ratio")
            .and_then(|v| v.as_str())
            .unwrap_or("1:1");
        let resolution = req
            .params
            .get("resolution")
            .and_then(|v| v.as_str())
            .unwrap_or("2k");

        let body = json!({
            "model": model,
            "prompt": prompt,
            "ratio": ratio,
            "resolution": resolution,
        });

        let resp = self
            .http
            .post(format!("{}/v1/images/generations", self.base_url))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("Jimeng image returned {}: {}", status, text);
        }

        let payload: Value = serde_json::from_str(&text)?;
        let url = payload["data"][0]["url"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("Jimeng image response missing data[0].url: {}", text))?;

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url: Some(url),
            output_data: None,
            metadata: json!({
                "model": model,
                "asset_type": "image",
                "ratio": ratio,
                "resolution": resolution,
            }),
            cost_usd: Some(0.01),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn generate_video(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        let model = req.model.as_deref().unwrap_or(DEFAULT_VIDEO_MODEL);
        let start = Instant::now();

        let ratio = req
            .params
            .get("ratio")
            .and_then(|v| v.as_str())
            .unwrap_or("4:3");
        let duration = req
            .params
            .get("duration")
            .and_then(|v| v.as_u64())
            .unwrap_or(5);

        let image_inputs = req.image_inputs();

        // If prompt doesn't contain @1 and there are images, prepend "@1 "
        let prompt = {
            let raw = req.prompt.as_deref().unwrap_or("");
            if !image_inputs.is_empty() && !raw.contains("@1") {
                format!("@1 {}", raw)
            } else {
                raw.to_string()
            }
        };

        let mut body = json!({
            "model": model,
            "prompt": prompt,
            "ratio": ratio,
            "duration": duration,
        });

        if !image_inputs.is_empty() {
            let urls: Vec<&str> = image_inputs.into_iter().collect();
            body["file_paths"] = json!(urls);
        }

        let client = reqwest::Client::builder()
            .timeout(VIDEO_TIMEOUT)
            .build()?;

        let resp = client
            .post(format!("{}/v1/videos/generations", self.base_url))
            .header("Authorization", format!("Bearer {}", self.token))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("Jimeng video returned {}: {}", status, text);
        }

        let payload: Value = serde_json::from_str(&text)?;
        let url = payload["data"][0]["url"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("Jimeng video response missing data[0].url: {}", text))?;

        let cost = if model.contains("fast-vip") {
            0.05
        } else {
            0.10
        };

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url: Some(url),
            output_data: None,
            metadata: json!({
                "model": model,
                "asset_type": "video",
                "ratio": ratio,
                "duration": duration,
            }),
            cost_usd: Some(cost),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }
}

#[async_trait::async_trait]
impl AssetProvider for JimengProvider {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        "Jimeng (ByteDance)"
    }

    fn asset_types(&self) -> &[AssetType] {
        &[AssetType::Image, AssetType::Video]
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            priority: 90,
            ..Default::default()
        }
    }

    async fn generate(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        match req.asset_type {
            AssetType::Image => self.generate_image(req).await,
            AssetType::Video => self.generate_video(req).await,
            other => anyhow::bail!("Jimeng does not support asset type: {}", other),
        }
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();
        let resp = self
            .http
            .get(format!("{}/ping", self.base_url))
            .send()
            .await;

        match resp {
            Ok(r) => {
                let body = r.text().await.unwrap_or_default();
                Ok(HealthStatus {
                    healthy: body.trim() == "pong",
                    latency_ms: Some(start.elapsed().as_millis() as u64),
                    message: None,
                })
            }
            Err(e) => Ok(HealthStatus {
                healthy: false,
                latency_ms: Some(start.elapsed().as_millis() as u64),
                message: Some(e.to_string()),
            }),
        }
    }
}
