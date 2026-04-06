use std::time::{Duration, Instant};

use crate::core::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};

/// WorldLabs Marble provider — text/image-to-3D world via WorldLabs Marble API.
pub struct WorldLabsProvider {
    pub id: String,
    pub api_key: String,
    pub base_url: String,
    http: reqwest::Client,
}

impl WorldLabsProvider {
    pub fn new(api_key: String) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(600))
            .build()
            .expect("failed to build WorldLabs HTTP client");
        Self {
            id: "worldlabs".into(),
            api_key,
            base_url: "https://api.worldlabs.ai/marble/v1".into(),
            http,
        }
    }

    #[allow(dead_code)]
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    /// Upload an image to WorldLabs and return the media_asset id.
    async fn upload_image(&self, image_bytes: &[u8]) -> anyhow::Result<String> {
        // Step 1: Prepare upload
        let prepare_resp = self
            .http
            .post(format!("{}/media-assets:prepare_upload", self.base_url))
            .header("WLT-Api-Key", &self.api_key)
            .json(&json!({
                "media_type": "image/png",
                "display_name": "input.png"
            }))
            .send()
            .await?;

        let status = prepare_resp.status();
        if !status.is_success() {
            let text = prepare_resp.text().await?;
            anyhow::bail!(
                "WorldLabs /media-assets:prepare_upload returned {}: {}",
                status,
                text
            );
        }

        let data: Value = prepare_resp.json().await?;
        let media_asset_id = data["media_asset"]["id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing media_asset.id in prepare_upload response"))?
            .to_string();
        let upload_uri = data["media_asset"]["upload"]["uri"]
            .as_str()
            .ok_or_else(|| {
                anyhow::anyhow!("missing media_asset.upload.uri in prepare_upload response")
            })?;

        // Step 2: PUT image bytes to the signed upload URI
        let put_resp = self
            .http
            .put(upload_uri)
            .header(reqwest::header::CONTENT_TYPE, "image/png")
            .body(image_bytes.to_vec())
            .send()
            .await?;

        let put_status = put_resp.status();
        if !put_status.is_success() {
            let text = put_resp.text().await?;
            anyhow::bail!(
                "WorldLabs image upload PUT returned {}: {}",
                put_status,
                text
            );
        }

        Ok(media_asset_id)
    }
}

#[async_trait::async_trait]
impl AssetProvider for WorldLabsProvider {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        "WorldLabs (Marble 3D World)"
    }

    fn asset_types(&self) -> &[AssetType] {
        &[AssetType::World]
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            max_concurrent: 2,
            priority: 100,
            ..Default::default()
        }
    }

    async fn generate(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        let prompt = req.prompt.as_deref().unwrap_or("").trim();
        if prompt.is_empty() {
            anyhow::bail!("WorldLabs requires a non-empty prompt");
        }

        let model = req.model.as_deref().unwrap_or("marble-1.1");

        let display_name = req
            .params
            .get("display_name")
            .and_then(Value::as_str)
            .unwrap_or("asset-gateway-world");

        let start = Instant::now();

        // Build world_prompt based on whether an input image is provided
        let world_prompt = if let Some(input) = req.input_file.as_deref() {
            // Image mode — resolve image bytes
            let image_bytes = if input.starts_with("data:") {
                let raw = input.splitn(2, ',').nth(1).unwrap_or(input);
                STANDARD.decode(raw)?
            } else if input.starts_with("http://") || input.starts_with("https://") {
                tracing::debug!(url = input, "downloading image for WorldLabs");
                let dl = self.http.get(input).send().await?;
                if !dl.status().is_success() {
                    anyhow::bail!("failed to download input image: HTTP {}", dl.status());
                }
                dl.bytes().await?.to_vec()
            } else {
                // Raw base64
                STANDARD.decode(input)?
            };

            let media_asset_id = self.upload_image(&image_bytes).await?;
            tracing::debug!(media_asset_id, "uploaded image to WorldLabs");

            json!({
                "type": "image",
                "media_asset_id": media_asset_id
            })
        } else {
            // Text mode
            json!({
                "type": "text",
                "text_prompt": prompt
            })
        };

        // Submit generation
        let gen_body = json!({
            "display_name": display_name,
            "model": model,
            "world_prompt": world_prompt,
        });

        let resp = self
            .http
            .post(format!("{}/worlds:generate", self.base_url))
            .header("WLT-Api-Key", &self.api_key)
            .json(&gen_body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await?;
            anyhow::bail!("WorldLabs /worlds:generate returned {}: {}", status, text);
        }

        let submit: Value = resp.json().await?;
        let operation_id = submit["operation_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing operation_id in /worlds:generate response"))?
            .to_string();

        tracing::debug!(operation_id, "WorldLabs generation submitted");

        // Poll for completion: 10s interval, max 60 iterations (600s)
        let poll_url = format!("{}/operations/{}", self.base_url, operation_id);
        let mut world_id: Option<String> = None;

        for iteration in 1..=60 {
            tokio::time::sleep(Duration::from_secs(10)).await;

            let poll_resp = self
                .http
                .get(&poll_url)
                .header("WLT-Api-Key", &self.api_key)
                .send()
                .await?;

            let poll_status = poll_resp.status();
            if !poll_status.is_success() {
                let text = poll_resp.text().await?;
                anyhow::bail!(
                    "WorldLabs /operations poll returned {}: {}",
                    poll_status,
                    text
                );
            }

            let op: Value = poll_resp.json().await?;

            tracing::debug!(
                operation_id,
                iteration,
                done = op["done"].as_bool().unwrap_or(false),
                "polling operation"
            );

            if let Some(err) = op.get("error") {
                let msg = err["message"]
                    .as_str()
                    .or_else(|| err.as_str())
                    .unwrap_or("unknown error");
                anyhow::bail!("WorldLabs operation {} failed: {}", operation_id, msg);
            }

            if op["done"].as_bool() == Some(true) {
                world_id = op["response"]["world_id"].as_str().map(String::from);
                break;
            }
        }

        let world_id = world_id.ok_or_else(|| {
            anyhow::anyhow!("WorldLabs operation {} timed out after 600s", operation_id)
        })?;

        tracing::debug!(world_id, "WorldLabs world generation complete");

        // Fetch world details
        let world_resp = self
            .http
            .get(format!("{}/worlds/{}", self.base_url, world_id))
            .header("WLT-Api-Key", &self.api_key)
            .send()
            .await?;

        let world_status = world_resp.status();
        if !world_status.is_success() {
            let text = world_resp.text().await?;
            anyhow::bail!(
                "WorldLabs /worlds/{} returned {}: {}",
                world_id,
                world_status,
                text
            );
        }

        let world: Value = world_resp.json().await?;
        let assets = world["assets"].as_array();

        // Find the full_res gaussian_splat (SPZ) asset
        let full_res_asset = assets
            .and_then(|arr| {
                arr.iter().find(|a| {
                    a["type"].as_str() == Some("gaussian_splat")
                        && a["variant"].as_str() == Some("full_res")
                })
            })
            .or_else(|| {
                // Fallback: largest gaussian_splat by size_bytes
                assets.and_then(|arr| {
                    arr.iter()
                        .filter(|a| a["type"].as_str() == Some("gaussian_splat"))
                        .max_by_key(|a| a["size_bytes"].as_u64().unwrap_or(0))
                })
            });

        let spz_url = full_res_asset
            .and_then(|a| a["url"].as_str())
            .ok_or_else(|| {
                anyhow::anyhow!("no gaussian_splat asset found in world {}", world_id)
            })?;

        // Download the SPZ file
        let dl_resp = self.http.get(spz_url).send().await?;
        if !dl_resp.status().is_success() {
            anyhow::bail!("WorldLabs SPZ download failed with {}", dl_resp.status());
        }
        let spz_bytes = dl_resp.bytes().await?;
        let spz_b64 = STANDARD.encode(&spz_bytes);

        // Collect all asset URLs for metadata
        let mut asset_urls = json!({});
        if let Some(arr) = assets {
            for asset in arr {
                let variant = asset["variant"].as_str().unwrap_or("unknown");
                let asset_type = asset["type"].as_str().unwrap_or("unknown");
                let key = format!("{}_{}", asset_type, variant);
                if let Some(url) = asset["url"].as_str() {
                    asset_urls[key] = json!(url);
                }
            }
        }

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url: Some(spz_url.to_string()),
            output_data: Some(spz_b64),
            metadata: json!({
                "world_id": world_id,
                "model": model,
                "operation_id": operation_id,
                "assets": asset_urls,
            }),
            cost_usd: Some(0.50),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();
        let resp = self
            .http
            .get(format!("{}/worlds", self.base_url))
            .header("WLT-Api-Key", &self.api_key)
            .query(&[("page_size", "1")])
            .timeout(Duration::from_secs(10))
            .send()
            .await;

        match resp {
            Ok(r) if r.status().is_success() => Ok(HealthStatus {
                healthy: true,
                latency_ms: Some(start.elapsed().as_millis() as u64),
                message: Some("WorldLabs API reachable".into()),
            }),
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
