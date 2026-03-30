pub mod assets;
pub mod auth;
pub mod config_api;
pub mod generate;
pub mod health;
pub mod jobs;
pub mod process;
pub mod process3d;
pub mod providers;
pub mod users;

use crate::server::ServerState;
use axum::{routing::get, Router};
use std::sync::Arc;

pub fn api_router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/health", get(health::health_check))
        .merge(config_api::router())
        .merge(generate::router())
        .merge(process::router())
        .merge(process3d::router())
        .merge(providers::router())
        .merge(jobs::router())
        .merge(users::router())
        .merge(assets::router())
}

pub fn auth_router() -> Router<Arc<ServerState>> {
    auth::router()
}
