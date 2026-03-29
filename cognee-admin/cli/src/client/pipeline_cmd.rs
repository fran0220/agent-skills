use serde_json::{json, Value};
use sqlx::PgPool;

use crate::error::AppError;

pub async fn list(pool: &PgPool, limit: i64) -> Result<Value, AppError> {
    let rows = sqlx::query_as::<
        _,
        (
            String,
            String,
            Option<String>,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
        ),
    >(
        r#"
        SELECT id::text, status, error,
               created_at AT TIME ZONE 'UTC' as created_at,
               updated_at AT TIME ZONE 'UTC' as updated_at
        FROM public.pipeline_runs
        ORDER BY created_at DESC
        LIMIT $1
        "#,
    )
    .bind(limit)
    .fetch_all(pool)
    .await?;

    let runs: Vec<Value> = rows
        .into_iter()
        .map(|(id, status, error, created_at, updated_at)| {
            json!({
                "id": id,
                "status": status,
                "error": error,
                "created_at": created_at.map(|t| t.to_rfc3339()),
                "updated_at": updated_at.map(|t| t.to_rfc3339()),
            })
        })
        .collect();

    Ok(json!({ "pipeline_runs": runs }))
}

pub async fn detail(pool: &PgPool, id: &str) -> Result<Value, AppError> {
    let row = sqlx::query_as::<
        _,
        (
            String,
            String,
            Option<String>,
            Option<Value>,
            Option<chrono::DateTime<chrono::Utc>>,
            Option<chrono::DateTime<chrono::Utc>>,
        ),
    >(
        r#"
        SELECT id::text, status, error, metadata,
               created_at AT TIME ZONE 'UTC' as created_at,
               updated_at AT TIME ZONE 'UTC' as updated_at
        FROM public.pipeline_runs
        WHERE id::text = $1
        "#,
    )
    .bind(id)
    .fetch_optional(pool)
    .await?;

    match row {
        Some((id, status, error, metadata, created_at, updated_at)) => Ok(json!({
            "id": id,
            "status": status,
            "error": error,
            "metadata": metadata,
            "created_at": created_at.map(|t| t.to_rfc3339()),
            "updated_at": updated_at.map(|t| t.to_rfc3339()),
        })),
        None => Err(AppError::NotFound(format!("Pipeline run '{id}' not found"))),
    }
}
