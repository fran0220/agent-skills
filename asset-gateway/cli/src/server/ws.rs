use std::collections::HashMap;
use std::sync::Arc;

use axum::{
    extract::{
        ws::{Message, WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
    routing::get,
    Router,
};
use chrono::Utc;
use serde_json::{json, Value};
use tokio::sync::{broadcast, RwLock};

use crate::error::AppResult;
use crate::server::routes::auth::CurrentUser;
use crate::server::ServerState;

const JOB_EVENT_BUFFER: usize = 64;

pub struct JobUpdate<'a> {
    pub user_id: &'a str,
    pub job_id: &'a str,
    pub asset_type: &'a str,
    pub provider_id: Option<&'a str>,
    pub status: &'a str,
    pub error_message: Option<&'a str>,
    pub output_path: Option<&'a str>,
    pub cost_usd: Option<f64>,
}

#[derive(Default)]
pub struct JobEventHub {
    channels: RwLock<HashMap<String, broadcast::Sender<Value>>>,
}

impl JobEventHub {
    pub async fn subscribe(&self, user_id: &str) -> broadcast::Receiver<Value> {
        let mut channels = self.channels.write().await;
        channels
            .entry(user_id.to_string())
            .or_insert_with(|| broadcast::channel(JOB_EVENT_BUFFER).0)
            .subscribe()
    }

    pub async fn publish(&self, user_id: &str, payload: Value) {
        let sender = {
            let mut channels = self.channels.write().await;
            channels
                .entry(user_id.to_string())
                .or_insert_with(|| broadcast::channel(JOB_EVENT_BUFFER).0)
                .clone()
        };
        let _ = sender.send(payload);
    }
}

pub async fn broadcast_job_update(state: &ServerState, update: JobUpdate<'_>) {
    state
        .job_events
        .publish(
            update.user_id,
            json!({
                "ok": true,
                "command": "job.update",
                "data": {
                    "job_id": update.job_id,
                    "user_id": update.user_id,
                    "asset_type": update.asset_type,
                    "provider_id": update.provider_id,
                    "status": update.status,
                    "error_message": update.error_message,
                    "output_path": update.output_path,
                    "cost_usd": update.cost_usd,
                    "timestamp": Utc::now().to_rfc3339(),
                }
            }),
        )
        .await;
}

async fn jobs_socket(
    ws: WebSocketUpgrade,
    State(state): State<Arc<ServerState>>,
    current_user: CurrentUser,
) -> AppResult<Response> {
    Ok(ws.on_upgrade(move |socket| handle_socket(socket, state, current_user)))
}

async fn handle_socket(mut socket: WebSocket, state: Arc<ServerState>, current_user: CurrentUser) {
    let mut receiver = state.job_events.subscribe(&current_user.id).await;

    let connected = json!({
        "ok": true,
        "command": "ws.jobs.connected",
        "data": {
            "user_id": current_user.id,
            "timestamp": Utc::now().to_rfc3339(),
        }
    });

    if socket
        .send(Message::Text(connected.to_string().into()))
        .await
        .is_err()
    {
        return;
    }

    loop {
        tokio::select! {
            result = receiver.recv() => {
                match result {
                    Ok(payload) => {
                        if socket.send(Message::Text(payload.to_string().into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Lagged(skipped)) => {
                        let warning = json!({
                            "ok": true,
                            "command": "ws.jobs.lagged",
                            "data": {
                                "skipped": skipped,
                                "timestamp": Utc::now().to_rfc3339(),
                            }
                        });
                        if socket.send(Message::Text(warning.to_string().into())).await.is_err() {
                            break;
                        }
                    }
                    Err(broadcast::error::RecvError::Closed) => break,
                }
            }
            incoming = socket.recv() => {
                match incoming {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Ok(_)) => {}
                    Some(Err(_)) => break,
                }
            }
        }
    }
}

pub fn router() -> Router<Arc<ServerState>> {
    Router::new().route("/ws/jobs", get(jobs_socket))
}
