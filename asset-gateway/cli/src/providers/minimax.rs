use std::time::Instant;

use crate::core::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};

/// MiniMax TTS provider — Chinese/multilingual text-to-speech via sync HTTP API.
///
/// Uses the synchronous `/minimax/v1/t2a_v2` endpoint through a proxy.
/// Returns hex-encoded audio inline, decoded to base64 for output.
pub struct MinimaxTtsProvider {
    pub id: String,
    pub proxy_url: String,
    pub proxy_key: String,
    http: reqwest::Client,
}

impl MinimaxTtsProvider {
    pub fn new(proxy_url: String, proxy_key: String) -> Self {
        Self {
            id: "minimax_tts".into(),
            proxy_url,
            proxy_key,
            http: reqwest::Client::new(),
        }
    }

    fn auth_header(&self) -> String {
        format!("Bearer {}", self.proxy_key)
    }

    fn build_request_body(req: &GenerateRequest) -> anyhow::Result<Value> {
        let text = req
            .prompt
            .as_deref()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .ok_or_else(|| anyhow::anyhow!("MiniMax TTS requires a non-empty prompt (text)"))?;

        let model = req
            .model
            .as_deref()
            .or_else(|| req.params.get("model").and_then(Value::as_str))
            .unwrap_or("speech-2.6-hd");

        let voice_id = req
            .params
            .get("voice_id")
            .and_then(Value::as_str)
            .unwrap_or("Chinese (Mandarin)_Lyrical_Voice");

        // Voice settings
        let mut voice_setting = json!({ "voice_id": voice_id });
        if let Some(speed) = req.params.get("speed").and_then(Value::as_f64) {
            voice_setting["speed"] = json!(speed);
        }
        if let Some(vol) = req.params.get("vol").and_then(Value::as_f64) {
            voice_setting["vol"] = json!(vol);
        }
        if let Some(pitch) = req.params.get("pitch").and_then(Value::as_i64) {
            voice_setting["pitch"] = json!(pitch);
        }
        if let Some(emotion) = req.params.get("emotion").and_then(Value::as_str) {
            voice_setting["emotion"] = json!(emotion);
        }

        let mut body = json!({
            "model": model,
            "text": text,
            "stream": false,
            "voice_setting": voice_setting,
        });

        // Language boost
        if let Some(lb) = req.params.get("language_boost").and_then(Value::as_str) {
            body["language_boost"] = json!(lb);
        }

        // Audio settings
        let format = req
            .params
            .get("format")
            .and_then(Value::as_str)
            .unwrap_or("mp3");
        let sample_rate = req
            .params
            .get("sample_rate")
            .and_then(Value::as_u64)
            .unwrap_or(32000);
        let bitrate = req
            .params
            .get("bitrate")
            .and_then(Value::as_u64)
            .unwrap_or(128000);
        let channel = req
            .params
            .get("channel")
            .and_then(Value::as_u64)
            .unwrap_or(1);

        body["audio_setting"] = json!({
            "sample_rate": sample_rate,
            "bitrate": bitrate,
            "format": format,
            "channel": channel,
        });

        // Output format: url returns a download link (valid 24h), hex returns inline data
        let output_format = req
            .params
            .get("output_format")
            .and_then(Value::as_str)
            .unwrap_or("url");
        body["output_format"] = json!(output_format);

        // Pronunciation dictionary (optional)
        if let Some(tone) = req.params.get("tone").and_then(Value::as_array) {
            body["pronunciation_dict"] = json!({ "tone": tone });
        }

        // Voice modify / effects (optional)
        if let Some(vm) = req.params.get("voice_modify").and_then(Value::as_object) {
            body["voice_modify"] = json!(vm);
        }

        Ok(body)
    }

    /// Decode hex string to bytes.
    fn hex_decode(hex: &str) -> anyhow::Result<Vec<u8>> {
        if hex.len() % 2 != 0 {
            anyhow::bail!("invalid hex string: odd length");
        }
        (0..hex.len())
            .step_by(2)
            .map(|i| {
                u8::from_str_radix(&hex[i..i + 2], 16)
                    .map_err(|e| anyhow::anyhow!("hex decode error at {}: {}", i, e))
            })
            .collect()
    }
}

#[async_trait::async_trait]
impl AssetProvider for MinimaxTtsProvider {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        "MiniMax TTS (Chinese/Multilingual)"
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
        let body = Self::build_request_body(req)?;
        let output_format = body["output_format"].as_str().unwrap_or("url");

        let resp = self
            .http
            .post(format!("{}/minimax/v1/t2a_v2", self.proxy_url))
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("MiniMax TTS returned {}: {}", status, text);
        }

        let json: Value = serde_json::from_str(&text)?;

        // Check API-level status
        let status_code = json["base_resp"]["status_code"].as_i64().unwrap_or(-1);
        if status_code != 0 {
            let msg = json["base_resp"]["status_msg"]
                .as_str()
                .unwrap_or("unknown error");
            anyhow::bail!("MiniMax TTS failed ({}): {}", status_code, msg);
        }

        let audio_raw = json["data"]["audio"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("MiniMax TTS: missing data.audio in response"))?;

        // Build response based on output format
        let (output_url, output_data) = if output_format == "url" {
            // audio field is a download URL
            (Some(audio_raw.to_string()), None)
        } else {
            // audio field is hex-encoded — decode to bytes then base64
            let bytes = Self::hex_decode(audio_raw)?;
            (None, Some(STANDARD.encode(bytes)))
        };

        let extra = &json["extra_info"];

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url,
            output_data,
            metadata: json!({
                "model": body["model"],
                "voice_id": body["voice_setting"]["voice_id"],
                "format": body["audio_setting"]["format"],
                "audio_length_ms": extra["audio_length"],
                "audio_size": extra["audio_size"],
                "word_count": extra["word_count"],
                "usage_characters": extra["usage_characters"],
                "trace_id": json["trace_id"],
            }),
            cost_usd: Some(0.005),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();

        let body = json!({
            "model": "speech-2.6-hd",
            "text": "hi",
            "stream": false,
            "voice_setting": { "voice_id": "English_radiant_girl" },
            "output_format": "url",
        });

        let resp = self
            .http
            .post(format!("{}/minimax/v1/t2a_v2", self.proxy_url))
            .header("Authorization", self.auth_header())
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await;

        match resp {
            Ok(r) => {
                let http_ok = r.status().is_success();
                let text = r.text().await.unwrap_or_default();
                let json: Value = serde_json::from_str(&text).unwrap_or_default();
                let api_code = json["base_resp"]["status_code"].as_i64().unwrap_or(-1);

                Ok(HealthStatus {
                    healthy: http_ok && api_code == 0,
                    latency_ms: Some(start.elapsed().as_millis() as u64),
                    message: if !http_ok || api_code != 0 {
                        Some(format!(
                            "http={}, api_code={}, msg={}",
                            http_ok,
                            api_code,
                            json["base_resp"]["status_msg"].as_str().unwrap_or("")
                        ))
                    } else {
                        None
                    },
                })
            }
            Err(e) => Ok(HealthStatus {
                healthy: false,
                latency_ms: Some(start.elapsed().as_millis() as u64),
                message: Some(e.to_string()),
            }),
        }
    }
}
