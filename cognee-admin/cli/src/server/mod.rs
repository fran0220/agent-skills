use anyhow::Context;
use axum::middleware as axum_mw;
use axum::Router;
use sqlx::PgPool;
use std::sync::Arc;
use tower_http::cors::CorsLayer;
use tower_http::trace::TraceLayer;

use crate::cognee_client::CogneeClient;

pub mod middleware;
pub mod routes;

pub struct AppState {
    pub client: CogneeClient,
    pub pool: PgPool,
}

pub async fn run(host: &str, port: u16, cognee_url: &str, pool: PgPool) -> anyhow::Result<()> {
    let service_jwt = std::env::var("COGNEE_SERVICE_JWT")
        .ok()
        .filter(|value| !value.trim().is_empty())
        .context("COGNEE_SERVICE_JWT is required for web server upstream Cognee API access")?;
    let client = CogneeClient::new(cognee_url)
        .with_token(service_jwt)
        .with_pool(pool.clone());
    let state = Arc::new(AppState { client, pool });

    let app = Router::new()
        .merge(routes::ui_routes())
        .merge(routes::api_routes())
        .layer(axum_mw::from_fn_with_state(
            state.clone(),
            middleware::auth_middleware,
        ))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("{}:{}", host, port);
    tracing::info!("Web panel starting at http://{}", addr);
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}
