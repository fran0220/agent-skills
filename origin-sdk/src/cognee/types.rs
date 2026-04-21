use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SearchType {
    #[serde(rename = "GRAPH_COMPLETION")]
    GraphCompletion,
    #[serde(rename = "SUMMARIES")]
    Summaries,
    #[serde(rename = "INSIGHTS")]
    Insights,
    #[serde(rename = "CHUNKS")]
    Chunks,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CognifyOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datasets: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub custom_prompt: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_in_background: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub chunks_per_batch: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SearchOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub search_type: Option<SearchType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub top_k: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub datasets: Option<Vec<String>>,
}
