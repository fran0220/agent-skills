pub mod auth_cmd;
pub mod config;
pub mod generate_cmd;
pub mod http;
pub mod job_cmd;
pub mod provider_cmd;

use clap::Subcommand;
use serde_json::{json, Value};

// ── Auth commands ──

#[derive(Subcommand)]
pub enum AuthCommands {
    /// Login to the gateway
    Login {
        /// Gateway URL
        #[arg(long)]
        url: Option<String>,
        /// Authentication token (admin or user token)
        #[arg(long)]
        token: Option<String>,
    },
    /// Logout
    Logout,
    /// Show current identity
    Whoami,
}

// ── Generate commands ──

#[derive(Subcommand)]
pub enum GenerateCommands {
    /// Generate an image
    Image {
        /// Text prompt
        #[arg(long)]
        prompt: String,
        /// Specific provider
        #[arg(long)]
        provider: Option<String>,
        /// Requires transparency
        #[arg(long)]
        transparent: bool,
        /// Model override
        #[arg(long)]
        model: Option<String>,
        /// Image size
        #[arg(long, default_value = "1024x1024")]
        size: String,
        /// Output directory for generated files
        #[arg(long, default_value = ".")]
        output_dir: String,
    },
    /// Generate a video
    Video {
        #[arg(long)]
        prompt: String,
        #[arg(long)]
        provider: Option<String>,
        /// Output directory for generated files
        #[arg(long, default_value = ".")]
        output_dir: String,
    },
    /// Generate audio (BGM/SFX)
    Audio {
        #[arg(long)]
        prompt: String,
        /// Audio type: bgm or sfx
        #[arg(long, default_value = "sfx")]
        r#type: String,
        #[arg(long)]
        duration: Option<f64>,
        /// Output directory for generated files
        #[arg(long, default_value = ".")]
        output_dir: String,
    },
    /// Generate a 3D model
    Model {
        /// Input image URL or path
        #[arg(long)]
        image: Option<String>,
        /// Text prompt (alternative to image)
        #[arg(long)]
        prompt: Option<String>,
        /// Output directory for generated files
        #[arg(long, default_value = ".")]
        output_dir: String,
    },
    /// Generate text via LLM
    Text {
        #[arg(long)]
        prompt: String,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        max_tokens: Option<u64>,
        /// Output directory for generated files
        #[arg(long, default_value = ".")]
        output_dir: String,
    },
}

// ── Provider commands ──

#[derive(Subcommand)]
pub enum ProviderCommands {
    /// List all providers
    List,
    /// Check provider health
    Health {
        /// Provider ID (omit for all)
        name: Option<String>,
    },
}

// ── Job commands ──

#[derive(Subcommand)]
pub enum JobCommands {
    /// List jobs
    List {
        #[arg(long)]
        status: Option<String>,
        #[arg(long, default_value_t = 20)]
        limit: u32,
    },
    /// Get job status
    Status { id: String },
    /// Cancel a job
    Cancel { id: String },
}

// ── Handlers ──

pub async fn handle_auth(cmd: AuthCommands, gateway_url: &str) -> anyhow::Result<()> {
    auth_cmd::handle(cmd, gateway_url).await
}

pub async fn handle_generate(cmd: GenerateCommands, gateway_url: &str) -> anyhow::Result<()> {
    generate_cmd::handle(cmd, gateway_url).await
}

pub async fn handle_provider(cmd: ProviderCommands, gateway_url: &str) -> anyhow::Result<()> {
    provider_cmd::handle(cmd, gateway_url).await
}

pub async fn handle_job(cmd: JobCommands, gateway_url: &str) -> anyhow::Result<()> {
    job_cmd::handle(cmd, gateway_url).await
}

fn describe_schemas() -> Value {
    json!({
        "serve": {
            "input": {
                "type": "object",
                "properties": {
                    "host": { "type": "string", "default": "0.0.0.0" },
                    "port": { "type": "integer", "default": 6700 },
                    "database_url": {
                        "type": "string",
                        "default": "postgres://localhost/asset_gateway",
                        "description": "Database connection URL for server mode"
                    }
                }
            },
            "output": {
                "type": "stream",
                "description": "Runs HTTP server until interrupted"
            }
        },
        "auth.login": {
            "input": {
                "type": "object",
                "required": ["token"],
                "properties": {
                    "url": { "type": "string", "description": "Optional gateway URL override" },
                    "token": { "type": "string", "description": "Admin or user token. Or ASSET_GATEWAY_TOKEN env" }
                }
            },
            "output": {
                "type": "object",
                "properties": {
                    "ok": { "type": "boolean" },
                    "command": { "const": "auth.login" },
                    "data": {
                        "type": "object",
                        "properties": {
                            "token": { "type": "string" },
                            "token_type": { "type": "string" },
                            "expires_at": { "type": "string" },
                            "role": { "type": "string" },
                            "api_key": { "type": ["string", "null"] }
                        }
                    }
                }
            }
        },
        "auth.logout": {
            "input": { "type": "object", "properties": {} },
            "output": {
                "type": "object",
                "properties": {
                    "ok": { "type": "boolean" },
                    "command": { "const": "auth.logout" },
                    "data": {
                        "type": "object",
                        "properties": {
                            "gateway_url": { "type": "string" },
                            "removed": { "type": "boolean" },
                            "message": { "type": "string" }
                        }
                    }
                }
            }
        },
        "auth.whoami": {
            "input": { "type": "object", "properties": {} },
            "output": {
                "type": "object",
                "properties": {
                    "ok": { "type": "boolean" },
                    "command": { "const": "auth.whoami" },
                    "data": {
                        "type": "object",
                        "properties": {
                            "logged_in": { "type": "boolean" },
                            "gateway_url": { "type": "string" },
                            "username": { "type": "string" },
                            "expires_at": { "type": ["string", "null"] },
                            "api_key": { "type": ["string", "null"] },
                        }
                    }
                }
            }
        },
        "generate.image": {
            "input": {
                "type": "object",
                "required": ["prompt"],
                "properties": {
                    "prompt": { "type": "string", "description": "Prompt for image generation" },
                    "provider": { "type": "string", "description": "Optional provider override" },
                    "transparent": { "type": "boolean", "default": false, "description": "Prefer transparent-capable provider" },
                    "model": { "type": "string", "description": "Optional provider model override" },
                    "size": { "type": "string", "default": "1024x1024", "description": "Target image size" },
                    "output_dir": { "type": "string", "default": ".", "description": "Directory to save generated files" }
                }
            },
            "output": {
                "type": "object",
                "properties": {
                    "ok": { "type": "boolean" },
                    "command": { "const": "generate" },
                    "data": {
                        "type": "object",
                        "properties": {
                            "job_id": { "type": "string" },
                            "provider_id": { "type": "string" },
                            "output_url": { "type": ["string", "null"] },
                            "output_data": { "type": ["string", "null"] },
                            "output_path": { "type": ["string", "null"] },
                            "local_path": { "type": ["string", "null"] }
                        }
                    }
                }
            }
        },
        "generate.video": {
            "input": {
                "type": "object",
                "required": ["prompt"],
                "properties": {
                    "prompt": { "type": "string", "description": "Prompt for video generation" },
                    "provider": { "type": "string", "description": "Optional provider override" },
                    "output_dir": { "type": "string", "default": ".", "description": "Directory to save generated files" }
                }
            },
            "output": { "$ref": "#/generate.image/output" }
        },
        "generate.audio": {
            "input": {
                "type": "object",
                "required": ["prompt"],
                "properties": {
                    "prompt": { "type": "string", "description": "Prompt for audio generation" },
                    "type": {
                        "type": "string",
                        "default": "sfx",
                        "enum": ["bgm", "sfx"],
                        "description": "Audio mode: background music or sound effect"
                    },
                    "duration": { "type": "number", "description": "Optional target duration in seconds" },
                    "output_dir": { "type": "string", "default": ".", "description": "Directory to save generated files" }
                }
            },
            "output": { "$ref": "#/generate.image/output" }
        },
        "generate.model": {
            "input": {
                "type": "object",
                "properties": {
                    "image": { "type": "string", "description": "Input image URL or local path" },
                    "prompt": { "type": "string", "description": "Text prompt alternative to image input" },
                    "output_dir": { "type": "string", "default": ".", "description": "Directory to save generated files" }
                }
            },
            "output": { "$ref": "#/generate.image/output" }
        },
        "generate.text": {
            "input": {
                "type": "object",
                "required": ["prompt"],
                "properties": {
                    "prompt": { "type": "string", "description": "Prompt for text generation" },
                    "model": { "type": "string", "description": "Optional model override" },
                    "max_tokens": { "type": "integer", "description": "Maximum output tokens" },
                    "output_dir": { "type": "string", "default": ".", "description": "Directory to save generated files" }
                }
            },
            "output": { "$ref": "#/generate.image/output" }
        },
        "provider.list": {
            "input": { "type": "object", "properties": {} },
            "output": {
                "type": "object",
                "properties": {
                    "ok": { "type": "boolean" },
                    "command": { "const": "provider.list" },
                    "data": {
                        "type": "object",
                        "properties": {
                            "providers": { "type": "array" }
                        }
                    }
                }
            }
        },
        "provider.health": {
            "input": {
                "type": "object",
                "properties": {
                    "name": { "type": "string", "description": "Provider ID; omit to check all" }
                }
            },
            "output": {
                "type": "object",
                "description": "Either provider.health or provider.health.list envelope"
            }
        },
        "provider.reload": {
            "input": { "type": "object", "properties": {} },
            "output": {
                "type": "object",
                "properties": {
                    "ok": { "type": "boolean" },
                    "command": { "const": "provider.reload" },
                    "data": {
                        "type": "object",
                        "properties": {
                            "loaded": { "type": "integer" },
                            "providers": { "type": "array", "items": { "type": "string" } }
                        }
                    }
                }
            }
        },
        "job.list": {
            "input": {
                "type": "object",
                "properties": {
                    "status": {
                        "type": "string",
                        "enum": ["pending", "running", "completed", "failed", "cancelled"],
                        "description": "Optional status filter"
                    },
                    "limit": { "type": "integer", "default": 20 }
                }
            },
            "output": {
                "type": "object",
                "properties": {
                    "ok": { "type": "boolean" },
                    "command": { "const": "job.list" },
                    "data": {
                        "type": "object",
                        "properties": {
                            "jobs": { "type": "array" }
                        }
                    }
                }
            }
        },
        "job.status": {
            "input": {
                "type": "object",
                "required": ["id"],
                "properties": {
                    "id": { "type": "string" }
                }
            },
            "output": {
                "type": "object",
                "properties": {
                    "ok": { "type": "boolean" },
                    "command": { "const": "job.status" },
                    "data": { "type": "object" }
                }
            }
        },
        "job.cancel": {
            "input": {
                "type": "object",
                "required": ["id"],
                "properties": {
                    "id": { "type": "string" }
                }
            },
            "output": {
                "type": "object",
                "description": "Requires server-side /api/jobs/:id/cancel support"
            }
        },
        "describe": {
            "input": {
                "type": "object",
                "properties": {
                    "command": { "type": "string", "description": "Optional command key (e.g. generate.image)" }
                }
            },
            "output": {
                "type": "object",
                "properties": {
                    "ok": { "type": "boolean" },
                    "command": { "const": "describe" },
                    "data": {
                        "type": "object",
                        "properties": {
                            "requested": { "type": "string" },
                            "schema": { "type": "object" },
                            "schemas": { "type": "object" }
                        }
                    }
                }
            }
        }
    })
}

pub async fn handle_describe(command: Option<String>) -> anyhow::Result<()> {
    let requested = command.unwrap_or_else(|| "all".to_string());
    let schemas = describe_schemas();

    if requested == "all" {
        crate::output::print_json(&crate::output::success(
            "describe",
            json!({
                "requested": requested,
                "schemas": schemas,
            }),
        ));
        return Ok(());
    }

    if let Some(schema) = schemas.get(&requested).cloned() {
        crate::output::print_json(&crate::output::success(
            "describe",
            json!({
                "requested": requested,
                "schema": schema,
            }),
        ));
        return Ok(());
    }

    crate::output::print_json(&crate::output::error(
        "describe",
        "UNKNOWN_COMMAND",
        &format!(
            "unknown command '{}'; use `asset-gateway describe` to list all schemas",
            requested
        ),
        Some("Run `asset-gateway describe` and choose one of the returned command keys."),
    ));
    Ok(())
}
