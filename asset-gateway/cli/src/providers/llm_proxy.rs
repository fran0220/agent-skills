use std::time::Instant;

use crate::core::*;
use serde_json::{json, Value};

const ANTHROPIC_VERSION: &str = "2023-06-01";

fn estimate_llm_cost(model: &str) -> f64 {
    if model.starts_with("claude-opus") {
        0.10
    } else if model.starts_with("claude-sonnet") || model.starts_with("claude-haiku") {
        0.02
    } else if model.starts_with("gpt-5.4") {
        0.03
    } else {
        0.02
    }
}

#[derive(Debug, Clone, Copy)]
enum Protocol {
    OpenAi,
    Anthropic,
}

impl Protocol {
    fn for_model(model: &str) -> Self {
        if model.trim_start().starts_with("claude-") {
            Self::Anthropic
        } else {
            Self::OpenAi
        }
    }

    fn as_str(&self) -> &'static str {
        match self {
            Self::OpenAi => "openai",
            Self::Anthropic => "anthropic",
        }
    }
}

/// Unified LLM proxy provider.
/// Routes Claude / GPT / Gemini / Grok / GLM through a single proxy gateway.
pub struct LlmProxyProvider {
    pub id: String,
    pub base_url: String,
    pub api_key: String,
    pub default_model: String,
    http: reqwest::Client,
}

impl LlmProxyProvider {
    pub fn new(base_url: String, api_key: String, default_model: String) -> Self {
        Self {
            id: "llm_proxy".into(),
            base_url,
            api_key,
            default_model,
            http: reqwest::Client::new(),
        }
    }

    async fn generate_openai(
        &self,
        req: &GenerateRequest,
        model: &str,
        prompt: &str,
        stream: bool,
    ) -> anyhow::Result<(String, Value)> {
        let mut body = json!({
            "model": model,
            "messages": [{"role": "user", "content": prompt}],
            "max_tokens": req
                .params
                .get("max_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(4096),
        });

        if let Some(temperature) = req.params.get("temperature").and_then(|v| v.as_f64()) {
            body["temperature"] = json!(temperature);
        }

        if let Some(top_p) = req.params.get("top_p").and_then(|v| v.as_f64()) {
            body["top_p"] = json!(top_p);
        }

        if stream {
            body["stream"] = json!(true);
        }

        let mut resp = self
            .http
            .post(format!("{}/v1/chat/completions", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await?;
            anyhow::bail!("LLM proxy (OpenAI) returned {}: {}", status, text);
        }

        if stream {
            let content = Self::read_sse(&mut resp, Self::extract_openai_delta).await?;
            return Ok((
                content,
                json!({"model": model, "protocol": "openai", "stream": true}),
            ));
        }

        let payload: Value = resp.json().await?;
        let content = payload["choices"][0]["message"]["content"]
            .as_str()
            .unwrap_or_default()
            .to_string();

        Ok((
            content,
            json!({"model": model, "protocol": "openai", "stream": false}),
        ))
    }

    async fn generate_anthropic(
        &self,
        req: &GenerateRequest,
        model: &str,
        prompt: &str,
        stream: bool,
    ) -> anyhow::Result<(String, Value)> {
        let mut body = json!({
            "model": model,
            "max_tokens": req
                .params
                .get("max_tokens")
                .and_then(|v| v.as_u64())
                .unwrap_or(4096),
            "messages": [{"role": "user", "content": prompt}],
        });

        if let Some(temperature) = req.params.get("temperature").and_then(|v| v.as_f64()) {
            body["temperature"] = json!(temperature);
        }

        if stream {
            body["stream"] = json!(true);
        }

        let mut resp = self
            .http
            .post(format!("{}/v1/messages", self.base_url))
            .header("x-api-key", &self.api_key)
            .header("anthropic-version", ANTHROPIC_VERSION)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .json(&body)
            .send()
            .await?;

        if !resp.status().is_success() {
            let status = resp.status();
            let text = resp.text().await?;
            anyhow::bail!("LLM proxy (Anthropic) returned {}: {}", status, text);
        }

        if stream {
            let content = Self::read_sse(&mut resp, Self::extract_anthropic_delta).await?;
            return Ok((
                content,
                json!({"model": model, "protocol": "anthropic", "stream": true}),
            ));
        }

        let payload: Value = resp.json().await?;
        let content = payload["content"]
            .as_array()
            .map(|parts| {
                parts
                    .iter()
                    .filter_map(|part| part.get("text").and_then(Value::as_str))
                    .collect::<String>()
            })
            .unwrap_or_default();

        Ok((
            content,
            json!({"model": model, "protocol": "anthropic", "stream": false}),
        ))
    }

    async fn read_sse<F>(
        response: &mut reqwest::Response,
        mut extract_delta: F,
    ) -> anyhow::Result<String>
    where
        F: FnMut(&Value) -> Option<String>,
    {
        let mut output = String::new();
        let mut buffer = String::new();

        while let Some(chunk) = response.chunk().await? {
            buffer.push_str(&String::from_utf8_lossy(&chunk));

            while let Some(newline_idx) = buffer.find('\n') {
                let mut line: String = buffer.drain(..=newline_idx).collect();
                if line.ends_with('\n') {
                    line.pop();
                }
                if line.ends_with('\r') {
                    line.pop();
                }

                if !line.starts_with("data:") {
                    continue;
                }

                let payload = line[5..].trim();
                if payload.is_empty() {
                    continue;
                }

                if payload == "[DONE]" {
                    return Ok(output);
                }

                let json: Value = match serde_json::from_str(payload) {
                    Ok(v) => v,
                    Err(err) => {
                        tracing::error!(error = %err, payload = payload, "invalid SSE payload");
                        continue;
                    }
                };

                if let Some(delta) = extract_delta(&json) {
                    if !delta.is_empty() {
                        tracing::info!(provider = "llm_proxy", token = %delta, "stream token");
                        output.push_str(&delta);
                    }
                }
            }
        }

        if let Some(last) = buffer.lines().last() {
            let payload = last.trim_start_matches("data:").trim();
            if !payload.is_empty() && payload != "[DONE]" {
                if let Ok(json) = serde_json::from_str::<Value>(payload) {
                    if let Some(delta) = extract_delta(&json) {
                        output.push_str(&delta);
                    }
                }
            }
        }

        Ok(output)
    }

    fn extract_openai_delta(payload: &Value) -> Option<String> {
        if let Some(text) = payload["choices"][0]["delta"]["content"].as_str() {
            return Some(text.to_string());
        }

        payload["choices"][0]["delta"]["content"]
            .as_array()
            .map(|parts| {
                parts
                    .iter()
                    .filter_map(|part| part.get("text").and_then(Value::as_str))
                    .collect::<String>()
            })
            .filter(|joined| !joined.is_empty())
    }

    fn extract_anthropic_delta(payload: &Value) -> Option<String> {
        if payload["type"].as_str() != Some("content_block_delta") {
            return None;
        }

        payload["delta"]["text"].as_str().map(|s| s.to_string())
    }
}

#[async_trait::async_trait]
impl AssetProvider for LlmProxyProvider {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn id(&self) -> &str {
        &self.id
    }

    fn display_name(&self) -> &str {
        "LLM Proxy (Claude/GPT/Gemini/Grok/GLM)"
    }

    fn asset_types(&self) -> &[AssetType] {
        &[AssetType::Text]
    }

    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            supports_streaming: true,
            priority: 200,
            ..Default::default()
        }
    }

    async fn generate(&self, req: &GenerateRequest) -> anyhow::Result<GenerateResponse> {
        let model = req.model.as_deref().unwrap_or(&self.default_model);
        let prompt = req.prompt.as_deref().unwrap_or("");
        let stream = req.wants_streaming();
        let protocol = Protocol::for_model(model);
        let start = Instant::now();

        let (content, metadata) = match protocol {
            Protocol::OpenAi => self.generate_openai(req, model, prompt, stream).await?,
            Protocol::Anthropic => self.generate_anthropic(req, model, prompt, stream).await?,
        };

        Ok(GenerateResponse {
            provider_id: self.id.clone(),
            output_path: None,
            output_url: None,
            output_data: Some(content),
            metadata: json!({
                "protocol": protocol.as_str(),
                "stream": stream,
                "provider_metadata": metadata,
            }),
            cost_usd: Some(estimate_llm_cost(model)),
            elapsed_ms: start.elapsed().as_millis() as u64,
        })
    }

    async fn health_check(&self) -> anyhow::Result<HealthStatus> {
        let start = Instant::now();
        let resp = self
            .http
            .get(format!("{}/v1/models", self.base_url))
            .header("Authorization", format!("Bearer {}", self.api_key))
            .send()
            .await;

        match resp {
            Ok(r) => Ok(HealthStatus {
                healthy: r.status().is_success(),
                latency_ms: Some(start.elapsed().as_millis() as u64),
                message: Some(format!("status: {}", r.status())),
            }),
            Err(e) => Ok(HealthStatus {
                healthy: false,
                latency_ms: Some(start.elapsed().as_millis() as u64),
                message: Some(e.to_string()),
            }),
        }
    }
}
