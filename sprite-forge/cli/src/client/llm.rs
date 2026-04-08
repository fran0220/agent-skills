use std::time::Instant;

use anyhow::{Result, anyhow};
use serde_json::{Value, json};
use tracing::{error, info};

pub struct LlmClient {
    http: reqwest::Client,
    base_url: String,
    api_key: String,
    model: String,
}

impl LlmClient {
    pub fn new(base_url: &str, api_key: &str, model: &str) -> Self {
        Self {
            http: reqwest::Client::new(),
            base_url: base_url.trim_end_matches('/').to_string(),
            api_key: api_key.to_string(),
            model: model.to_string(),
        }
    }

    pub async fn chat(
        &self,
        system_prompt: &str,
        user_prompt: &str,
        max_tokens: u32,
    ) -> Result<String> {
        let body = json!({
            "model": self.model,
            "messages": [
                { "role": "system", "content": system_prompt },
                { "role": "user", "content": user_prompt }
            ],
            "max_tokens": max_tokens
        });

        let url = format!("{}/v1/chat/completions", self.base_url);
        let started_at = Instant::now();

        let response = self
            .http
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
            .map_err(|err| anyhow!("LLM proxy request failed: {err}"))?;

        let status = response.status();
        let payload = response
            .text()
            .await
            .map_err(|err| anyhow!("Failed to read LLM proxy response body: {err}"))?;
        let elapsed = started_at.elapsed();

        if !status.is_success() {
            error!(
                status = %status,
                elapsed_ms = elapsed.as_millis(),
                body = %payload,
                "LLM proxy chat request failed"
            );
            return Err(anyhow!("LLM proxy returned {}: {}", status, payload));
        }

        let value: Value = serde_json::from_str(&payload).map_err(|err| {
            anyhow!("Failed to parse LLM proxy response JSON: {err}; body: {payload}")
        })?;

        if let Some(usage) = value.get("usage") {
            let prompt_tokens = usage
                .get("prompt_tokens")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            let completion_tokens = usage
                .get("completion_tokens")
                .and_then(Value::as_u64)
                .unwrap_or_default();
            let total_tokens = usage
                .get("total_tokens")
                .and_then(Value::as_u64)
                .unwrap_or(prompt_tokens + completion_tokens);

            info!(
                model = %self.model,
                elapsed_ms = elapsed.as_millis(),
                prompt_tokens,
                completion_tokens,
                total_tokens,
                "LLM proxy chat completed"
            );
        } else {
            info!(
                model = %self.model,
                elapsed_ms = elapsed.as_millis(),
                "LLM proxy chat completed"
            );
        }

        value["choices"][0]["message"]["content"]
            .as_str()
            .map(str::to_string)
            .ok_or_else(|| anyhow!("LLM proxy response missing choices[0].message.content"))
    }
}
