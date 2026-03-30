use anyhow::{Context, Result};
use once_cell::sync::Lazy;
use regex::Regex;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::config::AppConfig;

static THINKING_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"(?s)<think(?:ing)?>.*?</think(?:ing)?>").unwrap());

#[derive(Clone)]
pub struct AIClient {
    http: Client,
    config: AppConfig,
}

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
    stream: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Deserialize)]
pub struct ChatResponse {
    pub choices: Vec<Choice>,
    pub usage: Option<Usage>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub message: Message,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Usage {
    pub prompt_tokens: u32,
    pub completion_tokens: u32,
    pub total_tokens: u32,
}

pub fn strip_thinking(text: &str) -> String {
    THINKING_RE.replace_all(text, "").trim().to_string()
}

pub fn chat_completions_url(api_url: &str) -> String {
    let base = api_url.trim_end_matches('/');
    if base.ends_with("/v1") {
        format!("{base}/chat/completions")
    } else {
        format!("{base}/v1/chat/completions")
    }
}

impl AIClient {
    pub fn new(config: AppConfig) -> Self {
        let http = Client::builder()
            .timeout(Duration::from_secs(config.timeout_secs))
            .build()
            .expect("failed to build HTTP client");
        Self { http, config }
    }

    pub async fn chat_raw(
        &self,
        messages: Vec<Message>,
        model: &str,
    ) -> Result<(String, Option<Usage>)> {
        let request = ChatRequest {
            model: model.to_string(),
            messages,
            stream: false,
        };

        self.send_chat(request).await
    }

    async fn send_chat(&self, request: ChatRequest) -> Result<(String, Option<Usage>)> {
        let resp = self
            .http
            .post(chat_completions_url(&self.config.api_url))
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Bearer {}", self.config.api_key))
            .json(&request)
            .send()
            .await
            .context("failed to send request to AI API")?;

        let status = resp.status();
        if !status.is_success() {
            let body = resp.text().await.unwrap_or_default();
            anyhow::bail!("AI API returned HTTP {}: {}", status.as_u16(), body);
        }

        let data: ChatResponse = resp
            .json()
            .await
            .context("failed to parse AI API response")?;

        let content = data
            .choices
            .first()
            .map(|choice| strip_thinking(&choice.message.content))
            .unwrap_or_default();

        Ok((content, data.usage))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_strip_thinking() {
        assert_eq!(strip_thinking("<think>internal</think>Hello"), "Hello");
        assert_eq!(
            strip_thinking("<thinking>step 1\nstep 2</thinking>\nResult"),
            "Result"
        );
        assert_eq!(strip_thinking("No thinking here"), "No thinking here");
        assert_eq!(
            strip_thinking("<think>a</think>mid<thinking>b</thinking>end"),
            "midend"
        );
    }

    #[test]
    fn chat_url_handles_base_and_v1_prefix() {
        assert_eq!(
            chat_completions_url("https://api.xiaomao.chat"),
            "https://api.xiaomao.chat/v1/chat/completions"
        );
        assert_eq!(
            chat_completions_url("https://api.xiaomao.chat/v1"),
            "https://api.xiaomao.chat/v1/chat/completions"
        );
    }
}
