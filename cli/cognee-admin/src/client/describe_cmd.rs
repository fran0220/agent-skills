use serde_json::{json, Value};

pub fn run(command: Option<&str>) -> Value {
    match command {
        Some(cmd) => describe_command(cmd),
        None => describe_all(),
    }
}

fn describe_all() -> Value {
    json!({
        "name": "cognee-admin",
        "version": env!("CARGO_PKG_VERSION"),
        "description": "Cognee knowledge engine management CLI",
        "commands": {
            "health": {
                "description": "Check Cognee API health",
                "params": {
                    "--detailed": { "type": "bool", "default": false, "description": "Include detailed system info" }
                }
            },
            "login": {
                "description": "Authenticate with Cognee API",
                "params": {
                    "--username": { "type": "string", "required": true, "description": "Login username" },
                    "--password": { "type": "string", "required": true, "description": "Login password" }
                }
            },
            "dataset": {
                "description": "Dataset management",
                "subcommands": {
                    "list": { "description": "List all datasets" },
                    "create": {
                        "description": "Create a new dataset",
                        "params": { "--name": { "type": "string", "required": true } }
                    },
                    "delete": {
                        "description": "Delete a dataset",
                        "params": { "--id": { "type": "string", "required": true } }
                    },
                    "status": { "description": "Show dataset processing status" },
                    "graph": {
                        "description": "Get dataset knowledge graph",
                        "params": { "--id": { "type": "string", "required": true } }
                    }
                }
            },
            "data": {
                "description": "Data management within datasets",
                "subcommands": {
                    "add": {
                        "description": "Add text data to a dataset",
                        "params": {
                            "--dataset": { "type": "string", "required": true, "description": "Dataset name" },
                            "--content": { "type": "string", "required": true, "description": "Text content" }
                        }
                    },
                    "list": {
                        "description": "List data in a dataset",
                        "params": { "--dataset-id": { "type": "string", "required": true } }
                    },
                    "delete": {
                        "description": "Delete data from a dataset",
                        "params": {
                            "--dataset-id": { "type": "string", "required": true },
                            "--data-id": { "type": "string", "required": true }
                        }
                    },
                    "raw": {
                        "description": "Get raw data content",
                        "params": {
                            "--dataset-id": { "type": "string", "required": true },
                            "--data-id": { "type": "string", "required": true }
                        }
                    }
                }
            },
            "cognify": {
                "description": "Trigger cognification pipeline",
                "params": {
                    "--dataset-id": { "type": "string", "required": false, "description": "Optional dataset to cognify" }
                }
            },
            "search": {
                "description": "Search the knowledge base",
                "params": {
                    "--query": { "type": "string", "required": true },
                    "--type": { "type": "string", "default": "insights", "description": "Search type" },
                    "--top-k": { "type": "u32", "default": 10, "description": "Number of results" }
                }
            },
            "config": {
                "description": "Cognee settings management",
                "subcommands": {
                    "get": { "description": "Get current settings" },
                    "set": {
                        "description": "Update settings",
                        "params": { "--json": { "type": "string", "required": true, "description": "Settings as JSON" } }
                    }
                }
            },
            "logs": {
                "description": "Request log management",
                "subcommands": {
                    "list": {
                        "description": "List recent request logs",
                        "params": {
                            "--limit": { "type": "i64", "default": 50 },
                            "--offset": { "type": "i64", "default": 0 }
                        }
                    },
                    "stats": { "description": "Aggregate log statistics (last 24h)" }
                }
            },
            "pipeline": {
                "description": "Pipeline run management",
                "subcommands": {
                    "list": {
                        "description": "List recent pipeline runs",
                        "params": { "--limit": { "type": "i64", "default": 20 } }
                    },
                    "detail": {
                        "description": "Get pipeline run details",
                        "params": { "--id": { "type": "string", "required": true } }
                    }
                }
            },
            "describe": {
                "description": "Self-describe available commands (JSON Schema)",
                "params": {
                    "command": { "type": "string", "required": false, "description": "Specific command to describe" }
                }
            }
        }
    })
}

fn describe_command(cmd: &str) -> Value {
    let all = describe_all();
    match all.get("commands").and_then(|c| c.get(cmd)) {
        Some(desc) => json!({
            "command": cmd,
            "schema": desc
        }),
        None => json!({
            "error": format!("Unknown command: {cmd}"),
            "available": all.get("commands")
                .and_then(|c| c.as_object())
                .map(|o| o.keys().collect::<Vec<_>>())
                .unwrap_or_default()
        }),
    }
}
