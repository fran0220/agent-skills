use axum::{
    extract::{Path, State},
    routing::{get, put},
    Json, Router,
};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;

use crate::core::AssetType;
use crate::error::{AppError, AppResult};
use crate::server::routes::auth::{require_admin, CurrentUser};
use crate::server::{register_provider_by_id, ServerState};

#[derive(Deserialize)]
pub struct CreateProviderReq {
    pub id: String,
    pub display_name: String,
    pub adapter: String,
    #[serde(default)]
    pub asset_types: Vec<AssetType>,
    #[serde(default)]
    pub config: Value,
    #[serde(default)]
    pub priority: i32,
    #[serde(default = "enabled_by_default")]
    pub enabled: bool,
}

#[derive(Deserialize, Default)]
pub struct UpdateProviderReq {
    pub display_name: Option<String>,
    pub adapter: Option<String>,
    pub asset_types: Option<Vec<AssetType>>,
    pub config: Option<Value>,
    pub priority: Option<i32>,
    pub enabled: Option<bool>,
}

fn enabled_by_default() -> bool {
    true
}

fn is_supported_adapter(adapter: &str) -> bool {
    matches!(
        adapter,
        "llm_proxy" | "gpt_image" | "gemini_image" | "jimeng" | "elevenlabs" | "tripo3d"
    )
}

fn normalize_config(config: Value) -> Value {
    if config.is_object() {
        return config;
    }
    Value::Object(Default::default())
}

async fn list_providers(
    State(state): State<Arc<ServerState>>,
    _current_user: CurrentUser,
) -> AppResult<Json<Value>> {
    let rows = sqlx::query(
        "SELECT id, display_name, adapter, asset_types, priority, enabled FROM providers ORDER BY priority DESC, id ASC",
    )
    .fetch_all(&state.db)
    .await
    .map_err(AppError::internal)?;

    let mut providers = Vec::with_capacity(rows.len());
    for row in rows {
        let id: String = row.try_get("id").map_err(AppError::internal)?;
        let display_name: String = row.try_get("display_name").map_err(AppError::internal)?;
        let adapter: String = row.try_get("adapter").map_err(AppError::internal)?;
        let asset_types_raw: String = row.try_get("asset_types").map_err(AppError::internal)?;
        let priority: i32 = row.try_get("priority").map_err(AppError::internal)?;
        let enabled: bool = row.try_get("enabled").map_err(AppError::internal)?;

        let db_asset_types = serde_json::from_str::<Value>(&asset_types_raw).unwrap_or(json!([]));
        let runtime_provider = state.registry.get(&id).await;
        let runtime_asset_types = runtime_provider
            .as_ref()
            .map(|provider| {
                json!(provider
                    .asset_types()
                    .iter()
                    .map(|t| t.as_str())
                    .collect::<Vec<_>>())
            })
            .unwrap_or(db_asset_types);

        let capabilities = runtime_provider
            .as_ref()
            .map(|provider| json!(provider.capabilities()))
            .unwrap_or(Value::Null);

        providers.push(json!({
            "id": id,
            "display_name": display_name,
            "adapter": adapter,
            "asset_types": runtime_asset_types,
            "priority": priority,
            "enabled": enabled,
            "registered": runtime_provider.is_some(),
            "capabilities": capabilities,
        }));
    }

    Ok(Json(json!({
        "ok": true,
        "command": "provider.list",
        "data": { "providers": providers }
    })))
}

async fn create_provider(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
    Json(req): Json<CreateProviderReq>,
) -> AppResult<Json<Value>> {
    require_admin(&current_user)?;

    let id = req.id.trim().to_string();
    let display_name = req.display_name.trim().to_string();
    if id.is_empty() || display_name.is_empty() {
        return Err(AppError::bad_request(
            "provider id and display_name are required",
        ));
    }
    if !is_supported_adapter(&req.adapter) {
        return Err(AppError::bad_request(format!(
            "unsupported adapter: {}",
            req.adapter
        )));
    }

    let config = normalize_config(req.config);
    let asset_types = serde_json::to_string(&req.asset_types).map_err(AppError::internal)?;
    let config_text = serde_json::to_string(&config).map_err(AppError::internal)?;

    let insert_result = sqlx::query(
        "INSERT INTO providers (id, display_name, adapter, asset_types, config, priority, enabled) VALUES ($1, $2, $3, $4, $5, $6, $7)",
    )
    .bind(&id)
    .bind(&display_name)
    .bind(&req.adapter)
    .bind(asset_types)
    .bind(config_text)
    .bind(req.priority)
    .bind(req.enabled)
    .execute(&state.db)
    .await;

    if let Err(error) = insert_result {
        if let sqlx::Error::Database(db_error) = &error {
            if db_error.is_unique_violation() {
                return Err(AppError::conflict(format!(
                    "provider already exists: {}",
                    id
                )));
            }
        }
        return Err(AppError::internal(error));
    }

    let mut registered = false;
    if req.enabled {
        match register_provider_by_id(&state.db, &state.vault, &state.registry, &id).await {
            Ok(()) => {
                registered = true;
            }
            Err(error) => {
                let _ = sqlx::query("DELETE FROM providers WHERE id = $1")
                    .bind(&id)
                    .execute(&state.db)
                    .await;
                return Err(AppError::bad_request(format!(
                    "provider registration failed, creation rolled back: {}",
                    error
                ))
                .with_suggestion(
                    "Set required credentials via `asset-gateway credential set <key> <value> --provider <provider-id>` and retry.",
                ));
            }
        }
    }

    Ok(Json(json!({
        "ok": true,
        "command": "provider.create",
        "data": {
            "id": id,
            "adapter": req.adapter,
            "enabled": req.enabled,
            "registered": registered,
        }
    })))
}

async fn update_provider(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
    Path(id): Path<String>,
    Json(req): Json<UpdateProviderReq>,
) -> AppResult<Json<Value>> {
    require_admin(&current_user)?;

    let row = sqlx::query(
        "SELECT display_name, adapter, asset_types, config, priority, enabled FROM providers WHERE id = $1",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or_else(|| {
        AppError::not_found(format!("provider not found: {}", id))
            .with_suggestion("Run `asset-gateway provider list` to confirm provider IDs.")
    })?;

    let current_display_name: String = row.try_get("display_name").map_err(AppError::internal)?;
    let current_adapter: String = row.try_get("adapter").map_err(AppError::internal)?;
    let current_asset_types_raw: String = row.try_get("asset_types").map_err(AppError::internal)?;
    let current_config_raw: String = row.try_get("config").map_err(AppError::internal)?;
    let current_priority: i32 = row.try_get("priority").map_err(AppError::internal)?;
    let current_enabled: bool = row.try_get("enabled").map_err(AppError::internal)?;

    let display_name = req
        .display_name
        .as_deref()
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .map(str::to_string)
        .unwrap_or(current_display_name);
    let adapter = req.adapter.unwrap_or(current_adapter);
    if !is_supported_adapter(&adapter) {
        return Err(AppError::bad_request(format!(
            "unsupported adapter: {}",
            adapter
        )));
    }

    let current_asset_types =
        serde_json::from_str::<Vec<AssetType>>(&current_asset_types_raw).unwrap_or_default();
    let asset_types = req.asset_types.unwrap_or(current_asset_types);

    let current_config = serde_json::from_str::<Value>(&current_config_raw)
        .unwrap_or(Value::Object(Default::default()));
    let config = req.config.map(normalize_config).unwrap_or(current_config);
    let priority = req.priority.unwrap_or(current_priority);
    let enabled = req.enabled.unwrap_or(current_enabled);

    let asset_types_json = serde_json::to_string(&asset_types).map_err(AppError::internal)?;
    let config_json = serde_json::to_string(&config).map_err(AppError::internal)?;

    sqlx::query(
        "UPDATE providers SET display_name = $1, adapter = $2, asset_types = $3, config = $4, priority = $5, enabled = $6, updated_at = now() WHERE id = $7",
    )
    .bind(&display_name)
    .bind(&adapter)
    .bind(asset_types_json)
    .bind(config_json)
    .bind(priority)
    .bind(enabled)
    .bind(&id)
    .execute(&state.db)
    .await
    .map_err(AppError::internal)?;

    let unregistered = state.registry.unregister(&id).await;
    let mut registered = false;
    if enabled {
        register_provider_by_id(&state.db, &state.vault, &state.registry, &id)
            .await
            .map_err(|error| {
                AppError::bad_request(format!(
                    "provider updated but runtime registration failed: {}",
                    error
                ))
                .with_suggestion("Verify credentials/config and rerun provider update.")
            })?;
        registered = true;
    }

    Ok(Json(json!({
        "ok": true,
        "command": "provider.update",
        "data": {
            "id": id,
            "display_name": display_name,
            "adapter": adapter,
            "priority": priority,
            "enabled": enabled,
            "unregistered": unregistered,
            "registered": registered,
        }
    })))
}

async fn delete_provider(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
    Path(id): Path<String>,
) -> AppResult<Json<Value>> {
    require_admin(&current_user)?;

    let deleted = sqlx::query("DELETE FROM providers WHERE id = $1")
        .bind(&id)
        .execute(&state.db)
        .await
        .map_err(AppError::internal)?;

    if deleted.rows_affected() == 0 {
        return Err(AppError::not_found(format!("provider not found: {}", id))
            .with_suggestion("Run `asset-gateway provider list` to confirm provider IDs."));
    }

    let unregistered = state.registry.unregister(&id).await;
    Ok(Json(json!({
        "ok": true,
        "command": "provider.delete",
        "data": {
            "id": id,
            "unregistered": unregistered,
        }
    })))
}

async fn provider_health(
    State(state): State<Arc<ServerState>>,
    _current_user: CurrentUser,
    Path(id): Path<String>,
) -> AppResult<Json<Value>> {
    let provider = state.registry.get(&id).await.ok_or_else(|| {
        AppError::not_found(format!("provider not loaded: {}", id)).with_suggestion(
            "Run `asset-gateway provider list` to verify enabled providers and runtime state.",
        )
    })?;

    let health = provider
        .health_check()
        .await
        .map_err(|error| AppError::provider(error.to_string()))?;

    Ok(Json(json!({
        "ok": true,
        "command": "provider.health",
        "data": {
            "id": id,
            "health": health,
        }
    })))
}

async fn all_provider_health(
    State(state): State<Arc<ServerState>>,
    _current_user: CurrentUser,
) -> AppResult<Json<Value>> {
    let providers = state.registry.list().await;
    let mut health_items = Vec::with_capacity(providers.len());
    for provider in providers {
        let health = provider
            .health_check()
            .await
            .map_err(|error| AppError::provider(error.to_string()))?;
        health_items.push(json!({
            "id": provider.id(),
            "health": health,
        }));
    }

    Ok(Json(json!({
        "ok": true,
        "command": "provider.health.list",
        "data": {
            "providers": health_items,
        }
    })))
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/providers", get(list_providers).post(create_provider))
        .route("/providers/health", get(all_provider_health))
        .route(
            "/providers/{id}",
            put(update_provider).delete(delete_provider),
        )
        .route("/providers/{id}/health", get(provider_health))
}
