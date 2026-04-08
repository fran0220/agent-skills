use crate::migrations;
use crate::types::{
    AdminStatsResponse, AdminUpdateUserRequest, AnimationType, Job, JobStatus, NewJob, Pagination,
    SystemConfigEntry, SystemConfigValue, User, UserRecord, UserRole,
};
use anyhow::{Result, anyhow};
use axum::http::StatusCode;
use rusqlite::{Connection, OptionalExtension, params};
use std::path::Path;
use std::sync::Arc;
use tokio::sync::Mutex;

pub type Db = Arc<Mutex<Connection>>;

#[derive(Debug)]
pub enum DbError {
    NotFound(String),
    Conflict(String),
    Validation(String),
    Internal(anyhow::Error),
}

impl std::fmt::Display for DbError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotFound(message) => write!(f, "{message}"),
            Self::Conflict(message) => write!(f, "{message}"),
            Self::Validation(message) => write!(f, "{message}"),
            Self::Internal(err) => write!(f, "{err}"),
        }
    }
}

impl std::error::Error for DbError {}

impl From<anyhow::Error> for DbError {
    fn from(value: anyhow::Error) -> Self {
        Self::Internal(value)
    }
}

impl From<rusqlite::Error> for DbError {
    fn from(value: rusqlite::Error) -> Self {
        Self::Internal(value.into())
    }
}

impl From<DbError> for (StatusCode, String) {
    fn from(value: DbError) -> Self {
        match value {
            DbError::NotFound(message) => (StatusCode::NOT_FOUND, message),
            DbError::Conflict(message) => (StatusCode::CONFLICT, message),
            DbError::Validation(message) => (StatusCode::BAD_REQUEST, message),
            DbError::Internal(err) => (StatusCode::INTERNAL_SERVER_ERROR, err.to_string()),
        }
    }
}

pub async fn init_database(path: &str) -> Result<Db> {
    if let Some(parent) = Path::new(path).parent() {
        tokio::fs::create_dir_all(parent).await?;
    }

    let conn = Connection::open(path)?;
    migrations::run_migrations(&conn)?;
    Ok(Arc::new(Mutex::new(conn)))
}

pub async fn create_user(
    db: &Db,
    id: &str,
    email: &str,
    password_hash: &str,
    name: &str,
    role: UserRole,
    daily_quota: i64,
) -> Result<User, DbError> {
    let conn = db.lock().await;
    let result = conn.execute(
        r#"
        INSERT INTO users (id, email, password_hash, name, role, daily_quota, is_banned)
        VALUES (?1, ?2, ?3, ?4, ?5, ?6, 0)
        "#,
        params![id, email, password_hash, name, role.as_str(), daily_quota],
    );

    match result {
        Ok(_) => {}
        Err(err) if err.to_string().contains("UNIQUE constraint failed") => {
            return Err(DbError::Conflict(
                "Email has already been registered".to_string(),
            ));
        }
        Err(err) => return Err(DbError::Internal(err.into())),
    }

    get_user_by_id(db, id)
        .await?
        .ok_or_else(|| DbError::Internal(anyhow!("created user could not be loaded")))
}

pub async fn get_user_by_email(db: &Db, email: &str) -> Result<Option<UserRecord>, DbError> {
    let conn = db.lock().await;
    let user = conn
        .query_row(
            r#"
            SELECT id, email, password_hash, name, role, daily_quota, is_banned, created_at, updated_at
            FROM users
            WHERE email = ?1
            "#,
            params![email],
            |row| {
                Ok(UserRecord {
                    user: User {
                        id: row.get(0)?,
                        email: row.get(1)?,
                        name: row.get(3)?,
                        role: UserRole::from(row.get::<_, String>(4)?),
                        daily_quota: row.get(5)?,
                        is_banned: row.get::<_, i64>(6)? != 0,
                        created_at: row.get(7)?,
                        updated_at: row.get(8)?,
                    },
                    password_hash: row.get(2)?,
                })
            },
        )
        .optional()?;
    Ok(user)
}

pub async fn get_user_by_id(db: &Db, id: &str) -> Result<Option<User>, DbError> {
    let conn = db.lock().await;
    let user = conn
        .query_row(
            r#"
            SELECT id, email, name, role, daily_quota, is_banned, created_at, updated_at
            FROM users
            WHERE id = ?1
            "#,
            params![id],
            |row| {
                Ok(User {
                    id: row.get(0)?,
                    email: row.get(1)?,
                    name: row.get(2)?,
                    role: UserRole::from(row.get::<_, String>(3)?),
                    daily_quota: row.get(4)?,
                    is_banned: row.get::<_, i64>(5)? != 0,
                    created_at: row.get(6)?,
                    updated_at: row.get(7)?,
                })
            },
        )
        .optional()?;
    Ok(user)
}

pub async fn count_jobs_created_today(db: &Db, user_id: &str) -> Result<i64, DbError> {
    let conn = db.lock().await;
    let count = conn.query_row(
        r#"
        SELECT COUNT(*)
        FROM jobs
        WHERE user_id = ?1 AND date(created_at) = date('now')
        "#,
        params![user_id],
        |row| row.get(0),
    )?;
    Ok(count)
}

pub async fn create_job(db: &Db, new_job: &NewJob) -> Result<Job, DbError> {
    let conn = db.lock().await;
    conn.execute(
        r#"
        INSERT INTO jobs (
            id, user_id, status, character_description, character_image_url, animation_type,
            direction, grid_size, style, remove_background
        ) VALUES (?1, ?2, 'pending', ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        "#,
        params![
            new_job.id,
            new_job.user_id,
            new_job.character_description,
            new_job.character_image_url,
            new_job.animation_type.as_str(),
            new_job.direction.as_str(),
            new_job.grid_size.as_str(),
            new_job.style,
            if new_job.remove_background { 1 } else { 0 }
        ],
    )?;
    drop(conn);

    get_job_by_id(db, &new_job.id)
        .await?
        .ok_or_else(|| DbError::Internal(anyhow!("created job could not be loaded")))
}

pub async fn get_job_by_id(db: &Db, id: &str) -> Result<Option<Job>, DbError> {
    let conn = db.lock().await;
    let job = conn
        .query_row(
            r#"
            SELECT id, user_id, status, character_description, character_image_url, animation_type,
                   direction, grid_size, style, remove_background, enhanced_prompt, error_message,
                   output_dir, elapsed_ms, created_at, updated_at
            FROM jobs
            WHERE id = ?1
            "#,
            params![id],
            map_job_row,
        )
        .optional()?;
    Ok(job)
}

pub async fn list_jobs_for_user(
    db: &Db,
    user_id: &str,
    pagination: &Pagination,
    status: Option<&str>,
) -> Result<(Vec<Job>, u64), DbError> {
    let conn = db.lock().await;
    let mut jobs = Vec::new();

    let (query, count_query, params_vec): (&str, &str, Vec<String>) = if let Some(status) = status {
        (
            r#"
            SELECT id, user_id, status, character_description, character_image_url, animation_type,
                   direction, grid_size, style, remove_background, enhanced_prompt, error_message,
                   output_dir, elapsed_ms, created_at, updated_at
            FROM jobs
            WHERE user_id = ?1 AND status = ?2
            ORDER BY datetime(created_at) DESC
            LIMIT ?3 OFFSET ?4
            "#,
            "SELECT COUNT(*) FROM jobs WHERE user_id = ?1 AND status = ?2",
            vec![user_id.to_string(), status.to_string()],
        )
    } else {
        (
            r#"
            SELECT id, user_id, status, character_description, character_image_url, animation_type,
                   direction, grid_size, style, remove_background, enhanced_prompt, error_message,
                   output_dir, elapsed_ms, created_at, updated_at
            FROM jobs
            WHERE user_id = ?1
            ORDER BY datetime(created_at) DESC
            LIMIT ?2 OFFSET ?3
            "#,
            "SELECT COUNT(*) FROM jobs WHERE user_id = ?1",
            vec![user_id.to_string()],
        )
    };

    let mut stmt = conn.prepare(query)?;
    let rows = if params_vec.len() == 2 {
        stmt.query_map(
            params![
                params_vec[0],
                params_vec[1],
                pagination.limit as i64,
                pagination.offset() as i64
            ],
            map_job_row,
        )?
    } else {
        stmt.query_map(
            params![
                params_vec[0],
                pagination.limit as i64,
                pagination.offset() as i64
            ],
            map_job_row,
        )?
    };

    for row in rows {
        jobs.push(row?);
    }

    let total: i64 = if params_vec.len() == 2 {
        conn.query_row(count_query, params![params_vec[0], params_vec[1]], |row| {
            row.get(0)
        })?
    } else {
        conn.query_row(count_query, params![params_vec[0]], |row| row.get(0))?
    };

    Ok((jobs, total as u64))
}

pub async fn list_jobs_admin(
    db: &Db,
    pagination: &Pagination,
    status: Option<&str>,
    user_id: Option<&str>,
) -> Result<(Vec<Job>, u64), DbError> {
    let conn = db.lock().await;
    let mut jobs = Vec::new();

    let mut where_parts = Vec::new();
    let mut bind_values = Vec::new();

    if let Some(status) = status {
        where_parts.push("status = ?");
        bind_values.push(status.to_string());
    }

    if let Some(user_id) = user_id {
        where_parts.push("user_id = ?");
        bind_values.push(user_id.to_string());
    }

    let where_clause = if where_parts.is_empty() {
        String::new()
    } else {
        format!("WHERE {}", where_parts.join(" AND "))
    };

    let sql = format!(
        r#"
        SELECT id, user_id, status, character_description, character_image_url, animation_type,
               direction, grid_size, style, remove_background, enhanced_prompt, error_message,
               output_dir, elapsed_ms, created_at, updated_at
        FROM jobs
        {where_clause}
        ORDER BY datetime(created_at) DESC
        LIMIT ? OFFSET ?
        "#
    );
    let count_sql = format!("SELECT COUNT(*) FROM jobs {where_clause}");

    let mut stmt = conn.prepare(&sql)?;
    let rows = match bind_values.as_slice() {
        [] => stmt.query_map(
            params![pagination.limit as i64, pagination.offset() as i64],
            map_job_row,
        )?,
        [status] => stmt.query_map(
            params![status, pagination.limit as i64, pagination.offset() as i64],
            map_job_row,
        )?,
        [status, user_id] => stmt.query_map(
            params![
                status,
                user_id,
                pagination.limit as i64,
                pagination.offset() as i64
            ],
            map_job_row,
        )?,
        _ => {
            return Err(DbError::Internal(anyhow!(
                "unsupported admin job filter state"
            )));
        }
    };

    for row in rows {
        jobs.push(row?);
    }

    let total: i64 = match bind_values.as_slice() {
        [] => conn.query_row(&count_sql, [], |row| row.get(0))?,
        [status] => conn.query_row(&count_sql, params![status], |row| row.get(0))?,
        [status, user_id] => {
            conn.query_row(&count_sql, params![status, user_id], |row| row.get(0))?
        }
        _ => {
            return Err(DbError::Internal(anyhow!(
                "unsupported admin job filter state"
            )));
        }
    };

    Ok((jobs, total as u64))
}

pub async fn update_job_status(
    db: &Db,
    id: &str,
    status: JobStatus,
    enhanced_prompt: Option<&str>,
    error_message: Option<&str>,
    output_dir: Option<&str>,
    elapsed_ms: Option<i64>,
) -> Result<(), DbError> {
    let conn = db.lock().await;
    let affected = conn.execute(
        r#"
        UPDATE jobs
        SET status = ?2,
            enhanced_prompt = COALESCE(?3, enhanced_prompt),
            error_message = ?4,
            output_dir = COALESCE(?5, output_dir),
            elapsed_ms = COALESCE(?6, elapsed_ms),
            updated_at = datetime('now')
        WHERE id = ?1
        "#,
        params![
            id,
            status.as_str(),
            enhanced_prompt,
            error_message,
            output_dir,
            elapsed_ms
        ],
    )?;

    if affected == 0 {
        return Err(DbError::NotFound(format!("Job {id} was not found")));
    }

    Ok(())
}

pub async fn delete_job(db: &Db, id: &str) -> Result<(), DbError> {
    let conn = db.lock().await;
    let affected = conn.execute("DELETE FROM jobs WHERE id = ?1", params![id])?;
    if affected == 0 {
        return Err(DbError::NotFound(format!("Job {id} was not found")));
    }
    Ok(())
}

pub async fn admin_stats(db: &Db) -> Result<AdminStatsResponse, DbError> {
    let conn = db.lock().await;
    let total_users: i64 = conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?;
    let active_users_today: i64 = conn.query_row(
        "SELECT COUNT(DISTINCT user_id) FROM jobs WHERE date(created_at) = date('now')",
        [],
        |row| row.get(0),
    )?;
    let total_jobs: i64 = conn.query_row("SELECT COUNT(*) FROM jobs", [], |row| row.get(0))?;
    let pending_jobs: i64 = conn.query_row(
        "SELECT COUNT(*) FROM jobs WHERE status = 'pending'",
        [],
        |row| row.get(0),
    )?;
    let completed_jobs: i64 = conn.query_row(
        "SELECT COUNT(*) FROM jobs WHERE status = 'completed'",
        [],
        |row| row.get(0),
    )?;
    let failed_jobs: i64 = conn.query_row(
        "SELECT COUNT(*) FROM jobs WHERE status = 'failed'",
        [],
        |row| row.get(0),
    )?;

    Ok(AdminStatsResponse {
        total_users: total_users as u64,
        active_users_today: active_users_today as u64,
        total_jobs: total_jobs as u64,
        pending_jobs: pending_jobs as u64,
        completed_jobs: completed_jobs as u64,
        failed_jobs: failed_jobs as u64,
    })
}

pub async fn admin_list_users(
    db: &Db,
    pagination: &Pagination,
    search: Option<&str>,
) -> Result<(Vec<User>, u64), DbError> {
    let conn = db.lock().await;
    let mut users = Vec::new();
    let search_term = search.map(|value| format!("%{}%", value.trim()));

    let query = if search_term.is_some() {
        r#"
        SELECT id, email, name, role, daily_quota, is_banned, created_at, updated_at
        FROM users
        WHERE email LIKE ?1 OR name LIKE ?1
        ORDER BY datetime(created_at) DESC
        LIMIT ?2 OFFSET ?3
        "#
    } else {
        r#"
        SELECT id, email, name, role, daily_quota, is_banned, created_at, updated_at
        FROM users
        ORDER BY datetime(created_at) DESC
        LIMIT ?1 OFFSET ?2
        "#
    };

    let mut stmt = conn.prepare(query)?;
    let rows = if let Some(search_term) = &search_term {
        stmt.query_map(
            params![
                search_term,
                pagination.limit as i64,
                pagination.offset() as i64
            ],
            map_user_row,
        )?
    } else {
        stmt.query_map(
            params![pagination.limit as i64, pagination.offset() as i64],
            map_user_row,
        )?
    };

    for row in rows {
        users.push(row?);
    }

    let total: i64 = if let Some(search_term) = &search_term {
        conn.query_row(
            "SELECT COUNT(*) FROM users WHERE email LIKE ?1 OR name LIKE ?1",
            params![search_term],
            |row| row.get(0),
        )?
    } else {
        conn.query_row("SELECT COUNT(*) FROM users", [], |row| row.get(0))?
    };

    Ok((users, total as u64))
}

pub async fn admin_update_user(
    db: &Db,
    user_id: &str,
    request: &AdminUpdateUserRequest,
) -> Result<User, DbError> {
    let existing = get_user_by_id(db, user_id)
        .await?
        .ok_or_else(|| DbError::NotFound(format!("User {user_id} was not found")))?;

    let name = request
        .name
        .as_deref()
        .unwrap_or(existing.name.as_str())
        .trim()
        .to_string();
    if name.is_empty() {
        return Err(DbError::Validation("User name cannot be empty".to_string()));
    }

    let role = request.role.clone().unwrap_or(existing.role);
    let daily_quota = request.daily_quota.unwrap_or(existing.daily_quota);
    if daily_quota < 0 {
        return Err(DbError::Validation(
            "daily_quota must be non-negative".to_string(),
        ));
    }

    let is_banned = request.is_banned.unwrap_or(existing.is_banned);

    let conn = db.lock().await;
    conn.execute(
        r#"
        UPDATE users
        SET name = ?2, role = ?3, daily_quota = ?4, is_banned = ?5, updated_at = datetime('now')
        WHERE id = ?1
        "#,
        params![
            user_id,
            name,
            role.as_str(),
            daily_quota,
            if is_banned { 1 } else { 0 }
        ],
    )?;
    drop(conn);

    get_user_by_id(db, user_id)
        .await?
        .ok_or_else(|| DbError::NotFound(format!("User {user_id} was not found")))
}

pub async fn system_config_list(db: &Db) -> Result<Vec<SystemConfigEntry>, DbError> {
    let conn = db.lock().await;
    let mut stmt =
        conn.prepare("SELECT key, value, updated_at FROM system_config ORDER BY key ASC")?;
    let rows = stmt.query_map([], |row| {
        Ok(SystemConfigEntry {
            key: row.get(0)?,
            value: row.get(1)?,
            updated_at: row.get(2)?,
        })
    })?;

    let mut items = Vec::new();
    for row in rows {
        items.push(row?);
    }
    Ok(items)
}

pub async fn system_config_put(db: &Db, items: &[SystemConfigValue]) -> Result<(), DbError> {
    let mut conn = db.lock().await;
    let tx = conn.transaction()?;
    for item in items {
        tx.execute(
            r#"
            INSERT INTO system_config (key, value, updated_at)
            VALUES (?1, ?2, datetime('now'))
            ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = datetime('now')
            "#,
            params![item.key, item.value],
        )?;
    }
    tx.commit()?;
    Ok(())
}

pub async fn reset_job_for_retry(db: &Db, id: &str) -> Result<Job, DbError> {
    let conn = db.lock().await;
    let affected = conn.execute(
        r#"
        UPDATE jobs
        SET status = 'pending',
            enhanced_prompt = NULL,
            error_message = NULL,
            output_dir = NULL,
            elapsed_ms = NULL,
            updated_at = datetime('now')
        WHERE id = ?1
        "#,
        params![id],
    )?;
    drop(conn);
    if affected == 0 {
        return Err(DbError::NotFound(format!("Job {id} was not found")));
    }
    get_job_by_id(db, id)
        .await?
        .ok_or_else(|| DbError::NotFound(format!("Job {id} was not found")))
}

fn map_user_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<User> {
    Ok(User {
        id: row.get(0)?,
        email: row.get(1)?,
        name: row.get(2)?,
        role: UserRole::from(row.get::<_, String>(3)?),
        daily_quota: row.get(4)?,
        is_banned: row.get::<_, i64>(5)? != 0,
        created_at: row.get(6)?,
        updated_at: row.get(7)?,
    })
}

fn map_job_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<Job> {
    Ok(Job {
        id: row.get(0)?,
        user_id: row.get(1)?,
        status: JobStatus::from(row.get::<_, String>(2)?),
        character_description: row.get(3)?,
        character_image_url: row.get(4)?,
        animation_type: AnimationType::from(row.get::<_, String>(5)?),
        direction: crate::types::Direction::from(row.get::<_, String>(6)?),
        grid_size: crate::types::GridSize::from(row.get::<_, String>(7)?),
        style: row.get(8)?,
        remove_background: row.get::<_, i64>(9)? != 0,
        enhanced_prompt: row.get(10)?,
        error_message: row.get(11)?,
        output_dir: row.get(12)?,
        elapsed_ms: row.get(13)?,
        created_at: row.get(14)?,
        updated_at: row.get(15)?,
    })
}
