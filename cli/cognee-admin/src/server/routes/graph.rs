use axum::extract::{Path, State};
use axum::response::Html;
use axum::Json;
use serde_json::Value;
use std::sync::Arc;

use crate::error::AppError;
use super::AppState;
use super::base_html;

pub async fn page(State(state): State<Arc<AppState>>) -> Result<Html<String>, AppError> {
    let datasets = state.client.datasets().await.unwrap_or(Value::Array(vec![]));
    let dataset_arr = datasets.as_array().cloned().unwrap_or_default();

    let mut options = r##"<option value="">Select a dataset...</option>"##.to_string();
    for ds in &dataset_arr {
        let id = ds.get("id").and_then(|v| v.as_str()).unwrap_or("");
        let name = ds.get("name").and_then(|v| v.as_str()).unwrap_or(id);
        options.push_str(&format!(r##"<option value="{id}">{name}</option>"##));
    }

    let content = format!(
        r##"
        <div class="mb-8">
            <h2 class="text-2xl font-bold">Knowledge Graph</h2>
            <p class="text-gray-400 mt-1">Interactive visualization of knowledge graph data</p>
        </div>

        <div class="flex items-center gap-4 mb-6">
            <select id="dataset-select"
                    class="bg-gray-800 border border-gray-700 rounded-lg px-4 py-2 text-sm focus:border-cyan-400 focus:outline-none"
                    onchange="loadGraph(this.value)">
                {options}
            </select>
            <button onclick="loadGraph(document.getElementById('dataset-select').value)"
                    class="bg-cyan-600 hover:bg-cyan-700 px-4 py-2 rounded-lg text-sm font-medium transition-colors">
                Reload
            </button>
            <span id="graph-status" class="text-sm text-gray-500"></span>
        </div>

        <div class="card" style="height: 600px; padding: 0; overflow: hidden;">
            <div id="graph-container" style="width: 100%; height: 100%;"></div>
        </div>

        <div class="mt-4 card">
            <h3 class="text-sm font-semibold mb-2">Legend</h3>
            <div class="flex flex-wrap gap-4 text-xs text-gray-400">
                <span><span class="inline-block w-3 h-3 rounded-full bg-cyan-400 mr-1"></span> Entity</span>
                <span><span class="inline-block w-3 h-3 rounded-full bg-purple-400 mr-1"></span> Concept</span>
                <span><span class="inline-block w-3 h-3 rounded-full bg-amber-400 mr-1"></span> Document</span>
                <span><span class="inline-block w-3 h-3 rounded-full bg-green-400 mr-1"></span> Chunk</span>
                <span><span class="inline-block w-3 h-3 rounded-full bg-gray-400 mr-1"></span> Other</span>
            </div>
        </div>

        <script src="https://unpkg.com/vis-network/standalone/umd/vis-network.min.js"></script>
        <script>
        let network = null;

        const nodeColors = {{
            'entity': '#22d3ee',
            'concept': '#a78bfa',
            'document': '#fbbf24',
            'chunk': '#4ade80',
        }};

        function getNodeColor(type) {{
            return nodeColors[(type || '').toLowerCase()] || '#9ca3af';
        }}

        async function loadGraph(datasetId) {{
            if (!datasetId) return;
            const status = document.getElementById('graph-status');
            status.textContent = 'Loading...';

            try {{
                const res = await fetch('/api/graph/' + encodeURIComponent(datasetId));
                const data = await res.json();

                if (!data.nodes || !data.edges) {{
                    status.textContent = 'No graph data available';
                    return;
                }}

                const nodes = data.nodes.map(n => ({{
                    id: n.id,
                    label: n.label || n.name || n.id,
                    color: getNodeColor(n.type || n.node_type),
                    font: {{ color: '#e5e7eb', size: 12 }},
                    shape: 'dot',
                    size: 15,
                    title: JSON.stringify(n, null, 2),
                }}));

                const edges = data.edges.map(e => ({{
                    from: e.source || e.from,
                    to: e.target || e.to,
                    label: e.relationship_name || e.label || '',
                    color: {{ color: '#4b5563', hover: '#00d9ff' }},
                    font: {{ color: '#6b7280', size: 10 }},
                    arrows: 'to',
                }}));

                const container = document.getElementById('graph-container');
                const graphData = {{ nodes: new vis.DataSet(nodes), edges: new vis.DataSet(edges) }};
                const options = {{
                    physics: {{
                        solver: 'forceAtlas2Based',
                        forceAtlas2Based: {{ gravitationalConstant: -50, centralGravity: 0.01, springLength: 100 }},
                        stabilization: {{ iterations: 150 }},
                    }},
                    interaction: {{ hover: true, tooltipDelay: 200, zoomView: true }},
                    layout: {{ improvedLayout: true }},
                }};

                if (network) network.destroy();
                network = new vis.Network(container, graphData, options);
                status.textContent = nodes.length + ' nodes, ' + edges.length + ' edges';
            }} catch (err) {{
                status.textContent = 'Error: ' + err.message;
            }}
        }}
        </script>
    "##
    );

    Ok(Html(base_html("Knowledge Graph", "/graph", &content)))
}

pub async fn api_graph(
    State(state): State<Arc<AppState>>,
    Path(dataset_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    let data = state.client.dataset_graph(&dataset_id).await?;
    Ok(Json(data))
}
