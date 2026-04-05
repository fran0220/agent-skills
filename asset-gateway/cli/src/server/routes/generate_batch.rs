use axum::{extract::State, routing::post, Json, Router};
use futures::stream::{self, StreamExt};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::sync::Arc;
use std::time::Instant;
use uuid::Uuid;

use crate::core::pipeline::{ComposeDirection, Pipeline, ProcessOp, ProcessRequest};
use crate::core::{AssetType, GenerateRequest};
use crate::error::{AppError, AppResult};
use crate::server::routes::auth::CurrentUser;
use crate::server::routes::generate::enforce_quota;
use crate::server::ServerState;

const MAX_CONCURRENT_BATCH_REQUESTS: usize = 4;

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct BatchGenerateReq {
    pub asset_type: AssetType,
    pub prompts: Vec<String>,
    #[serde(default)]
    pub shared: BatchSharedParams,
    pub compose: Option<ComposeParams>,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
pub struct BatchSharedParams {
    pub transparent: Option<bool>,
    pub size: Option<String>,
    pub model: Option<String>,
    pub provider: Option<String>,
    pub reference_images: Option<Vec<String>>,
    pub input_file: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ComposeParams {
    pub direction: Option<String>,
    pub columns: Option<u32>,
    pub padding: Option<u32>,
    pub frame_width: Option<u32>,
    pub frame_height: Option<u32>,
}

#[derive(Debug, Serialize)]
struct BatchFrame {
    index: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    provider_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    output_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    cost_usd: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    elapsed_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<String>,
}

#[derive(Debug, Serialize)]
struct SpriteSheetData {
    output_data: String,
    width: u32,
    height: u32,
}

fn build_shared_params(shared: &BatchSharedParams) -> Value {
    let mut params = serde_json::Map::new();
    if let Some(size) = &shared.size {
        params.insert("size".into(), Value::String(size.clone()));
    }
    if let Some(transparent) = shared.transparent {
        params.insert("transparent".into(), Value::Bool(transparent));
    }
    Value::Object(params)
}

fn parse_compose_direction(raw: Option<&str>) -> AppResult<ComposeDirection> {
    match raw.unwrap_or("grid").trim().to_ascii_lowercase().as_str() {
        "horizontal" => Ok(ComposeDirection::Horizontal),
        "vertical" => Ok(ComposeDirection::Vertical),
        "grid" => Ok(ComposeDirection::Grid),
        other => Err(AppError::bad_request(format!(
            "unsupported compose direction: {}",
            other
        ))),
    }
}

async fn generate_batch(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
    Json(req): Json<BatchGenerateReq>,
) -> AppResult<Json<Value>> {
    if req.prompts.is_empty() {
        return Err(AppError::bad_request("prompts array must not be empty"));
    }
    if req.compose.is_some() && req.asset_type != AssetType::Image {
        return Err(AppError::bad_request(
            "compose is only supported for image batch generation",
        ));
    }
    if !current_user.is_admin() {
        enforce_quota(&state, &current_user.id).await?;
    }

    let started = Instant::now();
    let user_id = current_user.id.clone();
    let job_id = Uuid::new_v4().to_string();
    let provider_hint = req
        .shared
        .provider
        .clone()
        .unwrap_or_else(|| "auto".to_string());

    let request_payload = serde_json::to_string(&req).map_err(AppError::internal)?;
    sqlx::query(
        "INSERT INTO jobs (id, user_id, asset_type, provider_id, status, request, started_at) VALUES ($1, $2, $3, $4, 'pending', $5, now())",
    )
    .bind(&job_id)
    .bind(&user_id)
    .bind(req.asset_type.as_str())
    .bind(&provider_hint)
    .bind(request_payload)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    let shared_params = build_shared_params(&req.shared);
    let batch_requests = req
        .prompts
        .iter()
        .enumerate()
        .map(|(index, prompt)| {
            (
                index,
                GenerateRequest {
                    asset_type: req.asset_type,
                    prompt: Some(prompt.clone()),
                    model: req.shared.model.clone(),
                    input_file: req.shared.input_file.clone(),
                    reference_images: req.shared.reference_images.clone().unwrap_or_default(),
                    edit_mode: None,
                    session_id: None,
                    params: shared_params.clone(),
                },
            )
        })
        .collect::<Vec<_>>();

    let provider_override = req.shared.provider.clone();
    let mut frames = stream::iter(batch_requests.into_iter().map(|(index, gen_req)| {
        let state = state.clone();
        let provider_override = provider_override.clone();
        async move {
            let result = state
                .dispatcher
                .dispatch(&gen_req, provider_override.as_deref())
                .await;
            (index, result)
        }
    }))
    .buffer_unordered(MAX_CONCURRENT_BATCH_REQUESTS)
    .collect::<Vec<_>>()
    .await
    .into_iter()
    .map(|(index, result)| match result {
        Ok(response) => BatchFrame {
            index,
            provider_id: Some(response.provider_id),
            output_data: response.output_data,
            output_url: response.output_url,
            cost_usd: response.cost_usd,
            elapsed_ms: Some(response.elapsed_ms),
            error: None,
        },
        Err(error) => BatchFrame {
            index,
            provider_id: None,
            output_data: None,
            output_url: None,
            cost_usd: None,
            elapsed_ms: None,
            error: Some(error.to_string()),
        },
    })
    .collect::<Vec<_>>();
    frames.sort_by_key(|frame| frame.index);

    let spritesheet = if let Some(compose) = &req.compose {
        let inputs = frames
            .iter()
            .filter_map(|frame| {
                frame
                    .output_data
                    .clone()
                    .or_else(|| frame.output_url.clone())
            })
            .collect::<Vec<_>>();

        if inputs.is_empty() {
            None
        } else {
            let result = Pipeline::run(&ProcessRequest {
                input: None,
                inputs,
                operations: vec![ProcessOp::Compose {
                    direction: parse_compose_direction(compose.direction.as_deref())?,
                    columns: compose.columns,
                    padding: compose.padding,
                    frame_width: compose.frame_width,
                    frame_height: compose.frame_height,
                }],
            })
            .await
            .map_err(|error| AppError::provider(error.to_string()))?;

            Some(SpriteSheetData {
                output_data: result.output_data,
                width: result.width,
                height: result.height,
            })
        }
    } else {
        None
    };

    let total_cost_usd = frames
        .iter()
        .filter_map(|frame| frame.cost_usd)
        .sum::<f64>();
    let resolved_provider_id = {
        let providers = frames
            .iter()
            .filter_map(|frame| frame.provider_id.as_deref())
            .collect::<BTreeSet<_>>();
        match providers.len() {
            0 => provider_hint.clone(),
            1 => providers.iter().next().unwrap().to_string(),
            _ => "mixed".to_string(),
        }
    };

    let mut data = json!({
        "job_id": job_id.clone(),
        "user_id": user_id.clone(),
        "frames": frames,
        "total_cost_usd": total_cost_usd,
        "elapsed_ms": started.elapsed().as_millis() as u64,
    });
    if let Some(sheet) = spritesheet {
        data["spritesheet"] = serde_json::to_value(sheet).map_err(AppError::internal)?;
    }

    let response_json = serde_json::to_string(&data).map_err(AppError::internal)?;
    sqlx::query(
        "UPDATE jobs SET provider_id = $1, status = 'completed', response = $2, cost_usd = $3, completed_at = now() WHERE id = $4",
    )
    .bind(&resolved_provider_id)
    .bind(response_json)
    .bind(total_cost_usd)
    .bind(&job_id)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    sqlx::query(
        "UPDATE users SET api_key_quota_used = api_key_quota_used + 1, updated_at = now() WHERE id = $1 AND api_key_quota IS NOT NULL",
    )
    .bind(&user_id)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    Ok(Json(json!({
        "ok": true,
        "command": "generate.batch",
        "data": data,
    })))
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new().route("/generate/batch", post(generate_batch))
}
