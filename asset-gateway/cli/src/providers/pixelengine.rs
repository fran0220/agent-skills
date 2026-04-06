use std::time::{Duration, Instant};

use crate::core::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};

/// PixelEngine sprite animation provider — image-to-animation via PixelEngine AI.
pub struct PixelEngineProvider {
    pub id: String,
    pub api_key: String,
    pub base_url: String,
    http: reqwest::Client,
}

impl PixelEngineProvider {
    pub fn new(api_key: String) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(180))
            .build()
            .expect("failed to build PixelEngine HTTP client");
        Self {
            id: "pixelengine".into(),
            api_key,
            base_url: "https://api.pixelengine.ai/functions/v1".into(),
            http,
        }
    }

    #[allow(dead_code)]
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    /// Call POST /enhance-prompt to rewrite a prompt for the generation model.
    pub async fn enhance_prompt(
        &self,
        prompt: &str,
        image: Option<&str>,
        model: Option<&str>,
    ) -> anyhow::Result<String> {
        let mut body = json!({ "prompt": prompt });
        if let Some(img) = image {
            body["image"] = json!(img);
        }
        if let Some(m) = model {
            body["model"] = json!(m);
        }

        let resp = self
            .http
            .post(format!("{}/enhance-prompt", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await?;
            anyhow::bail!("PixelEngine enhance-prompt returned {}: {}", status, text);
        }

        let data: Value = resp.json().await?;
        data["enhanced_prompt"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("missing enhanced_prompt in response"))
    }
}

#[async_trait::async_trait]
impl AssetProvider for PixelEngineProvider {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        "PixelEngine (Sprite Animation)"
    }

    fn asset_types(&self) -> &[AssetType] {
        &[AssetType::Sprite]
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            max_concurrent: 3,
            priority: 100,
            ..Default::default()
        }
    }

    async fn generate(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        let prompt = req.prompt.as_deref().unwrap_or("").trim();
        if prompt.is_empty() {
            anyhow::bail!("PixelEngine requires a non-empty prompt");
        }

        let image = req
            .input_file
            .as_deref()
            .or_else(|| req.params.get("image").and_then(Value::as_str))
            .ok_or_else(|| anyhow::anyhow!("PixelEngine requires an input image (input_file or params.image)"))?;

        let model = req
            .model
            .as_deref()
            .or_else(|| req.params.get("model").and_then(Value::as_str))
            .unwrap_or("pixel-engine-v1.1");

        let output_format = req
            .params
            .get("output_format")
            .and_then(Value::as_str)
            .unwrap_or("spritesheet");

        let mut body = json!({
            "image": image,
            "prompt": prompt,
            "model": model,
            "output_format": output_format,
        });

        if let Some(neg) = req.params.get("negative_prompt").and_then(Value::as_str) {
            body["negative_prompt"] = json!(neg);
        }
        if let Some(pc) = req.params.get("pixel_config") {
            body["pixel_config"] = pc.clone();
        }
        if let Some(frames) = req.params.get("output_frames").and_then(Value::as_u64) {
            body["output_frames"] = json!(frames);
        }
        if let Some(seed) = req.params.get("seed").and_then(Value::as_u64) {
            body["seed"] = json!(seed);
        }
        if let Some(matte) = req.params.get("matte_color").and_then(Value::as_str) {
            body["matte_color"] = json!(matte);
        }

        let start = Instant::now();

        // Submit animation job
        let resp = self
            .http
            .post(format!("{}/animate", self.base_url))
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await?;
            anyhow::bail!("PixelEngine /animate returned {}: {}", status, text);
        }

        let submit: Value = resp.json().await?;
        let job_id = submit["api_job_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing api_job_id in /animate response"))?;

        tracing::debug!(job_id, "PixelEngine animation job submitted");

        // Poll for completion: 4s interval, max 40 iterations (160s)
        let poll_url = format!("{}/jobs?id={}", self.base_url, job_id);
        let mut output: Option<Value> = None;

        for iteration in 1..=40 {
            tokio::time::sleep(Duration::from_secs(4)).await;

            let poll_resp = self
                .http
                .get(&poll_url)
                .bearer_auth(&self.api_key)
                .send()
                .await?;

            let poll_status = poll_resp.status();
            if !poll_status.is_success() {
                let text = poll_resp.text().await?;
                anyhow::bail!("PixelEngine /jobs poll returned {}: {}", poll_status, text);
            }

            let job: Value = poll_resp.json().await?;
            let job_status = job["status"].as_str().unwrap_or("unknown");

            tracing::debug!(job_id, iteration, status = job_status, "polling job");

            match job_status {
                "success" => {
                    output = Some(job);
                    break;
                }
                "failure" => {
                    let err = job["error"]
                        .as_str()
                        .unwrap_or("unknown error");
                    anyhow::bail!("PixelEngine job {} failed: {}", job_id, err);
                }
                "cancelled" => {
                    anyhow::bail!("PixelEngine job {} was cancelled", job_id);
                }
                "queued" | "pending" => continue,
                other => {
                    tracing::debug!(job_id, status = other, "unexpected job status, continuing poll");
                    continue;
                }
            }
        }

        let job = output.ok_or_else(|| {
            anyhow::anyhow!("PixelEngine job {} timed out after 160s", job_id)
        })?;

        // Download the output asset
        let download_url = job["output"]["url"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing output.url in completed job"))?;

        let dl_resp = self.http.get(download_url).send().await?;
        if !dl_resp.status().is_success() {
            anyhow::bail!(
                "PixelEngine download failed with {}",
                dl_resp.status()
            );
        }

        let content_type = dl_resp
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);

        let bytes = dl_resp.bytes().await?;
        let b64 = STANDARD.encode(&bytes);

        let metadata_output = &job["output"]["metadata"];
        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url: Some(download_url.to_string()),
            output_data: Some(b64),
            metadata: json!({
                "job_id": job_id,
                "model": model,
                "output_format": output_format,
                "content_type": content_type.unwrap_or_else(|| "image/png".to_string()),
                "frame_count": metadata_output.get("frame_count"),
                "frame_w": metadata_output.get("frame_w"),
                "frame_h": metadata_output.get("frame_h"),
                "width": metadata_output.get("width"),
                "height": metadata_output.get("height"),
                "fps": metadata_output.get("fps"),
            }),
            cost_usd: Some(0.10),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();
        let resp = self
            .http
            .get(format!("{}/balance", self.base_url))
            .bearer_auth(&self.api_key)
            .timeout(Duration::from_secs(10))
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => {
                let body: Value = r.json().await.unwrap_or_default();
                let available = body["available"].as_f64().unwrap_or(0.0);
                Ok(HealthStatus {
                    healthy: available > 0.0,
                    latency_ms: Some(start.elapsed().as_millis() as u64),
                    message: if available > 0.0 {
                        Some(format!("{} credits available", available))
                    } else {
                        Some("no credits available".into())
                    },
                })
            }
            Ok(r) => Ok(HealthStatus {
                healthy: false,
                latency_ms: Some(start.elapsed().as_millis() as u64),
                message: Some(format!("HTTP {}", r.status())),
            }),
            Err(e) => Ok(HealthStatus {
                healthy: false,
                latency_ms: Some(start.elapsed().as_millis() as u64),
                message: Some(e.to_string()),
            }),
        }
    }
}
