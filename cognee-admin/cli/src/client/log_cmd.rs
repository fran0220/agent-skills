use serde_json::{json, Value};
use sqlx::PgPool;

use crate::error::AppError;

pub async fn list(pool: &PgPool, limit: i64, offset: i64) -> Result<Value, AppError> {
    let rows = sqlx::query_as::<
        _,
        (
            i64,
            String,
            String,
            i32,
            Option<i64>,
            chrono::DateTime<chrono::Utc>,
        ),
    >(
        r#"
        SELECT id, endpoint, method, status_code, latency_ms, timestamp
        FROM cognee_admin.request_logs
        ORDER BY timestamp DESC
        LIMIT $1 OFFSET $2
        "#,
    )
    .bind(limit)
    .bind(offset)
    .fetch_all(pool)
    .await?;

    let logs: Vec<Value> = rows
        .into_iter()
        .map(
            |(id, endpoint, method, status_code, latency_ms, timestamp)| {
                json!({
                    "id": id,
                    "endpoint": endpoint,
                    "method": method,
                    "status_code": status_code,
                    "latency_ms": latency_ms,
                    "timestamp": timestamp.to_rfc3339(),
                })
            },
        )
        .collect();

    Ok(json!({ "logs": logs, "limit": limit, "offset": offset }))
}

pub async fn stats(pool: &PgPool) -> Result<Value, AppError> {
    let total: (i64,) = sqlx::query_as(
        r#"
        SELECT COUNT(*) FROM cognee_admin.request_logs
        WHERE timestamp > NOW() - INTERVAL '24 hours'
        "#,
    )
    .fetch_one(pool)
    .await?;

    let avg_latency: (Option<f64>,) = sqlx::query_as(
        r#"
        SELECT AVG(latency_ms)::float8 FROM cognee_admin.request_logs
        WHERE timestamp > NOW() - INTERVAL '24 hours'
        "#,
    )
    .fetch_one(pool)
    .await?;

    let by_endpoint = sqlx::query_as::<_, (String, i64)>(
        r#"
        SELECT endpoint, COUNT(*) as count
        FROM cognee_admin.request_logs
        WHERE timestamp > NOW() - INTERVAL '24 hours'
        GROUP BY endpoint
        ORDER BY count DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    let by_status = sqlx::query_as::<_, (i32, i64)>(
        r#"
        SELECT status_code, COUNT(*) as count
        FROM cognee_admin.request_logs
        WHERE timestamp > NOW() - INTERVAL '24 hours'
        GROUP BY status_code
        ORDER BY count DESC
        "#,
    )
    .fetch_all(pool)
    .await?;

    let endpoint_counts: Vec<Value> = by_endpoint
        .into_iter()
        .map(|(endpoint, count)| json!({ "endpoint": endpoint, "count": count }))
        .collect();

    let status_counts: Vec<Value> = by_status
        .into_iter()
        .map(|(status_code, count)| json!({ "status_code": status_code, "count": count }))
        .collect();

    Ok(json!({
        "period": "last_24h",
        "total_requests": total.0,
        "avg_latency_ms": avg_latency.0,
        "by_endpoint": endpoint_counts,
        "by_status_code": status_counts,
    }))
}
