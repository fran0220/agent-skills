use anyhow::Result;
use bcrypt::{DEFAULT_COST, hash};
use rusqlite::{Connection, OptionalExtension, params};
use uuid::Uuid;

const ADMIN_EMAIL: &str = "admin@spriteforge.com";
const ADMIN_PASSWORD: &str = "admin123";

pub fn run_migrations(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        r#"
        PRAGMA foreign_keys = ON;

        CREATE TABLE IF NOT EXISTS users (
            id TEXT PRIMARY KEY,
            email TEXT UNIQUE NOT NULL,
            password_hash TEXT NOT NULL,
            name TEXT NOT NULL,
            role TEXT NOT NULL DEFAULT 'user',
            daily_quota INTEGER NOT NULL DEFAULT 5,
            is_banned INTEGER NOT NULL DEFAULT 0,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS jobs (
            id TEXT PRIMARY KEY,
            user_id TEXT NOT NULL REFERENCES users(id),
            status TEXT NOT NULL DEFAULT 'pending',
            character_description TEXT,
            character_image_url TEXT,
            animation_type TEXT NOT NULL,
            direction TEXT NOT NULL DEFAULT 'right',
            grid_size TEXT NOT NULL DEFAULT '3x3',
            style TEXT,
            remove_background INTEGER NOT NULL DEFAULT 1,
            enhanced_prompt TEXT,
            error_message TEXT,
            output_dir TEXT,
            elapsed_ms INTEGER,
            created_at TEXT NOT NULL DEFAULT (datetime('now')),
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );

        CREATE TABLE IF NOT EXISTS system_config (
            key TEXT PRIMARY KEY,
            value TEXT NOT NULL,
            updated_at TEXT NOT NULL DEFAULT (datetime('now'))
        );
        "#,
    )?;

    seed_admin(conn)?;
    Ok(())
}

fn seed_admin(conn: &Connection) -> Result<()> {
    let existing = conn
        .query_row(
            "SELECT id FROM users WHERE email = ?1",
            params![ADMIN_EMAIL],
            |row| row.get::<_, String>(0),
        )
        .optional()?;

    if existing.is_some() {
        return Ok(());
    }

    let password_hash = hash(ADMIN_PASSWORD, DEFAULT_COST)?;
    conn.execute(
        r#"
        INSERT INTO users (id, email, password_hash, name, role, daily_quota, is_banned)
        VALUES (?1, ?2, ?3, ?4, 'admin', 999999, 0)
        "#,
        params![
            Uuid::new_v4().to_string(),
            ADMIN_EMAIL,
            password_hash,
            "SpriteForge Admin"
        ],
    )?;

    Ok(())
}
