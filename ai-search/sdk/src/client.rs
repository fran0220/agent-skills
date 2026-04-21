use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::Result;
use crate::transport::HttpTransport;
use crate::types::{SearchMode, SearchOptions, SearchResponse};

const DEFAULT_BASE_URL: &str = "https://search.xiaomao.chat";

#[derive(Debug, Clone)]
pub struct SearchClient {
    transport: HttpTransport,
}

impl SearchClient {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self::builder(api_key).build()
    }

    pub fn builder(api_key: impl Into<String>) -> SearchClientBuilder {
        SearchClientBuilder {
            api_key: api_key.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
            http_client: None,
        }
    }

    pub async fn search(
        &self,
        query: &str,
        options: Option<SearchOptions>,
    ) -> Result<SearchResponse> {
        let options = options.unwrap_or(SearchOptions {
            mode: None,
            model: None,
            split: None,
            num: None,
        });

        let mut body = json!({ "query": query });
        let payload = body
            .as_object_mut()
            .expect("search request payload should be an object");

        if let Some(mode) = options.mode {
            payload.insert("mode".to_string(), serde_json::to_value(mode)?);
        }
        if let Some(model) = options.model {
            payload.insert("model".to_string(), Value::String(model));
        }
        if let Some(split) = options.split {
            payload.insert("split".to_string(), json!(split));
        }
        if let Some(num) = options.num {
            payload.insert("num".to_string(), json!(num));
        }

        self.transport.post("/api/search", &body).await
    }

    pub async fn search_fast(&self, query: &str) -> Result<SearchResponse> {
        self.search(
            query,
            Some(SearchOptions {
                mode: Some(SearchMode::Fast),
                model: None,
                split: None,
                num: None,
            }),
        )
        .await
    }

    pub async fn search_deep(&self, query: &str) -> Result<SearchResponse> {
        self.search(
            query,
            Some(SearchOptions {
                mode: Some(SearchMode::Deep),
                model: None,
                split: None,
                num: None,
            }),
        )
        .await
    }

    pub async fn search_answer(&self, query: &str) -> Result<SearchResponse> {
        self.search(
            query,
            Some(SearchOptions {
                mode: Some(SearchMode::Answer),
                model: None,
                split: None,
                num: None,
            }),
        )
        .await
    }

    pub async fn models(&self) -> Result<Vec<String>> {
        let response: ModelsResponse = self.transport.get("/api/models").await?;
        Ok(response.models)
    }

    pub async fn providers(&self) -> Result<Value> {
        self.transport.get("/api/providers").await
    }

    pub async fn health(&self) -> Result<bool> {
        let payload: Value = self.transport.get("/health").await?;

        let status = payload.get("status").and_then(Value::as_str).or_else(|| {
            payload
                .get("data")
                .and_then(|data| data.get("status"))
                .and_then(Value::as_str)
        });

        Ok(matches!(status, Some("ok")))
    }
}

#[derive(Debug, Clone)]
pub struct SearchClientBuilder {
    api_key: String,
    base_url: String,
    http_client: Option<reqwest::Client>,
}

impl SearchClientBuilder {
    pub fn base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.http_client = Some(client);
        self
    }

    pub fn build(self) -> SearchClient {
        let mut transport = HttpTransport::new(self.base_url, self.api_key);

        if let Some(client) = self.http_client {
            transport = transport.with_client(client);
        }

        SearchClient { transport }
    }
}

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    models: Vec<String>,
}
