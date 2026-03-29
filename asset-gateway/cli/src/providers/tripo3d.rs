use std::time::{Duration, Instant};

use crate::core::*;
use reqwest::StatusCode;
use serde_json::{json, Value};

/// Tripo3D provider — image/text to 3D model generation.
pub struct Tripo3dProvider {
    pub id: String,
    pub api_key: String,
    pub base_url: String,
    http: reqwest::Client,
}

impl Tripo3dProvider {
    pub fn new(api_key: String) -> Self {
        Self {
            id: "tripo3d".into(),
            api_key,
            base_url: "https://api.tripo3d.ai/v2/openapi".into(),
            http: reqwest::Client::new(),
        }
    }

    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = url;
        self
    }

    /// Known top-level Tripo3D API fields (not passed through from params).
    const RESERVED_PARAMS: &[&str] = &[
        "timeout_seconds", "style", "quality", "transparent", "stream",
        "output_format",
    ];

    fn build_task_body(req: &GenerateRequest) -> anyhow::Result<Value> {
        let mut body = if let Some(input_file) = req.input_file.as_ref() {
            json!({
                "type": "image_to_model",
                "file": {
                    "type": "url",
                    "url": input_file,
                }
            })
        } else if let Some(prompt) = req.prompt.as_ref() {
            json!({
                "type": "text_to_model",
                "prompt": prompt,
            })
        } else {
            anyhow::bail!("Tripo3D requires either input_file or prompt");
        };

        // Tripo3D uses `model_version`, not `model`
        if let Some(model) = req.model.as_ref() {
            body["model_version"] = json!(model);
        }

        // Pass through extra params (e.g. face_limit, texture, pbr) at top level
        if let Some(params_obj) = req.params.as_object() {
            for (k, v) in params_obj {
                if !Self::RESERVED_PARAMS.contains(&k.as_str()) {
                    body[k] = v.clone();
                }
            }
        }

        Ok(body)
    }

    /// Extract the model download URL from the completed task response.
    /// Tripo3D response: `data.output.model` or `data.output.pbr_model` (URL strings).
    fn extract_model_url(payload: &Value) -> Option<String> {
        let output = &payload["data"]["output"];
        // Prefer `model`, then `pbr_model`, then `base_model`
        output["model"]
            .as_str()
            .or_else(|| output["pbr_model"].as_str())
            .or_else(|| output["base_model"].as_str())
            .map(str::to_string)
    }
}

#[async_trait::async_trait]
impl AssetProvider for Tripo3dProvider {
    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        "Tripo3D (Image/Text to 3D)"
    }

    fn asset_types(&self) -> &[AssetType] {
        &[AssetType::Model3d]
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            max_concurrent: 2,
            priority: 100,
            ..Default::default()
        }
    }

    async fn generate(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        let start = Instant::now();
        let body = Self::build_task_body(req)?;

        let create_resp = self
            .http
            .post(format!("{}/task", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        let create_status = create_resp.status();
        let create_text = create_resp.text().await?;

        if !create_status.is_success() {
            anyhow::bail!(
                "Tripo3D task creation returned {}: {}",
                create_status,
                create_text
            );
        }

        let create_json: Value = serde_json::from_str(&create_text)?;
        let task_id = create_json["data"]["task_id"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("missing task_id in response"))?
            .to_string();

        let timeout_secs = req
            .params
            .get("timeout_seconds")
            .and_then(Value::as_u64)
            .unwrap_or(300);
        let deadline = Instant::now() + Duration::from_secs(timeout_secs);
        let mut poll_interval = Duration::from_secs(2);

        loop {
            if Instant::now() >= deadline {
                anyhow::bail!("Tripo3D task {} timed out after {}s", task_id, timeout_secs);
            }

            tokio::time::sleep(poll_interval).await;

            let poll_resp = self
                .http
                .get(format!("{}/task/{}", self.base_url, task_id))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .send()
                .await?;

            let poll_status = poll_resp.status();
            let poll_text = poll_resp.text().await?;

            if !poll_status.is_success() {
                if poll_status.is_server_error() || poll_status == StatusCode::TOO_MANY_REQUESTS {
                    poll_interval = (poll_interval * 2).min(Duration::from_secs(20));
                    continue;
                }
                anyhow::bail!(
                    "Tripo3D polling returned {} for task {}: {}",
                    poll_status,
                    task_id,
                    poll_text
                );
            }

            let poll_json: Value = serde_json::from_str(&poll_text)?;
            let task_status = poll_json["data"]["status"]
                .as_str()
                .unwrap_or("unknown")
                .to_ascii_lowercase();

            match task_status.as_str() {
                "success" | "completed" | "succeeded" => {
                    let model_url = Self::extract_model_url(&poll_json)
                        .ok_or_else(|| {
                            anyhow::anyhow!(
                                "Tripo3D task completed without model URL. output: {}",
                                serde_json::to_string(&poll_json["data"]["output"])
                                    .unwrap_or_default()
                            )
                        })?;

                    let rendered_image = poll_json["data"]["output"]["rendered_image"]
                        .as_str()
                        .map(str::to_string);

                    return Ok(GenerateResponse {
                        provider_id: self.id.clone(),
                        output_path: None,
                        output_url: Some(model_url),
                        output_data: None,
                        metadata: json!({
                            "task_id": task_id,
                            "rendered_image": rendered_image,
                        }),
                        cost_usd: None,
                        elapsed_ms: start.elapsed().as_millis() as u64,
                    });
                }
                "failed" | "error" | "canceled" | "cancelled" => {
                    let err = poll_json["data"]["error"]
                        .as_str()
                        .unwrap_or("unknown error");
                    anyhow::bail!("Tripo3D task {} failed: {}", task_id, err);
                }
                _ => {
                    poll_interval = (poll_interval * 2).min(Duration::from_secs(20));
                }
            }
        }
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();
        let resp = self
            .http
            .get(format!("{}/user/balance", self.base_url))
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
