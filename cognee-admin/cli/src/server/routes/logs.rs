use axum::extract::{Query, State};
use axum::response::Html;
use serde::Deserialize;
use std::sync::Arc;

use super::base_html;
use super::AppState;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
pub struct LogsParams {
    pub page: Option<i64>,
    pub limit: Option<i64>,
    pub endpoint: Option<String>,
}

pub async fn page(State(_state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let content = r##"
        <div class="mb-8">
            <h2 class="text-2xl font-bold">Request Logs</h2>
            <p class="text-gray-400 mt-1">API request history and performance</p>
        </div>

        <div class="flex items-center gap-4 mb-6">
            <input type="text" id="log-filter" placeholder="Filter by endpoint..."
                   class="bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm focus:border-cyan-400 focus:outline-none"
                   hx-get="/api/logs"
                   hx-trigger="keyup changed delay:300ms"
                   hx-target="#logs-panel"
                   hx-swap="innerHTML"
                   hx-include="this"
                   name="endpoint">
            <button hx-get="/api/logs" hx-target="#logs-panel" hx-swap="innerHTML"
                    class="bg-gray-800 hover:bg-gray-700 px-4 py-2 rounded-lg text-sm transition-colors border border-gray-700">
                Refresh
            </button>
        </div>

        <div id="logs-panel"
             hx-get="/api/logs"
             hx-trigger="load"
             hx-swap="innerHTML">
            <div class="text-gray-500">Loading logs...</div>
        </div>
    "##;

    Ok(Html(base_html("Request Logs", "/logs", content)))
}

pub async fn api_logs(
    State(state): State<Arc<AppState>>,
    Query(params): Query<LogsParams>,
) -> Result<Html<String>, AppError> {
    let limit = params.limit.unwrap_or(50).min(200);
    let page_num = params.page.unwrap_or(1).max(1);
    let offset = (page_num - 1) * limit;

    let (rows, total): (Vec<LogRow>, i64) = if let Some(ref ep) = params.endpoint {
        if ep.is_empty() {
            fetch_logs_unfiltered(&state.pool, limit, offset).await?
        } else {
            let pattern = format!("%{}%", ep);
            fetch_logs_filtered(&state.pool, limit, offset, &pattern).await?
        }
    } else {
        fetch_logs_unfiltered(&state.pool, limit, offset).await?
    };

    if rows.is_empty() {
        return Ok(Html(
            r##"<div class="card text-gray-500 text-center py-8">No request logs yet.</div>"##
                .to_string(),
        ));
    }

    let mut row_html = String::new();
    for r in &rows {
        let status_color = match r.status_code {
            Some(c) if (200..300).contains(&c) => "text-green-400",
            Some(c) if (400..500).contains(&c) => "text-yellow-400",
            Some(c) if c >= 500 => "text-red-400",
            _ => "text-gray-400",
        };
        let latency_color = match r.latency_ms {
            Some(l) if l < 100 => "text-green-400",
            Some(l) if l < 500 => "text-yellow-400",
            Some(_) => "text-red-400",
            None => "text-gray-500",
        };
        let ts = r.timestamp.format("%Y-%m-%d %H:%M:%S").to_string();
        let method = &r.method;
        let endpoint = &r.endpoint;
        let status = r
            .status_code
            .map(|c| c.to_string())
            .unwrap_or_else(|| "—".to_string());
        let latency = r
            .latency_ms
            .map(|l| format!("{}ms", l))
            .unwrap_or_else(|| "—".to_string());
        let source = &r.source;

        row_html.push_str(&format!(
            r##"<tr class="border-b border-gray-800 hover:bg-gray-800/50">
                <td class="py-2.5 px-4 text-xs text-gray-500 font-mono">{ts}</td>
                <td class="py-2.5 px-4"><span class="bg-gray-800 px-2 py-0.5 rounded text-xs font-mono">{method}</span></td>
                <td class="py-2.5 px-4 text-sm font-mono">{endpoint}</td>
                <td class="py-2.5 px-4 {status_color} font-mono text-sm">{status}</td>
                <td class="py-2.5 px-4 {latency_color} text-sm">{latency}</td>
                <td class="py-2.5 px-4 text-xs text-gray-500">{source}</td>
            </tr>"##
        ));
    }

    let total_pages = (total as f64 / limit as f64).ceil() as i64;
    let mut pagination = String::new();
    if total_pages > 1 {
        pagination.push_str(r##"<div class="flex items-center gap-2 mt-4 justify-center">"##);
        if page_num > 1 {
            pagination.push_str(&format!(
                r##"<button hx-get="/api/logs?page={}&limit={}" hx-target="#logs-panel" hx-swap="innerHTML"
                          class="bg-gray-800 hover:bg-gray-700 px-3 py-1 rounded text-sm border border-gray-700">← Prev</button>"##,
                page_num - 1, limit
            ));
        }
        pagination.push_str(&format!(
            r##"<span class="text-sm text-gray-500">Page {} of {}</span>"##,
            page_num, total_pages
        ));
        if page_num < total_pages {
            pagination.push_str(&format!(
                r##"<button hx-get="/api/logs?page={}&limit={}" hx-target="#logs-panel" hx-swap="innerHTML"
                          class="bg-gray-800 hover:bg-gray-700 px-3 py-1 rounded text-sm border border-gray-700">Next →</button>"##,
                page_num + 1, limit
            ));
        }
        pagination.push_str("</div>");
    }

    let html = format!(
        r##"<div class="card p-0 overflow-hidden">
            <table class="w-full text-sm">
                <thead>
                    <tr class="bg-gray-800/50 text-left text-xs uppercase text-gray-500">
                        <th class="py-2.5 px-4">Timestamp</th>
                        <th class="py-2.5 px-4">Method</th>
                        <th class="py-2.5 px-4">Endpoint</th>
                        <th class="py-2.5 px-4">Status</th>
                        <th class="py-2.5 px-4">Latency</th>
                        <th class="py-2.5 px-4">Source</th>
                    </tr>
                </thead>
                <tbody>{row_html}</tbody>
            </table>
        </div>
        <div class="mt-2 text-xs text-gray-500">{total} total log(s)</div>
        {pagination}"##
    );

    Ok(Html(html))
}

async fn fetch_logs_unfiltered(
    pool: &sqlx::PgPool,
    limit: i64,
    offset: i64,
) -> Result<(Vec<LogRow>, i64), AppError> {
    let rows = sqlx::query_as::<_, LogRow>(
        "SELECT id, timestamp, method, endpoint, status_code, latency_ms, source \
         FROM cognee_admin.request_logs ORDER BY timestamp DESC LIMIT $1 OFFSET $2",
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let total: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM cognee_admin.request_logs")
        .fetch_one(pool)
        .await?;

    Ok((rows, total.0))
}

async fn fetch_logs_filtered(
    pool: &sqlx::PgPool,
    limit: i64,
    offset: i64,
    pattern: &str,
) -> Result<(Vec<LogRow>, i64), AppError> {
    let rows = sqlx::query_as::<_, LogRow>(
        "SELECT id, timestamp, method, endpoint, status_code, latency_ms, source \
         FROM cognee_admin.request_logs WHERE endpoint ILIKE $3 ORDER BY timestamp DESC LIMIT $1 OFFSET $2"
    )
    .bind(limit)
    .bind(offset)
    .bind(pattern)
    .fetch_all(pool)
    .await?;

    let total: (i64,) =
        sqlx::query_as("SELECT COUNT(*) FROM cognee_admin.request_logs WHERE endpoint ILIKE $1")
            .bind(pattern)
            .fetch_one(pool)
            .await?;

    Ok((rows, total.0))
}

#[derive(Debug, sqlx::FromRow)]
struct LogRow {
    #[allow(dead_code)]
    id: i64,
    timestamp: chrono::DateTime<chrono::Utc>,
    method: String,
    endpoint: String,
    status_code: Option<i32>,
    latency_ms: Option<i32>,
    source: String,
}
