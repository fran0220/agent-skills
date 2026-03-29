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
                    "--detailed": { "type": "bool", "default": false, "description": "Include detailed system info" },
                    "--watch": { "type": "bool", "default": false, "description": "Continuously poll until Ctrl+C" },
                    "--interval": { "type": "u64", "default": 30, "description": "Polling interval in seconds for watch mode" }
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
                        "params": { "name": { "type": "string", "required": true } }
                    },
                    "delete": {
                        "description": "Delete a dataset",
                        "params": { "id": { "type": "string", "required": true } }
                    },
                    "delete-all": {
                        "description": "Delete every dataset",
                        "params": { "--yes": { "type": "bool", "required": true, "description": "Confirm destructive delete-all operation" } }
                    },
                    "status": { "description": "Show dataset processing status" },
                    "graph": {
                        "description": "Get dataset knowledge graph",
                        "params": { "id": { "type": "string", "required": true } }
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
                            "content": { "type": "string", "required": true, "description": "Text content" }
                        }
                    },
                    "add-file": {
                        "description": "Upload a single file to a dataset",
                        "params": {
                            "--dataset": { "type": "string", "required": true, "description": "Dataset name" },
                            "path": { "type": "string", "required": true, "description": "File path to upload" }
                        }
                    },
                    "add-dir": {
                        "description": "Upload matching files from a directory in batches of 10",
                        "params": {
                            "--dataset": { "type": "string", "required": true, "description": "Dataset name" },
                            "--glob": { "type": "string", "required": false, "default": "*", "description": "Glob pattern used to filter files" },
                            "dir": { "type": "string", "required": true, "description": "Directory to scan recursively" }
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
                    },
                    "update": {
                        "description": "Replace an existing data item with a new file",
                        "params": {
                            "--dataset-id": { "type": "string", "required": true },
                            "--data-id": { "type": "string", "required": true },
                            "path": { "type": "string", "required": true, "description": "File path to upload" }
                        }
                    }
                }
            },
            "cognify": {
                "description": "Trigger cognification pipeline",
                "params": {
                    "--dataset-id": { "type": "string", "required": false, "description": "Optional dataset ID to cognify" },
                    "--dataset-name": { "type": "string", "required": false, "description": "Optional dataset name to cognify" },
                    "--custom-prompt": { "type": "string", "required": false, "description": "Override the default cognify prompt" },
                    "--custom-prompt-file": { "type": "path", "required": false, "description": "Read custom prompt text from a file" },
                    "--background": { "type": "bool", "default": false, "description": "Run cognify in background mode" },
                    "--chunks-per-batch": { "type": "u32", "required": false, "description": "Override chunks per batch for the pipeline" }
                }
            },
            "search": {
                "description": "Search the knowledge base",
                "params": {
                    "query": { "type": "string", "required": true, "description": "Search query unless using the history subcommand" },
                    "--search-type": { "type": "string", "default": "INSIGHTS", "description": "Search type" },
                    "--top-k": { "type": "u32", "default": 5, "description": "Number of results" },
                    "--datasets": { "type": "string[]", "required": false, "description": "Comma-separated dataset names to filter by" },
                    "--verbose": { "type": "bool", "default": false, "description": "Include request metadata in the response" }
                },
                "subcommands": {
                    "history": { "description": "Fetch search history" }
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
            "log": {
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
            "ontology": {
                "description": "Ontology management",
                "subcommands": {
                    "upload": {
                        "description": "Upload an ontology OWL file",
                        "params": {
                            "--key": { "type": "string", "required": true, "description": "Ontology key to store under" },
                            "file": { "type": "path", "required": true, "description": "Path to the OWL file" }
                        }
                    },
                    "list": { "description": "List uploaded ontologies" }
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
