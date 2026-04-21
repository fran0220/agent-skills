pub mod types;
pub use types::*;

use serde::Deserialize;
use serde_json::{json, Value};

use crate::error::Result;
use crate::transport::HttpTransport;

#[derive(Debug, Clone)]
pub struct SearchClient {
    pub(crate) transport: HttpTransport,
}

impl SearchClient {
    pub fn new(transport: HttpTransport) -> Self {
        Self { transport }
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

#[derive(Debug, Deserialize)]
struct ModelsResponse {
    models: Vec<String>,
}
