use axum::extract::State;
use axum::response::Html;
use serde_json::json;
use std::sync::Arc;

use crate::error::AppError;
use super::AppState;
use super::base_html;

pub async fn page(State(_state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let content = r##"
        <div class="mb-8">
            <h2 class="text-2xl font-bold">Dashboard</h2>
            <p class="text-gray-400 mt-1">Cognee Knowledge Engine status overview</p>
        </div>

        <div id="health-panel"
             hx-get="/api/health"
             hx-trigger="load, every 30s"
             hx-swap="innerHTML"
             hx-indicator="#health-loading">
            <div class="flex items-center gap-2 text-gray-500">
                <span id="health-loading" class="htmx-indicator">
                    <svg class="animate-spin h-5 w-5" viewBox="0 0 24 24">
                        <circle class="opacity-25" cx="12" cy="12" r="10" stroke="currentColor" stroke-width="4" fill="none"/>
                        <path class="opacity-75" fill="currentColor" d="M4 12a8 8 0 018-8V0C5.373 0 0 5.373 0 12h4z"/>
                    </svg>
                </span>
                Loading health status...
            </div>
        </div>

        <div class="mt-8">
            <h3 class="text-lg font-semibold mb-4">Latency History</h3>
            <div class="card">
                <canvas id="latencyChart" height="80"></canvas>
            </div>
        </div>

        <script>
        let latencyChart;
        const latencyData = { labels: [], datasets: [] };

        document.addEventListener('DOMContentLoaded', () => {
            const ctx = document.getElementById('latencyChart');
            if (ctx) {
                latencyChart = new Chart(ctx, {
                    type: 'line',
                    data: latencyData,
                    options: {
                        responsive: true,
                        plugins: { legend: { labels: { color: '#9ca3af' } } },
                        scales: {
                            x: { ticks: { color: '#6b7280' }, grid: { color: '#1f2937' } },
                            y: { ticks: { color: '#6b7280' }, grid: { color: '#1f2937' }, title: { display: true, text: 'ms', color: '#6b7280' } }
                        }
                    }
                });
            }
        });

        document.body.addEventListener('htmx:afterSwap', (e) => {
            if (e.detail.target.id === 'health-panel') {
                const el = document.getElementById('chart-data');
                if (el && latencyChart) {
                    try {
                        const d = JSON.parse(el.textContent);
                        if (d.labels) {
                            latencyChart.data = d;
                            latencyChart.update();
                        }
                    } catch(e) {}
                }
            }
        });
        </script>
    "##;

    Ok(Html(base_html("Dashboard", "/", content)))
}

pub async fn api_health(State(state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let health = state.client.health_detailed().await;

    let html = match health {
        Ok(data) => {
            let status = data.get("status")
                .and_then(|s| s.as_str())
                .unwrap_or("unknown");
            let components = data.get("components")
                .cloned()
                .unwrap_or(json!({}));
            let uptime = data.get("uptime")
                .and_then(|u| u.as_i64())
                .unwrap_or(0);

            let _ = sqlx::query(
                "INSERT INTO cognee_admin.health_snapshots (status, components, uptime) VALUES ($1, $2, $3)"
            )
            .bind(status)
            .bind(&components)
            .bind(uptime as i32)
            .execute(&state.pool)
            .await;

            let version = data.get("version")
                .and_then(|v| v.as_str())
                .unwrap_or("unknown");

            let uptime_str = format_uptime(uptime);

            let status_color = match status {
                "healthy" => "green",
                "degraded" => "yellow",
                _ => "red",
            };

            let component_names = [
                ("relational_db", "Relational DB", "🗄️"),
                ("vector_db", "Vector DB", "📐"),
                ("graph_db", "Graph DB", "🕸️"),
                ("file_storage", "File Storage", "📂"),
                ("llm_provider", "LLM Provider", "🤖"),
                ("embedding_service", "Embedding Service", "🧮"),
            ];

            let mut cards = String::new();
            for (key, label, icon) in &component_names {
                let comp = components.get(*key);
                let comp_status = comp
                    .and_then(|c| c.get("status"))
                    .and_then(|s| s.as_str())
                    .unwrap_or("unknown");
                let comp_latency = comp
                    .and_then(|c| c.get("latency_ms"))
                    .and_then(|l| l.as_f64())
                    .map(|l| format!("{:.0}ms", l))
                    .unwrap_or_else(|| "N/A".to_string());
                let color = match comp_status {
                    "healthy" | "ok" => "green",
                    "degraded" => "yellow",
                    _ => "red",
                };

                cards.push_str(&format!(
                    r##"<div class="card">
                        <div class="flex items-center justify-between mb-3">
                            <span class="text-lg">{icon}</span>
                            <span class="status-dot {color}"></span>
                        </div>
                        <h4 class="font-semibold text-sm">{label}</h4>
                        <p class="text-xs text-gray-500 mt-1">Status: <span class="status-{color}">{comp_status}</span></p>
                        <p class="text-xs text-gray-500">Latency: {comp_latency}</p>
                    </div>"##
                ));
            }

            let chart_data = build_chart_data(&state.pool).await;

            format!(
                r##"<div class="grid grid-cols-1 md:grid-cols-3 gap-4 mb-6">
                    <div class="card">
                        <p class="text-xs text-gray-500 uppercase tracking-wider">Status</p>
                        <div class="flex items-center gap-2 mt-2">
                            <span class="status-dot {status_color}"></span>
                            <span class="text-lg font-semibold status-{status_color}">{status}</span>
                        </div>
                    </div>
                    <div class="card">
                        <p class="text-xs text-gray-500 uppercase tracking-wider">Version</p>
                        <p class="text-lg font-semibold mt-2 text-cyan-400">{version}</p>
                    </div>
                    <div class="card">
                        <p class="text-xs text-gray-500 uppercase tracking-wider">Uptime</p>
                        <p class="text-lg font-semibold mt-2">{uptime_str}</p>
                    </div>
                </div>
                <h3 class="text-lg font-semibold mb-4">Components</h3>
                <div class="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 gap-4">
                    {cards}
                </div>
                <script type="application/json" id="chart-data">{chart_data}</script>"##
            )
        }
        Err(e) => {
            format!(
                r##"<div class="card border-red-900">
                    <div class="flex items-center gap-3">
                        <span class="status-dot red"></span>
                        <div>
                            <h4 class="font-semibold text-red-400">Connection Failed</h4>
                            <p class="text-sm text-gray-400 mt-1">{}</p>
                            <p class="text-xs text-gray-500 mt-2">Cognee API may be unreachable. Check if the service is running.</p>
                        </div>
                    </div>
                </div>"##,
                html_escape(&e.to_string())
            )
        }
    };

    Ok(Html(html))
}

async fn build_chart_data(pool: &sqlx::PgPool) -> String {
    let rows = sqlx::query_as::<_, (chrono::DateTime<chrono::Utc>, serde_json::Value)>(
        "SELECT timestamp, components FROM cognee_admin.health_snapshots ORDER BY timestamp DESC LIMIT 20"
    )
    .fetch_all(pool)
    .await
    .unwrap_or_default();

    if rows.is_empty() {
        return "{}".to_string();
    }

    let mut labels = Vec::new();
    let mut db_latencies = Vec::new();
    let mut vector_latencies = Vec::new();
    let mut graph_latencies = Vec::new();

    for (ts, components) in rows.iter().rev() {
        labels.push(ts.format("%H:%M").to_string());
        db_latencies.push(
            components.get("relational_db")
                .and_then(|c| c.get("latency_ms"))
                .and_then(|l| l.as_f64())
                .unwrap_or(0.0)
        );
        vector_latencies.push(
            components.get("vector_db")
                .and_then(|c| c.get("latency_ms"))
                .and_then(|l| l.as_f64())
                .unwrap_or(0.0)
        );
        graph_latencies.push(
            components.get("graph_db")
                .and_then(|c| c.get("latency_ms"))
                .and_then(|l| l.as_f64())
                .unwrap_or(0.0)
        );
    }

    let chart = json!({
        "labels": labels,
        "datasets": [
            { "label": "Relational DB", "data": db_latencies, "borderColor": "#06b6d4", "tension": 0.3 },
            { "label": "Vector DB", "data": vector_latencies, "borderColor": "#8b5cf6", "tension": 0.3 },
            { "label": "Graph DB", "data": graph_latencies, "borderColor": "#f59e0b", "tension": 0.3 },
        ]
    });

    serde_json::to_string(&chart).unwrap_or_else(|_| "{}".to_string())
}

fn format_uptime(seconds: i64) -> String {
    if seconds <= 0 {
        return "N/A".to_string();
    }
    let days = seconds / 86400;
    let hours = (seconds % 86400) / 3600;
    let mins = (seconds % 3600) / 60;
    if days > 0 {
        format!("{}d {}h {}m", days, hours, mins)
    } else if hours > 0 {
        format!("{}h {}m", hours, mins)
    } else {
        format!("{}m", mins)
    }
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
