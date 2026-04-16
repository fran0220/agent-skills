use crate::core::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const DEFAULT_BASE_URL: &str = "https://dashscope-intl.aliyuncs.com";
const CUSTOMIZATION_PATH: &str = "/api/v1/services/audio/tts/customization";
const VOICE_DESIGN_MODEL: &str = "qwen-voice-design";

fn parse_voice_kind(raw: &str) -> anyhow::Result<&'static str> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "vd" | "design" => Ok("design"),
        other => anyhow::bail!("invalid voice type '{other}', expected 'vd'"),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QwenVoiceDesignResult {
    pub voice: String,
    pub request_id: Option<String>,
    pub preview_audio_data: Option<String>,
    pub sample_rate: Option<u32>,
    pub response: Value,
}

/// Qwen3-TTS voice customization provider (clone / design / list / delete).
pub struct QwenTtsProvider {
    pub id: String,
    pub base_url: String,
    pub api_key: String,
    http: reqwest::Client,
}

impl QwenTtsProvider {
    pub fn new(base_url: String, api_key: String) -> Self {
        let base_url = if base_url.trim().is_empty() {
            DEFAULT_BASE_URL.to_string()
        } else {
            base_url.trim_end_matches('/').to_string()
        };

        Self {
            id: "qwen_tts".into(),
            base_url,
            api_key,
            http: reqwest::Client::new(),
        }
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    fn request_builder(&self, path: &str) -> reqwest::RequestBuilder {
        self.http
            .post(self.endpoint(path))
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
    }

    fn response_message(json: &Value) -> String {
        json.get("message")
            .and_then(Value::as_str)
            .or_else(|| json.pointer("/output/message").and_then(Value::as_str))
            .or_else(|| json.get("code").and_then(Value::as_str))
            .unwrap_or("unknown error")
            .to_string()
    }

    fn api_status_ok(json: &Value) -> bool {
        json.get("status_code")
            .and_then(Value::as_u64)
            .map(|status| status == 200)
            .unwrap_or(true)
    }

    async fn parse_json_response(resp: reqwest::Response, label: &str) -> anyhow::Result<Value> {
        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("Qwen TTS {} returned {}: {}", label, status, text);
        }

        let json: Value = serde_json::from_str(&text)?;
        if !Self::api_status_ok(&json) {
            let code = json.get("status_code").and_then(Value::as_u64).unwrap_or(0);
            anyhow::bail!(
                "Qwen TTS {} failed ({}): {}",
                label,
                code,
                Self::response_message(&json)
            );
        }

        Ok(json)
    }

    pub async fn design_voice(
        &self,
        target_model: &str,
        preferred_name: &str,
        voice_prompt: &str,
        preview_text: &str,
        language: &str,
    ) -> anyhow::Result<QwenVoiceDesignResult> {
        let target_model = target_model.trim();
        if target_model.is_empty() {
            anyhow::bail!("Qwen voice design requires target_model");
        }

        let preferred_name = preferred_name.trim();
        if preferred_name.is_empty() {
            anyhow::bail!("Qwen voice design requires preferred_name");
        }
        if preferred_name.len() > 16
            || !preferred_name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            anyhow::bail!(
                "preferred_name must be 1-16 characters, only letters/digits/underscores (got '{preferred_name}')"
            );
        }

        let voice_prompt = voice_prompt.trim();
        if voice_prompt.is_empty() {
            anyhow::bail!("Qwen voice design requires voice_prompt");
        }

        let preview_text = preview_text.trim();
        if preview_text.is_empty() {
            anyhow::bail!("Qwen voice design requires preview_text");
        }

        let language = language.trim();

        let mut body = json!({
            "model": VOICE_DESIGN_MODEL,
            "input": {
                "action": "create",
                "target_model": target_model,
                "voice_prompt": voice_prompt,
                "preview_text": preview_text,
                "preferred_name": preferred_name,
            },
        });
        if !language.is_empty() {
            body["input"]["language"] = json!(language);
        }

        let json = Self::parse_json_response(
            self.request_builder(CUSTOMIZATION_PATH)
                .json(&body)
                .send()
                .await?,
            "voice design",
        )
        .await?;

        let voice = json
            .pointer("/output/voice")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow::anyhow!("Qwen voice design: missing output.voice in response"))?
            .to_string();

        Ok(QwenVoiceDesignResult {
            voice,
            request_id: json
                .get("request_id")
                .and_then(Value::as_str)
                .map(str::to_string),
            preview_audio_data: json
                .pointer("/output/preview_audio/data")
                .or_else(|| json.pointer("/output/data"))
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
            sample_rate: json
                .pointer("/output/preview_audio/sample_rate")
                .or_else(|| json.pointer("/output/sample_rate"))
                .and_then(Value::as_u64)
                .map(|value| value as u32),
            response: json,
        })
    }

    pub async fn list_voices(
        &self,
        voice_type: &str,
        page_size: u32,
        page_index: u32,
    ) -> anyhow::Result<Value> {
        let _kind = parse_voice_kind(voice_type)?;
        let body = json!({
            "model": VOICE_DESIGN_MODEL,
            "input": {
                "action": "list",
                "page_size": page_size,
                "page_index": page_index,
            },
        });

        Self::parse_json_response(
            self.request_builder(CUSTOMIZATION_PATH)
                .json(&body)
                .send()
                .await?,
            "voice design list",
        )
        .await
    }

    /// Synthesize speech using a designed voice via DashScope TTS API.
    pub async fn synthesize(
        &self,
        voice: &str,
        text: &str,
        model: Option<&str>,
        language: Option<&str>,
    ) -> anyhow::Result<Value> {
        let voice = voice.trim();
        if voice.is_empty() {
            anyhow::bail!("Qwen TTS synthesize requires a voice name");
        }
        let text = text.trim();
        if text.is_empty() {
            anyhow::bail!("Qwen TTS synthesize requires non-empty text");
        }

        // Default model for voice-design voices
        let model = model
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or("qwen3-tts-vd-realtime-2025-12-16");

        let language = language
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or("Auto");

        let url = format!(
            "{}/api/v1/services/aigc/multimodal-generation/generation",
            self.base_url
        );

        let body = json!({
            "model": model,
            "input": {
                "text": text,
                "voice": voice,
                "language_type": language,
            },
        });

        let resp = self
            .http
            .post(&url)
            .bearer_auth(&self.api_key)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        Self::parse_json_response(resp, "synthesize").await
    }

    pub async fn delete_voice(&self, voice_type: &str, voice_id: &str) -> anyhow::Result<Value> {
        let _kind = parse_voice_kind(voice_type)?;
        let voice_id = voice_id.trim();
        if voice_id.is_empty() {
            anyhow::bail!("Qwen voice delete requires a voice id");
        }

        let body = json!({
            "model": VOICE_DESIGN_MODEL,
            "input": {
                "action": "delete",
                "voice": voice_id,
            },
        });

        Self::parse_json_response(
            self.request_builder(CUSTOMIZATION_PATH)
                .json(&body)
                .send()
                .await?,
            "voice design delete",
        )
        .await
    }
}

#[async_trait::async_trait]
impl AssetProvider for QwenTtsProvider {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        "Qwen TTS Voice Customization"
    }

    fn asset_types(&self) -> &[AssetType] {
        &[]
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities::default()
    }

    async fn generate(&self, _req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        anyhow::bail!("QwenTtsProvider does not support direct generation; use MOSS-TTS-Nano instead")
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        Ok(HealthStatus {
            healthy: true,
            latency_ms: None,
            message: Some("voice customization only — no TTS generation".into()),
        })
    }
}
