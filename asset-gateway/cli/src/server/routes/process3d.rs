use axum::{extract::State, routing::post, Json, Router};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use sqlx::Row;
use std::path::Path;
use std::sync::Arc;
use tempfile::tempdir;
use tokio::process::Command;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::providers::tripo3d::Tripo3dProvider;
use crate::server::routes::auth::CurrentUser;
use crate::server::ws::{broadcast_job_update, JobUpdate};
use crate::server::ServerState;

const RENDER_SPRITES_SCRIPT: &str = include_str!("../../../scripts/render_sprites.py");
const DEFAULT_RENDER_FRAME_COUNT: u32 = 8;
const DEFAULT_RENDER_RESOLUTION: u32 = 64;
const DEFAULT_RENDER_CAMERA_ANGLE: &str = "front";
const DEFAULT_RENDER_DIRECTIONS: u32 = 1;
const DEFAULT_RENDER_TIMEOUT_SECS: u64 = 300;

#[derive(Deserialize, Serialize)]
pub struct Process3dReq {
    pub task_id: String,
    pub operation: String,
    #[serde(default)]
    pub params: Value,
    #[serde(default)]
    pub frame_count: Option<u32>,
    #[serde(default)]
    pub resolution: Option<u32>,
    #[serde(default)]
    pub camera_angle: Option<String>,
    #[serde(default)]
    pub directions: Option<u32>,
}

#[derive(Serialize)]
struct RenderSpriteFrame {
    filename: String,
    direction: u32,
    frame_index: u32,
    source_frame: u32,
    image_base64: String,
}

#[derive(Serialize)]
struct RenderSpritesResponse {
    output_url: Option<String>,
    metadata: Value,
    frames: Vec<RenderSpriteFrame>,
    elapsed_ms: u64,
}

impl Process3dReq {
    fn merged_params(&self) -> Value {
        let mut params = self
            .params
            .as_object()
            .cloned()
            .unwrap_or_else(Map::<String, Value>::new);

        if let Some(frame_count) = self.frame_count {
            params.insert("frame_count".into(), json!(frame_count));
        }
        if let Some(resolution) = self.resolution {
            params.insert("resolution".into(), json!(resolution));
        }
        if let Some(camera_angle) = self.camera_angle.as_ref() {
            params.insert("camera_angle".into(), json!(camera_angle));
        }
        if let Some(directions) = self.directions {
            params.insert("directions".into(), json!(directions));
        }

        Value::Object(params)
    }
}

async fn enforce_quota(state: &ServerState, user_id: &str) -> AppResult<()> {
    let row = sqlx::query("SELECT api_key_quota, api_key_quota_used FROM users WHERE id = $1")
        .bind(user_id)
        .fetch_optional(&state.db)
        .await
        .map_err(AppError::internal)?
        .ok_or_else(|| AppError::unauthorized("user no longer exists"))?;

    let quota_limit: Option<i64> = row.try_get("api_key_quota").map_err(AppError::internal)?;
    let quota_used: i64 = row
        .try_get("api_key_quota_used")
        .map_err(AppError::internal)?;

    if let Some(limit) = quota_limit {
        if quota_used >= limit {
            return Err(AppError::forbidden("quota exceeded for this account"));
        }
    }

    Ok(())
}

async fn process3d(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
    Json(req): Json<Process3dReq>,
) -> AppResult<Json<Value>> {
    if !current_user.is_admin() {
        enforce_quota(&state, &current_user.id).await?;
    }

    let provider = state
        .registry
        .get("tripo3d")
        .await
        .ok_or_else(|| AppError::not_found("provider not loaded: tripo3d"))?;
    let tripo = provider
        .as_any()
        .downcast_ref::<Tripo3dProvider>()
        .ok_or_else(|| AppError::internal("provider tripo3d has unexpected concrete type"))?;

    let job_id = Uuid::new_v4().to_string();
    let request_payload = serde_json::to_string(&req).map_err(AppError::internal)?;
    sqlx::query(
        "INSERT INTO jobs (id, user_id, asset_type, provider_id, status, request, started_at) VALUES ($1, $2, $3, $4, 'pending', $5, now())",
    )
    .bind(&job_id)
    .bind(&current_user.id)
    .bind("model3d")
    .bind("tripo3d")
    .bind(request_payload)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;
    broadcast_job_update(
        &state,
        JobUpdate {
            user_id: &current_user.id,
            job_id: &job_id,
            asset_type: "model3d",
            provider_id: Some("tripo3d"),
            status: "pending",
            error_message: None,
            output_path: None,
            cost_usd: None,
        },
    )
    .await;

    sqlx::query("UPDATE jobs SET status = 'running' WHERE id = $1")
        .bind(&job_id)
        .execute(&state.db)
        .await
        .map_err(AppError::internal)?;
    broadcast_job_update(
        &state,
        JobUpdate {
            user_id: &current_user.id,
            job_id: &job_id,
            asset_type: "model3d",
            provider_id: Some("tripo3d"),
            status: "running",
            error_message: None,
            output_path: None,
            cost_usd: None,
        },
    )
    .await;

    let params = req.merged_params();

    let response = match run_process3d_request(tripo, &req, &params).await {
        Ok(result) => {
            let response_json = serde_json::to_string(&sanitize_response_for_storage(&result))
                .map_err(AppError::internal)?;
            sqlx::query(
                "UPDATE jobs SET provider_id = $1, status = 'completed', response = $2, completed_at = now() WHERE id = $3",
            )
            .bind("tripo3d")
            .bind(response_json)
            .bind(&job_id)
            .execute(&state.db)
            .await
            .map_err(AppError::internal)?;
            broadcast_job_update(
                &state,
                JobUpdate {
                    user_id: &current_user.id,
                    job_id: &job_id,
                    asset_type: "model3d",
                    provider_id: Some("tripo3d"),
                    status: "completed",
                    error_message: None,
                    output_path: None,
                    cost_usd: None,
                },
            )
            .await;

            sqlx::query(
                "UPDATE users SET api_key_quota_used = api_key_quota_used + 1, updated_at = now() WHERE id = $1 AND api_key_quota IS NOT NULL",
            )
            .bind(&current_user.id)
            .execute(&state.db)
            .await
            .map_err(AppError::internal)?;

            result
        }
        Err(error) => {
            let error_message = error.to_string();
            sqlx::query(
                "UPDATE jobs SET status = 'failed', error_message = $1, completed_at = now() WHERE id = $2",
            )
            .bind(&error_message)
            .bind(&job_id)
            .execute(&state.db)
            .await
            .map_err(AppError::internal)?;
            broadcast_job_update(
                &state,
                JobUpdate {
                    user_id: &current_user.id,
                    job_id: &job_id,
                    asset_type: "model3d",
                    provider_id: Some("tripo3d"),
                    status: "failed",
                    error_message: Some(&error_message),
                    output_path: None,
                    cost_usd: None,
                },
            )
            .await;
            return Err(AppError::provider(error_message));
        }
    };

    let mut data = response;
    if let Value::Object(ref mut object) = data {
        object.insert("job_id".into(), Value::String(job_id));
        object.insert("user_id".into(), Value::String(current_user.id));
    }

    Ok(Json(json!({
        "ok": true,
        "command": "process3d",
        "data": data,
    })))
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new().route("/process3d", post(process3d))
}

async fn run_process3d_request(
    tripo: &Tripo3dProvider,
    req: &Process3dReq,
    params: &Value,
) -> anyhow::Result<Value> {
    if req.operation == "render_sprites" {
        return render_sprites(tripo, &req.task_id, params).await;
    }

    serde_json::to_value(
        tripo
            .process3d(&req.task_id, &req.operation, params)
            .await?,
    )
    .map_err(Into::into)
}

async fn render_sprites(
    tripo: &Tripo3dProvider,
    task_id: &str,
    params: &Value,
) -> anyhow::Result<Value> {
    let start = std::time::Instant::now();
    let task_id = task_id.trim();
    if task_id.is_empty() {
        anyhow::bail!("render_sprites requires a non-empty task_id");
    }

    let params = params
        .as_object()
        .cloned()
        .unwrap_or_else(Map::<String, Value>::new);
    let frame_count = parse_u32_param(&params, "frame_count", DEFAULT_RENDER_FRAME_COUNT)?;
    let resolution = parse_u32_param(&params, "resolution", DEFAULT_RENDER_RESOLUTION)?;
    let camera_angle = parse_camera_angle(&params)?;
    let directions = parse_directions(&params)?;
    let timeout_seconds = parse_u64_param(&params, "timeout_seconds", DEFAULT_RENDER_TIMEOUT_SECS)?;

    let payload = tripo.client().poll_task(task_id, timeout_seconds).await?;
    let model_url = Tripo3dProvider::extract_model_url(&payload)
        .ok_or_else(|| anyhow::anyhow!("Tripo3D task {} completed without a model URL", task_id))?;

    let source_extension = infer_source_extension(&model_url)?;
    let download = reqwest::Client::new().get(&model_url).send().await?;
    if !download.status().is_success() {
        anyhow::bail!(
            "failed to download source model from {}: HTTP {}",
            model_url,
            download.status()
        );
    }
    let model_bytes = download.bytes().await?;

    let temp_dir = tempdir()?;
    let input_path = temp_dir.path().join(format!("input.{source_extension}"));
    let script_path = temp_dir.path().join("render_sprites.py");
    let frames_dir = temp_dir.path().join("frames");

    tokio::fs::write(&input_path, model_bytes).await?;
    tokio::fs::write(&script_path, RENDER_SPRITES_SCRIPT).await?;
    tokio::fs::create_dir_all(&frames_dir).await?;

    // Use xvfb-run on headless servers (EEVEE needs a display).
    // Falls back to direct blender if xvfb-run is not available.
    let has_xvfb = tokio::process::Command::new("which")
        .arg("xvfb-run")
        .output()
        .await
        .is_ok_and(|o| o.status.success());

    let render = if has_xvfb {
        Command::new("xvfb-run")
            .args(["-a", "--server-args=-screen 0 1024x768x24"])
            .arg("blender")
            .arg("--background")
            .arg("--python")
            .arg(&script_path)
            .arg("--")
            .arg(&input_path)
            .arg(&frames_dir)
            .arg(frame_count.to_string())
            .arg(resolution.to_string())
            .arg(&camera_angle)
            .arg(directions.to_string())
            .output()
            .await
            .map_err(|error| anyhow::anyhow!("failed to spawn Blender via xvfb-run: {}", error))?
    } else {
        Command::new("blender")
            .arg("--background")
            .arg("--python")
            .arg(&script_path)
            .arg("--")
            .arg(&input_path)
            .arg(&frames_dir)
            .arg(frame_count.to_string())
            .arg(resolution.to_string())
            .arg(&camera_angle)
            .arg(directions.to_string())
            .output()
            .await
            .map_err(|error| anyhow::anyhow!("failed to spawn Blender: {}", error))?
    };

    if !render.status.success() {
        let stderr = String::from_utf8_lossy(&render.stderr).trim().to_string();
        let stdout = String::from_utf8_lossy(&render.stdout).trim().to_string();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        anyhow::bail!("Blender sprite render failed: {}", detail);
    }

    let metadata_path = frames_dir.join("metadata.json");
    let metadata_bytes = tokio::fs::read(&metadata_path).await?;
    let mut metadata: Value = serde_json::from_slice(&metadata_bytes)?;

    if let Value::Object(ref mut object) = metadata {
        object.insert("operation".into(), json!("render_sprites"));
        object.insert("source_task_id".into(), json!(task_id));
        object.insert("source_model_url".into(), json!(model_url));
        object.insert(
            "render_stdout".into(),
            json!(String::from_utf8_lossy(&render.stdout).trim()),
        );
    }

    let file_entries = metadata
        .get("files")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut frames = Vec::with_capacity(file_entries.len());

    for entry in file_entries {
        let filename = entry
            .get("filename")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("render metadata missing file name"))?
            .to_string();
        let direction = entry.get("direction").and_then(Value::as_u64).unwrap_or(0) as u32;
        let frame_index = entry
            .get("frame_index")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32;
        let source_frame = entry
            .get("source_frame")
            .and_then(Value::as_u64)
            .unwrap_or(0) as u32;
        let bytes = tokio::fs::read(frames_dir.join(&filename)).await?;

        frames.push(RenderSpriteFrame {
            filename,
            direction,
            frame_index,
            source_frame,
            image_base64: STANDARD.encode(bytes),
        });
    }

    serde_json::to_value(RenderSpritesResponse {
        output_url: None,
        metadata,
        frames,
        elapsed_ms: start.elapsed().as_millis() as u64,
    })
    .map_err(Into::into)
}

fn sanitize_response_for_storage(response: &Value) -> Value {
    let mut sanitized = response.clone();
    if let Some(frames) = sanitized.get_mut("frames").and_then(Value::as_array_mut) {
        for frame in frames {
            if let Some(object) = frame.as_object_mut() {
                object.remove("image_base64");
            }
        }
    }
    sanitized
}

fn parse_u32_param(params: &Map<String, Value>, key: &str, default: u32) -> anyhow::Result<u32> {
    match params.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(value) => {
            let Some(number) = value.as_u64() else {
                anyhow::bail!("{} must be a positive integer", key);
            };
            if number == 0 || number > u32::MAX as u64 {
                anyhow::bail!("{} must be between 1 and {}", key, u32::MAX);
            }
            Ok(number as u32)
        }
    }
}

fn parse_u64_param(params: &Map<String, Value>, key: &str, default: u64) -> anyhow::Result<u64> {
    match params.get(key) {
        None | Some(Value::Null) => Ok(default),
        Some(value) => value
            .as_u64()
            .filter(|number| *number > 0)
            .ok_or_else(|| anyhow::anyhow!("{} must be a positive integer", key)),
    }
}

fn parse_camera_angle(params: &Map<String, Value>) -> anyhow::Result<String> {
    let angle = params
        .get("camera_angle")
        .and_then(Value::as_str)
        .unwrap_or(DEFAULT_RENDER_CAMERA_ANGLE)
        .trim()
        .to_string();

    match angle.as_str() {
        "front" | "side" | "iso" | "3/4" | "top" => Ok(angle),
        _ => anyhow::bail!("camera_angle must be one of: front, side, iso, 3/4, top"),
    }
}

fn parse_directions(params: &Map<String, Value>) -> anyhow::Result<u32> {
    let directions = parse_u32_param(params, "directions", DEFAULT_RENDER_DIRECTIONS)?;
    match directions {
        1 | 4 | 8 => Ok(directions),
        _ => anyhow::bail!("directions must be one of: 1, 4, 8"),
    }
}

fn infer_source_extension(model_url: &str) -> anyhow::Result<&'static str> {
    let path = model_url.split('?').next().unwrap_or(model_url);
    let extension = Path::new(path)
        .extension()
        .and_then(|value| value.to_str())
        .unwrap_or("glb")
        .trim_start_matches('.')
        .to_ascii_lowercase();

    match extension.as_str() {
        "glb" => Ok("glb"),
        "gltf" => Ok("gltf"),
        _ => anyhow::bail!(
            "render_sprites currently supports GLB/GLTF source tasks, got .{}",
            extension
        ),
    }
}
