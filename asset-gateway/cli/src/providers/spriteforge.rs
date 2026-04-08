use std::io::Cursor;
use std::time::{Duration, Instant};

use crate::core::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use image::codecs::gif::{GifEncoder, Repeat};
use image::{DynamicImage, Frame, GenericImage, GenericImageView, ImageFormat, RgbaImage};
use serde_json::{json, Value};

const SPRITE_PROMPT_SYSTEM: &str = r#"You are an expert sprite sheet artist and prompt engineer for AI image generation.

Your task is to generate a detailed, frame-by-frame prompt for an AI image generation model (Google Gemini) to create a sprite animation grid.

Rules:
1. Describe each frame by its grid position (Row X, Col Y)
2. Use precise anatomical terms for poses (e.g., "left leg forward at 45 degrees", "right arm swings back")
3. EMPHASIZE consistency: same style, colors, proportions, and ALL accessories/weapons must appear in EVERY frame
4. Include technical constraints: white background per cell, thin black grid lines separating frames, equal frame sizes
5. Describe a logical animation sequence that loops seamlessly
6. Output ONLY the final prompt text, no explanations or preamble

The prompt you generate will be sent directly to Gemini's image editing API along with a reference character image."#;

/// SpriteForge provider — AI-driven sprite animation generation.
///
/// Embeds the full sprite-forge pipeline:
/// 1. LLM prompt enhancement (OpenAI-compatible)
/// 2. Gemini native image generation
/// 3. Grid post-processing into horizontal sprite sheet
pub struct SpriteForgeProvider {
    pub id: String,
    proxy_url: String,
    proxy_key: String,
    llm_model: String,
    gemini_model: String,
    http: reqwest::Client,
}

impl SpriteForgeProvider {
    pub fn new(proxy_url: String, proxy_key: String) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(300))
            .build()
            .expect("failed to build SpriteForge HTTP client");
        Self {
            id: "spriteforge".into(),
            proxy_url,
            proxy_key,
            llm_model: "gemini-3-flash-preview".into(),
            gemini_model: "gemini-3.1-flash-image-preview".into(),
            http,
        }
    }

    /// Call the LLM to enhance a sprite prompt via OpenAI-compatible chat API.
    async fn enhance_prompt(
        &self,
        character_desc: &str,
        animation_type: &str,
        direction: &str,
        grid_size: &str,
        style: Option<&str>,
    ) -> anyhow::Result<String> {
        let (cols, rows) = Self::parse_grid_size(grid_size);
        let total_frames = cols * rows;
        let mut user_content = format!(
            "Create a detailed sprite sheet generation prompt for this character and animation.\n\n\
             Character description:\n{character_desc}\n\n\
             Animation requirements:\n\
             - Animation type: {animation_type}\n\
             - Facing direction: {direction}\n\
             - Grid: {cols}x{rows} grid with {total_frames} total frames\n\
             - The animation must read clearly from this direction and loop seamlessly.\n\
             - Every frame must preserve the exact same character identity, costume, silhouette, colors, proportions, and props.\n\
             - Use a clean white background in each cell, with thin black grid lines separating frames and equal frame dimensions.\n\
             - Describe every frame in order by row and column."
        );
        if let Some(s) = style.filter(|v| !v.trim().is_empty()) {
            user_content.push_str(&format!("\n- Visual style: {s}"));
        }

        let body = json!({
            "model": self.llm_model,
            "max_tokens": 2000,
            "messages": [
                { "role": "system", "content": SPRITE_PROMPT_SYSTEM },
                { "role": "user", "content": user_content }
            ]
        });

        let resp = self
            .http
            .post(format!("{}/v1/chat/completions", self.proxy_url))
            .bearer_auth(&self.proxy_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await?;
            anyhow::bail!("SpriteForge LLM prompt enhancement returned {status}: {text}");
        }

        let data: Value = resp.json().await?;
        data["choices"][0]["message"]["content"]
            .as_str()
            .map(String::from)
            .ok_or_else(|| anyhow::anyhow!("missing choices[0].message.content in LLM response"))
    }

    /// Call Gemini native generateContent API for image generation.
    async fn generate_image(
        &self,
        enhanced_prompt: &str,
        reference_image: Option<(&[u8], &str)>,
    ) -> anyhow::Result<Vec<u8>> {
        let mut parts = Vec::new();

        if let Some((bytes, mime)) = reference_image {
            parts.push(json!({
                "inlineData": {
                    "mimeType": mime,
                    "data": STANDARD.encode(bytes)
                }
            }));
        }

        parts.push(json!({ "text": enhanced_prompt }));

        let body = json!({
            "contents": [{ "role": "user", "parts": parts }],
            "generationConfig": {
                "responseModalities": ["IMAGE", "TEXT"]
            }
        });

        let resp = self
            .http
            .post(format!(
                "{}/v1beta/models/{}:generateContent",
                self.proxy_url, self.gemini_model
            ))
            .header("x-goog-api-key", &self.proxy_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("SpriteForge Gemini image generation returned {status}: {text}");
        }

        let payload: Value = serde_json::from_str(&text)?;
        let image_data = payload["candidates"][0]["content"]["parts"]
            .as_array()
            .and_then(|parts| {
                parts
                    .iter()
                    .find_map(|p| p["inlineData"]["data"].as_str().map(String::from))
            })
            .ok_or_else(|| anyhow::anyhow!("Gemini response does not contain inlineData image"))?;

        Ok(STANDARD.decode(&image_data)?)
    }

    /// Split a grid image into individual frames.
    fn split_grid(img: &DynamicImage, cols: u32, rows: u32) -> Vec<DynamicImage> {
        let (w, h) = img.dimensions();
        let frame_w = w / cols;
        let frame_h = h / rows;
        let mut frames = Vec::new();
        for row in 0..rows {
            for col in 0..cols {
                let frame = img.crop_imm(col * frame_w, row * frame_h, frame_w, frame_h);
                frames.push(frame);
            }
        }
        frames
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

    /// Parse "CxR" grid size string into (cols, rows).
    fn parse_grid_size(s: &str) -> (u32, u32) {
        if let Some((c, r)) = s.split_once('x').or_else(|| s.split_once('X')) {
            let cols = c.trim().parse().unwrap_or(3);
            let rows = r.trim().parse().unwrap_or(3);
            (cols.max(1), rows.max(1))
        } else {
            (3, 3)
        }
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

    /// Download an image from a URL and return (bytes, mime_type).
    async fn download_image(&self, url: &str) -> anyhow::Result<(Vec<u8>, String)> {
        let resp = self.http.get(url).send().await?;
        if !resp.status().is_success() {
            anyhow::bail!("failed to download reference image: HTTP {}", resp.status());
        }
        let content_type = resp
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("image/png")
            .split(';')
            .next()
            .unwrap_or("image/png")
            .to_string();
        let bytes = resp.bytes().await?.to_vec();
        Ok((bytes, content_type))
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
        "SpriteForge (AI Sprite Animation)"
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
            .unwrap_or("idle");
        let direction = req
            .params
            .get("direction")
            .and_then(Value::as_str)
            .unwrap_or("right");
        let grid_size_str = req
            .params
            .get("grid_size")
            .and_then(Value::as_str)
            .unwrap_or("3x3");
        let style = req.params.get("style").and_then(Value::as_str);
        let output_format = req
            .params
            .get("output_format")
            .and_then(Value::as_str)
            .unwrap_or("spritesheet");
        let fps = req.params.get("fps").and_then(Value::as_u64).unwrap_or(8) as u32;
        let (cols, rows) = Self::parse_grid_size(grid_size_str);

        // Step 1: LLM prompt enhancement
        tracing::info!(
            prompt,
            animation_type,
            direction,
            grid_size = grid_size_str,
            "SpriteForge: enhancing prompt"
        );
        let enhanced_prompt = self
            .enhance_prompt(prompt, animation_type, direction, grid_size_str, style)
            .await?;
        tracing::debug!(
            enhanced_prompt_len = enhanced_prompt.len(),
            "SpriteForge: prompt enhanced"
        );

        // Step 2: Download reference image if provided
        let reference = if let Some(url) = req.input_file.as_deref() {
            let (bytes, mime) = self.download_image(url).await?;
            Some((bytes, mime))
        } else {
            None
        };

        // Step 3: Generate grid image via Gemini
        tracing::info!("SpriteForge: generating grid image via Gemini");
        let grid_bytes = self
            .generate_image(
                &enhanced_prompt,
                reference.as_ref().map(|(b, m)| (b.as_slice(), m.as_str())),
            )
            .await?;

        // Step 4: Post-process — split grid into frames
        let grid_img = image::load_from_memory(&grid_bytes)
            .map_err(|e| anyhow::anyhow!("failed to decode Gemini output image: {e}"))?;
        let frames = Self::split_grid(&grid_img, cols, rows);
        let frame_count = frames.len();

        let (output_b64, content_type) = match output_format {
            "gif" => {
                let gif_bytes = Self::encode_gif(&frames, fps)?;
                (STANDARD.encode(&gif_bytes), "image/gif")
            }
            _ => {
                // Default: horizontal sprite sheet PNG
                let sheet = Self::compose_sprite_sheet(&frames);
                let mut png_buf = Cursor::new(Vec::new());
                sheet
                    .write_to(&mut png_buf, ImageFormat::Png)
                    .map_err(|e| anyhow::anyhow!("failed to encode sprite sheet: {e}"))?;
                (STANDARD.encode(png_buf.get_ref()), "image/png")
            }
        };

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url: None,
            output_data: Some(output_b64),
            metadata: json!({
                "model": self.llm_model,
                "gemini_model": self.gemini_model,
                "animation_type": animation_type,
                "direction": direction,
                "grid_size": grid_size_str,
                "frame_count": frame_count,
                "output_format": output_format,
                "content_type": content_type,
                "fps": fps,
                "enhanced_prompt": enhanced_prompt,
            }),
            cost_usd: Some(0.05),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();
        let resp = self
            .http
            .get(format!("{}/v1/models", self.proxy_url))
            .bearer_auth(&self.proxy_key)
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
