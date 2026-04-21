use std::path::Path;

use reqwest::multipart::{Form, Part};
use serde_json::{json, Value};

use crate::{
    error::{OriginError, Result},
    transport::HttpTransport,
    CognifyOptions, SearchOptions, SearchType, DEFAULT_BASE_URL,
};

/// Builder for configuring a [`GameDbClient`].
#[derive(Debug, Clone)]
pub struct GameDbClientBuilder {
    token: String,
    base_url: String,
    client: Option<reqwest::Client>,
}

impl GameDbClientBuilder {
    pub fn new(token: impl Into<String>) -> Self {
        Self {
            token: token.into(),
            base_url: DEFAULT_BASE_URL.to_string(),
            client: None,
        }
    }

    /// Override the GameDB API base URL.
    pub fn base_url(mut self, url: impl Into<String>) -> Self {
        self.base_url = url.into();
        self
    }

    /// Use a custom `reqwest::Client` (e.g. custom timeout, proxy, TLS settings).
    pub fn http_client(mut self, client: reqwest::Client) -> Self {
        self.client = Some(client);
        self
    }

    /// Build the [`GameDbClient`].
    pub fn build(self) -> GameDbClient {
        let transport = HttpTransport::new(self.base_url, self.token);
        let transport = if let Some(client) = self.client {
            transport.with_client(client)
        } else {
            transport
        };

        GameDbClient::from_transport(transport)
    }
}

/// Client for the Origin GameDB knowledge graph service.
#[derive(Debug, Clone)]
pub struct GameDbClient {
    pub(crate) transport: HttpTransport,
}

impl GameDbClient {
    /// Create a client with the default GameDB API URL.
    pub fn new(token: impl Into<String>) -> Self {
        Self::builder(token).build()
    }

    /// Create a builder for fine-grained configuration.
    pub fn builder(token: impl Into<String>) -> GameDbClientBuilder {
        GameDbClientBuilder::new(token)
    }

    pub fn from_transport(transport: HttpTransport) -> Self {
        Self { transport }
    }

    pub async fn health(&self) -> Result<bool> {
        self.transport.get("/health").await
    }

    pub async fn health_detailed(&self) -> Result<Value> {
        self.transport.get("/health/detailed").await
    }

    pub async fn datasets(&self) -> Result<Value> {
        self.transport.get("/api/v1/datasets").await
    }

    pub async fn create_dataset(&self, name: &str) -> Result<Value> {
        self.transport
            .post("/api/v1/datasets", &json!({ "name": name }))
            .await
    }

    pub async fn delete_dataset(&self, id: &str) -> Result<Value> {
        self.transport
            .delete(&format!("/api/v1/datasets/{id}"))
            .await
    }

    pub async fn delete_all_datasets(&self) -> Result<Value> {
        self.transport.delete("/api/v1/datasets").await
    }

    pub async fn dataset_status(&self) -> Result<Value> {
        self.transport.get("/api/v1/datasets/status").await
    }

    pub async fn dataset_graph(&self, id: &str) -> Result<Value> {
        self.transport
            .get(&format!("/api/v1/datasets/{id}/graph"))
            .await
    }

    pub async fn dataset_data(&self, id: &str) -> Result<Value> {
        self.transport
            .get(&format!("/api/v1/datasets/{id}/data"))
            .await
    }

    pub async fn delete_data(&self, dataset_id: &str, data_id: &str) -> Result<Value> {
        self.transport
            .delete(&format!("/api/v1/datasets/{dataset_id}/data/{data_id}"))
            .await
    }

    pub async fn add_text(&self, dataset_name: &str, content: &str) -> Result<Value> {
        let form = Form::new()
            .text("datasetName", dataset_name.to_owned())
            .part(
                "data",
                Part::text(content.to_owned())
                    .file_name("inline.txt")
                    .mime_str("text/plain")?,
            );

        self.transport.post_multipart("/api/v1/add", form).await
    }

    pub async fn add_file(&self, dataset_name: &str, file_path: &str) -> Result<Value> {
        let bytes = tokio::fs::read(file_path).await?;
        let file_name = Path::new(file_path)
            .file_name()
            .and_then(|name| name.to_str())
            .ok_or_else(|| OriginError::config(format!("invalid file path: {file_path}")))?
            .to_owned();

        let form = Form::new()
            .text("datasetName", dataset_name.to_owned())
            .part("data", Part::bytes(bytes).file_name(file_name));

        self.transport.post_multipart("/api/v1/add", form).await
    }

    pub async fn cognify(&self, options: &CognifyOptions) -> Result<Value> {
        self.transport
            .post("/api/v1/cognify", &json!(options))
            .await
    }

    pub async fn search(&self, query: &str, options: Option<SearchOptions>) -> Result<Value> {
        let options = options.unwrap_or_default();
        let mut body = json!({
            "query": query,
            "search_type": options.search_type.unwrap_or(SearchType::GraphCompletion),
            "top_k": options.top_k.unwrap_or(5),
        });

        if let Some(datasets) = options.datasets.filter(|datasets| !datasets.is_empty()) {
            body["datasets"] = json!(datasets);
        }

        self.transport.post("/api/v1/search", &body).await
    }

    pub async fn search_history(&self) -> Result<Value> {
        self.transport.get("/api/v1/search").await
    }

    pub async fn settings(&self) -> Result<Value> {
        self.transport.get("/api/v1/settings").await
    }

    pub async fn save_settings(&self, settings: &Value) -> Result<Value> {
        self.transport.post("/api/v1/settings", settings).await
    }

    pub async fn ontologies(&self) -> Result<Value> {
        self.transport.get("/api/v1/ontologies").await
    }
}
