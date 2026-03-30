pub mod auth_cmd;
pub mod config;
pub mod generate_cmd;
pub mod http;
pub mod job_cmd;
pub mod process3d_cmd;
pub mod process_cmd;
pub mod provider_cmd;
pub mod voice_cmd;

use clap::{Subcommand, ValueEnum};
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
        /// Input image URL for editing (Gemini/Grok)
        #[arg(long, alias = "image")]
        input: Option<String>,
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
    /// Text-to-speech synthesis via Qwen3-TTS
    Tts {
        /// Text to synthesize
        #[arg(long)]
        prompt: String,
        /// Voice name or custom voice ID (default: Cherry)
        #[arg(long)]
        voice: Option<String>,
        /// Qwen3-TTS model (default: qwen3-tts-flash)
        #[arg(long)]
        model: Option<String>,
        /// Language hint: Auto, Chinese, English, Japanese, etc.
        #[arg(long)]
        language: Option<String>,
        /// Natural language speaking instructions (requires qwen3-tts-instruct-flash)
        #[arg(long)]
        instructions: Option<String>,
        /// Specific provider
        #[arg(long)]
        provider: Option<String>,
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
        /// Tripo model version (default: P1-20260311)
        #[arg(long)]
        model_version: Option<String>,
        /// Max face count (48-20000)
        #[arg(long)]
        face_limit: Option<u32>,
        /// Enable PBR textures
        #[arg(long)]
        pbr: bool,
        /// Texture quality: standard or detailed
        #[arg(long)]
        texture_quality: Option<String>,
        /// Auto-scale to real-world dimensions
        #[arg(long)]
        auto_size: bool,
        /// Negative prompt
        #[arg(long)]
        negative_prompt: Option<String>,
        /// Multiview images (4 URLs or file paths: front,left,back,right)
        #[arg(long, value_delimiter = ',')]
        multiview: Option<Vec<String>>,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
pub enum VoiceType {
    Vc,
    Vd,
}

impl VoiceType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Vc => "vc",
            Self::Vd => "vd",
        }
    }
}

// ── Voice commands ──

#[derive(Subcommand)]
pub enum VoiceCommands {
    /// Clone a custom voice from a local audio sample
    Clone {
        /// Input audio file path
        #[arg(long)]
        audio: String,
        /// Preferred voice name
        #[arg(long)]
        name: String,
        /// Target Qwen VC model override
        #[arg(long)]
        target_model: Option<String>,
        /// Explicit audio MIME type override
        #[arg(long)]
        mime: Option<String>,
    },
    /// Design a custom voice from a text description
    Design {
        /// Voice description prompt
        #[arg(long)]
        prompt: String,
        /// Text used for the preview sample
        #[arg(long)]
        preview_text: String,
        /// Preferred voice name
        #[arg(long)]
        name: String,
        /// Target Qwen VD model override
        #[arg(long)]
        target_model: Option<String>,
        /// Language hint (e.g. zh, en)
        #[arg(long)]
        language: Option<String>,
    },
    /// List custom voices
    List {
        /// Voice type: vc (clone) or vd (design)
        #[arg(long = "type", value_enum, default_value_t = VoiceType::Vc)]
        r#type: VoiceType,
        /// Zero-based page index
        #[arg(long, default_value_t = 0)]
        page: u32,
        /// Page size
        #[arg(long, default_value_t = 20)]
        page_size: u32,
    },
    /// Delete a custom voice
    Delete {
        /// Voice ID to delete
        voice_id: String,
        /// Voice type: vc (clone) or vd (design)
        #[arg(long = "type", value_enum, default_value_t = VoiceType::Vc)]
        r#type: VoiceType,
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

// ── Process commands ──

#[derive(Subcommand)]
pub enum ProcessCommands {
    /// Remove background from an image
    RemoveBg {
        /// Input image (file path or URL)
        #[arg(long)]
        input: String,
        /// Also smart-crop to power-of-2 after removing background
        #[arg(long)]
        smart_crop: bool,
        /// Output directory
        #[arg(long, default_value = ".")]
        output_dir: String,
    },
    /// Smart crop an image (trim transparent borders)
    Crop {
        #[arg(long)]
        input: String,
        /// Crop mode: tightest or power_of_2
        #[arg(long, default_value = "tightest")]
        mode: String,
        #[arg(long, default_value = ".")]
        output_dir: String,
    },
    /// Resize an image to exact dimensions
    Resize {
        #[arg(long)]
        input: String,
        #[arg(long)]
        width: u32,
        #[arg(long)]
        height: u32,
        #[arg(long, default_value = ".")]
        output_dir: String,
    },
    /// AI upscale an image (2x or 4x via Real-ESRGAN)
    Upscale {
        #[arg(long)]
        input: String,
        /// Scale factor (2 or 4)
        #[arg(long, default_value_t = 4)]
        scale: u32,
        #[arg(long, default_value = ".")]
        output_dir: String,
    },
}

// ── 3D Process commands ──

#[derive(Subcommand)]
pub enum Process3dCommands {
    /// Convert 3D model format
    Convert {
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        format: String,
        #[arg(long)]
        quad: bool,
        #[arg(long)]
        face_limit: Option<u32>,
        #[arg(long, default_value = ".")]
        output_dir: String,
    },
    /// Re-texture a 3D model
    Texture {
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        prompt: Option<String>,
        #[arg(long)]
        pbr: bool,
        #[arg(long)]
        quality: Option<String>,
        #[arg(long, default_value = ".")]
        output_dir: String,
    },
    /// Auto-rig a 3D model (add skeleton)
    Rig {
        #[arg(long)]
        task_id: String,
        #[arg(long, default_value = "glb")]
        format: String,
        #[arg(long, default_value = "mixamo")]
        spec: String,
        #[arg(long, default_value = ".")]
        output_dir: String,
    },
    /// Apply preset animation to a rigged model
    Animate {
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        animation: String,
        #[arg(long, default_value = "glb")]
        format: String,
        #[arg(long, default_value = ".")]
        output_dir: String,
    },
    /// Reduce polygon count (high-poly to low-poly)
    Reduce {
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        face_limit: Option<u32>,
        #[arg(long)]
        quad: bool,
        #[arg(long, default_value = ".")]
        output_dir: String,
    },
    /// Stylize a model (lego, voxel, voronoi, minecraft)
    Stylize {
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        style: String,
        #[arg(long, default_value = ".")]
        output_dir: String,
    },
    /// Segment mesh into parts
    Segment {
        #[arg(long)]
        task_id: String,
    },
    /// Check if model can be rigged
    Prerigcheck {
        #[arg(long)]
        task_id: String,
    },
}

// ── Handlers ──

pub async fn handle_auth(cmd: AuthCommands, gateway_url: &str) -> anyhow::Result<()> {
    auth_cmd::handle(cmd, gateway_url).await
}

pub async fn handle_generate(cmd: GenerateCommands, gateway_url: &str) -> anyhow::Result<()> {
    generate_cmd::handle(cmd, gateway_url).await
}

pub async fn handle_process(cmd: ProcessCommands, gateway_url: &str) -> anyhow::Result<()> {
    process_cmd::handle(cmd, gateway_url).await
}

pub async fn handle_process3d(cmd: Process3dCommands, gateway_url: &str) -> anyhow::Result<()> {
    process3d_cmd::handle(cmd, gateway_url).await
}

pub async fn handle_provider(cmd: ProviderCommands, gateway_url: &str) -> anyhow::Result<()> {
    provider_cmd::handle(cmd, gateway_url).await
}

pub async fn handle_voice(cmd: VoiceCommands, gateway_url: &str) -> anyhow::Result<()> {
    voice_cmd::handle(cmd, gateway_url).await
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
                    "prompt": { "type": "string", "description": "Prompt for audio generation (BGM/SFX)" },
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
        "generate.tts": {
            "input": {
                "type": "object",
                "required": ["prompt"],
                "properties": {
                    "prompt": { "type": "string", "description": "Text to synthesize into speech" },
                    "voice": { "type": "string", "description": "System voice name (Cherry, Serena, Ethan, Chelsie) or custom voice ID" },
                    "model": { "type": "string", "description": "TTS model: qwen3-tts-flash (default) or qwen3-tts-instruct-flash" },
                    "language": { "type": "string", "description": "Language hint: Auto, Chinese, English, Japanese, Korean, etc." },
                    "instructions": { "type": "string", "description": "Natural language speaking instructions (requires qwen3-tts-instruct-flash)" },
                    "provider": { "type": "string", "description": "Optional provider override" },
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
                    "model_version": { "type": "string", "description": "Tripo model version override (default P1-20260311)" },
                    "face_limit": { "type": "integer", "description": "Output face cap between 48 and 20000" },
                    "pbr": { "type": "boolean", "default": false, "description": "Enable PBR textures explicitly from the CLI" },
                    "texture_quality": { "type": "string", "enum": ["standard", "detailed"], "description": "Texture quality preset" },
                    "auto_size": { "type": "boolean", "default": false, "description": "Auto-scale to real-world dimensions" },
                    "negative_prompt": { "type": "string", "description": "Negative prompt to avoid unwanted geometry or texture features" },
                    "multiview": {
                        "type": "array",
                        "minItems": 4,
                        "maxItems": 4,
                        "items": { "type": "string" },
                        "description": "Exactly 4 images in front,left,back,right order"
                    },
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
        "process": {
            "input": {
                "type": "object",
                "required": ["input", "operations"],
                "properties": {
                    "input": { "type": "string", "description": "Image URL, local file path, or base64" },
                    "operations": {
                        "type": "array",
                        "items": {
                            "type": "object",
                            "properties": {
                                "op": { "type": "string", "enum": ["remove_bg", "smart_crop", "resize", "upscale"] },
                                "mode": { "type": "string", "enum": ["tightest", "power_of2"] },
                                "width": { "type": "integer" },
                                "height": { "type": "integer" },
                                "scale": { "type": "integer", "enum": [2, 4] }
                            }
                        }
                    }
                }
            },
            "output": {
                "type": "object",
                "properties": {
                    "ok": { "type": "boolean" },
                    "command": { "const": "process" },
                    "data": {
                        "type": "object",
                        "properties": {
                            "local_path": { "type": "string" },
                            "width": { "type": "integer" },
                            "height": { "type": "integer" },
                            "operations_applied": { "type": "array", "items": { "type": "string" } },
                            "elapsed_ms": { "type": "integer" }
                        }
                    }
                }
            }
        },
        "process3d.convert": {
            "input": {
                "type": "object",
                "required": ["task_id", "format"],
                "properties": {
                    "task_id": { "type": "string", "description": "Tripo task ID from a previous generation or processing step" },
                    "format": { "type": "string", "enum": ["GLTF", "FBX", "USDZ", "OBJ", "STL", "3MF"] },
                    "quad": { "type": "boolean", "default": false },
                    "face_limit": { "type": "integer" },
                    "output_dir": { "type": "string", "default": "." }
                }
            },
            "output": {
                "type": "object",
                "properties": {
                    "ok": { "type": "boolean" },
                    "command": { "const": "process3d" },
                    "data": {
                        "type": "object",
                        "properties": {
                            "job_id": { "type": "string" },
                            "output_url": { "type": ["string", "null"] },
                            "metadata": { "type": "object" },
                            "elapsed_ms": { "type": "integer" },
                            "local_path": { "type": ["string", "null"] }
                        }
                    }
                }
            }
        },
        "process3d.texture": {
            "input": {
                "type": "object",
                "required": ["task_id"],
                "properties": {
                    "task_id": { "type": "string" },
                    "prompt": { "type": "string", "description": "Optional text prompt describing the new texture" },
                    "pbr": { "type": "boolean", "default": false },
                    "quality": { "type": "string", "enum": ["standard", "detailed"] },
                    "output_dir": { "type": "string", "default": "." }
                }
            },
            "output": { "$ref": "#/process3d.convert/output" }
        },
        "process3d.rig": {
            "input": {
                "type": "object",
                "required": ["task_id"],
                "properties": {
                    "task_id": { "type": "string" },
                    "format": { "type": "string", "default": "glb" },
                    "spec": { "type": "string", "enum": ["mixamo", "tripo"], "default": "mixamo" },
                    "output_dir": { "type": "string", "default": "." }
                }
            },
            "output": { "$ref": "#/process3d.convert/output" }
        },
        "process3d.animate": {
            "input": {
                "type": "object",
                "required": ["task_id", "animation"],
                "properties": {
                    "task_id": { "type": "string" },
                    "animation": { "type": "string", "description": "Preset animation like preset:walk or preset:idle" },
                    "format": { "type": "string", "default": "glb" },
                    "output_dir": { "type": "string", "default": "." }
                }
            },
            "output": { "$ref": "#/process3d.convert/output" }
        },
        "process3d.reduce": {
            "input": {
                "type": "object",
                "required": ["task_id"],
                "properties": {
                    "task_id": { "type": "string" },
                    "face_limit": { "type": "integer" },
                    "quad": { "type": "boolean", "default": false },
                    "output_dir": { "type": "string", "default": "." }
                }
            },
            "output": { "$ref": "#/process3d.convert/output" }
        },
        "process3d.stylize": {
            "input": {
                "type": "object",
                "required": ["task_id", "style"],
                "properties": {
                    "task_id": { "type": "string" },
                    "style": { "type": "string", "description": "Stylization preset such as lego, voxel, voronoi, or minecraft" },
                    "output_dir": { "type": "string", "default": "." }
                }
            },
            "output": { "$ref": "#/process3d.convert/output" }
        },
        "process3d.segment": {
            "input": {
                "type": "object",
                "required": ["task_id"],
                "properties": {
                    "task_id": { "type": "string" }
                }
            },
            "output": { "$ref": "#/process3d.convert/output" }
        },
        "process3d.prerigcheck": {
            "input": {
                "type": "object",
                "required": ["task_id"],
                "properties": {
                    "task_id": { "type": "string" }
                }
            },
            "output": { "$ref": "#/process3d.convert/output" }
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
