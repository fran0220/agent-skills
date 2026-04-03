use axum::{extract::State, routing::post, Json, Router};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::core::{AssetType, GenerateRequest};
use crate::error::{AppError, AppResult};
use crate::server::routes::auth::CurrentUser;
use crate::server::ServerState;

#[derive(Deserialize)]
pub struct GenerateReq {
    pub asset_type: AssetType,
    pub prompt: Option<String>,
    pub model: Option<String>,
    pub input_file: Option<String>,
    pub provider: Option<String>,
    /// Top-level size field sent by the npm CLI (e.g. "1792x1024").
    pub size: Option<String>,
    /// Top-level transparent field sent by the npm CLI.
    pub transparent: Option<bool>,
    #[serde(default)]
    pub params: serde_json::Value,
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

async fn generate(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
    Json(req): Json<GenerateReq>,
) -> AppResult<Json<Value>> {
    if !current_user.is_admin() {
        enforce_quota(&state, &current_user.id).await?;
    }

    let job_id = Uuid::new_v4().to_string();
    let provider_hint = req.provider.clone().unwrap_or_else(|| "auto".into());

    // Merge top-level fields (from npm CLI) into params so providers can read them.
    let mut params = match req.params {
        Value::Object(map) => Value::Object(map),
        _ => Value::Object(serde_json::Map::new()),
    };
    if let Some(size) = &req.size {
        if params.get("size").is_none() {
            params["size"] = Value::String(size.clone());
        }
    }
    if let Some(true) = req.transparent {
        if params.get("transparent").is_none() {
            params["transparent"] = Value::Bool(true);
        }
    }

    let gen_req = GenerateRequest {
        asset_type: req.asset_type,
        prompt: req.prompt,
        model: req.model,
        input_file: req.input_file,
        params,
    };

    let request_payload = serde_json::to_string(&gen_req).map_err(AppError::internal)?;
    sqlx::query(
        "INSERT INTO jobs (id, user_id, asset_type, provider_id, status, request, started_at) VALUES ($1, $2, $3, $4, 'pending', $5, now())",
    )
    .bind(&job_id)
    .bind(&current_user.id)
    .bind(gen_req.asset_type.as_str())
    .bind(&provider_hint)
    .bind(request_payload)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    let dispatch_result = state
        .dispatcher
        .dispatch(&gen_req, req.provider.as_deref())
        .await;

    let result = match dispatch_result {
        Ok(result) => {
            let response_json = serde_json::to_string(&result).map_err(AppError::internal)?;
            sqlx::query(
                "UPDATE jobs SET provider_id = $1, status = 'completed', response = $2, output_path = $3, cost_usd = $4, completed_at = now() WHERE id = $5",
            )
            .bind(&result.provider_id)
            .bind(response_json)
            .bind(result.output_path.as_deref())
            .bind(result.cost_usd)
            .bind(&job_id)
            .execute(&state.db)
            .await
            .map_err(AppError::internal)?;

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
            return Err(AppError::provider(error_message));
        }
    };

    let mut data = serde_json::to_value(result).map_err(AppError::internal)?;
    if let Value::Object(ref mut object) = data {
        object.insert("job_id".into(), Value::String(job_id));
        object.insert("user_id".into(), Value::String(current_user.id));
    }

    Ok(Json(json!({
        "ok": true,
        "command": "generate",
        "data": data,
    })))
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new().route("/generate", post(generate))
}
