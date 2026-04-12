use axum::{
    extract::{Path, Query, State},
    routing::{delete, get, post},
    Json, Router,
};
use chrono::{DateTime, Utc};
use serde::Deserialize;
use serde_json::{json, Value};
use sqlx::Row;
use std::sync::Arc;
use uuid::Uuid;

use crate::error::{AppError, AppResult};
use crate::server::routes::auth::CurrentUser;
use crate::server::ServerState;

#[derive(Deserialize)]
struct PublishGalleryReq {
    job_id: String,
    title: Option<String>,
    is_public: Option<bool>,
}

#[derive(Deserialize)]
struct ListGalleryQuery {
    asset_type: Option<String>,
    limit: Option<u32>,
    offset: Option<u32>,
}

async fn publish_job(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
    Json(req): Json<PublishGalleryReq>,
) -> AppResult<Json<Value>> {
    let job_id = req.job_id.trim().to_string();
    if job_id.is_empty() {
        return Err(AppError::bad_request("job_id cannot be empty"));
    }

    let row = if current_user.is_admin() {
        sqlx::query("SELECT id, user_id, asset_type, status FROM jobs WHERE id = $1")
            .bind(&job_id)
            .fetch_optional(&state.db)
            .await
    } else {
        sqlx::query(
            "SELECT id, user_id, asset_type, status FROM jobs WHERE id = $1 AND user_id = $2",
        )
        .bind(&job_id)
        .bind(&current_user.id)
        .fetch_optional(&state.db)
        .await
    }
    .map_err(AppError::internal)?
    .ok_or_else(|| AppError::not_found(format!("completed job not found: {}", job_id)))?;

    let status: String = row.try_get("status").map_err(AppError::internal)?;
    if !status.eq_ignore_ascii_case("completed") {
        return Err(AppError::conflict(
            "only completed jobs can be published to the gallery",
        ));
    }

    let owner_id: String = row.try_get("user_id").map_err(AppError::internal)?;
    let asset_type: String = row.try_get("asset_type").map_err(AppError::internal)?;
    let title = req
        .title
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("Untitled")
        .to_string();
    let gallery_id = Uuid::new_v4().to_string();

    let insert_result = sqlx::query(
        "INSERT INTO gallery (id, user_id, job_id, asset_type, title, is_public) VALUES ($1, $2, $3, $4, $5, $6)",
    )
    .bind(&gallery_id)
    .bind(&owner_id)
    .bind(&job_id)
    .bind(&asset_type)
    .bind(&title)
    .bind(req.is_public.unwrap_or(true))
    .execute(&state.db)
    .await;

    if let Err(error) = insert_result {
        if let sqlx::Error::Database(db_error) = &error {
            if db_error.is_unique_violation() {
                return Err(AppError::conflict(
                    "job is already published to the gallery",
                ));
            }
        }
        return Err(AppError::internal(error));
    }

    Ok(Json(json!({
        "ok": true,
        "command": "gallery.publish",
        "data": {
            "id": gallery_id,
            "job_id": job_id,
            "user_id": owner_id,
            "asset_type": asset_type,
            "title": title,
            "is_public": req.is_public.unwrap_or(true),
        }
    })))
}

async fn list_gallery(
    State(state): State<Arc<ServerState>>,
    Query(query): Query<ListGalleryQuery>,
) -> AppResult<Json<Value>> {
    let limit = query.limit.unwrap_or(20).clamp(1, 100) as i64;
    let offset = query.offset.unwrap_or(0) as i64;

    let rows = if let Some(asset_type) = query.asset_type.as_deref() {
        sqlx::query(
            "SELECT g.id, g.user_id, g.job_id, g.asset_type, g.title, g.is_public, g.likes_count, g.created_at, u.username, u.avatar_url, j.output_path, j.response FROM gallery g JOIN users u ON u.id = g.user_id LEFT JOIN jobs j ON j.id = g.job_id WHERE g.is_public = TRUE AND g.asset_type = $1 ORDER BY g.created_at DESC LIMIT $2 OFFSET $3",
        )
        .bind(asset_type)
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await
        .map_err(AppError::internal)?
    } else {
        sqlx::query(
            "SELECT g.id, g.user_id, g.job_id, g.asset_type, g.title, g.is_public, g.likes_count, g.created_at, u.username, u.avatar_url, j.output_path, j.response FROM gallery g JOIN users u ON u.id = g.user_id LEFT JOIN jobs j ON j.id = g.job_id WHERE g.is_public = TRUE ORDER BY g.created_at DESC LIMIT $1 OFFSET $2",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&state.db)
        .await
        .map_err(AppError::internal)?
    };

    let mut items = Vec::with_capacity(rows.len());
    for row in rows {
        let response_text: Option<String> = row.try_get("response").map_err(AppError::internal)?;
        let response_json = response_text
            .as_deref()
            .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
            .or_else(|| response_text.map(Value::String));

        items.push(json!({
            "id": row.try_get::<String, _>("id").map_err(AppError::internal)?,
            "user_id": row.try_get::<String, _>("user_id").map_err(AppError::internal)?,
            "job_id": row.try_get::<String, _>("job_id").map_err(AppError::internal)?,
            "asset_type": row.try_get::<String, _>("asset_type").map_err(AppError::internal)?,
            "title": row.try_get::<String, _>("title").map_err(AppError::internal)?,
            "likes_count": row.try_get::<i64, _>("likes_count").map_err(AppError::internal)?,
            "created_at": row.try_get::<DateTime<Utc>, _>("created_at").map_err(AppError::internal)?.to_rfc3339(),
            "author": {
                "username": row.try_get::<String, _>("username").map_err(AppError::internal)?,
                "avatar_url": row.try_get::<Option<String>, _>("avatar_url").map_err(AppError::internal)?,
            },
            "output_path": row.try_get::<Option<String>, _>("output_path").map_err(AppError::internal)?,
            "response": response_json,
        }));
    }

    Ok(Json(json!({
        "ok": true,
        "command": "gallery.list",
        "data": {
            "items": items,
            "limit": limit,
            "offset": offset,
        }
    })))
}

async fn like_item(
    State(state): State<Arc<ServerState>>,
    _current_user: CurrentUser,
    Path(id): Path<String>,
) -> AppResult<Json<Value>> {
    let row = sqlx::query(
        "UPDATE gallery SET likes_count = likes_count + 1 WHERE id = $1 AND is_public = TRUE RETURNING likes_count",
    )
    .bind(&id)
    .fetch_optional(&state.db)
    .await
    .map_err(AppError::internal)?
    .ok_or_else(|| AppError::not_found(format!("gallery item not found: {}", id)))?;

    Ok(Json(json!({
        "ok": true,
        "command": "gallery.like",
        "data": {
            "id": id,
            "likes_count": row.try_get::<i64, _>("likes_count").map_err(AppError::internal)?,
        }
    })))
}

async fn delete_item(
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
    Path(id): Path<String>,
) -> AppResult<Json<Value>> {
    let result = if current_user.is_admin() {
        sqlx::query("DELETE FROM gallery WHERE id = $1")
            .bind(&id)
            .execute(&state.db)
            .await
    } else {
        sqlx::query("DELETE FROM gallery WHERE id = $1 AND user_id = $2")
            .bind(&id)
            .bind(&current_user.id)
            .execute(&state.db)
            .await
    }
    .map_err(AppError::internal)?;

    if result.rows_affected() == 0 {
        return Err(AppError::not_found(format!(
            "gallery item not found: {}",
            id
        )));
    }

    Ok(Json(json!({
        "ok": true,
        "command": "gallery.delete",
        "data": {
            "id": id,
            "deleted": true,
        }
    })))
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/gallery", get(list_gallery).post(publish_job))
        .route("/gallery/{id}/like", post(like_item))
        .route("/gallery/{id}", delete(delete_item))
}
