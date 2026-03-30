use axum::{extract::State, routing::post, Json, Router};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::providers::tripo3d::Tripo3dProvider;
use crate::server::routes::auth::CurrentUser;
use crate::server::ServerState;

#[derive(Deserialize, Serialize)]
pub struct Process3dReq {
    pub task_id: String,
    pub operation: String,
    #[serde(default)]
    pub params: Value,
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

    let response = match tripo
        .process3d(&req.task_id, &req.operation, &req.params)
        .await
    {
        Ok(result) => {
            let response_json = serde_json::to_string(&result).map_err(AppError::internal)?;
            sqlx::query(
                "UPDATE jobs SET provider_id = $1, status = 'completed', response = $2, completed_at = now() WHERE id = $3",
            )
            .bind("tripo3d")
            .bind(response_json)
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

    let mut data = serde_json::to_value(response).map_err(AppError::internal)?;
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
