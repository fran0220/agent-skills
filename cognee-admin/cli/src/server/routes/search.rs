use axum::extract::{Query, State};
use axum::response::Html;
use serde::Deserialize;
use serde_json::Value;
use std::sync::Arc;

use crate::error::AppError;
use super::AppState;
use super::base_html;

#[derive(Debug, Deserialize)]
pub struct SearchParams {
    pub q: Option<String>,
    pub search_type: Option<String>,
    pub top_k: Option<u32>,
}

pub async fn page(State(_state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let content = r##"
        <div class="mb-8">
            <h2 class="text-2xl font-bold">Search</h2>
            <p class="text-gray-400 mt-1">Query the knowledge engine</p>
        </div>

        <div class="card mb-6">
            <form hx-post="/api/search" hx-target="#search-results" hx-swap="innerHTML" hx-indicator="#search-loading">
                <div class="flex flex-col gap-4">
                    <div>
                        <label class="block text-sm text-gray-400 mb-1">Query</label>
                        <input type="text" name="q" placeholder="Enter your search query..."
                               class="w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-3 text-sm focus:border-cyan-400 focus:outline-none"
                               required>
                    </div>
                    <div class="flex gap-4">
                        <div class="flex-1">
                            <label class="block text-sm text-gray-400 mb-1">Search Type</label>
                            <select name="search_type"
                                    class="w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm focus:border-cyan-400 focus:outline-none">
                                <option value="insights">Insights</option>
                                <option value="chunks">Chunks</option>
                                <option value="summaries">Summaries</option>
                            </select>
                        </div>
                        <div class="w-32">
                            <label class="block text-sm text-gray-400 mb-1">Top K</label>
                            <input type="number" name="top_k" value="10" min="1" max="100"
                                   class="w-full bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm focus:border-cyan-400 focus:outline-none">
                        </div>
                    </div>
                    <div class="flex items-center gap-3">
                        <button type="submit"
                                class="bg-cyan-600 hover:bg-cyan-700 px-6 py-2 rounded-lg text-sm font-medium transition-colors">
                            Search
                        </button>
                        <span id="search-loading" class="htmx-indicator text-gray-500 text-sm">Searching...</span>
                    </div>
                </div>
            </form>
        </div>

        <div id="search-results"></div>
    "##;

    Ok(Html(base_html("Search", "/search", content)))
}

pub async fn api_search(
    State(state): State<Arc<AppState>>,
    Query(params): Query<SearchParams>,
) -> Result<Html<String>, AppError> {
    let query = params.q.as_deref().unwrap_or("");
    if query.is_empty() {
        return Ok(Html(r##"<div class="text-gray-500 text-sm">Enter a query to search.</div>"##.to_string()));
    }
    let data = state.client.search(query, params.search_type.as_deref(), params.top_k).await?;
    Ok(Html(render_results(&data)))
}

pub async fn api_search_post(
    State(state): State<Arc<AppState>>,
    axum::Form(params): axum::Form<SearchParams>,
) -> Result<Html<String>, AppError> {
    let query = params.q.as_deref().unwrap_or("");
    if query.is_empty() {
        return Ok(Html(r##"<div class="text-gray-500 text-sm">Enter a query to search.</div>"##.to_string()));
    }
    let data = state.client.search(query, params.search_type.as_deref(), params.top_k).await?;
    Ok(Html(render_results(&data)))
}

fn render_results(data: &Value) -> String {
    let results = data.as_array().cloned().unwrap_or_else(|| {
        data.get("results")
            .and_then(|r| r.as_array())
            .cloned()
            .unwrap_or_default()
    });

    if results.is_empty() {
        return r##"<div class="card text-gray-500 text-center py-8">No results found.</div>"##.to_string();
    }

    let mut cards = String::new();
    for (i, result) in results.iter().enumerate() {
        let text = result.get("text")
            .or_else(|| result.get("content"))
            .or_else(|| result.get("chunk_text"))
            .and_then(|v| v.as_str())
            .unwrap_or("(no text)");
        let score = result.get("score")
            .and_then(|v| v.as_f64())
            .map(|s| format!("{:.4}", s))
            .unwrap_or_else(|| "—".to_string());
        let source = result.get("document_name")
            .or_else(|| result.get("source"))
            .and_then(|v| v.as_str())
            .unwrap_or("unknown");

        cards.push_str(&format!(
            r##"<div class="card mb-3">
                <div class="flex items-center justify-between mb-2">
                    <span class="text-xs text-gray-500">Result #{}</span>
                    <span class="text-xs bg-gray-800 px-2 py-1 rounded font-mono">score: {score}</span>
                </div>
                <p class="text-sm leading-relaxed">{}</p>
                <p class="text-xs text-gray-500 mt-2">Source: {source}</p>
            </div>"##,
            i + 1,
            html_escape(text),
        ));
    }

    format!(
        r##"<div class="mb-4 text-sm text-gray-400">{} result(s)</div>{cards}"##,
        results.len()
    )
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
