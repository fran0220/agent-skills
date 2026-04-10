use std::io::Cursor;
use std::time::{Duration, Instant};

use crate::core::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::codecs::gif::{GifEncoder, Repeat};
use image::{DynamicImage, Frame, GenericImage, GenericImageView, ImageFormat, RgbaImage};
use serde_json::{json, Value};

/// SpriteForge provider — AI-driven character animation generation.
///
/// Pipeline:
/// 1. Build prompt from user description (no LLM enhancement)
/// 2. Generate short video via xAI Grok (`grok-imagine-video`)
/// 3. Extract frames via ffmpeg
/// 4. Remove white background → compose spritesheet or GIF
pub struct SpriteForgeProvider {
    pub id: String,
    xai_key: String,
    http: reqwest::Client,
}

impl SpriteForgeProvider {
    pub fn new(xai_key: String) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(300))
            .build()
            .expect("failed to build SpriteForge HTTP client");
        Self {
            id: "spriteforge".into(),
            xai_key,
            http,
        }
    }

    /// Resolve camera view from direction + explicit view param.
    /// `auto` infers from facing direction; `none` omits view clause.
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

    /// Resolve framing clause. Defaults to full-body.
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

    /// Build a video generation prompt from request parameters.
    fn build_prompt(
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

    /// Submit a video generation request to xAI and poll until done.
    async fn generate_video(
        &self,
        prompt: &str,
        duration: u32,
        image_url: Option<&str>,
    ) -> anyhow::Result<String> {
        let mut body = json!({
            "model": "grok-imagine-video",
            "prompt": prompt,
            "duration": duration,
            "aspect_ratio": "1:1",
            "resolution": "480p"
        });
        if let Some(url) = image_url {
            body["image_url"] = json!(url);
        }

        let resp = self
            .http
            .post("https://api.x.ai/v1/videos/generations")
            .bearer_auth(&self.xai_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;
        if !status.is_success() {
            anyhow::bail!("SpriteForge video generation returned {status}: {text}");
        }

        let data: Value = serde_json::from_str(&text)?;
        let request_id = data["request_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing request_id in video response"))?;
        tracing::info!(request_id, "SpriteForge: video generation submitted");

        // Poll until done (up to 5 minutes)
        for i in 0..60 {
            tokio::time::sleep(Duration::from_secs(5)).await;
            let poll = self
                .http
                .get(format!("https://api.x.ai/v1/videos/{request_id}"))
                .bearer_auth(&self.xai_key)
                .send()
                .await?;

            let poll_data: Value = poll.json().await?;
            let poll_status = poll_data["status"].as_str().unwrap_or("");
            tracing::debug!(
                attempt = i + 1,
                status = poll_status,
                "SpriteForge: polling"
            );

            match poll_status {
                "done" => {
                    let url = poll_data["video"]["url"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("missing video url in done response"))?;
                    return Ok(url.to_string());
                }
                "expired" | "failed" => {
                    anyhow::bail!("SpriteForge video generation {poll_status}: {poll_data}");
                }
                _ => continue,
            }
        }
        anyhow::bail!("SpriteForge video generation timed out after 5 minutes")
    }

    /// Download video, extract frames via ffmpeg, return as DynamicImage vec.
    async fn extract_frames(&self, video_url: &str, fps: u32) -> anyhow::Result<Vec<DynamicImage>> {
        // Download video to temp file
        let resp = self.http.get(video_url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("failed to download video: HTTP {}", resp.status());
        }
        let video_bytes = resp.bytes().await?;

        let tmp_dir = tempfile::tempdir()?;
        let video_path = tmp_dir.path().join("input.mp4");
        tokio::fs::write(&video_path, &video_bytes).await?;

        // Extract frames with ffmpeg
        let frame_pattern = tmp_dir.path().join("frame_%03d.png");
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

        // Read extracted frames
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

        tracing::info!(count = frames.len(), fps, "SpriteForge: frames extracted");
        Ok(frames)
    }

    /// Remove white background from frames using a binary cutoff based on
    /// Euclidean distance from pure white.
    ///
    /// A soft-alpha falloff was tempting, but it causes white matting:
    /// edge pixels keep their near-white RGB values (e.g. `(216, 210, 215)`)
    /// and, when composited onto any non-white background, produce a visible
    /// white fringe/halo. A hard threshold avoids that entirely and is robust
    /// for sprite use where crisp silhouettes matter more than sub-pixel edges.
    ///
    /// Threshold of 80 catches the full halo range (pure white all the way
    /// down to `(200, 200, 200)`-ish gray) without touching character pixels,
    /// which are typically at distance > 100.
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
}

#[async_trait::async_trait]
impl AssetProvider for SpriteForgeProvider {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        "SpriteForge (AI Character Animation)"
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
            anyhow::bail!("SpriteForge requires a non-empty prompt (character description)");
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
            .unwrap_or("right");
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
        let duration = req
            .params
            .get("duration")
            .and_then(Value::as_u64)
            .unwrap_or(2) as u32;

        let (_, should_remove_bg) = Self::resolve_background(background);

        // Step 1: Build prompt (no LLM enhancement)
        let video_prompt = Self::build_prompt(
            prompt,
            animation_type,
            direction,
            view,
            framing,
            background,
            style,
        );
        tracing::info!(
            prompt,
            animation_type,
            direction,
            duration,
            "SpriteForge: generating video"
        );

        // Step 2: Generate video via Grok
        let image_url = req.input_file.as_deref();
        let video_url = self
            .generate_video(&video_prompt, duration, image_url)
            .await?;

        // Step 3: Extract frames
        let raw_frames = self.extract_frames(&video_url, fps).await?;

        // Step 4: Conditionally remove white background
        let frames = if should_remove_bg {
            Self::remove_white_bg(&raw_frames)
        } else {
            raw_frames
        };
        let frame_count = frames.len();

        // Step 5: Compose output
        let (output_b64, content_type) = match output_format {
            "gif" => {
                let gif_bytes = Self::encode_gif(&frames, fps)?;
                (STANDARD.encode(&gif_bytes), "image/gif")
            }
            _ => {
                let sheet = Self::compose_sprite_sheet(&frames);
                let mut png_buf = Cursor::new(Vec::new());
                sheet
                    .write_to(&mut png_buf, ImageFormat::Png)
                    .map_err(|e| anyhow::anyhow!("failed to encode sprite sheet: {e}"))?;
                (STANDARD.encode(png_buf.get_ref()), "image/png")
            }
        };

        let cost = 0.05 * duration as f64;

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url: None,
            output_data: Some(output_b64),
            metadata: json!({
                "video_model": "grok-imagine-video",
                "animation_type": animation_type,
                "direction": direction,
                "view": view.unwrap_or("auto"),
                "framing": framing.unwrap_or("full-body"),
                "background": background.unwrap_or("auto"),
                "background_removed": should_remove_bg,
                "video_duration": duration,
                "frame_count": frame_count,
                "output_format": output_format,
                "content_type": content_type,
                "fps": fps,
                "video_prompt": video_prompt,
                "has_reference_image": image_url.is_some(),
            }),
            cost_usd: Some(cost),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();
        let resp = self
            .http
            .get("https://api.x.ai/v1/models")
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
