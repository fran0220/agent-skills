use anyhow::{Context, Result};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;

use crate::providers::{SearchProvider, SearchResult};

pub struct TavilySearchProvider {
    http: Client,
    api_key: String,
}

#[derive(Serialize)]
struct TavilyRequest<'a> {
    api_key: &'a str,
    query: &'a str,
    max_results: u32,
    include_answer: bool,
}

#[derive(Deserialize)]
struct TavilyResponse {
    #[serde(default)]
    results: Vec<TavilyItem>,
    answer: Option<String>,
}

#[derive(Deserialize)]
struct TavilyItem {
    #[serde(default)]
    title: String,
    #[serde(default)]
    url: String,
    content: Option<String>,
    #[serde(rename = "published_date")]
    published_date: Option<String>,
}

impl TavilySearchProvider {
    pub fn new(api_key: String, timeout_secs: u64) -> Result<Self> {
        let http = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .context("failed to build Tavily HTTP client")?;
        Ok(Self { http, api_key })
    }

    pub async fn search_with_answer(
        &self,
        query: &str,
        num: u32,
    ) -> Result<(Vec<SearchResult>, Option<String>)> {
        let response = self.search_inner(query, num, true).await?;
        Ok((response.results, response.answer))
    }

    async fn search_inner(
        &self,
        query: &str,
        num: u32,
        include_answer: bool,
    ) -> Result<TavilySearchOutput> {
        let response = self
            .http
            .post("https://api.tavily.com/search")
            .header("Content-Type", "application/json")
            .json(&TavilyRequest {
                api_key: &self.api_key,
                query,
                max_results: num,
                include_answer,
            })
            .send()
            .await
            .context("failed to send Tavily search request")?;

        let status = response.status();
        if !status.is_success() {
            let body = response.text().await.unwrap_or_default();
            anyhow::bail!("Tavily returned HTTP {}: {}", status.as_u16(), body);
        }

        let data: TavilyResponse = response
            .json()
            .await
            .context("failed to parse Tavily response")?;

        Ok(TavilySearchOutput {
            results: data
                .results
                .into_iter()
                .filter(|item| !item.url.trim().is_empty())
                .map(|item| SearchResult {
                    title: item.title,
                    url: item.url,
                    snippet: item.content.unwrap_or_default(),
                    published_date: item.published_date.filter(|value| !value.trim().is_empty()),
                    source: self.id().to_string(),
                    score: 0.0,
                })
                .collect(),
            answer: data.answer.filter(|value| !value.trim().is_empty()),
        })
    }
}

struct TavilySearchOutput {
    results: Vec<SearchResult>,
    answer: Option<String>,
}

#[async_trait]
impl SearchProvider for TavilySearchProvider {
    fn id(&self) -> &str {
        "tavily"
    }

    fn display_name(&self) -> &str {
        "Tavily"
    }

    async fn search(&self, query: &str, num: u32) -> Result<Vec<SearchResult>> {
        self.search_inner(query, num, false)
            .await
            .map(|output| output.results)
    }

    async fn health_check(&self) -> Result<bool> {
        self.search("health check", 1).await.map(|_| true)
    }
}
