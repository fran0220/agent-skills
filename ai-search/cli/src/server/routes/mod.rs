pub mod auth;
pub mod config_api;
pub mod health;
pub mod jobs;
pub mod models;
pub mod providers;
pub mod search;

use std::sync::Arc;

use axum::Router;

use crate::server::ServerState;

pub fn api_router() -> Router<Arc<ServerState>> {
    Router::new()
        .merge(search::router())
        .merge(models::router())
        .merge(providers::router())
        .merge(config_api::router())
        .merge(jobs::router())
}
