use serde_json::{json, Value};

use crate::cognee_client::CogneeClient;
use crate::error::AppError;

pub async fn add(client: &CogneeClient, dataset_name: &str, content: &str) -> Result<Value, AppError> {
    let payload = json!({
        "data": content
    });
    client.add_data(dataset_name, payload).await
}

pub async fn list(client: &CogneeClient, dataset_id: &str) -> Result<Value, AppError> {
    client.dataset_data(dataset_id).await
}

pub async fn delete(client: &CogneeClient, dataset_id: &str, data_id: &str) -> Result<Value, AppError> {
    client.delete_data(dataset_id, data_id).await
}

pub async fn raw(client: &CogneeClient, dataset_id: &str, data_id: &str) -> Result<Value, AppError> {
    client.raw_data(dataset_id, data_id).await
}
