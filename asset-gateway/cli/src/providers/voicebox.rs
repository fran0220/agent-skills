use std::time::{Duration, Instant};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::{json, Value};

use crate::core::*;

const DEFAULT_BASE_URL: &str = "http://127.0.0.1:17493";
const DEFAULT_MODEL_SIZE: &str = "1.7B";
const POLL_INTERVAL: Duration = Duration::from_secs(1);
const MAX_WAIT: Duration = Duration::from_secs(120);

pub struct VoiceBoxProvider {
    pub id: String,
    pub base_url: String,
    http: reqwest::Client,
}

impl VoiceBoxProvider {
    pub fn new(base_url: String) -> Self {
        let base_url = if base_url.trim().is_empty() {
            DEFAULT_BASE_URL.to_string()
        } else {
            base_url.trim_end_matches('/').to_string()
        };

        Self {
            id: "voicebox".into(),
            base_url,
            http: reqwest::Client::new(),
        }
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    async fn parse_json_response(resp: reqwest::Response, label: &str) -> anyhow::Result<Value> {
        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("VoiceBox {} returned {}: {}", label, status, text);
        }

        let json: Value = serde_json::from_str(&text)?;
        Ok(json)
    }

    pub async fn ensure_model_loaded(&self) -> anyhow::Result<()> {
        let resp = self.http.get(self.endpoint("/health")).send().await?;
        let json = Self::parse_json_response(resp, "health").await?;

        let healthy = json
            .get("status")
            .and_then(Value::as_str)
            .map(|s| s == "healthy")
            .unwrap_or(false);
        let model_loaded = json
            .get("model_loaded")
            .and_then(Value::as_bool)
            .unwrap_or(false);

        if healthy && model_loaded {
            return Ok(());
        }

        let resp = self
            .http
            .post(self.endpoint("/models/load"))
            .query(&[("model_size", DEFAULT_MODEL_SIZE)])
            .send()
            .await?;
        Self::parse_json_response(resp, "model load").await?;
        Ok(())
    }

    pub async fn create_profile(&self, name: &str, language: &str) -> anyhow::Result<Value> {
        let resp = self
            .http
            .post(self.endpoint("/profiles"))
            .json(&json!({ "name": name, "language": language }))
            .send()
            .await?;
        Self::parse_json_response(resp, "create profile").await
    }

    pub async fn upload_sample(
        &self,
        profile_id: &str,
        audio_data: &[u8],
        reference_text: &str,
    ) -> anyhow::Result<Value> {
        let form = reqwest::multipart::Form::new()
            .part(
                "file",
                reqwest::multipart::Part::bytes(audio_data.to_vec())
                    .file_name("sample.wav")
                    .mime_str("audio/wav")?,
            )
            .text("reference_text", reference_text.to_string());

        let resp = self
            .http
            .post(self.endpoint(&format!("/profiles/{}/samples", profile_id)))
            .multipart(form)
            .send()
            .await?;
        Self::parse_json_response(resp, "upload sample").await
    }

    pub async fn list_profiles(&self) -> anyhow::Result<Value> {
        let resp = self.http.get(self.endpoint("/profiles")).send().await?;
        Self::parse_json_response(resp, "list profiles").await
    }
}

#[async_trait::async_trait]
impl AssetProvider for VoiceBoxProvider {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        "VoiceBox TTS (Self-hosted)"
    }

    fn asset_types(&self) -> &[AssetType] {
        &[AssetType::Tts]
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            max_concurrent: 1,
            priority: 110,
            ..Default::default()
        }
    }

    async fn generate(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        let start = Instant::now();

        let text = req
            .prompt
            .as_deref()
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| anyhow::anyhow!("VoiceBox TTS requires a non-empty prompt (text)"))?;

        let profile_id = req
            .params
            .get("profile_id")
            .or_else(|| req.params.get("voice"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .ok_or_else(|| {
                anyhow::anyhow!("VoiceBox TTS requires profile_id or voice in params")
            })?;

        let language = req
            .params
            .get("language_type")
            .or_else(|| req.params.get("language"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or("en");

        let engine = req
            .params
            .get("engine")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .unwrap_or("qwen");

        let instruct = req
            .params
            .get("instructions")
            .or_else(|| req.params.get("instruct"))
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|v| !v.is_empty());

        let seed = req.params.get("seed").and_then(Value::as_u64);

        // POST /generate
        let mut gen_body = json!({
            "profile_id": profile_id,
            "text": text,
            "language": language,
            "engine": engine,
        });
        if let Some(instruct) = instruct {
            gen_body["instruct"] = json!(instruct);
        }
        if let Some(seed) = seed {
            gen_body["seed"] = json!(seed);
        }
        let resp = self
            .http
            .post(self.endpoint("/generate"))
            .json(&gen_body)
            .send()
            .await?;
        let gen_json = Self::parse_json_response(resp, "generate").await?;

        let generation_id = gen_json
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("VoiceBox generate: missing id in response"))?
            .to_string();

        // Poll GET /history/{id}
        let poll_start = Instant::now();
        let (audio_path, duration) = loop {
            if poll_start.elapsed() > MAX_WAIT {
                anyhow::bail!(
                    "VoiceBox TTS: generation {} timed out after {}s",
                    generation_id,
                    MAX_WAIT.as_secs()
                );
            }

            tokio::time::sleep(POLL_INTERVAL).await;

            let resp = self
                .http
                .get(self.endpoint(&format!("/history/{}", generation_id)))
                .send()
                .await?;
            let history = Self::parse_json_response(resp, "history poll").await?;

            let status = history
                .get("status")
                .and_then(Value::as_str)
                .unwrap_or("unknown");

            match status {
                "completed" => {
                    let audio_path = history
                        .get("audio_path")
                        .and_then(Value::as_str)
                        .map(str::to_string);
                    let duration = history.get("duration").and_then(Value::as_f64);
                    break (audio_path, duration);
                }
                "error" => {
                    let msg = history
                        .get("error")
                        .or_else(|| history.get("message"))
                        .and_then(Value::as_str)
                        .unwrap_or("unknown error");
                    anyhow::bail!("VoiceBox TTS generation failed: {}", msg);
                }
                _ => continue,
            }
        };

        // GET /audio/{id} — download WAV binary
        let resp = self
            .http
            .get(self.endpoint(&format!("/audio/{}", generation_id)))
            .send()
            .await?;
        let audio_status = resp.status();
        if !audio_status.is_success() {
            anyhow::bail!("VoiceBox TTS: audio download returned {}", audio_status);
        }
        let wav_bytes = resp.bytes().await?;
        let output_data = STANDARD.encode(&wav_bytes);

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url: None,
            output_data: Some(output_data),
            metadata: json!({
                "generation_id": generation_id,
                "profile_id": profile_id,
                "language": language,
                "engine": engine,
                "instruct": instruct,
                "seed": seed,
                "audio_path": audio_path,
                "duration_secs": duration,
            }),
            cost_usd: None,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();

        let resp = self.http.get(self.endpoint("/health")).send().await;
        match resp {
            Ok(r) => {
                let http_ok = r.status().is_success();
                let text = r.text().await.unwrap_or_default();
                let json: Value = serde_json::from_str(&text).unwrap_or_default();

                let status_healthy = json
                    .get("status")
                    .and_then(Value::as_str)
                    .map(|s| s == "healthy")
                    .unwrap_or(false);
                let model_loaded = json
                    .get("model_loaded")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);

                let healthy = http_ok && status_healthy && model_loaded;

                if healthy {
                    Ok(HealthStatus {
                        healthy: true,
                        latency_ms: Some(start.elapsed().as_millis() as u64),
                        message: None,
                    })
                } else {
                    // Try loading the model
                    let load_result = self
                        .http
                        .post(self.endpoint("/models/load"))
                        .query(&[("model_size", DEFAULT_MODEL_SIZE)])
                        .send()
                        .await;

                    let loaded = matches!(load_result, Ok(r) if r.status().is_success());

                    Ok(HealthStatus {
                        healthy: loaded,
                        latency_ms: Some(start.elapsed().as_millis() as u64),
                        message: if loaded {
                            Some("model was not loaded, loaded successfully".into())
                        } else {
                            Some(format!(
                                "http_ok={}, status_healthy={}, model_loaded={}",
                                http_ok, status_healthy, model_loaded
                            ))
                        },
                    })
                }
            }
            Err(error) => Ok(HealthStatus {
                healthy: false,
                latency_ms: Some(start.elapsed().as_millis() as u64),
                message: Some(error.to_string()),
            }),
        }
    }
}
