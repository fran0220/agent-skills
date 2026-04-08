use crate::db;
use crate::pipeline;
use crate::server::{admin, auth, middleware, state::AppState, ws};
use crate::types::{
    CreateJobRequest, CreateJobResponse, Job, JobListQuery, JobListResponse, JobProgress,
    JobStatus, Pagination, SpriteResponse,
};
use axum::{
    Json, Router,
    body::Body,
    extract::{Path, Query, State},
    http::{HeaderValue, Response, StatusCode, header},
    middleware::from_fn_with_state,
    response::IntoResponse,
    routing::{get, post},
};
use std::path::{Path as FsPath, PathBuf};
use std::sync::Arc;
use std::time::Instant;
use tokio::fs;
use tracing::{error, info};
use uuid::Uuid;

pub fn router(state: Arc<AppState>) -> Router {
    let protected = Router::new()
        .route("/api/auth/me", get(auth::me))
        .route("/api/jobs", post(create_job).get(list_jobs))
        .route("/api/jobs/{id}", get(get_job).delete(delete_job))
        .merge(admin::router())
        .layer(from_fn_with_state(state.clone(), middleware::require_auth));

    Router::new()
        .route("/api/health", get(health))
        .route("/api/auth/register", post(auth::register))
        .route("/api/auth/login", post(auth::login))
        .route("/ws/jobs/{id}", get(ws::jobs_ws))
        .route("/assets/{job_id}/{filename}", get(get_asset))
        .merge(protected)
        .with_state(state)
}

async fn create_job(
    State(state): State<Arc<AppState>>,
    current_user: middleware::CurrentUser,
    Json(request): Json<CreateJobRequest>,
) -> Result<Json<CreateJobResponse>, (StatusCode, String)> {
    let user = db::get_user_by_id(&state.db, &current_user.claims.sub)
        .await?
        .ok_or_else(|| (StatusCode::UNAUTHORIZED, "User not found".to_string()))?;

    if user.is_banned {
        return Err((StatusCode::FORBIDDEN, "User account is banned".to_string()));
    }

    let used_today = db::count_jobs_created_today(&state.db, &user.id).await?;
    if used_today >= user.daily_quota {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            "Daily quota exceeded".to_string(),
        ));
    }

    let job_id = Uuid::new_v4().to_string();
    let new_job = request.into_job_insert(user.id.clone(), job_id.clone());
    db::create_job(&state.db, &new_job).await?;

    enqueue_existing_job(state.clone(), job_id.clone()).await?;

    Ok(Json(CreateJobResponse {
        job_id,
        status: "pending".to_string(),
    }))
}

async fn list_jobs(
    State(state): State<Arc<AppState>>,
    current_user: middleware::CurrentUser,
    Query(query): Query<JobListQuery>,
) -> Result<Json<JobListResponse>, (StatusCode, String)> {
    let pagination = Pagination::new(query.page, query.limit);
    let (jobs, total) = db::list_jobs_for_user(
        &state.db,
        &current_user.claims.sub,
        &pagination,
        query.status.as_deref(),
    )
    .await?;

    Ok(Json(JobListResponse {
        jobs,
        total,
        page: pagination.page,
        limit: pagination.limit,
    }))
}

async fn get_job(
    State(state): State<Arc<AppState>>,
    current_user: middleware::CurrentUser,
    Path(job_id): Path<String>,
) -> Result<Json<Job>, (StatusCode, String)> {
    let job = db::get_job_by_id(&state.db, &job_id)
        .await?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Job {job_id} was not found")))?;

    if job.user_id != current_user.claims.sub && !current_user.claims.is_admin() {
        return Err((StatusCode::FORBIDDEN, "Access denied".to_string()));
    }

    Ok(Json(job))
}

async fn delete_job(
    State(state): State<Arc<AppState>>,
    current_user: middleware::CurrentUser,
    Path(job_id): Path<String>,
) -> Result<StatusCode, (StatusCode, String)> {
    let job = db::get_job_by_id(&state.db, &job_id)
        .await?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Job {job_id} was not found")))?;

    if job.user_id != current_user.claims.sub && !current_user.claims.is_admin() {
        return Err((StatusCode::FORBIDDEN, "Access denied".to_string()));
    }

    if let Some(output_dir) = &job.output_dir {
        let _ = fs::remove_dir_all(output_dir).await;
    }

    db::delete_job(&state.db, &job_id).await?;
    state.job_channels.remove(&job_id);

    Ok(StatusCode::NO_CONTENT)
}

pub async fn enqueue_existing_job(
    state: Arc<AppState>,
    job_id: String,
) -> Result<(), (StatusCode, String)> {
    let job = db::get_job_by_id(&state.db, &job_id)
        .await?
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Job {job_id} was not found")))?;

    let sender = state.job_sender(&job_id);
    tokio::spawn(run_job(state, job, sender));

    Ok(())
}

async fn run_job(
    state: Arc<AppState>,
    job: Job,
    sender: tokio::sync::broadcast::Sender<JobProgress>,
) {
    let start = Instant::now();
    let request = job.to_sprite_request();

    publish_progress(&sender, &job.id, JobStatus::Pending, 0, "Job queued");
    if let Err(err) = db::update_job_status(
        &state.db,
        &job.id,
        JobStatus::PromptEnhancing,
        None,
        None,
        None,
        None,
    )
    .await
    {
        error!(job_id = job.id, error = %err, "failed to mark job as prompt enhancing");
        return;
    }
    publish_progress(
        &sender,
        &job.id,
        JobStatus::PromptEnhancing,
        10,
        "Enhancing prompt",
    );

    let result = pipeline::run_pipeline(&state.llm, &state.image_client, &request).await;
    match result {
        Ok(result) => {
            publish_progress(
                &sender,
                &job.id,
                JobStatus::Processing,
                85,
                "Writing output files",
            );
            let job_dir = FsPath::new(&state.config.output_dir).join(&job.id);
            match persist_pipeline_output(&job_dir, &result).await {
                Ok(response) => {
                    let elapsed_ms = start.elapsed().as_millis() as i64;
                    if let Err(err) = db::update_job_status(
                        &state.db,
                        &job.id,
                        JobStatus::Completed,
                        Some(&result.enhanced_prompt),
                        None,
                        Some(&job_dir.display().to_string()),
                        Some(elapsed_ms),
                    )
                    .await
                    {
                        error!(job_id = job.id, error = %err, "failed to mark job as completed");
                        return;
                    }

                    publish_progress(&sender, &job.id, JobStatus::Completed, 100, "Job completed");
                    info!(
                        job_id = job.id,
                        elapsed_ms,
                        sprite_sheet = ?response.sprite_sheet_path,
                        "job completed"
                    );
                }
                Err(err) => {
                    let _ = db::update_job_status(
                        &state.db,
                        &job.id,
                        JobStatus::Failed,
                        Some(&result.enhanced_prompt),
                        Some(&err),
                        Some(&job_dir.display().to_string()),
                        Some(start.elapsed().as_millis() as i64),
                    )
                    .await;
                    publish_progress(&sender, &job.id, JobStatus::Failed, 100, &err);
                }
            }
        }
        Err(err) => {
            let error_message = format!("Pipeline failed: {err:#}");
            let _ = db::update_job_status(
                &state.db,
                &job.id,
                JobStatus::Failed,
                None,
                Some(&error_message),
                None,
                Some(start.elapsed().as_millis() as i64),
            )
            .await;
            publish_progress(&sender, &job.id, JobStatus::Failed, 100, &error_message);
            error!(job_id = job.id, error = %error_message, "job failed");
        }
    }
}

async fn persist_pipeline_output(
    job_dir: &FsPath,
    result: &pipeline::PipelineResult,
) -> Result<SpriteResponse, String> {
    fs::create_dir_all(job_dir).await.map_err(|err| {
        format!(
            "Failed to create output directory {}: {err}",
            job_dir.display()
        )
    })?;

    let grid_path = job_dir.join("grid.png");
    fs::write(&grid_path, &result.grid_image)
        .await
        .map_err(|err| format!("Failed to write grid image {}: {err}", grid_path.display()))?;

    let sprite_sheet_path = job_dir.join("sprite_sheet.png");
    fs::write(&sprite_sheet_path, &result.sprite_sheet)
        .await
        .map_err(|err| {
            format!(
                "Failed to write sprite sheet {}: {err}",
                sprite_sheet_path.display()
            )
        })?;

    let mut individual_frames = Vec::with_capacity(result.frames.len());
    for (index, frame) in result.frames.iter().enumerate() {
        let frame_path = job_dir.join(format!("frame_{index:02}.png"));
        fs::write(&frame_path, frame)
            .await
            .map_err(|err| format!("Failed to write frame {}: {err}", frame_path.display()))?;
        individual_frames.push(frame_path.display().to_string());
    }

    let prompt_path = job_dir.join("enhanced_prompt.txt");
    fs::write(&prompt_path, result.enhanced_prompt.as_bytes())
        .await
        .map_err(|err| {
            format!(
                "Failed to write enhanced prompt {}: {err}",
                prompt_path.display()
            )
        })?;

    Ok(SpriteResponse {
        job_id: job_dir
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or_default()
            .to_string(),
        status: JobStatus::Completed,
        sprite_sheet_path: Some(sprite_sheet_path.display().to_string()),
        individual_frames,
        grid_image_path: Some(grid_path.display().to_string()),
        enhanced_prompt: Some(result.enhanced_prompt.clone()),
        elapsed_ms: 0,
    })
}

async fn get_asset(
    State(state): State<Arc<AppState>>,
    Path((job_id, filename)): Path<(String, String)>,
) -> Result<Response<Body>, (StatusCode, String)> {
    if std::path::Path::new(&filename).components().count() != 1 {
        return Err((StatusCode::BAD_REQUEST, "Invalid asset path".to_string()));
    }

    let asset_path = PathBuf::from(&state.config.output_dir)
        .join(job_id)
        .join(filename);
    let bytes = fs::read(&asset_path)
        .await
        .map_err(|_| (StatusCode::NOT_FOUND, "Asset not found".to_string()))?;
    let mime = mime_guess::from_path(&asset_path).first_or_octet_stream();

    let mut response = Response::new(Body::from(bytes));
    response.headers_mut().insert(
        header::CONTENT_TYPE,
        HeaderValue::from_str(mime.as_ref())
            .unwrap_or_else(|_| HeaderValue::from_static("application/octet-stream")),
    );
    Ok(response)
}

async fn health() -> impl IntoResponse {
    Json(serde_json::json!({ "status": "ok" }))
}

fn publish_progress(
    sender: &tokio::sync::broadcast::Sender<JobProgress>,
    job_id: &str,
    status: JobStatus,
    progress_pct: u8,
    message: &str,
) {
    let _ = sender.send(JobProgress {
        job_id: job_id.to_string(),
        status: status.as_str().to_string(),
        progress_pct,
        message: message.to_string(),
    });
}
