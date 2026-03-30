use std::time::Instant;

use crate::core::*;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};

const DEFAULT_BASE_URL: &str = "https://dashscope-intl.aliyuncs.com";
const SYNTHESIS_PATH: &str = "/api/v1/services/aigc/multimodal-generation/generation";
const CUSTOMIZATION_PATH: &str = "/api/v1/services/audio/tts/customization";
const DEFAULT_MODEL: &str = "qwen3-tts-flash";
const INSTRUCT_MODEL: &str = "qwen3-tts-instruct-flash";
const DEFAULT_VOICE: &str = "Cherry";
const VOICE_ENROLLMENT_MODEL: &str = "qwen-voice-enrollment";
const VOICE_DESIGN_MODEL: &str = "qwen-voice-design";
const COST_PER_100_CHARACTERS_USD: f64 = 0.00115;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QwenVoiceKind {
    Clone,
    Design,
}

impl QwenVoiceKind {
    fn customization_model(self) -> &'static str {
        match self {
            Self::Clone => VOICE_ENROLLMENT_MODEL,
            Self::Design => VOICE_DESIGN_MODEL,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Clone => "voice clone",
            Self::Design => "voice design",
        }
    }
}

fn parse_voice_kind(raw: &str) -> anyhow::Result<QwenVoiceKind> {
    match raw.trim().to_ascii_lowercase().as_str() {
        "vc" | "clone" => Ok(QwenVoiceKind::Clone),
        "vd" | "design" => Ok(QwenVoiceKind::Design),
        other => anyhow::bail!("invalid voice type '{other}', expected 'vc' or 'vd'"),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QwenVoiceCloneResult {
    pub voice: String,
    pub request_id: Option<String>,
    pub response: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QwenVoiceDesignResult {
    pub voice: String,
    pub request_id: Option<String>,
    pub preview_audio_data: Option<String>,
    pub sample_rate: Option<u32>,
    pub response: Value,
}

/// Qwen3-TTS provider backed by DashScope International (Singapore region).
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

    fn estimate_cost_usd(characters: u64) -> f64 {
        (characters as f64 / 100.0) * COST_PER_100_CHARACTERS_USD
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

    fn extract_audio_url(json: &Value) -> Option<String> {
        json.pointer("/output/audio/url")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }

    fn extract_audio_data(json: &Value) -> Option<String> {
        json.pointer("/output/audio/data")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string)
    }

    fn has_valid_audio_output(json: &Value) -> bool {
        Self::extract_audio_url(json).is_some() || Self::extract_audio_data(json).is_some()
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

    fn build_synthesis_body(req: &GenerateRequest) -> anyhow::Result<Value> {
        let text = req
            .prompt
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow::anyhow!("Qwen TTS requires a non-empty prompt (text)"))?;

        let instructions = req
            .params
            .get("instructions")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty());

        let requested_model = req
            .model
            .as_deref()
            .or_else(|| req.params.get("model").and_then(Value::as_str))
            .map(str::trim)
            .filter(|value| !value.is_empty());

        let model = match (requested_model, instructions.is_some()) {
            (Some(model), _) => model,
            (None, true) => INSTRUCT_MODEL,
            (None, false) => DEFAULT_MODEL,
        };

        if instructions.is_some() && model != INSTRUCT_MODEL {
            anyhow::bail!(
                "Qwen TTS instructions require model {} (got {})",
                INSTRUCT_MODEL,
                model
            );
        }

        let voice = req
            .params
            .get("voice")
            .or_else(|| req.params.get("voice_id"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or(DEFAULT_VOICE);

        let mut input = json!({
            "text": text,
            "voice": voice,
        });

        if let Some(language_type) = req
            .params
            .get("language_type")
            .or_else(|| req.params.get("language_boost"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
        {
            input["language_type"] = json!(language_type);
        }

        if let Some(instructions) = instructions {
            input["instructions"] = json!(instructions);
        }

        if let Some(optimize_instructions) = req
            .params
            .get("optimize_instructions")
            .and_then(Value::as_bool)
        {
            input["optimize_instructions"] = json!(optimize_instructions);
        }

        Ok(json!({
            "model": model,
            "input": input,
        }))
    }

    pub async fn clone_voice(
        &self,
        target_model: &str,
        preferred_name: &str,
        audio_data_uri: &str,
    ) -> anyhow::Result<QwenVoiceCloneResult> {
        let target_model = target_model.trim();
        if target_model.is_empty() {
            anyhow::bail!("Qwen voice clone requires target_model");
        }

        let preferred_name = preferred_name.trim();
        if preferred_name.is_empty() {
            anyhow::bail!("Qwen voice clone requires preferred_name");
        }

        let audio_data_uri = audio_data_uri.trim();
        if audio_data_uri.is_empty() {
            anyhow::bail!("Qwen voice clone requires audio data");
        }

        let body = json!({
            "model": VOICE_ENROLLMENT_MODEL,
            "input": {
                "action": "create",
                "target_model": target_model,
                "preferred_name": preferred_name,
                "audio": {
                    "data": audio_data_uri,
                },
            },
        });

        let json = Self::parse_json_response(
            self.request_builder(CUSTOMIZATION_PATH)
                .json(&body)
                .send()
                .await?,
            "voice clone",
        )
        .await?;

        let voice = json
            .pointer("/output/voice")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .ok_or_else(|| anyhow::anyhow!("Qwen voice clone: missing output.voice in response"))?
            .to_string();

        Ok(QwenVoiceCloneResult {
            voice,
            request_id: json
                .get("request_id")
                .and_then(Value::as_str)
                .map(str::to_string),
            response: json,
        })
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
                .pointer("/output/data")
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(str::to_string),
            sample_rate: json
                .pointer("/output/sample_rate")
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
        let kind = parse_voice_kind(voice_type)?;
        let body = json!({
            "model": kind.customization_model(),
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
            kind.label(),
        )
        .await
    }

    pub async fn delete_voice(&self, voice_type: &str, voice_id: &str) -> anyhow::Result<Value> {
        let kind = parse_voice_kind(voice_type)?;
        let voice_id = voice_id.trim();
        if voice_id.is_empty() {
            anyhow::bail!("Qwen voice delete requires a voice id");
        }

        let body = json!({
            "model": kind.customization_model(),
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
            &format!("{} delete", kind.label()),
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
        "Qwen3 TTS (DashScope Intl)"
    }

    fn asset_types(&self) -> &[AssetType] {
        &[AssetType::Tts]
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            max_concurrent: 3,
            priority: 100,
            ..Default::default()
        }
    }

    async fn generate(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        let start = Instant::now();
        let body = Self::build_synthesis_body(req)?;
        let json = Self::parse_json_response(
            self.request_builder(SYNTHESIS_PATH)
                .json(&body)
                .send()
                .await?,
            "synthesis",
        )
        .await?;

        let output_url = Self::extract_audio_url(&json);
        let output_data = Self::extract_audio_data(&json);
        if output_url.is_none() && output_data.is_none() {
            anyhow::bail!("Qwen TTS: missing output.audio.url or output.audio.data in response");
        }

        let usage_characters = json.pointer("/usage/characters").and_then(Value::as_u64);

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url,
            output_data,
            metadata: json!({
                "model": body["model"],
                "voice": body["input"]["voice"],
                "language_type": body["input"]["language_type"],
                "instructions": body["input"]["instructions"],
                "optimize_instructions": body["input"]["optimize_instructions"],
                "request_id": json["request_id"],
                "finish_reason": json["output"]["finish_reason"],
                "audio_id": json["output"]["audio"]["id"],
                "audio_expires_at": json["output"]["audio"]["expires_at"],
                "usage_characters": usage_characters,
            }),
            cost_usd: usage_characters.map(Self::estimate_cost_usd),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();
        let body = json!({
            "model": DEFAULT_MODEL,
            "input": {
                "text": "hi",
                "voice": DEFAULT_VOICE,
            },
        });

        let resp = self
            .request_builder(SYNTHESIS_PATH)
            .json(&body)
            .send()
            .await;

        match resp {
            Ok(r) => {
                let http_ok = r.status().is_success();
                let text = r.text().await.unwrap_or_default();
                let json: Value = serde_json::from_str(&text).unwrap_or_default();
                let healthy =
                    http_ok && Self::api_status_ok(&json) && Self::has_valid_audio_output(&json);

                Ok(HealthStatus {
                    healthy,
                    latency_ms: Some(start.elapsed().as_millis() as u64),
                    message: if healthy {
                        None
                    } else {
                        Some(format!(
                            "http_ok={}, api_status_ok={}, has_audio_output={}, message={}",
                            http_ok,
                            Self::api_status_ok(&json),
                            Self::has_valid_audio_output(&json),
                            Self::response_message(&json)
                        ))
                    },
                })
            }
            Err(error) => Ok(HealthStatus {
                healthy: false,
                latency_ms: Some(start.elapsed().as_millis() as u64),
                message: Some(error.to_string()),
            }),
        }
    }
}
