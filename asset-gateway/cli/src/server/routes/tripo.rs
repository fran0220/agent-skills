use axum::{extract::State, routing::get, Json, Router};
use serde_json::{json, Value};
use std::sync::Arc;

use crate::error::{AppError, AppResult};
use crate::providers::tripo3d::Tripo3dProvider;
use crate::server::routes::auth::{require_admin, CurrentUser};
use crate::server::ServerState;

async fn tripo_balance(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
) -> AppResult<Json<Value>> {
    require_admin(&current_user)?;

    let provider = state
        .registry
        .get("tripo3d")
        .await
        .ok_or_else(|| AppError::not_found("tripo3d provider not configured"))?;
    let tripo = provider
        .as_any()
        .downcast_ref::<Tripo3dProvider>()
        .ok_or_else(|| AppError::internal("tripo3d provider has unexpected type"))?;

    let balances = tripo.client().query_all_balances().await;
    let total_balance: f64 = balances.iter().filter_map(|b| b["balance"].as_f64()).sum();
    let total_frozen: f64 = balances.iter().filter_map(|b| b["frozen"].as_f64()).sum();
    let total_available: f64 = balances
        .iter()
        .filter_map(|b| b["available"].as_f64())
        .sum();
    let key_count = balances.len();
    let healthy_count = balances.iter().filter(|b| b["status"] == "ok").count();

    Ok(Json(json!({
        "ok": true,
        "data": {
            "keys": balances,
            "summary": {
                "total_keys": key_count,
                "healthy_keys": healthy_count,
                "total_balance": total_balance,
                "total_frozen": total_frozen,
                "total_available": total_available,
            }
        }
    })))
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new().route("/tripo/balance", get(tripo_balance))
}
