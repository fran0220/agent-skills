pub mod exa;
pub mod grok;
pub mod tavily;

use anyhow::Result;
use async_trait::async_trait;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub published_date: Option<String>,
    pub source: String,
    pub score: f64,
}

#[allow(dead_code)]
#[async_trait]
pub trait SearchProvider: Send + Sync {
    fn id(&self) -> &str;
    fn display_name(&self) -> &str;
    async fn search(&self, query: &str, num: u32) -> Result<Vec<SearchResult>>;
    async fn health_check(&self) -> Result<bool>;
}
