use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::str::FromStr;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::config::AppConfig;
use crate::search::{SearchEngine, SearchMode};

#[derive(Deserialize)]
struct JsonRpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    id: Option<Value>,
    method: String,
    params: Option<Value>,
}

#[derive(Serialize)]
struct JsonRpcResponse {
    jsonrpc: String,
    id: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<JsonRpcError>,
}

#[derive(Serialize)]
struct JsonRpcError {
    code: i32,
    message: String,
}

impl JsonRpcResponse {
    fn success(id: Option<Value>, result: Value) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: Some(result),
            error: None,
        }
    }

    fn error(id: Option<Value>, code: i32, message: String) -> Self {
        Self {
            jsonrpc: "2.0".to_string(),
            id,
            result: None,
            error: Some(JsonRpcError { code, message }),
        }
    }
}

pub async fn run_stdio(config: AppConfig) -> Result<()> {
    let engine = SearchEngine::new(config)?;
    let reader = BufReader::new(tokio::io::stdin());
    let mut lines = reader.lines();
    let mut stdout = tokio::io::stdout();

    while let Some(line) = lines.next_line().await? {
        let line = line.trim().to_string();
        if line.is_empty() {
            continue;
        }

        let req: JsonRpcRequest = match serde_json::from_str(&line) {
            Ok(request) => request,
            Err(error) => {
                let resp = JsonRpcResponse::error(None, -32700, format!("Parse error: {error}"));
                let out = serde_json::to_string(&resp)?;
                stdout.write_all(out.as_bytes()).await?;
                stdout.write_all(b"\n").await?;
                stdout.flush().await?;
                continue;
            }
        };

        let resp = handle_request(&engine, &req).await;
        let out = serde_json::to_string(&resp)?;
        stdout.write_all(out.as_bytes()).await?;
        stdout.write_all(b"\n").await?;
        stdout.flush().await?;
    }

    Ok(())
}

async fn handle_request(engine: &SearchEngine, req: &JsonRpcRequest) -> JsonRpcResponse {
    match req.method.as_str() {
        "initialize" => JsonRpcResponse::success(
            req.id.clone(),
            json!({
                "protocolVersion": "2024-11-05",
                "capabilities": {"tools": {}},
                "serverInfo": {
                    "name": "ai-search",
                    "version": env!("CARGO_PKG_VERSION")
                }
            }),
        ),
        "notifications/initialized" => JsonRpcResponse::success(req.id.clone(), json!({})),
        "tools/list" => JsonRpcResponse::success(
            req.id.clone(),
            json!({
                "tools": [{
                    "name": "ai_search",
                    "description": "Search the web in fast, deep, or answer mode with Grok, Exa, and Tavily.",
                    "inputSchema": {
                        "type": "object",
                        "properties": {
                            "query": {
                                "type": "string",
                                "description": "The search query"
                            },
                            "mode": {
                                "type": "string",
                                "enum": ["fast", "deep", "answer"],
                                "description": "Search mode (default comes from config)"
                            },
                            "model": {
                                "type": "string",
                                "description": "Optional Grok model override for fast/deep mode"
                            },
                            "split": {
                                "type": "integer",
                                "description": "Max sub-queries for complex searches"
                            }
                        },
                        "required": ["query"]
                    }
                }]
            }),
        ),
        "tools/call" => {
            let params = req.params.as_ref();
            let tool_name = params
                .and_then(|value| value.get("name"))
                .and_then(Value::as_str)
                .unwrap_or("");

            if tool_name != "ai_search" {
                return JsonRpcResponse::error(
                    req.id.clone(),
                    -32602,
                    format!("Unknown tool: {tool_name}"),
                );
            }

            let arguments = params
                .and_then(|value| value.get("arguments"))
                .cloned()
                .unwrap_or_else(|| json!({}));

            let query = arguments
                .get("query")
                .and_then(Value::as_str)
                .unwrap_or("")
                .trim()
                .to_string();
            if query.is_empty() {
                return JsonRpcResponse::error(
                    req.id.clone(),
                    -32602,
                    "Missing required parameter: query".to_string(),
                );
            }

            let mode = match arguments.get("mode").and_then(Value::as_str) {
                Some(mode) => match SearchMode::from_str(mode) {
                    Ok(mode) => mode,
                    Err(error) => {
                        return JsonRpcResponse::error(req.id.clone(), -32602, error.to_string())
                    }
                },
                None => {
                    SearchMode::from_str(&engine.config().default_mode).unwrap_or(SearchMode::Deep)
                }
            };
            let model = arguments.get("model").and_then(Value::as_str);
            let split = arguments
                .get("split")
                .and_then(Value::as_u64)
                .map(|value| value as u32)
                .unwrap_or(engine.config().max_split);

            match engine.search(&query, mode, model, split).await {
                Ok(result) => JsonRpcResponse::success(
                    req.id.clone(),
                    json!({
                        "content": [{
                            "type": "text",
                            "text": result.content,
                        }],
                        "structuredContent": result,
                    }),
                ),
                Err(error) => JsonRpcResponse::error(
                    req.id.clone(),
                    -32603,
                    format!("Search failed: {error}"),
                ),
            }
        }
        _ => JsonRpcResponse::error(
            req.id.clone(),
            -32601,
            format!("Method not found: {}", req.method),
        ),
    }
}
