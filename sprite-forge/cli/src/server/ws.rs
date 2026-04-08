use crate::server::auth;
use crate::server::state::AppState;
use axum::{
    extract::{
        Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::Response,
};
use futures_util::StreamExt;
use serde::Deserialize;
use std::sync::Arc;

#[derive(Debug, Deserialize)]
pub struct WsAuthQuery {
    pub token: String,
}

pub async fn jobs_ws(
    State(state): State<Arc<AppState>>,
    axum::extract::Path(job_id): axum::extract::Path<String>,
    Query(query): Query<WsAuthQuery>,
    ws: WebSocketUpgrade,
) -> Result<Response, (StatusCode, String)> {
    let _claims = auth::decode_token(&state.config.jwt_secret, &query.token)?;
    let receiver = state.job_sender(&job_id).subscribe();

    Ok(ws.on_upgrade(move |socket| handle_socket(socket, receiver)))
}

async fn handle_socket(
    mut socket: WebSocket,
    mut receiver: tokio::sync::broadcast::Receiver<crate::types::JobProgress>,
) {
    loop {
        tokio::select! {
            message = receiver.recv() => {
                match message {
                    Ok(progress) => {
                        let payload = match serde_json::to_string(&progress) {
                            Ok(payload) => payload,
                            Err(_) => break,
                        };
                        if socket.send(Message::Text(payload.into())).await.is_err() {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
                }
            }
            inbound = socket.next() => {
                match inbound {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(Message::Ping(payload))) => {
                        if socket.send(Message::Pong(payload)).await.is_err() {
                            break;
                        }
                    }
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
        }
    }
}
