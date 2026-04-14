use std::time::{Duration, Instant};

use crate::core::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};

const BASE_URL: &str = "https://www.autosprite.io";
const POLL_INTERVAL: Duration = Duration::from_secs(5);
const POLL_TIMEOUT: Duration = Duration::from_secs(180);

const PRESET_KINDS: &[&str] = &[
    "walk", "run", "idle", "jump", "attack", "death", "cast", "dance", "wave", "interact",
];

/// AutoSprite provider — dedicated sprite generation with consistent frame animations.
///
/// Generates game-ready spritesheets via AutoSprite's character + spritesheet API.
/// Supports 12 art styles and preset/custom animation kinds.
pub struct AutoSpriteProvider {
    pub id: String,
    api_key: String,
    http: reqwest::Client,
}

impl AutoSpriteProvider {
    pub fn new(api_key: String) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(300))
            .build()
            .expect("failed to build AutoSprite HTTP client");
        Self {
            id: "autosprite".into(),
            api_key,
            http,
        }
    }

    /// Create a character from a text prompt.
    async fn create_character_from_text(
        &self,
        name: &str,
        prompt: &str,
        is_humanoid: bool,
    ) -> anyhow::Result<String> {
        let body = json!({
            "name": name,
            "prompt": prompt,
            "isHumanoid": is_humanoid,
        });

        let resp = self
            .http
            .post(format!("{}/api/v1/characters", BASE_URL))
            .header("x-api-key", &self.api_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("AutoSprite create character returned {}: {}", status, text);
        }

        let data: Value = serde_json::from_str(&text)?;
        data["id"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("missing id in character response: {}", text))
    }

    /// Create a character from an image URL.
    async fn create_character_from_image(
        &self,
        name: &str,
        image_url: &str,
        description: &str,
        is_humanoid: bool,
    ) -> anyhow::Result<String> {
        let body = json!({
            "name": name,
            "imageUrl": image_url,
            "characterDescription": description,
            "isHumanoid": is_humanoid,
        });

        let resp = self
            .http
            .post(format!("{}/api/v1/characters", BASE_URL))
            .header("x-api-key", &self.api_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!(
                "AutoSprite create character (image) returned {}: {}",
                status,
                text
            );
        }

        let data: Value = serde_json::from_str(&text)?;
        data["id"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("missing id in character response: {}", text))
    }

    /// Generate a spritesheet for a character. Returns the job ID.
    async fn generate_spritesheet(
        &self,
        character_id: &str,
        animation_type: &str,
        frame_count: u64,
        frame_size: u64,
    ) -> anyhow::Result<String> {
        let kind = if PRESET_KINDS.contains(&animation_type) {
            animation_type
        } else {
            "custom"
        };

        let mut anim = json!({
            "kind": kind,
            "name": animation_type,
        });
        if kind == "custom" {
            anim["prompt"] = json!(animation_type);
        }

        let body = json!({
            "animations": [anim],
            "frameCount": frame_count,
            "frameSize": frame_size,
        });

        let resp = self
            .http
            .post(format!(
                "{}/api/v1/characters/{}/spritesheets",
                BASE_URL, character_id
            ))
            .header("x-api-key", &self.api_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!(
                "AutoSprite generate spritesheet returned {}: {}",
                status,
                text
            );
        }

        let data: Value = serde_json::from_str(&text)?;
        data["workflows"][0]["jobId"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("missing jobId in spritesheet response: {}", text))
    }

    /// Poll a job until it succeeds or fails. Returns spritesheet IDs.
    async fn poll_job(&self, job_id: &str) -> anyhow::Result<Vec<String>> {
        let deadline = Instant::now() + POLL_TIMEOUT;

        loop {
            if Instant::now() > deadline {
                anyhow::bail!(
                    "AutoSprite job {} timed out after {:?}",
                    job_id,
                    POLL_TIMEOUT
                );
            }

            tokio::time::sleep(POLL_INTERVAL).await;

            let resp = self
                .http
                .get(format!("{}/api/v1/jobs/{}", BASE_URL, job_id))
                .header("x-api-key", &self.api_key)
                .send()
                .await?;

            let data: Value = resp.json().await?;
            let status = data["status"].as_str().unwrap_or("");
            tracing::debug!(job_id, status, "AutoSprite: polling job");

            match status {
                "succeeded" => {
                    let ids = data["spritesheetIds"]
                        .as_array()
                        .map(|arr| {
                            arr.iter()
                                .filter_map(|v| v.as_str().map(String::from))
                                .collect()
                        })
                        .unwrap_or_default();
                    return Ok(ids);
                }
                "failed" => {
                    anyhow::bail!("AutoSprite job {} failed: {}", job_id, data);
                }
                _ => continue,
            }
        }
    }

    /// Get spritesheet details.
    async fn get_spritesheet(&self, spritesheet_id: &str) -> anyhow::Result<Value> {
        let resp = self
            .http
            .get(format!(
                "{}/api/v1/spritesheets/{}",
                BASE_URL, spritesheet_id
            ))
            .header("x-api-key", &self.api_key)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("AutoSprite get spritesheet returned {}: {}", status, text);
        }

        Ok(serde_json::from_str(&text)?)
    }

    /// Download an image URL and return base64-encoded data.
    async fn download_as_base64(&self, url: &str) -> anyhow::Result<String> {
        let bytes = self.http.get(url).send().await?.bytes().await?;
        Ok(STANDARD.encode(&bytes))
    }

    /// Delete a character (cleanup).
    async fn delete_character(&self, character_id: &str) {
        let _ = self
            .http
            .delete(format!("{}/api/v1/characters/{}", BASE_URL, character_id))
            .header("x-api-key", &self.api_key)
            .send()
            .await;
    }
}

#[async_trait::async_trait]
impl AssetProvider for AutoSpriteProvider {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        "AutoSprite"
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
        let prompt = req.prompt.as_deref().unwrap_or("");
        let start = Instant::now();

        let animation_type = req
            .params
            .get("animation_type")
            .and_then(|v| v.as_str())
            .unwrap_or("walk");
        let frame_count = req
            .params
            .get("frame_count")
            .and_then(|v| v.as_u64())
            .unwrap_or(8);
        let frame_size = req
            .params
            .get("frame_size")
            .and_then(|v| v.as_u64())
            .unwrap_or(256);
        let is_humanoid = req
            .params
            .get("is_humanoid")
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        let style = req.style().unwrap_or("pixel_art");

        let char_name = format!("asset-gw-{}", chrono::Utc::now().timestamp_millis());

        // Step A: Create character
        let character_id = if let Some(input) = req.input_file.as_deref() {
            self.create_character_from_image(&char_name, input, prompt, is_humanoid)
                .await
        } else {
            self.create_character_from_text(&char_name, prompt, is_humanoid)
                .await
        };

        let character_id = match character_id {
            Ok(id) => id,
            Err(e) => return Err(e),
        };

        // From here on, ensure cleanup on failure
        let result = async {
            // Step B: Generate spritesheet
            let job_id = self
                .generate_spritesheet(&character_id, animation_type, frame_count, frame_size)
                .await?;

            // Step C: Poll job
            let spritesheet_ids = self.poll_job(&job_id).await?;
            let ss_id = spritesheet_ids
                .first()
                .ok_or_else(|| anyhow::anyhow!("no spritesheet IDs returned from job"))?;

            // Step D: Get spritesheet details and download
            let ss = self.get_spritesheet(ss_id).await?;
            let sheet_url = ss["sheetUrl"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("missing sheetUrl in spritesheet"))?;
            let atlas_url = ss["atlasUrl"].as_str().map(String::from);

            let output_data = self.download_as_base64(sheet_url).await?;

            Ok(GenerateResponse {
                provider_id: self.id.clone(),
                output_path: None,
                output_url: Some(sheet_url.to_string()),
                output_data: Some(output_data),
                metadata: json!({
                    "model": "autosprite",
                    "asset_type": "sprite",
                    "animation_type": animation_type,
                    "style": style,
                    "frame_count": ss["frameCount"],
                    "frame_width": ss["frameWidth"],
                    "frame_height": ss["frameHeight"],
                    "columns": ss["columns"],
                    "atlas_url": atlas_url,
                    "spritesheet_id": ss_id,
                }),
                cost_usd: Some(0.10),
                elapsed_ms: start.elapsed().as_millis() as u64,
            })
        }
        .await;

        // Step E: Cleanup — always delete character
        self.delete_character(&character_id).await;

        result
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();
        let resp = self
            .http
            .get(format!("{}/api/v1/characters", BASE_URL))
            .header("x-api-key", &self.api_key)
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
