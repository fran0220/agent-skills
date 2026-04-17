pub mod api_keys;
pub mod assets;
pub mod auth;
pub mod config_api;
pub mod gallery;
pub mod generate;
pub mod generate_batch;
pub mod health;
pub mod jobs;
pub mod plans;
pub mod process;
pub mod process3d;
pub mod providers;
pub mod tripo;
pub mod users;

use crate::server::ServerState;
use axum::{routing::get, Router};
use std::sync::Arc;

pub fn api_router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/health", get(health::health_check))
        .merge(api_keys::router())
        .merge(config_api::router())
        .merge(gallery::router())
        .merge(generate::router())
        .merge(generate_batch::router())
        .merge(process::router())
        .merge(process3d::router())
        .merge(plans::router())
        .merge(providers::router())
        .merge(jobs::router())
        .merge(users::router())
        .merge(assets::router())
        .merge(tripo::router())
}

pub fn auth_router() -> Router<Arc<ServerState>> {
    auth::router()
}
