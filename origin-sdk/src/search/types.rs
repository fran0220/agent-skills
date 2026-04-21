use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SearchMode {
    Fast,
    Deep,
    Answer,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResult {
    pub title: Option<String>,
    pub url: Option<String>,
    pub snippet: Option<String>,
    pub source: Option<String>,
    pub score: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubQueryResult {
    pub sub_query: String,
    pub content: String,
    pub providers: Vec<String>,
    pub results: Vec<SearchResult>,
    pub tokens: u32,
    pub answer: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchResponse {
    pub query: String,
    pub mode: String,
    pub model: Option<String>,
    pub content: String,
    pub providers: Vec<String>,
    pub results: Vec<SearchResult>,
    pub tokens: u32,
    pub answer: Option<String>,
    pub intent: Option<String>,
    pub sub_results: Option<Vec<SubQueryResult>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SearchOptions {
    pub mode: Option<SearchMode>,
    pub model: Option<String>,
    pub split: Option<u32>,
    pub num: Option<u32>,
}
