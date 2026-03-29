use std::path::Path;

use serde_json::Value;

use crate::cognee_client::CogneeClient;
use crate::error::AppError;

pub async fn upload(client: &CogneeClient, key: &str, file_path: &Path) -> Result<Value, AppError> {
    client.upload_ontology(key, file_path).await
}

pub async fn list(client: &CogneeClient) -> Result<Value, AppError> {
    client.list_ontologies().await
}
