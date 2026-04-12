use std::io::Cursor;
use std::time::{Duration, Instant};

use crate::core::*;
use crate::vertex_auth::VertexAuth;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::codecs::gif::{GifEncoder, Repeat};
use image::{DynamicImage, Frame, GenericImage, GenericImageView, ImageFormat, RgbaImage};
use serde_json::{json, Value};

const VEO_MODEL: &str = "veo-3.1-lite-generate-001";
const VEO_LOCATION: &str = "us-central1";
const VEO_DURATION: u32 = 4;

const GEMINI_IMAGE_MODEL: &str = "gemini-3.1-flash-image-preview";

const POLL_INTERVAL: Duration = Duration::from_secs(10);
const POLL_TIMEOUT: Duration = Duration::from_secs(300);

/// Character Animation provider — AI-driven animation generation via Vertex AI.
///
/// Pipeline:
/// 1. Ensure a first-frame image exists (generate via Gemini Image if text-only)
/// 2. Generate animation video via Veo (first/last frame for looping)
/// 3. Extract frames via ffmpeg
/// 4. Remove white background → compose spritesheet, GIF, or MP4
pub struct CharAnimProvider {
    pub id: String,
    vertex_auth: VertexAuth,
    vertex_project: String,
    http: reqwest::Client,
}

impl CharAnimProvider {
    pub fn new(auth: VertexAuth, project: String) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(600))
            .build()
            .expect("failed to build CharAnim HTTP client");
        Self {
            id: "charanim".into(),
            vertex_auth: auth,
            vertex_project: project,
            http,
        }
    }

    // ── Step A: Ensure first-frame image ──

    /// Generate a character still image via Gemini Image on Vertex AI.
    /// Returns base64-encoded PNG.
    async fn generate_character_image(
        &self,
        prompt: &str,
        style: Option<&str>,
    ) -> anyhow::Result<String> {
        let mut full_prompt = format!(
            "A single character standing in a neutral pose, full body visible, \
             centered in frame, plain white background. Character: {}",
            prompt
        );
        if let Some(s) = style.filter(|v| !v.trim().is_empty()) {
            full_prompt.push_str(&format!(". Style: {s}"));
        }

        let body = json!({
            "contents": [{"role": "user", "parts": [{"text": full_prompt}]}],
            "generationConfig": {
                "responseModalities": ["IMAGE", "TEXT"],
                "imageConfig": {
                    "aspectRatio": "9:16"
                }
            }
        });

        let token = self.vertex_auth.access_token().await?;
        let resp = self
            .http
            .post(format!(
                "https://aiplatform.googleapis.com/v1/projects/{}/locations/global/publishers/google/models/{}:generateContent",
                self.vertex_project, GEMINI_IMAGE_MODEL
            ))
            .header("Authorization", format!("Bearer {}", token))
            .timeout(Duration::from_secs(60))
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("CharAnim image generation returned {}: {}", status, text);
        }

        let payload: Value = serde_json::from_str(&text)?;
        let parts = payload["candidates"][0]["content"]["parts"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("Gemini Image response missing parts"))?;

        for part in parts {
            if let Some(data) = part["inlineData"]["data"].as_str() {
                return Ok(data.to_string());
            }
        }
        anyhow::bail!("Gemini Image response contains no image data")
    }

    /// Fetch a remote image URL and return as base64 PNG.
    async fn fetch_image_as_base64(&self, url: &str) -> anyhow::Result<(String, String)> {
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

    // ── Step B: Veo video generation ──

    /// Submit a Veo predictLongRunning request. Returns the operation name.
    async fn veo_submit(
        &self,
        prompt: &str,
        first_frame_b64: &str,
        first_frame_mime: &str,
        loop_animation: bool,
    ) -> anyhow::Result<String> {
        let mut instance = json!({
            "prompt": prompt,
            "image": {
                "bytesBase64Encoded": first_frame_b64,
                "mimeType": first_frame_mime,
            }
        });

        if loop_animation {
            instance["lastFrame"] = json!({
                "bytesBase64Encoded": first_frame_b64,
                "mimeType": first_frame_mime,
            });
        }

        let body = json!({
            "instances": [instance],
            "parameters": {
                "sampleCount": 1,
                "durationSeconds": VEO_DURATION,
                "aspectRatio": "9:16",
                "generateAudio": false,
                "resolution": "720p",
            }
        });

        let token = self.vertex_auth.access_token().await?;
        let resp = self
            .http
            .post(format!(
                "https://{}-aiplatform.googleapis.com/v1/projects/{}/locations/{}/publishers/google/models/{}:predictLongRunning",
                VEO_LOCATION, self.vertex_project, VEO_LOCATION, VEO_MODEL
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

    /// Poll Veo operation via fetchPredictOperation until done.
    /// Returns the base64-encoded MP4 video data.
    async fn veo_poll(&self, operation_name: &str) -> anyhow::Result<String> {
        let deadline = Instant::now() + POLL_TIMEOUT;

        loop {
            if Instant::now() > deadline {
                anyhow::bail!(
                    "Veo operation timed out after {}s: {}",
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
                    VEO_LOCATION, self.vertex_project, VEO_LOCATION, VEO_MODEL
                ))
                .header("Authorization", format!("Bearer {}", token))
                .header("Content-Type", "application/json")
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
            let done = payload["done"].as_bool().unwrap_or(false);

            if !done {
                tracing::debug!(operation = operation_name, "CharAnim: Veo still processing");
                continue;
            }

            // Check for error
            if let Some(error) = payload.get("error") {
                let msg = error["message"].as_str().unwrap_or("unknown error");
                anyhow::bail!("Veo operation failed: {}", msg);
            }

            // Extract video data
            let videos = payload["response"]["videos"]
                .as_array()
                .ok_or_else(|| anyhow::anyhow!("Veo response missing videos array"))?;

            let video_b64 = videos
                .first()
                .and_then(|v| v["bytesBase64Encoded"].as_str())
                .ok_or_else(|| anyhow::anyhow!("Veo response missing video data"))?;

            return Ok(video_b64.to_string());
        }
    }

    // ── Step C: Frame extraction & post-processing ──

    /// Decode base64 MP4, extract frames via ffmpeg.
    async fn extract_frames(video_b64: &str, fps: u32) -> anyhow::Result<Vec<DynamicImage>> {
        let video_bytes = STANDARD.decode(video_b64)?;

        let tmp_dir = tempfile::tempdir()?;
        let video_path = tmp_dir.path().join("input.mp4");
        tokio::fs::write(&video_path, &video_bytes).await?;

        let frame_pattern = tmp_dir.path().join("frame_%04d.png");
        let output = tokio::process::Command::new("ffmpeg")
            .args([
                "-y",
                "-i",
                video_path.to_str().unwrap(),
                "-vf",
                &format!("fps={fps}"),
                frame_pattern.to_str().unwrap(),
            ])
            .output()
            .await?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            anyhow::bail!("ffmpeg frame extraction failed: {stderr}");
        }

        let mut frames = Vec::new();
        let mut entries: Vec<_> = std::fs::read_dir(tmp_dir.path())?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .file_name()
                    .and_then(|n| n.to_str())
                    .is_some_and(|n| n.starts_with("frame_") && n.ends_with(".png"))
            })
            .collect();
        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let img = image::open(entry.path()).map_err(|e| {
                anyhow::anyhow!("failed to load frame {}: {e}", entry.path().display())
            })?;
            frames.push(img);
        }

        tracing::info!(count = frames.len(), fps, "CharAnim: frames extracted");
        Ok(frames)
    }

    /// Remove white background from frames using binary threshold.
    fn remove_white_bg(frames: &[DynamicImage]) -> Vec<DynamicImage> {
        const THRESHOLD_SQ: u32 = 80 * 80;

        frames
            .iter()
            .map(|frame| {
                let mut rgba = frame.to_rgba8();
                for pixel in rgba.pixels_mut() {
                    let [r, g, b, _] = pixel.0;
                    let dr = 255 - r as i32;
                    let dg = 255 - g as i32;
                    let db = 255 - b as i32;
                    let dist_sq = (dr * dr + dg * dg + db * db) as u32;
                    if dist_sq <= THRESHOLD_SQ {
                        pixel.0 = [0, 0, 0, 0];
                    }
                }
                DynamicImage::ImageRgba8(rgba)
            })
            .collect()
    }

    /// Compose frames into a horizontal sprite sheet.
    fn compose_sprite_sheet(frames: &[DynamicImage]) -> RgbaImage {
        if frames.is_empty() {
            return RgbaImage::new(1, 1);
        }
        let (fw, fh) = frames[0].dimensions();
        let total_w = fw * frames.len() as u32;
        let mut sheet = RgbaImage::new(total_w, fh);
        for (i, frame) in frames.iter().enumerate() {
            let rgba = frame.to_rgba8();
            sheet.copy_from(&rgba, i as u32 * fw, 0).ok();
        }
        sheet
    }

    /// Encode frames into an animated GIF.
    fn encode_gif(frames: &[DynamicImage], fps: u32) -> anyhow::Result<Vec<u8>> {
        let mut buf = Vec::new();
        {
            let mut encoder = GifEncoder::new_with_speed(&mut buf, 10);
            encoder.set_repeat(Repeat::Infinite)?;
            let delay = image::Delay::from_numer_denom_ms(1000, fps);
            for frame in frames {
                let rgba = frame.to_rgba8();
                encoder.encode_frame(Frame::from_parts(rgba, 0, 0, delay))?;
            }
        }
        Ok(buf)
    }

    // ── Prompt building ──

    /// Resolve camera view from direction + explicit view param.
    fn resolve_view(direction: &str, view: Option<&str>) -> Option<String> {
        match view.map(|v| v.trim().to_ascii_lowercase()).as_deref() {
            None | Some("") | Some("auto") => {
                match direction.trim().to_ascii_lowercase().as_str() {
                    "left" | "right" => Some("side view".into()),
                    "front" => Some("front view".into()),
                    "back" => Some("back view".into()),
                    _ => None,
                }
            }
            Some("none") => None,
            Some(v) => Some(format!("{v} view")),
        }
    }

    /// Resolve framing clause.
    fn resolve_framing(framing: Option<&str>) -> Option<String> {
        match framing.map(|v| v.trim().to_ascii_lowercase()).as_deref() {
            None | Some("") | Some("full-body") => {
                Some("full body character fully visible, centered in frame".into())
            }
            Some("none") => None,
            Some(v) => Some(format!("{v} framing")),
        }
    }

    /// Resolve background clause and whether white-bg removal should run.
    fn resolve_background(background: Option<&str>) -> (Option<String>, bool) {
        match background.map(|v| v.trim().to_ascii_lowercase()).as_deref() {
            None | Some("") | Some("auto") | Some("white") => {
                (Some("plain white background".into()), true)
            }
            Some("none") => (None, false),
            Some(v) => (Some(format!("{v} background")), false),
        }
    }

    /// Determine if this animation type should loop.
    fn is_loop_animation(animation_type: &str) -> bool {
        matches!(
            animation_type.trim().to_ascii_lowercase().as_str(),
            "walk" | "run" | "idle" | "float" | "breathe" | "dance" | "swim" | "fly"
        )
    }

    /// Build a video generation prompt from request parameters.
    fn build_video_prompt(
        character_desc: &str,
        animation_type: &str,
        direction: &str,
        view: Option<&str>,
        framing: Option<&str>,
        background: Option<&str>,
        style: Option<&str>,
    ) -> String {
        let mut parts = vec![
            character_desc.trim().to_string(),
            format!("{animation_type} animation"),
            format!("facing {direction}"),
        ];

        if let Some(v) = Self::resolve_view(direction, view) {
            parts.push(v);
        }
        if let Some(f) = Self::resolve_framing(framing) {
            parts.push(f);
        }
        let (bg_clause, _) = Self::resolve_background(background);
        if let Some(bg) = bg_clause {
            parts.push(bg);
        }

        parts.push("smooth looping motion".into());

        if let Some(s) = style.filter(|v| !v.trim().is_empty()) {
            parts.push(format!("{s} style"));
        }

        format!("{}.", parts.join(". "))
    }
}

#[async_trait::async_trait]
impl AssetProvider for CharAnimProvider {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        "Character Animation (Vertex AI)"
    }

    fn asset_types(&self) -> &[AssetType] {
        &[AssetType::Sprite]
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
            anyhow::bail!("CharAnim requires a non-empty prompt (character description)");
        }

        let start = Instant::now();

        let animation_type = req
            .params
            .get("animation_type")
            .and_then(Value::as_str)
            .unwrap_or("walk");
        let direction = req
            .params
            .get("direction")
            .and_then(Value::as_str)
            .unwrap_or("front");
        let view = req.params.get("view").and_then(Value::as_str);
        let framing = req.params.get("framing").and_then(Value::as_str);
        let background = req.params.get("background").and_then(Value::as_str);
        let style = req.params.get("style").and_then(Value::as_str);
        let output_format = req
            .params
            .get("output_format")
            .and_then(Value::as_str)
            .unwrap_or("spritesheet");
        let fps = req.params.get("fps").and_then(Value::as_u64).unwrap_or(8) as u32;

        let (_, should_remove_bg) = Self::resolve_background(background);
        let loop_anim = Self::is_loop_animation(animation_type);

        // Step A: Ensure first-frame image
        let image_url = req.input_file.as_deref();
        let generated_image = image_url.is_none();

        let (first_frame_b64, first_frame_mime) = if let Some(url) = image_url {
            tracing::info!("CharAnim: using provided reference image");
            self.fetch_image_as_base64(url).await?
        } else {
            tracing::info!("CharAnim: generating character image via Gemini Image");
            let b64 = self.generate_character_image(prompt, style).await?;
            (b64, "image/png".to_string())
        };

        // Step B: Build prompt & generate video via Veo
        let video_prompt = Self::build_video_prompt(
            prompt,
            animation_type,
            direction,
            view,
            framing,
            background,
            style,
        );
        tracing::info!(
            animation_type,
            direction,
            loop_anim,
            "CharAnim: submitting Veo video generation"
        );

        let operation_name = self
            .veo_submit(
                &video_prompt,
                &first_frame_b64,
                &first_frame_mime,
                loop_anim,
            )
            .await?;
        tracing::info!(operation = %operation_name, "CharAnim: Veo operation started");

        let video_b64 = self.veo_poll(&operation_name).await?;
        tracing::info!("CharAnim: Veo video received, extracting frames");

        // Step C: Extract frames & post-process
        let raw_frames = Self::extract_frames(&video_b64, fps).await?;

        let frames = if should_remove_bg {
            Self::remove_white_bg(&raw_frames)
        } else {
            raw_frames
        };
        let frame_count = frames.len();

        // Step D: Compose output
        let (output_b64, content_type) = match output_format {
            "gif" => {
                let gif_bytes = Self::encode_gif(&frames, fps)?;
                (STANDARD.encode(&gif_bytes), "image/gif")
            }
            "mp4" => {
                // Return original video as-is
                (video_b64, "video/mp4")
            }
            _ => {
                // spritesheet (default)
                let sheet = Self::compose_sprite_sheet(&frames);
                let mut png_buf = Cursor::new(Vec::new());
                sheet
                    .write_to(&mut png_buf, ImageFormat::Png)
                    .map_err(|e| anyhow::anyhow!("failed to encode sprite sheet: {e}"))?;
                (STANDARD.encode(png_buf.get_ref()), "image/png")
            }
        };

        // Cost: Veo Lite 720p video-only $0.03/s × 4s = $0.12
        //        + optional Gemini Image generation ~$0.04
        let cost = if generated_image { 0.16 } else { 0.12 };

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url: None,
            output_data: Some(output_b64),
            metadata: json!({
                "video_model": VEO_MODEL,
                "image_model": if generated_image { Some(GEMINI_IMAGE_MODEL) } else { None },
                "animation_type": animation_type,
                "direction": direction,
                "view": view.unwrap_or("auto"),
                "framing": framing.unwrap_or("full-body"),
                "background": background.unwrap_or("auto"),
                "background_removed": should_remove_bg,
                "loop": loop_anim,
                "video_duration": VEO_DURATION,
                "frame_count": frame_count,
                "output_format": output_format,
                "content_type": content_type,
                "fps": fps,
                "video_prompt": video_prompt,
                "generated_first_frame": generated_image,
                "has_reference_image": image_url.is_some(),
            }),
            cost_usd: Some(cost),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();
        match self.vertex_auth.access_token().await {
            Ok(_) => Ok(HealthStatus {
                healthy: true,
                latency_ms: Some(start.elapsed().as_millis() as u64),
                message: None,
            }),
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
    fn resolve_view_auto() {
        assert_eq!(
            CharAnimProvider::resolve_view("right", Some("auto")),
            Some("side view".into())
        );
        assert_eq!(
            CharAnimProvider::resolve_view("front", None),
            Some("front view".into())
        );
        assert_eq!(
            CharAnimProvider::resolve_view("back", Some("")),
            Some("back view".into())
        );
    }

    #[test]
    fn resolve_view_explicit() {
        assert_eq!(
            CharAnimProvider::resolve_view("right", Some("three-quarter")),
            Some("three-quarter view".into())
        );
        assert_eq!(CharAnimProvider::resolve_view("right", Some("none")), None);
    }

    #[test]
    fn loop_animation_types() {
        assert!(CharAnimProvider::is_loop_animation("walk"));
        assert!(CharAnimProvider::is_loop_animation("run"));
        assert!(CharAnimProvider::is_loop_animation("idle"));
        assert!(CharAnimProvider::is_loop_animation("dance"));
        assert!(!CharAnimProvider::is_loop_animation("attack"));
        assert!(!CharAnimProvider::is_loop_animation("jump"));
        assert!(!CharAnimProvider::is_loop_animation("wave"));
    }

    #[test]
    fn background_resolution() {
        let (clause, remove) = CharAnimProvider::resolve_background(Some("auto"));
        assert_eq!(clause, Some("plain white background".into()));
        assert!(remove);

        let (clause, remove) = CharAnimProvider::resolve_background(Some("none"));
        assert!(clause.is_none());
        assert!(!remove);

        let (clause, remove) = CharAnimProvider::resolve_background(Some("forest"));
        assert_eq!(clause, Some("forest background".into()));
        assert!(!remove);
    }
}
