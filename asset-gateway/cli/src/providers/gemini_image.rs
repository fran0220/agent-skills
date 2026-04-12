use std::time::Instant;

use crate::core::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};

const DEFAULT_MODEL: &str = "gemini-3.1-flash-image-preview";

/// Gemini Flash Image provider (Google).
/// Cost-effective, no transparency support.
pub struct GeminiImageProvider {
    pub id: String,
    pub base_url: String,
    pub api_key: String,
    // Vertex AI direct access (priority over proxy)
    vertex_auth: Option<crate::vertex_auth::VertexAuth>,
    vertex_endpoint: Option<String>,
    vertex_project: Option<String>,
    vertex_location: Option<String>,
    http: reqwest::Client,
}

impl GeminiImageProvider {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            id: "gemini_image".into(),
            base_url,
            api_key,
            vertex_auth: None,
            vertex_endpoint: None,
            vertex_project: None,
            vertex_location: None,
            http: reqwest::Client::new(),
        }
    }

    pub fn with_vertex(
        mut self,
        auth: crate::vertex_auth::VertexAuth,
        project: String,
        location: String,
    ) -> Self {
        self.vertex_endpoint = Some(if location == "global" {
            "https://aiplatform.googleapis.com".to_string()
        } else {
            format!("https://{}-aiplatform.googleapis.com", location)
        });
        self.vertex_project = Some(project);
        self.vertex_location = Some(location);
        self.vertex_auth = Some(auth);
        self
    }

    fn build_prompt(&self, req: &GenerateRequest) -> String {
        let mut prompt = req.prompt.clone().unwrap_or_default();

        if let Some(style) = req.style() {
            prompt.push_str("\nStyle: ");
            prompt.push_str(style);
        }

        if let Some(quality) = req.quality() {
            prompt.push_str("\nQuality: ");
            prompt.push_str(quality.as_str());
        }

        prompt
    }

    fn apply_edit_mode(prompt: &str, edit_mode: Option<ImageEditMode>, has_input: bool) -> String {
        match edit_mode {
            Some(ImageEditMode::Inpaint) if has_input => format!(
                "Using the provided image, change only the specific element described below. \
Keep everything else in the image exactly the same, preserving the original \
style, lighting, and composition.\n\nEdit: {prompt}"
            ),
            Some(ImageEditMode::Restyle) if has_input => format!(
                "Transform the provided image into a new artistic style as described below. \
Preserve the original composition and subject matter but render it with the \
new style.\n\nStyle: {prompt}"
            ),
            Some(ImageEditMode::Expand) if has_input => format!(
                "Expand the provided image outward, extending the scene naturally beyond \
its current borders while maintaining visual consistency.\n\nDirection: {prompt}"
            ),
            Some(ImageEditMode::Edit) if has_input => prompt.to_string(),
            _ => prompt.to_string(),
        }
    }

    /// Fetch image and return as a Gemini inlineData part with correct MIME type.
    async fn fetch_image_part(&self, input: &str) -> anyhow::Result<Value> {
        if input.starts_with("data:") {
            // Parse data URI: data:<mime>;base64,<data>
            let (header, data) = input
                .split_once(";base64,")
                .ok_or_else(|| anyhow::anyhow!("Invalid data URI: missing ;base64, segment"))?;
            let mime = header.strip_prefix("data:").unwrap_or("image/png");
            Ok(json!({"inlineData": {"mimeType": mime, "data": data}}))
        } else if input.starts_with("http://") || input.starts_with("https://") {
            let resp = self.http.get(input).send().await?;
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
            Ok(json!({"inlineData": {"mimeType": content_type, "data": STANDARD.encode(&bytes)}}))
        } else {
            // Local file
            let bytes = tokio::fs::read(input).await?;
            let mime = Self::infer_mime_from_path(input);
            Ok(json!({"inlineData": {"mimeType": mime, "data": STANDARD.encode(&bytes)}}))
        }
    }

    fn infer_mime_from_path(path: &str) -> &'static str {
        match path
            .rsplit('.')
            .next()
            .map(|e| e.to_ascii_lowercase())
            .as_deref()
        {
            Some("png") => "image/png",
            Some("webp") => "image/webp",
            Some("gif") => "image/gif",
            Some("svg") => "image/svg+xml",
            _ => "image/jpeg",
        }
    }

    /// Parse a "WxH" size string into Gemini `imageConfig` fields.
    ///
    /// Returns `(imageSize, aspectRatio)` — e.g. `("2K", "16:9")`.
    fn parse_size(size: &str) -> Option<(String, String)> {
        let (w, h) = size.split_once('x').or_else(|| size.split_once('X'))?;
        let w: u32 = w.trim().parse().ok()?;
        let h: u32 = h.trim().parse().ok()?;
        if w == 0 || h == 0 {
            return None;
        }

        let max_dim = w.max(h);
        let image_size = match max_dim {
            0..=512 => "512",
            513..=1024 => "1K",
            1025..=2048 => "2K",
            _ => "4K",
        };

        // Find closest standard aspect ratio
        let ratio = w as f64 / h as f64;
        let candidates: &[(&str, f64)] = &[
            ("1:1", 1.0),
            ("16:9", 16.0 / 9.0),
            ("9:16", 9.0 / 16.0),
            ("4:3", 4.0 / 3.0),
            ("3:4", 3.0 / 4.0),
            ("3:2", 3.0 / 2.0),
            ("2:3", 2.0 / 3.0),
        ];
        let aspect_ratio = candidates
            .iter()
            .min_by(|a, b| {
                (a.1 - ratio)
                    .abs()
                    .partial_cmp(&(b.1 - ratio).abs())
                    .unwrap()
            })
            .map(|(name, _)| *name)
            .unwrap_or("1:1");

        Some((image_size.to_string(), aspect_ratio.to_string()))
    }

    fn extract_inline_image(payload: &Value) -> Option<(String, Option<String>)> {
        let parts = payload["candidates"][0]["content"]["parts"].as_array()?;

        for part in parts {
            let data = part["inlineData"]["data"].as_str().map(str::to_string);
            if let Some(data) = data {
                let mime = part["inlineData"]["mimeType"].as_str().map(str::to_string);
                return Some((data, mime));
            }
        }

        None
    }
}

#[async_trait::async_trait]
impl AssetProvider for GeminiImageProvider {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        if self.vertex_auth.is_some() {
            "Gemini Flash Image (Vertex AI)"
        } else {
            "Gemini Flash Image (Google)"
        }
    }

    fn asset_types(&self) -> &[AssetType] {
        &[AssetType::Image]
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_transparency: false,
            priority: 100,
            ..Default::default()
        }
    }

    async fn generate(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        let model = req.model.as_deref().unwrap_or(DEFAULT_MODEL);
        let start = Instant::now();

        // Build image parts from all inputs
        let image_inputs = req.image_inputs();
        let raw_prompt = self.build_prompt(req);
        let prompt = Self::apply_edit_mode(&raw_prompt, req.edit_mode, !image_inputs.is_empty());
        let mut image_parts = Vec::new();
        for input in &image_inputs {
            image_parts.push(self.fetch_image_part(input).await?);
        }

        // Build current user turn parts: images first, then text
        let mut current_parts = image_parts;
        current_parts.push(json!({"text": prompt}));

        // Build contents array (multi-turn or single-turn)
        let mut contents: Vec<Value> = Vec::new();

        // Load prior conversation history from session state if available
        if let Some(session_state) = req.params.get("_session_state") {
            if let Some(prior_contents) = session_state.get("contents").and_then(|c| c.as_array()) {
                contents.extend(prior_contents.iter().cloned());
            }
        }

        // Add current user turn
        contents.push(json!({"role": "user", "parts": current_parts}));

        let mut gen_config = json!({
            "responseModalities": ["IMAGE", "TEXT"],
        });

        if let Some(size_str) = req.params.get("size").and_then(|v| v.as_str()) {
            if let Some((image_size, aspect_ratio)) = Self::parse_size(size_str) {
                gen_config["imageConfig"] = json!({
                    "imageSize": image_size,
                    "aspectRatio": aspect_ratio,
                });
            }
        }

        let body = json!({
            "contents": contents,
            "generationConfig": gen_config,
        });

        let resp = if let (Some(auth), Some(endpoint), Some(project), Some(location)) = (
            &self.vertex_auth,
            &self.vertex_endpoint,
            &self.vertex_project,
            &self.vertex_location,
        ) {
            let token = auth.access_token().await?;
            self.http
                .post(format!(
                    "{}/v1/projects/{}/locations/{}/publishers/google/models/{}:generateContent",
                    endpoint, project, location, model
                ))
                .header("Authorization", format!("Bearer {}", token))
                .json(&body)
                .send()
                .await?
        } else {
            self.http
                .post(format!(
                    "{}/v1beta/models/{}:generateContent",
                    self.base_url, model
                ))
                .header("x-goog-api-key", &self.api_key)
                .json(&body)
                .send()
                .await?
        };

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("Gemini Image returned {}: {}", status, text);
        }

        let payload: Value = serde_json::from_str(&text)?;
        let (image_data, mime_type) = Self::extract_inline_image(&payload)
            .ok_or_else(|| anyhow::anyhow!("Gemini response does not contain inlineData image"))?;

        // Build session state: prior contents + current user turn + model response
        let model_content = payload["candidates"][0]["content"].clone();
        let mut session_contents = contents;
        session_contents.push(model_content);

        let has_refs = !image_inputs.is_empty();
        let is_multi_turn = req.session_id.is_some() || req.params.get("_session_state").is_some();
        let cost = if is_multi_turn {
            0.06
        } else if has_refs {
            0.08
        } else {
            0.04
        };

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url: None,
            output_data: Some(image_data),
            metadata: json!({
                "model": model,
                "mime_type": mime_type,
                "quality": req.quality().map(|q| q.as_str()),
                "style": req.style(),
                "editing": has_refs,
                "edit_mode": req.edit_mode,
                "multi_turn": is_multi_turn,
                "image_count": image_inputs.len(),
                "size": req.params.get("size"),
                "_session_state": {
                    "contents": session_contents
                }
            }),
            cost_usd: Some(cost),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();
        let resp = if let (Some(auth), Some(endpoint), Some(project), Some(location)) = (
            &self.vertex_auth,
            &self.vertex_endpoint,
            &self.vertex_project,
            &self.vertex_location,
        ) {
            let token = auth.access_token().await.ok();
            let mut req = self.http.get(format!(
                "{}/v1/projects/{}/locations/{}/publishers/google/models",
                endpoint, project, location
            ));
            if let Some(token) = token {
                req = req.header("Authorization", format!("Bearer {}", token));
            }
            req.send().await
        } else {
            self.http
                .get(format!("{}/v1beta/models", self.base_url))
                .header("x-goog-api-key", &self.api_key)
                .send()
                .await
        };

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
    fn parse_size_standard_cases() {
        let cases = vec![
            ("1024x1024", "1K", "1:1"),
            ("512x512", "512", "1:1"),
            ("1792x1024", "2K", "16:9"),
            ("1024x1792", "2K", "9:16"),
            ("2048x2048", "2K", "1:1"),
            ("4096x2304", "4K", "16:9"),
            ("1024x768", "1K", "4:3"),
            ("1536x1024", "2K", "3:2"),
        ];
        for (input, expected_size, expected_ratio) in cases {
            let (size, ratio) = GeminiImageProvider::parse_size(input)
                .unwrap_or_else(|| panic!("parse_size({input}) returned None"));
            assert_eq!(size, expected_size, "size mismatch for {input}");
            assert_eq!(ratio, expected_ratio, "ratio mismatch for {input}");
        }
    }

    #[test]
    fn parse_size_invalid() {
        assert!(GeminiImageProvider::parse_size("abc").is_none());
        assert!(GeminiImageProvider::parse_size("0x0").is_none());
        assert!(GeminiImageProvider::parse_size("1024").is_none());
    }

    #[test]
    fn apply_edit_mode_prefixes_only_when_input_is_present() {
        let prompt = "replace the sky with a sunset";

        let inpaint =
            GeminiImageProvider::apply_edit_mode(prompt, Some(ImageEditMode::Inpaint), true);
        assert!(inpaint.starts_with(
            "Using the provided image, change only the specific element described below."
        ));
        assert!(inpaint.ends_with(prompt));

        let restyle =
            GeminiImageProvider::apply_edit_mode(prompt, Some(ImageEditMode::Restyle), true);
        assert!(restyle.starts_with(
            "Transform the provided image into a new artistic style as described below."
        ));
        assert!(restyle.ends_with(prompt));

        let expand =
            GeminiImageProvider::apply_edit_mode(prompt, Some(ImageEditMode::Expand), true);
        assert!(expand.starts_with("Expand the provided image outward, extending the scene naturally beyond its current borders while maintaining visual consistency."));
        assert!(expand.ends_with(prompt));

        let unchanged =
            GeminiImageProvider::apply_edit_mode(prompt, Some(ImageEditMode::Inpaint), false);
        assert_eq!(unchanged, prompt);
    }
}
