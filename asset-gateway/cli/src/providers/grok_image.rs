use std::time::Instant;

use crate::core::*;
use serde_json::{json, Value};

const DEFAULT_MODEL: &str = "grok-imagine-1.0";
const EDIT_MODEL: &str = "grok-imagine-1.0-edit";
const VIDEO_MODEL: &str = "grok-imagine-1.0-video";

/// Grok Image provider (xAI grok-imagine via OpenAI-compatible chat completions).
/// Supports image generation, image editing, and video generation.
pub struct GrokImageProvider {
    pub id: String,
    pub base_url: String,
    pub api_key: String,
    http: reqwest::Client,
}

impl GrokImageProvider {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            id: "grok_image".into(),
            base_url,
            api_key,
            http: reqwest::Client::new(),
        }
    }
}

#[async_trait::async_trait]
impl AssetProvider for GrokImageProvider {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        "Grok Image (xAI)"
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
        let start = Instant::now();

        let (model, messages) = match req.asset_type {
            AssetType::Video => (VIDEO_MODEL, json!([{ "role": "user", "content": prompt }])),
            _ => {
                if let Some(ref image_url) = req.input_file {
                    (
                        EDIT_MODEL,
                        json!([{
                            "role": "user",
                            "content": [
                                { "type": "image_url", "image_url": { "url": image_url } },
                                { "type": "text", "text": prompt }
                            ]
                        }]),
                    )
                } else {
                    (
                        DEFAULT_MODEL,
                        json!([{ "role": "user", "content": prompt }]),
                    )
                }
            }
        };

        let body = json!({
            "model": model,
            "messages": messages,
            "stream": false,
        });

        let resp = self
            .http
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("Grok Image returned {}: {}", status, text);
        }

        let payload: Value = serde_json::from_str(&text)?;
        let content = payload["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or("");

        let output_url = match req.asset_type {
            AssetType::Video => extract_video_url(content),
            _ => extract_image_url(content),
        };

        let output_url = output_url
            .ok_or_else(|| anyhow::anyhow!("Grok Image response did not contain a valid URL"))?;

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url: Some(output_url),
            output_data: None,
            metadata: json!({
                "model": model,
                "asset_type": req.asset_type.to_string(),
            }),
            cost_usd: Some(match req.asset_type {
                AssetType::Video => 0.10,
                _ => 0.07,
            }),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();
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

/// Trim trailing characters that are common URL-wrapping delimiters but not part of the URL.
fn trim_url(raw: &str) -> &str {
    let mut s = raw;
    // Strip pairs of wrapping quotes/brackets if present
    for (open, close) in [('\"', '\"'), ('\'', '\''), ('<', '>'), ('(', ')')] {
        if s.starts_with(open) && s.ends_with(close) && s.len() >= 2 {
            s = &s[open.len_utf8()..s.len() - close.len_utf8()];
        }
    }
    // Strip trailing delimiters that commonly follow pasted URLs
    s.trim_end_matches(|c: char| matches!(c, ')' | ']' | '"' | '\'' | '>' | ',' | ';'))
}

/// Extract a quoted attribute value: given `attr="value"` or `attr='value'`, return `value`.
/// `haystack` should start right after the tag name (e.g. the attributes portion).
fn extract_attr<'a>(attrs: &'a str, name: &str) -> Option<&'a str> {
    // Try both `name="..."` and `name='...'`
    for quote in ['"', '\''] {
        let needle = format!("{}={}", name, quote);
        if let Some(pos) = attrs.find(&needle) {
            let start = pos + needle.len();
            if let Some(end) = attrs[start..].find(quote) {
                let val = &attrs[start..start + end];
                if !val.is_empty() {
                    return Some(val);
                }
            }
        }
    }
    None
}

/// Find the first URL (http/https) starting at or after `start` in `content`,
/// reading until a whitespace or delimiter character.
fn scan_url(content: &str) -> Option<String> {
    let hay = content;
    for scheme in ["https://", "http://"] {
        let mut search_from = 0;
        while let Some(pos) = hay[search_from..].find(scheme) {
            let abs = search_from + pos;
            // Walk forward to find the end of the URL
            let url_end = hay[abs..]
                .find(|c: char| {
                    c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | ')' | ']')
                })
                .map_or(hay.len(), |e| abs + e);
            let candidate = &hay[abs..url_end];
            if candidate.len() > scheme.len() {
                return Some(trim_url(candidate).to_string());
            }
            search_from = abs + scheme.len();
        }
    }
    None
}

/// Extract the first HTML tag of the given name and return its inner attributes string.
/// e.g. for `tag = "img"` and `<img src="x" alt="y">`, returns `src="x" alt="y"`.
fn find_tag_attrs<'a>(content: &'a str, tag: &str) -> Option<&'a str> {
    let open = format!("<{}", tag);
    let pos = content.find(&open)?;
    let rest = &content[pos + open.len()..];
    // The tag name must be followed by whitespace or `>` (not e.g. `<imgs...`)
    let first = rest.chars().next()?;
    if first != '>' && first != '/' && !first.is_whitespace() {
        return None;
    }
    let end = rest.find('>')?;
    Some(rest[..end].trim())
}

/// Extract an image URL (http/https) from response content.
///
/// Tries, in order:
/// 1. Markdown image `![...](URL)`
/// 2. HTML `<img src="URL">`
/// 3. HTML `<a href="URL">`
/// 4. Bare URL scan
fn extract_image_url(content: &str) -> Option<String> {
    // 1. Markdown image: ![alt](url)
    //    Find `](` then read until closing `)`
    {
        let mut search_from = 0;
        while let Some(pos) = content[search_from..].find("](") {
            let abs = search_from + pos;
            // Walk backward to verify there's a `![` somewhere before
            if abs > 0 && content[..abs].contains("![") {
                let url_start = abs + 2;
                if let Some(paren_end) = content[url_start..].find(')') {
                    let candidate = content[url_start..url_start + paren_end].trim();
                    if candidate.starts_with("http://") || candidate.starts_with("https://") {
                        return Some(trim_url(candidate).to_string());
                    }
                }
            }
            search_from = abs + 2;
        }
    }

    // 2. HTML <img ... src="URL" ...>
    if let Some(attrs) = find_tag_attrs(content, "img") {
        if let Some(url) = extract_attr(attrs, "src") {
            if url.starts_with("http://") || url.starts_with("https://") {
                return Some(trim_url(url).to_string());
            }
        }
    }

    // 3. HTML <a ... href="URL" ...>
    if let Some(attrs) = find_tag_attrs(content, "a") {
        if let Some(url) = extract_attr(attrs, "href") {
            if url.starts_with("http://") || url.starts_with("https://") {
                return Some(trim_url(url).to_string());
            }
        }
    }

    // 4. Bare URL scan
    scan_url(content)
}

/// Extract a video URL from response content.
///
/// Tries, in order:
/// 1. HTML `<video ... src="URL">` or `<source ... src="URL">`
/// 2. Any URL containing `.mp4`
/// 3. Fallback to `extract_image_url`
fn extract_video_url(content: &str) -> Option<String> {
    // 1. <video src="URL"> or <source src="URL">
    for tag in ["video", "source"] {
        if let Some(attrs) = find_tag_attrs(content, tag) {
            if let Some(url) = extract_attr(attrs, "src") {
                if url.starts_with("http://") || url.starts_with("https://") {
                    return Some(trim_url(url).to_string());
                }
            }
        }
    }

    // 2. Scan for any URL containing `.mp4`
    for scheme in ["https://", "http://"] {
        let mut search_from = 0;
        while let Some(pos) = content[search_from..].find(scheme) {
            let abs = search_from + pos;
            let url_end = content[abs..]
                .find(|c: char| {
                    c.is_whitespace() || matches!(c, '"' | '\'' | '<' | '>' | ')' | ']')
                })
                .map_or(content.len(), |e| abs + e);
            let candidate = trim_url(&content[abs..url_end]);
            if candidate.contains(".mp4") {
                return Some(candidate.to_string());
            }
            search_from = abs + scheme.len();
        }
    }

    // 3. Fallback
    extract_image_url(content)
}

#[cfg(test)]
mod url_extraction_tests {
    use super::*;

    #[test]
    fn markdown_image() {
        let c = "Here is your image: ![result](https://example.com/img.png)";
        assert_eq!(extract_image_url(c).unwrap(), "https://example.com/img.png");
    }

    #[test]
    fn html_img_tag() {
        let c = r#"<img src="https://cdn.example.com/a.jpg" alt="pic">"#;
        assert_eq!(
            extract_image_url(c).unwrap(),
            "https://cdn.example.com/a.jpg"
        );
    }

    #[test]
    fn html_a_href() {
        let c = r#"Click <a href="https://example.com/download.png">here</a>"#;
        assert_eq!(
            extract_image_url(c).unwrap(),
            "https://example.com/download.png"
        );
    }

    #[test]
    fn bare_url() {
        let c = "Generated: https://example.com/output.png enjoy!";
        assert_eq!(
            extract_image_url(c).unwrap(),
            "https://example.com/output.png"
        );
    }

    #[test]
    fn bare_url_with_trailing_paren() {
        let c = "(https://example.com/img.png)";
        assert_eq!(extract_image_url(c).unwrap(), "https://example.com/img.png");
    }

    #[test]
    fn video_tag_src() {
        let c = r#"<video src="https://cdn.example.com/v.mp4" controls></video>"#;
        assert_eq!(
            extract_video_url(c).unwrap(),
            "https://cdn.example.com/v.mp4"
        );
    }

    #[test]
    fn source_tag_src() {
        let c = r#"<video><source src="https://cdn.example.com/v.mp4" type="video/mp4"></video>"#;
        assert_eq!(
            extract_video_url(c).unwrap(),
            "https://cdn.example.com/v.mp4"
        );
    }

    #[test]
    fn mp4_in_plain_text() {
        let c = "Your video: https://example.com/output.mp4 is ready";
        assert_eq!(
            extract_video_url(c).unwrap(),
            "https://example.com/output.mp4"
        );
    }

    #[test]
    fn video_fallback_to_image() {
        let c = "![thumb](https://example.com/thumb.jpg)";
        assert_eq!(
            extract_video_url(c).unwrap(),
            "https://example.com/thumb.jpg"
        );
    }

    #[test]
    fn single_quoted_attr() {
        let c = "<img src='https://example.com/img.png' />";
        assert_eq!(extract_image_url(c).unwrap(), "https://example.com/img.png");
    }

    #[test]
    fn no_url_returns_none() {
        assert!(extract_image_url("no url here").is_none());
        assert!(extract_video_url("no url here").is_none());
    }
}
