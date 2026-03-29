use std::time::Instant;

use crate::core::*;
use serde_json::{json, Value};

/// Jimeng (即梦) provider — image + video generation.
/// Communicates with jimeng-gateway's OpenAI-compatible endpoints.
pub struct JimengProvider {
    pub id: String,
    pub base_url: String,
    pub api_key: String,
    http: reqwest::Client,
}

impl JimengProvider {
    pub fn new(base_url: String, api_key: String) -> Self {
        Self {
            id: "jimeng".into(),
            base_url,
            api_key,
            http: reqwest::Client::builder()
                .timeout(std::time::Duration::from_secs(660))
                .build()
                .unwrap_or_else(|_| reqwest::Client::new()),
        }
    }
}

#[async_trait::async_trait]
impl AssetProvider for JimengProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        "Jimeng (即梦 Image + Seedance Video)"
    }

    fn asset_types(&self) -> &[AssetType] {
        &[AssetType::Image, AssetType::Video]
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            max_concurrent: 3,
            priority: 80,
            ..Default::default()
        }
    }

    async fn generate(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        let prompt = req.prompt.as_deref().unwrap_or("");
        let start = Instant::now();

        let endpoint = if req.asset_type == AssetType::Video {
            "/v1/videos/generations"
        } else {
            "/v1/images/generations"
        };

        let mut body = json!({ "prompt": prompt });

        if let Some(model) = req.model.as_deref() {
            body["model"] = json!(model);
        }

        if let Some(size) = req.params.get("size").and_then(Value::as_str) {
            body["size"] = json!(size);
        }

        if let Some(ratio) = req.params.get("ratio").and_then(Value::as_str) {
            body["ratio"] = json!(ratio);
        }

        if let Some(duration) = req.params.get("duration").and_then(Value::as_i64) {
            body["duration"] = json!(duration);
        }

        let resp = self
            .http
            .post(format!("{}{}", self.base_url, endpoint))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        let text = resp.text().await?;

        if !status.is_success() {
            anyhow::bail!("Jimeng returned {}: {}", status, text);
        }

        let payload: Value = serde_json::from_str(&text)?;

        // OpenAI-compatible response: {"data": [{"url": "...", "revised_prompt": "..."}]}
        let output_url = payload["data"][0]["url"]
            .as_str()
            .map(str::to_string);

        if output_url.is_none() {
            anyhow::bail!("Jimeng response missing data[0].url: {}", text);
        }

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url,
            output_data: None,
            metadata: json!({
                "asset_type": req.asset_type.as_str(),
                "model": req.model,
            }),
            cost_usd: None,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();
        let resp = self
            .http
            .get(format!("{}/ping", self.base_url))
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
