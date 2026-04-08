use crate::db;
use crate::server::middleware::{CurrentUser, ensure_admin};
use crate::server::routes;
use crate::server::state::AppState;
use crate::types::{
    AdminJobsQuery, AdminStatsResponse, AdminUpdateUserRequest, AdminUserListQuery,
    AdminUsersResponse, JobListResponse, Pagination, RetryJobResponse, SystemConfigResponse,
    UpdateSystemConfigRequest,
};
use axum::{
    Json, Router,
    extract::{Path, Query, State},
    http::StatusCode,
    routing::{get, patch, post},
};
use std::sync::Arc;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/api/admin/stats", get(get_stats))
        .route("/api/admin/users", get(list_users))
        .route("/api/admin/users/{id}", patch(update_user))
        .route("/api/admin/jobs", get(list_jobs))
        .route("/api/admin/jobs/{id}/retry", post(retry_job))
        .route("/api/admin/config", get(get_config).put(put_config))
}

async fn get_stats(
    State(state): State<Arc<AppState>>,
    current_user: CurrentUser,
) -> Result<Json<AdminStatsResponse>, (StatusCode, String)> {
    ensure_admin(&current_user)?;
    Ok(Json(db::admin_stats(&state.db).await?))
}

async fn list_users(
    State(state): State<Arc<AppState>>,
    current_user: CurrentUser,
    Query(query): Query<AdminUserListQuery>,
) -> Result<Json<AdminUsersResponse>, (StatusCode, String)> {
    ensure_admin(&current_user)?;
    let pagination = Pagination::new(query.page, query.limit);
    let (users, total) =
        db::admin_list_users(&state.db, &pagination, query.search.as_deref()).await?;

    Ok(Json(AdminUsersResponse {
        users,
        total,
        page: pagination.page,
        limit: pagination.limit,
    }))
}

async fn update_user(
    State(state): State<Arc<AppState>>,
    current_user: CurrentUser,
    Path(user_id): Path<String>,
    Json(request): Json<AdminUpdateUserRequest>,
) -> Result<Json<crate::types::User>, (StatusCode, String)> {
    ensure_admin(&current_user)?;
    Ok(Json(
        db::admin_update_user(&state.db, &user_id, &request).await?,
    ))
}

async fn list_jobs(
    State(state): State<Arc<AppState>>,
    current_user: CurrentUser,
    Query(query): Query<AdminJobsQuery>,
) -> Result<Json<JobListResponse>, (StatusCode, String)> {
    ensure_admin(&current_user)?;
    let pagination = Pagination::new(query.page, query.limit);
    let (jobs, total) = db::list_jobs_admin(
        &state.db,
        &pagination,
        query.status.as_deref(),
        query.user_id.as_deref(),
    )
    .await?;

    Ok(Json(JobListResponse {
        jobs,
        total,
        page: pagination.page,
        limit: pagination.limit,
    }))
}

async fn retry_job(
    State(state): State<Arc<AppState>>,
    current_user: CurrentUser,
    Path(job_id): Path<String>,
) -> Result<Json<RetryJobResponse>, (StatusCode, String)> {
    ensure_admin(&current_user)?;
    db::reset_job_for_retry(&state.db, &job_id).await?;
    routes::enqueue_existing_job(state.clone(), job_id.clone()).await?;

    Ok(Json(RetryJobResponse {
        job_id,
        status: "pending".to_string(),
    }))
}

async fn get_config(
    State(state): State<Arc<AppState>>,
    current_user: CurrentUser,
) -> Result<Json<SystemConfigResponse>, (StatusCode, String)> {
    ensure_admin(&current_user)?;
    Ok(Json(SystemConfigResponse {
        items: db::system_config_list(&state.db).await?,
    }))
}

async fn put_config(
    State(state): State<Arc<AppState>>,
    current_user: CurrentUser,
    Json(request): Json<UpdateSystemConfigRequest>,
) -> Result<Json<SystemConfigResponse>, (StatusCode, String)> {
    ensure_admin(&current_user)?;
    db::system_config_put(&state.db, &request.items).await?;
    Ok(Json(SystemConfigResponse {
        items: db::system_config_list(&state.db).await?,
    }))
}
