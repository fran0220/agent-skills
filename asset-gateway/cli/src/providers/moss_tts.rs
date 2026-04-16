use std::time::{Duration, Instant};

use serde_json::{json, Value};

use crate::core::*;

pub struct MossTtsProvider {
    pub id: String,
    pub base_url: String,
    api_key: String,
    http: reqwest::Client,
}

impl MossTtsProvider {
    pub fn new(base_url: String, api_key: String) -> Self {
        let base_url = base_url.trim_end_matches('/').to_string();

        Self {
            id: "moss_tts".into(),
            base_url,
            api_key,
            http: reqwest::Client::builder()
                .timeout(Duration::from_secs(120))
                .build()
                .expect("failed to build reqwest client"),
        }
    }

    fn endpoint(&self, path: &str) -> String {
        format!("{}{}", self.base_url, path)
    }

    async fn parse_json_response(resp: reqwest::Response, label: &str) -> anyhow::Result<Value> {
        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("MOSS-TTS {} returned {}: {}", label, status, text);
        }

        let json: Value = serde_json::from_str(&text)?;
        Ok(json)
    }

    async fn download_bytes(&self, url: &str) -> anyhow::Result<Vec<u8>> {
        let resp = self.http.get(url).send().await?;
        let status = resp.status();
        if !status.is_success() {
            anyhow::bail!("MOSS-TTS: failed to download {}: {}", url, status);
        }
        Ok(resp.bytes().await?.to_vec())
    }
}

#[async_trait::async_trait]
impl AssetProvider for MossTtsProvider {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        "MOSS-TTS-Nano (Self-hosted)"
    }

    fn asset_types(&self) -> &[AssetType] {
        &[AssetType::Tts]
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            max_concurrent: 2,
            priority: 120,
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
            .ok_or_else(|| anyhow::anyhow!("MOSS-TTS requires a non-empty prompt (text)"))?;

        let mut form = reqwest::multipart::Form::new().text("text", text.to_string());

        // Optional prompt_audio: base64 in params, or URL in input_file
        let has_prompt_audio;
        if let Some(b64) = req.params.get("prompt_audio").and_then(Value::as_str) {
            use base64::{engine::general_purpose::STANDARD, Engine as _};
            let audio_data = STANDARD.decode(b64)?;
            form = form.part(
                "prompt_audio",
                reqwest::multipart::Part::bytes(audio_data)
                    .file_name("prompt_audio.wav")
                    .mime_str("audio/wav")?,
            );
            has_prompt_audio = true;
        } else if let Some(input) = req.input_file.as_deref() {
            let audio_data = if input.starts_with("data:") {
                use base64::{engine::general_purpose::STANDARD, Engine as _};
                let raw = input.split_once(',').map_or(input, |(_, r)| r);
                STANDARD.decode(raw)?
            } else {
                self.download_bytes(input).await?
            };
            form = form.part(
                "prompt_audio",
                reqwest::multipart::Part::bytes(audio_data)
                    .file_name("prompt_audio.wav")
                    .mime_str("audio/wav")?,
            );
            has_prompt_audio = true;
        } else {
            has_prompt_audio = false;
        }

        let resp = self
            .http
            .post(self.endpoint("/api/generate"))
            .bearer_auth(&self.api_key)
            .multipart(form)
            .send()
            .await?;
        let json = Self::parse_json_response(resp, "generate").await?;

        let audio_base64 = json
            .get("audio_base64")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("MOSS-TTS: missing audio_base64 in response"))?
            .to_string();

        let sample_rate = json.get("sample_rate").and_then(Value::as_u64);
        let run_status = json
            .get("run_status")
            .and_then(Value::as_str)
            .map(str::to_string);

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url: None,
            output_data: Some(audio_base64),
            metadata: json!({
                "sample_rate": sample_rate,
                "run_status": run_status,
                "has_prompt_audio": has_prompt_audio,
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

                let status_ok = json
                    .get("status")
                    .and_then(Value::as_str)
                    .map(|s| s == "ok")
                    .unwrap_or(false);

                Ok(HealthStatus {
                    healthy: http_ok && status_ok,
                    latency_ms: Some(start.elapsed().as_millis() as u64),
                    message: if http_ok && status_ok {
                        None
                    } else {
                        Some(format!("http_ok={}, status_ok={}", http_ok, status_ok))
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
