use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::providers::{SearchProvider, SearchResult};

pub struct ExaSearchProvider {
    http: Client,
    api_key: String,
}

#[derive(Serialize)]
struct ExaRequest<'a> {
    query: &'a str,
    #[serde(rename = "numResults")]
    num_results: u32,
    #[serde(rename = "type")]
    search_type: &'static str,
}

#[derive(Deserialize)]
struct ExaResponse {
    #[serde(default)]
    results: Vec<ExaItem>,
}

#[derive(Deserialize)]
struct ExaItem {
    #[serde(default)]
    title: String,
    #[serde(default)]
    url: String,
    text: Option<String>,
    snippet: Option<String>,
    #[serde(rename = "publishedDate")]
    published_date: Option<String>,
}

impl ExaSearchProvider {
    pub fn new(api_key: String, timeout_secs: u64) -> Result<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .context("failed to build Exa HTTP client")?;
        Ok(Self { http, api_key })
    }
}

#[async_trait]
impl SearchProvider for ExaSearchProvider {
    fn id(&self) -> &str {
        "exa"
    }

    fn display_name(&self) -> &str {
        "Exa"
    }

    async fn search(&self, query: &str, num: u32) -> Result<Vec<SearchResult>> {
        let response = self
            .http
            .post("https://api.exa.ai/search")
            .header("x-api-key", &self.api_key)
            .header("Content-Type", "application/json")
            .json(&ExaRequest {
                query,
                num_results: num,
                search_type: "auto",
            })
            .send()
            .await
            .context("failed to send Exa search request")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Exa returned HTTP {}: {}", status.as_u16(), body);
        }

        let data: ExaResponse = response
            .json()
            .await
            .context("failed to parse Exa response")?;
        Ok(data
            .results
            .into_iter()
            .filter(|item| !item.url.trim().is_empty())
            .map(|item| SearchResult {
                title: item.title,
                url: item.url,
                snippet: item.text.or(item.snippet).unwrap_or_default(),
                published_date: item.published_date.filter(|value| !value.trim().is_empty()),
                source: self.id().to_string(),
                score: 0.0,
            })
            .collect())
    }

    async fn health_check(&self) -> Result<bool> {
        self.search("health check", 1).await.map(|_| true)
    }
}
