use sqlx::postgres::PgPoolOptions;
use sqlx::PgPool;

pub async fn connect(database_url: &str) -> anyhow::Result<PgPool> {
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(database_url)
        .await?;
    Ok(pool)
}

pub async fn run_migrations(pool: &PgPool) -> anyhow::Result<()> {
    let m1 = include_str!("../migrations/001_create_admin_schema.sql");
    sqlx::raw_sql(m1).execute(pool).await?;
    let m2 = include_str!("../migrations/002_create_api_tokens.sql");
    sqlx::raw_sql(m2).execute(pool).await?;
    Ok(())
}
