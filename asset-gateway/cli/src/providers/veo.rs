use std::time::{Duration, Instant};

use crate::core::*;
use crate::vertex_auth::VertexAuth;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};

const MODEL: &str = "veo-3.1-lite-generate-001";
const LOCATION: &str = "us-central1";
const POLL_INTERVAL: Duration = Duration::from_secs(10);
const POLL_TIMEOUT: Duration = Duration::from_secs(300);

/// Veo video provider (Google) — general video generation via Vertex AI direct access.
///
/// Supports text-to-video and image-to-video using Veo 3.1 Lite.
/// Uses predictLongRunning + fetchPredictOperation pattern.
pub struct VeoProvider {
    pub id: String,
    vertex_auth: VertexAuth,
    vertex_project: String,
    http: reqwest::Client,
}

impl VeoProvider {
    pub fn new(auth: VertexAuth, project: String) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(600))
            .build()
            .expect("failed to build Veo HTTP client");
        Self {
            id: "veo".into(),
            vertex_auth: auth,
            vertex_project: project,
            http,
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
            Some("16:9") // square defaults to landscape
        }
    }

    /// Fetch an image URL and return (base64, mime_type).
    async fn fetch_image_b64(&self, url: &str) -> anyhow::Result<(String, String)> {
        if url.starts_with("data:") {
            let (header, data) = url
                .split_once(";base64,")
                .ok_or_else(|| anyhow::anyhow!("invalid data URI"))?;
            let mime = header.strip_prefix("data:").unwrap_or("image/png");
            return Ok((data.to_string(), mime.to_string()));
        }
        let resp = self.http.get(url).send().await?;
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/png")
            .split(';')
            .next()
            .unwrap_or("image/png")
            .to_string();
        let bytes = resp.bytes().await?;
        Ok((STANDARD.encode(&bytes), content_type))
    }

    /// Submit a predictLongRunning request. Returns the operation name.
    async fn submit(&self, instance: Value, params: Value) -> anyhow::Result<String> {
        let body = json!({
            "instances": [instance],
            "parameters": params,
        });

        let token = self.vertex_auth.access_token().await?;
        let resp = self
            .http
            .post(format!(
                "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:predictLongRunning",
                LOCATION, self.vertex_project, LOCATION, MODEL
            ))
            .header("Authorization", format!("Bearer {}", token))
            .timeout(Duration::from_secs(30))
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("Veo submit returned {}: {}", status, text);
        }

        let payload: Value = serde_json::from_str(&text)?;
        payload["name"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("Veo submit response missing operation name: {}", text))
    }

    /// Poll via fetchPredictOperation until done. Returns the response payload.
    async fn poll(&self, operation_name: &str) -> anyhow::Result<Value> {
        let deadline = Instant::now() + POLL_TIMEOUT;

        loop {
            if Instant::now() > deadline {
                anyhow::bail!(
                    "Veo timed out after {}s: {}",
                    POLL_TIMEOUT.as_secs(),
                    operation_name
                );
            }

            tokio::time::sleep(POLL_INTERVAL).await;

            let token = self.vertex_auth.access_token().await?;
            let resp = self
                .http
                .post(format!(
                    "https://{}-aiplatform.googleapis.com/v1beta1/projects/{}/locations/{}/publishers/google/models/{}:fetchPredictOperation",
                    LOCATION, self.vertex_project, LOCATION, MODEL
                ))
                .header("Authorization", format!("Bearer {}", token))
                .json(&json!({"operationName": operation_name}))
                .timeout(Duration::from_secs(30))
                .send()
                .await?;

            let status = resp.status();
            let text = resp.text().await?;
            if !status.is_success() {
                anyhow::bail!("Veo poll returned {}: {}", status, text);
            }

            let payload: Value = serde_json::from_str(&text)?;

            if !payload["done"].as_bool().unwrap_or(false) {
                continue;
            }

            if let Some(error) = payload.get("error") {
                let msg = error["message"].as_str().unwrap_or("unknown error");
                anyhow::bail!("Veo operation failed: {}", msg);
            }

            return Ok(payload);
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
        "Veo (Vertex AI)"
    }

    fn asset_types(&self) -> &[AssetType] {
        &[AssetType::Video]
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_transparency: false,
            priority: 70,
            ..Default::default()
        }
    }

    async fn generate(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        let prompt = req.prompt.as_deref().unwrap_or("");
        let start = Instant::now();

        // Build aspect ratio
        let aspect_ratio = req
            .params
            .get("aspect_ratio")
            .and_then(|v| v.as_str())
            .map(String::from)
            .or_else(|| {
                req.params
                    .get("size")
                    .and_then(|v| v.as_str())
                    .and_then(Self::map_aspect_ratio)
                    .map(String::from)
            })
            .unwrap_or_else(|| "16:9".into());

        let duration = req
            .params
            .get("duration")
            .and_then(|v| v.as_u64())
            .unwrap_or(4) as u32;

        // Build instance: text-to-video or image-to-video
        let image_inputs = req.image_inputs();
        let mut instance = json!({"prompt": prompt});

        if let Some(first_image) = image_inputs.first() {
            let (b64, mime) = self.fetch_image_b64(first_image).await?;
            instance["image"] = json!({
                "bytesBase64Encoded": b64,
                "mimeType": mime,
            });
        }

        let params = json!({
            "sampleCount": 1,
            "durationSeconds": duration,
            "aspectRatio": aspect_ratio,
            "generateAudio": true,
            "resolution": "720p",
        });

        // Submit → poll
        let operation_name = self.submit(instance, params).await?;
        tracing::info!(operation = %operation_name, "Veo: operation started");

        let result = self.poll(&operation_name).await?;

        // Extract video — either base64 inline or GCS URI
        let videos = result["response"]["videos"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Veo response missing videos array"))?;

        let video = videos
            .first()
            .ok_or_else(|| anyhow::anyhow!("Veo response has empty videos array"))?;

        let (output_url, output_data) = if let Some(b64) = video["bytesBase64Encoded"].as_str() {
            (None, Some(b64.to_string()))
        } else if let Some(uri) = video["gcsUri"].as_str() {
            (Some(uri.to_string()), None)
        } else {
            anyhow::bail!("Veo response has no video data");
        };

        // Veo Lite 720p: $0.03/s (video-only) or $0.05/s (with audio)
        let cost = 0.05 * duration as f64;

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url,
            output_data,
            metadata: json!({
                "model": MODEL,
                "asset_type": "video",
                "operation": operation_name,
                "aspect_ratio": aspect_ratio,
                "duration": duration,
                "has_reference_image": !image_inputs.is_empty(),
                "image_count": image_inputs.len(),
            }),
            cost_usd: Some(cost),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();
        let token = self.vertex_auth.access_token().await;

        match token {
            Ok(t) => {
                let resp = self
                    .http
                    .get(format!(
                        "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models",
                        LOCATION, self.vertex_project, LOCATION
                    ))
                    .header("Authorization", format!("Bearer {}", t))
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
            Err(e) => Ok(HealthStatus {
                healthy: false,
                latency_ms: Some(start.elapsed().as_millis() as u64),
                message: Some(format!("Vertex AI auth failed: {e}")),
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
        assert_eq!(VeoProvider::map_aspect_ratio("1024x1024"), Some("16:9"));
    }

    #[test]
    fn map_aspect_ratio_invalid() {
        assert_eq!(VeoProvider::map_aspect_ratio("bad"), None);
        assert_eq!(VeoProvider::map_aspect_ratio("0x0"), None);
    }
}
