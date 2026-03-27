pub mod auth;
pub mod credentials;
pub mod generate;
pub mod health;
pub mod jobs;
pub mod providers;
pub mod users;

use crate::server::ServerState;
use axum::{routing::get, Router};
use std::sync::Arc;

pub fn api_router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/health", get(health::health_check))
        .merge(generate::router())
        .merge(providers::router())
        .merge(credentials::router())
        .merge(jobs::router())
        .merge(users::router())
}

pub fn auth_router() -> Router<Arc<ServerState>> {
    auth::router()
}
