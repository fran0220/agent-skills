use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::server::routes::auth::CurrentUser;
use crate::server::ws::{broadcast_job_update, JobUpdate};
use crate::server::ServerState;

fn format_datetime(value: DateTime<Utc>) -> String {
    value.to_rfc3339()
}

fn format_optional_datetime(value: Option<DateTime<Utc>>) -> Option<String> {
    value.map(|ts| ts.to_rfc3339())
}

fn compute_duration_ms(
    started_at: Option<DateTime<Utc>>,
    completed_at: Option<DateTime<Utc>>,
) -> Option<i64> {
    let start = started_at?;
    let end = completed_at.unwrap_or_else(Utc::now);
    let duration = (end - start).num_milliseconds();
    Some(duration.max(0))
}

#[derive(serde::Deserialize)]
pub struct ListJobsQuery {
    pub status: Option<String>,
    pub limit: Option<u32>,
}

async fn list_jobs(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
    Query(query): Query<ListJobsQuery>,
) -> AppResult<Json<Value>> {
    let limit = query.limit.unwrap_or(20).clamp(1, 200) as i64;

    let rows = if current_user.is_admin() {
        if let Some(status) = query.status.as_deref() {
            sqlx::query(
                "SELECT id, user_id, asset_type, provider_id, status, error_message, output_path, cost_usd, created_at, started_at, completed_at FROM jobs WHERE status = $1 ORDER BY created_at DESC LIMIT $2",
            )
            .bind(status)
            .bind(limit)
            .fetch_all(&state.db)
            .await
            .map_err(AppError::internal)?
        } else {
            sqlx::query(
                "SELECT id, user_id, asset_type, provider_id, status, error_message, output_path, cost_usd, created_at, started_at, completed_at FROM jobs ORDER BY created_at DESC LIMIT $1",
            )
            .bind(limit)
            .fetch_all(&state.db)
            .await
            .map_err(AppError::internal)?
        }
    } else if let Some(status) = query.status.as_deref() {
        sqlx::query(
            "SELECT id, user_id, asset_type, provider_id, status, error_message, output_path, cost_usd, created_at, started_at, completed_at FROM jobs WHERE user_id = $1 AND status = $2 ORDER BY created_at DESC LIMIT $3",
        )
        .bind(&current_user.id)
        .bind(status)
        .bind(limit)
        .fetch_all(&state.db)
        .await
        .map_err(AppError::internal)?
    } else {
        sqlx::query(
            "SELECT id, user_id, asset_type, provider_id, status, error_message, output_path, cost_usd, created_at, started_at, completed_at FROM jobs WHERE user_id = $1 ORDER BY created_at DESC LIMIT $2",
        )
        .bind(&current_user.id)
        .bind(limit)
        .fetch_all(&state.db)
        .await
        .map_err(AppError::internal)?
    };

    let mut jobs = Vec::with_capacity(rows.len());
    for row in rows {
        let started_at: Option<DateTime<Utc>> =
            row.try_get("started_at").map_err(AppError::internal)?;
        let completed_at: Option<DateTime<Utc>> =
            row.try_get("completed_at").map_err(AppError::internal)?;

        jobs.push(json!({
            "id": row.try_get::<String, _>("id").map_err(AppError::internal)?,
            "user_id": row.try_get::<Option<String>, _>("user_id").map_err(AppError::internal)?,
            "asset_type": row.try_get::<String, _>("asset_type").map_err(AppError::internal)?,
            "provider_id": row.try_get::<String, _>("provider_id").map_err(AppError::internal)?,
            "status": row.try_get::<String, _>("status").map_err(AppError::internal)?,
            "error_message": row.try_get::<Option<String>, _>("error_message").map_err(AppError::internal)?,
            "output_path": row.try_get::<Option<String>, _>("output_path").map_err(AppError::internal)?,
            "cost_usd": row.try_get::<Option<f64>, _>("cost_usd").map_err(AppError::internal)?,
            "created_at": format_datetime(row.try_get::<DateTime<Utc>, _>("created_at").map_err(AppError::internal)?),
            "started_at": format_optional_datetime(started_at),
            "completed_at": format_optional_datetime(completed_at),
            "duration_ms": compute_duration_ms(started_at, completed_at),
        }));
    }

    Ok(Json(json!({
        "ok": true,
        "command": "job.list",
        "data": { "jobs": jobs }
    })))
}

async fn get_job(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
    Path(id): Path<String>,
) -> AppResult<Json<Value>> {
    let row = if current_user.is_admin() {
        sqlx::query(
            "SELECT id, user_id, asset_type, provider_id, status, request, response, error_message, output_path, cost_usd, created_at, started_at, completed_at FROM jobs WHERE id = $1",
        )
        .bind(&id)
        .fetch_optional(&state.db)
        .await
    } else {
        sqlx::query(
            "SELECT id, user_id, asset_type, provider_id, status, request, response, error_message, output_path, cost_usd, created_at, started_at, completed_at FROM jobs WHERE id = $1 AND user_id = $2",
        )
        .bind(&id)
        .bind(&current_user.id)
        .fetch_optional(&state.db)
        .await
    }
    .map_err(AppError::internal)?
    .ok_or_else(|| AppError::not_found(format!("job not found: {}", id)))?;

    let request_text: String = row.try_get("request").map_err(AppError::internal)?;
    let response_text: Option<String> = row.try_get("response").map_err(AppError::internal)?;
    let request_json =
        serde_json::from_str::<Value>(&request_text).unwrap_or(Value::String(request_text));
    let response_json = response_text
        .as_deref()
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .or_else(|| response_text.map(Value::String));

    let started_at: Option<DateTime<Utc>> =
        row.try_get("started_at").map_err(AppError::internal)?;
    let completed_at: Option<DateTime<Utc>> =
        row.try_get("completed_at").map_err(AppError::internal)?;

    Ok(Json(json!({
        "ok": true,
        "command": "job.status",
        "data": {
            "id": row.try_get::<String, _>("id").map_err(AppError::internal)?,
            "user_id": row.try_get::<Option<String>, _>("user_id").map_err(AppError::internal)?,
            "asset_type": row.try_get::<String, _>("asset_type").map_err(AppError::internal)?,
            "provider_id": row.try_get::<String, _>("provider_id").map_err(AppError::internal)?,
            "status": row.try_get::<String, _>("status").map_err(AppError::internal)?,
            "request": request_json,
            "response": response_json,
            "error_message": row.try_get::<Option<String>, _>("error_message").map_err(AppError::internal)?,
            "output_path": row.try_get::<Option<String>, _>("output_path").map_err(AppError::internal)?,
            "cost_usd": row.try_get::<Option<f64>, _>("cost_usd").map_err(AppError::internal)?,
            "created_at": format_datetime(row.try_get::<DateTime<Utc>, _>("created_at").map_err(AppError::internal)?),
            "started_at": format_optional_datetime(started_at),
            "completed_at": format_optional_datetime(completed_at),
            "duration_ms": compute_duration_ms(started_at, completed_at),
        }
    })))
}

async fn cancel_job(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
    Path(id): Path<String>,
) -> AppResult<Json<Value>> {
    let row = if current_user.is_admin() {
        sqlx::query("SELECT user_id, asset_type, provider_id, status FROM jobs WHERE id = $1")
            .bind(&id)
            .fetch_optional(&state.db)
            .await
    } else {
        sqlx::query(
            "SELECT user_id, asset_type, provider_id, status FROM jobs WHERE id = $1 AND user_id = $2",
        )
        .bind(&id)
        .bind(&current_user.id)
        .fetch_optional(&state.db)
        .await
    }
    .map_err(AppError::internal)?
    .ok_or_else(|| AppError::not_found(format!("job not found: {}", id)))?;

    let user_id: Option<String> = row.try_get("user_id").map_err(AppError::internal)?;
    let asset_type: String = row.try_get("asset_type").map_err(AppError::internal)?;
    let provider_id: String = row.try_get("provider_id").map_err(AppError::internal)?;
    let status: String = row.try_get("status").map_err(AppError::internal)?;
    if !matches!(status.as_str(), "pending" | "running") {
        return Err(AppError::conflict(format!(
            "cannot cancel job {} in state {}; only pending/running jobs are cancellable",
            id, status
        ))
        .with_suggestion("Run `asset-gateway job status <id>` to inspect the latest state."));
    }

    let update_result = sqlx::query(
        "UPDATE jobs SET status = 'cancelled', error_message = COALESCE(error_message, 'cancelled by user'), completed_at = now() WHERE id = $1 AND status IN ('pending', 'running')",
    )
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    if update_result.rows_affected() == 0 {
        return Err(
            AppError::conflict(format!("job state changed before cancellation: {}", id))
                .with_suggestion(
                    "Retry `asset-gateway job status <id>` to inspect the latest state.",
                ),
        );
    }

    if let Some(user_id) = user_id.as_deref() {
        broadcast_job_update(
            &state,
            JobUpdate {
                user_id,
                job_id: &id,
                asset_type: &asset_type,
                provider_id: Some(&provider_id),
                status: "cancelled",
                error_message: Some("cancelled by user"),
                output_path: None,
                cost_usd: None,
            },
        )
        .await;
    }

    Ok(Json(json!({
        "ok": true,
        "command": "job.cancel",
        "data": {
            "id": id,
            "previous_status": status,
            "status": "cancelled",
        }
    })))
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/jobs", get(list_jobs))
        .route("/jobs/{id}", get(get_job))
        .route("/jobs/{id}/cancel", post(cancel_job))
}
