use std::time::{Duration, Instant};

use crate::core::*;
use base64::{engine::general_purpose::STANDARD, Engine as _};
use reqwest::header::CONTENT_TYPE;
use serde_json::{json, Value};

/// ElevenLabs audio generation provider — sound effects (SFX).
pub struct ElevenLabsProvider {
    pub id: String,
    pub api_key: String,
    pub base_url: String,
    http: reqwest::Client,
}

impl ElevenLabsProvider {
    pub fn new(api_key: String) -> Self {
        let http = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(120))
            .build()
            .expect("failed to build ElevenLabs HTTP client");
        Self {
            id: "elevenlabs".into(),
            api_key,
            base_url: "https://api.elevenlabs.io".into(),
            http,
        }
    }

    #[allow(dead_code)]
    pub fn with_base_url(mut self, base_url: String) -> Self {
        self.base_url = base_url;
        self
    }

    async fn send_audio_request(
        &self,
        req: reqwest::RequestBuilder,
        label: &str,
    ) -> anyhow::Result<(Vec<u8>, Option<String>)> {
        let resp = req.send().await?;
        let status = resp.status();
        let content_type = resp
            .headers()
            .get(CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .map(str::to_string);

        if !status.is_success() {
            let text = resp.text().await?;
            anyhow::bail!("ElevenLabs {} returned {}: {}", label, status, text);
        }

        let bytes = resp.bytes().await?;
        Ok((bytes.to_vec(), content_type))
    }

    async fn generate_sound(
        &self,
        req: &GenerateRequest,
        prompt: &str,
    ) -> anyhow::Result<(Vec<u8>, Option<String>, &'static str)> {
        let mut body = json!({
            "text": prompt,
        });

        if let Some(duration) = req.params.get("duration_seconds").and_then(Value::as_f64) {
            body["duration_seconds"] = json!(duration);
        }

        if let Some(quality) = req.quality() {
            body["quality"] = json!(quality.as_str());
        }

        if let Some(style) = req.style() {
            body["style"] = json!(style);
        }

        let model = req
            .model
            .as_deref()
            .or_else(|| req.params.get("model_id").and_then(Value::as_str));
        if let Some(model) = model {
            body["model_id"] = json!(model);
        }

        let endpoint = format!("{}/v1/sound-generation", self.base_url);
        let request = self
            .http
            .post(endpoint)
            .header("xi-api-key", &self.api_key)
            .json(&body);

        let (bytes, content_type) = self.send_audio_request(request, "sound-generation").await?;
        Ok((bytes, content_type, "sound_generation"))
    }
}

#[async_trait::async_trait]
impl AssetProvider for ElevenLabsProvider {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        "ElevenLabs (Audio SFX)"
    }

    fn asset_types(&self) -> &[AssetType] {
        &[AssetType::Audio]
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            max_concurrent: 3,
            priority: 100,
            ..Default::default()
        }
    }

    async fn generate(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        let prompt = req.prompt.as_deref().unwrap_or("").trim();
        if prompt.is_empty() {
            anyhow::bail!("ElevenLabs requires a non-empty prompt");
        }

        let start = Instant::now();

        let (bytes, content_type, route) = self.generate_sound(req, prompt).await?;

        let b64 = STANDARD.encode(bytes);

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url: None,
            output_data: Some(b64),
            metadata: json!({
                "route": route,
                "content_type": content_type.unwrap_or_else(|| "audio/mpeg".to_string()),
                "duration_seconds": req.params.get("duration_seconds"),
            }),
            cost_usd: Some(0.05),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();
        let resp = self
            .http
            .get(format!("{}/v1/models", self.base_url))
            .header("xi-api-key", &self.api_key)
            .timeout(Duration::from_secs(10))
            .send()
            .await;

        match resp {
            Ok(r) => Ok(HealthStatus {
                healthy: r.status().is_success(),
                latency_ms: Some(start.elapsed().as_millis() as u64),
                message: if r.status().is_success() {
                    None
                } else {
                    Some(format!("HTTP {}", r.status()))
                },
            }),
            Err(e) => Ok(HealthStatus {
                healthy: false,
                latency_ms: Some(start.elapsed().as_millis() as u64),
                message: Some(e.to_string()),
            }),
        }
    }
}
