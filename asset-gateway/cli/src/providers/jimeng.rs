use std::time::{Duration, Instant};

use crate::core::*;
use reqwest::StatusCode;
use serde_json::{json, Value};

/// Jimeng (即梦) provider — image + video generation.
/// Supports Jimeng image and Seedance video via gateway APIs.
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
            http: reqwest::Client::new(),
        }
    }

    fn ensure_params_object(params: &mut Value) -> &mut serde_json::Map<String, Value> {
        if !params.is_object() {
            *params = json!({});
        }

        params
            .as_object_mut()
            .expect("params should be an object after normalization")
    }

    fn extract_task_id(payload: &Value) -> Option<String> {
        payload["data"]["task_id"]
            .as_str()
            .or_else(|| payload["task_id"].as_str())
            .or_else(|| payload["data"]["id"].as_str())
            .map(str::to_string)
    }

    fn extract_status(payload: &Value) -> Option<String> {
        payload["data"]["status"]
            .as_str()
            .or_else(|| payload["status"].as_str())
            .or_else(|| payload["data"]["task"]["status"].as_str())
            .map(|s| s.to_ascii_lowercase())
    }

    fn extract_output(payload: &Value) -> Option<(Option<String>, Option<String>)> {
        let output_url = payload["data"]["url"]
            .as_str()
            .or_else(|| payload["data"]["output_url"].as_str())
            .or_else(|| payload["data"]["video_url"].as_str())
            .or_else(|| payload["output"]["url"].as_str())
            .or_else(|| payload["result"]["url"].as_str())
            .map(str::to_string);

        let output_data = payload["data"]["base64"]
            .as_str()
            .or_else(|| payload["data"]["b64_json"].as_str())
            .or_else(|| payload["output"]["base64"].as_str())
            .or_else(|| payload["result"]["base64"].as_str())
            .map(str::to_string);

        if output_url.is_none() && output_data.is_none() {
            None
        } else {
            Some((output_url, output_data))
        }
    }

    async fn fetch_task_status(
        &self,
        asset_type: AssetType,
        task_id: &str,
    ) -> anyhow::Result<Value> {
        let kind = if asset_type == AssetType::Video {
            "video"
        } else {
            "image"
        };

        let paths = [
            format!("/v1/{}/tasks/{}", kind, task_id),
            format!("/v1/{}/status/{}", kind, task_id),
            format!("/v1/tasks/{}", task_id),
            format!("/v1/task/{}", task_id),
        ];

        let mut last_err: Option<anyhow::Error> = None;

        for path in &paths {
            let resp = self
                .http
                .get(format!("{}{}", self.base_url, path))
                .header("Authorization", format!("Bearer {}", self.api_key))
                .send()
                .await?;

            let status = resp.status();
            let text = resp.text().await?;

            if status == StatusCode::NOT_FOUND {
                continue;
            }

            if !status.is_success() {
                last_err = Some(anyhow::anyhow!(
                    "Jimeng poll endpoint {} returned {}: {}",
                    path,
                    status,
                    text
                ));
                continue;
            }

            match serde_json::from_str::<Value>(&text) {
                Ok(payload) => return Ok(payload),
                Err(err) => {
                    last_err = Some(anyhow::anyhow!(
                        "Jimeng poll endpoint {} returned invalid JSON: {}",
                        path,
                        err
                    ));
                }
            }
        }

        Err(last_err.unwrap_or_else(|| anyhow::anyhow!("no Jimeng status endpoint succeeded")))
    }

    async fn poll_task(
        &self,
        asset_type: AssetType,
        task_id: &str,
        params: &Value,
    ) -> anyhow::Result<Value> {
        let timeout_secs = params
            .get("timeout_seconds")
            .and_then(Value::as_u64)
            .unwrap_or(300);

        let started = Instant::now();
        let mut backoff_secs = 2_u64;

        loop {
            if started.elapsed().as_secs() >= timeout_secs {
                anyhow::bail!("Jimeng task {} timed out after {}s", task_id, timeout_secs);
            }

            tokio::time::sleep(Duration::from_secs(backoff_secs)).await;
            let payload = self.fetch_task_status(asset_type, task_id).await?;

            if let Some((url, data)) = Self::extract_output(&payload) {
                if url.is_some() || data.is_some() {
                    return Ok(payload);
                }
            }

            match Self::extract_status(&payload).as_deref() {
                Some("success") | Some("succeeded") | Some("completed") => {
                    return Ok(payload);
                }
                Some("failed") | Some("error") | Some("canceled") | Some("cancelled") => {
                    anyhow::bail!("Jimeng task {} failed: {}", task_id, payload);
                }
                _ => {
                    backoff_secs = (backoff_secs * 2).min(20);
                }
            }
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
            "/v1/video/generate"
        } else {
            "/v1/image/generate"
        };

        let mut params = req.params.clone();
        let params_obj = Self::ensure_params_object(&mut params);

        if let Some(model) = req.model.as_ref() {
            params_obj
                .entry("model".to_string())
                .or_insert_with(|| json!(model));
        }

        if let Some(style) = req.style() {
            params_obj
                .entry("style".to_string())
                .or_insert_with(|| json!(style));
        }

        if let Some(quality) = req.quality() {
            params_obj
                .entry("quality".to_string())
                .or_insert_with(|| json!(quality.as_str()));
        }

        let body = json!({
            "prompt": prompt,
            "params": params,
        });

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

        let first_payload: Value = serde_json::from_str(&text)?;

        if let Some((output_url, output_data)) = Self::extract_output(&first_payload) {
            return Ok(GenerateResponse {
                provider_id: self.id.clone(),
                output_path: None,
                output_url,
                output_data,
                metadata: json!({
                    "asset_type": req.asset_type.as_str(),
                    "async": false,
                    "raw_response": first_payload,
                }),
                cost_usd: None,
                elapsed_ms: start.elapsed().as_millis() as u64,
            });
        }

        let task_id = Self::extract_task_id(&first_payload)
            .ok_or_else(|| anyhow::anyhow!("Jimeng response missing output and task_id"))?;

        let final_payload = self
            .poll_task(req.asset_type, &task_id, &req.params)
            .await?;
        let (output_url, output_data) = Self::extract_output(&final_payload)
            .ok_or_else(|| anyhow::anyhow!("Jimeng task {} completed without output", task_id))?;

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url,
            output_data,
            metadata: json!({
                "asset_type": req.asset_type.as_str(),
                "async": true,
                "task_id": task_id,
                "raw_response": final_payload,
            }),
            cost_usd: None,
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();
        let resp = self
            .http
            .get(format!("{}/health", self.base_url))
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
