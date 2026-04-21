use reqwest::{Client, Method, Response};
use serde::de::DeserializeOwned;
use serde_json::Value;

use crate::error::{OriginError, Result};

/// Shared HTTP transport handling auth, envelope parsing, and error mapping.
#[derive(Debug, Clone)]
pub struct HttpTransport {
    client: Client,
    base_url: String,
    api_key: String,
}

impl HttpTransport {
    pub fn new(base_url: impl Into<String>, api_key: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into().trim_end_matches('/').to_string(),
            api_key: api_key.into(),
        }
    }

    /// Use a custom `reqwest::Client` (e.g. with proxy or custom timeout).
    pub fn with_client(mut self, client: Client) -> Self {
        self.client = client;
        self
    }

    pub fn client(&self) -> &Client {
        &self.client
    }

    pub fn base_url(&self) -> &str {
        &self.base_url
    }

    /// Send a JSON request and parse the envelope response.
    ///
    /// All three Origin services return `{"ok": true, "data": ...}` on success
    /// and `{"ok": false, "error": ...}` on failure.
    pub async fn request<T: DeserializeOwned>(
        &self,
        method: Method,
        path: &str,
        body: Option<&Value>,
    ) -> Result<T> {
        let url = format!("{}{}", self.base_url, path);

        let mut builder = self.client.request(method, &url).bearer_auth(&self.api_key);

        if let Some(body) = body {
            builder = builder.json(body);
        }

        let response = builder.send().await?;
        self.parse_response(response).await
    }

    /// GET convenience.
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request(Method::GET, path, None).await
    }

    /// POST convenience.
    pub async fn post<T: DeserializeOwned>(&self, path: &str, body: &Value) -> Result<T> {
        self.request(Method::POST, path, Some(body)).await
    }

    /// DELETE convenience.
    pub async fn delete<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        self.request(Method::DELETE, path, None).await
    }

    /// Parse the HTTP response, handling the `{"ok", "data"|"error"}` envelope.
    async fn parse_response<T: DeserializeOwned>(&self, response: Response) -> Result<T> {
        let status = response.status().as_u16();
        let text = response.text().await?;

        if text.is_empty() {
            return Err(OriginError::api(status, "empty response body"));
        }

        let payload: Value = serde_json::from_str(&text).map_err(|_| {
            OriginError::api(status, format!("invalid JSON: {}", truncate(&text, 256)))
        })?;

        // Check for envelope format: {"ok": bool, "data": ..., "error": ...}
        if let Some(ok) = payload.get("ok").and_then(|v| v.as_bool()) {
            if ok {
                // Extract "data" field and deserialize into T
                let data = payload.get("data").cloned().unwrap_or(Value::Null);
                let result = serde_json::from_value(data)?;
                return Ok(result);
            } else {
                // Error envelope
                let command = payload
                    .get("command")
                    .and_then(|v| v.as_str())
                    .map(String::from);
                let error = payload.get("error").cloned().unwrap_or(Value::Null);
                let (code, message) = parse_error_payload(&error);
                return Err(OriginError::api_full(status, code, message, command));
            }
        }

        // Non-envelope response (e.g. Cognee native API)
        if status >= 400 {
            return Err(OriginError::api(status, truncate(&text, 512).to_string()));
        }

        let result = serde_json::from_value(payload)?;
        Ok(result)
    }
}

fn parse_error_payload(error: &Value) -> (Option<String>, String) {
    match error {
        Value::String(s) => (None, s.clone()),
        Value::Object(map) => {
            let code = map.get("code").and_then(|v| v.as_str()).map(String::from);
            let message = map
                .get("message")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown error")
                .to_string();
            (code, message)
        }
        _ => (None, error.to_string()),
    }
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max {
        s
    } else {
        &s[..max]
    }
}
