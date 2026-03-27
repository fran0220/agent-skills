use serde_json::{json, Value};

use crate::cognee_client::CogneeClient;
use crate::error::AppError;

pub async fn run(client: &CogneeClient, dataset_id: Option<&str>) -> Result<Value, AppError> {
    let payload = match dataset_id {
        Some(id) => json!({ "datasets": [id] }),
        None => json!({}),
    };
    client.cognify(payload).await
}
