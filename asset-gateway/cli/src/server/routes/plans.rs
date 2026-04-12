use axum::{
    extract::State,
    routing::{get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::server::routes::auth::CurrentUser;
use crate::server::ServerState;

async fn list_plans(State(state): State<Arc<ServerState>>) -> AppResult<Json<Value>> {
    let rows = sqlx::query(
        "SELECT id, name, monthly_price_cents, quota_per_month, features FROM plans ORDER BY monthly_price_cents ASC, name ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    let mut plans = Vec::with_capacity(rows.len());
    for row in rows {
        plans.push(json!({
            "id": row.try_get::<String, _>("id").map_err(AppError::internal)?,
            "name": row.try_get::<String, _>("name").map_err(AppError::internal)?,
            "monthly_price_cents": row.try_get::<i64, _>("monthly_price_cents").map_err(AppError::internal)?,
            "quota_per_month": row.try_get::<i64, _>("quota_per_month").map_err(AppError::internal)?,
            "features": row.try_get::<Value, _>("features").map_err(AppError::internal)?,
        }));
    }

    Ok(Json(json!({
        "ok": true,
        "command": "plan.list",
        "data": {
            "plans": plans,
        }
    })))
}

async fn get_subscription(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
) -> AppResult<Json<Value>> {
    let row = sqlx::query(
        "SELECT s.id, s.plan_id, s.status, s.stripe_subscription_id, s.current_period_start, s.current_period_end, p.name, p.monthly_price_cents, p.quota_per_month, p.features FROM subscriptions s JOIN plans p ON p.id = s.plan_id WHERE s.user_id = $1 ORDER BY COALESCE(s.current_period_end, s.current_period_start) DESC NULLS LAST LIMIT 1",
    )
    .bind(&current_user.id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?;

    let data = if let Some(row) = row {
        json!({
            "subscription": {
                "id": row.try_get::<String, _>("id").map_err(AppError::internal)?,
                "plan_id": row.try_get::<String, _>("plan_id").map_err(AppError::internal)?,
                "status": row.try_get::<String, _>("status").map_err(AppError::internal)?,
                "stripe_subscription_id": row.try_get::<Option<String>, _>("stripe_subscription_id").map_err(AppError::internal)?,
                "current_period_start": row.try_get::<Option<DateTime<Utc>>, _>("current_period_start").map_err(AppError::internal)?.map(|value| value.to_rfc3339()),
                "current_period_end": row.try_get::<Option<DateTime<Utc>>, _>("current_period_end").map_err(AppError::internal)?.map(|value| value.to_rfc3339()),
                "plan": {
                    "id": row.try_get::<String, _>("plan_id").map_err(AppError::internal)?,
                    "name": row.try_get::<String, _>("name").map_err(AppError::internal)?,
                    "monthly_price_cents": row.try_get::<i64, _>("monthly_price_cents").map_err(AppError::internal)?,
                    "quota_per_month": row.try_get::<i64, _>("quota_per_month").map_err(AppError::internal)?,
                    "features": row.try_get::<Value, _>("features").map_err(AppError::internal)?,
                }
            }
        })
    } else {
        json!({ "subscription": Value::Null })
    };

    Ok(Json(json!({
        "ok": true,
        "command": "subscription.get",
        "data": data,
    })))
}

async fn stripe_webhook(
    State(_state): State<Arc<ServerState>>,
    body: String,
) -> AppResult<Json<Value>> {
    tracing::info!(
        body_len = body.len(),
        "received stripe webhook placeholder request"
    );

    Ok(Json(json!({
        "ok": true,
        "command": "webhooks.stripe",
        "data": {
            "received": true,
        }
    })))
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/plans", get(list_plans))
        .route("/subscription", get(get_subscription))
        .route("/webhooks/stripe", post(stripe_webhook))
}
