use std::time::Instant;

use crate::core::*;
use serde_json::{json, Value};

const DEFAULT_MODEL: &str = "gemini-3.1-flash-tts-preview";

/// Gemini TTS provider (Google).
/// Supports single-speaker and multi-speaker text-to-speech via generateContent.
/// Output is raw PCM (s16le, 24000 Hz, mono) returned as base64 in `output_data`.
pub struct GeminiTtsProvider {
    pub id: String,
    pub base_url: String,
    pub api_key: String,
    vertex_auth: Option<crate::vertex_auth::VertexAuth>,
    vertex_endpoint: Option<String>,
    vertex_project: Option<String>,
    vertex_location: Option<String>,
    http: reqwest::Client,
}

impl GeminiTtsProvider {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            id: "gemini_tts".into(),
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

    /// Build `speechConfig` for single-speaker or multi-speaker mode.
    fn build_speech_config(req: &GenerateRequest) -> Value {
        if let Some(speakers) = req.params.get("speakers").and_then(|v| v.as_object()) {
            // Multi-speaker mode
            let configs: Vec<Value> = speakers
                .iter()
                .map(|(name, voice)| {
                    let voice_name = voice.as_str().unwrap_or("Kore");
                    json!({
                        "speaker": name,
                        "voiceConfig": {
                            "prebuiltVoiceConfig": { "voiceName": voice_name }
                        }
                    })
                })
                .collect();

            json!({
                "multiSpeakerVoiceConfig": {
                    "speakerVoiceConfigs": configs
                }
            })
        } else {
            // Single-speaker mode
            let voice = req
                .params
                .get("voice")
                .and_then(|v| v.as_str())
                .unwrap_or("Kore");

            json!({
                "voiceConfig": {
                    "prebuiltVoiceConfig": { "voiceName": voice }
                }
            })
        }
    }
}

#[async_trait::async_trait]
impl AssetProvider for GeminiTtsProvider {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        if self.vertex_auth.is_some() {
            "Gemini TTS (Vertex AI)"
        } else {
            "Gemini TTS (Google)"
        }
    }

    fn asset_types(&self) -> &[AssetType] {
        &[AssetType::Tts]
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            priority: 150,
            ..Default::default()
        }
    }

    async fn generate(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        let model = req.model.as_deref().unwrap_or(DEFAULT_MODEL);
        let start = Instant::now();

        let prompt = req
            .prompt
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("Gemini TTS requires a text prompt"))?;

        let multi_speaker = req
            .params
            .get("speakers")
            .and_then(|v| v.as_object())
            .is_some();

        let speech_config = Self::build_speech_config(req);

        let body = json!({
            "contents": [{"role": "user", "parts": [{"text": prompt}]}],
            "generationConfig": {
                "responseModalities": ["AUDIO"],
                "speechConfig": speech_config
            }
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
            anyhow::bail!("Gemini TTS returned {}: {}", status, text);
        }

        let payload: Value = serde_json::from_str(&text)?;

        let audio_data = payload["candidates"][0]["content"]["parts"][0]["inlineData"]["data"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("Gemini TTS response does not contain audio data"))?
            .to_string();

        let voice = req
            .params
            .get("voice")
            .and_then(|v| v.as_str())
            .unwrap_or("Kore");

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url: None,
            output_data: Some(audio_data),
            metadata: json!({
                "model": model,
                "mime_type": "audio/l16; rate=24000; channels=1",
                "voice": voice,
                "multi_speaker": multi_speaker,
            }),
            cost_usd: Some(0.01),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();
        if let Some(auth) = &self.vertex_auth {
            match auth.access_token().await {
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
        } else {
            let resp = self
                .http
                .get(format!("{}/v1beta/models", self.base_url))
                .header("x-goog-api-key", &self.api_key)
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
}
