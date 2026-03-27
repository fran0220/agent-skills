use serde_json::Value;

use crate::cognee_client::CogneeClient;
use crate::error::AppError;

pub async fn list(client: &CogneeClient) -> Result<Value, AppError> {
    client.datasets().await
}

pub async fn create(client: &CogneeClient, name: &str) -> Result<Value, AppError> {
    client.create_dataset(name).await
}

pub async fn delete(client: &CogneeClient, id: &str) -> Result<Value, AppError> {
    client.delete_dataset(id).await
}

pub async fn status(client: &CogneeClient) -> Result<Value, AppError> {
    client.dataset_status().await
}

pub async fn graph(client: &CogneeClient, id: &str) -> Result<Value, AppError> {
    client.dataset_graph(id).await
}
