pub mod pages;

use axum::{response::Html, routing::get, Router};
use std::sync::Arc;

use crate::server::ServerState;

async fn admin_panel() -> Html<&'static str> {
    Html(pages::ADMIN_HTML)
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new()
        .route("/", get(admin_panel))
        .route("/admin", get(admin_panel))
}
