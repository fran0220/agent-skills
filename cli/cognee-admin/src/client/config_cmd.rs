use serde_json::Value;

use crate::cognee_client::CogneeClient;
use crate::error::AppError;

pub async fn get(client: &CogneeClient) -> Result<Value, AppError> {
    client.get_settings().await
}

pub async fn set(client: &CogneeClient, settings_json: &str) -> Result<Value, AppError> {
    let settings: Value = serde_json::from_str(settings_json)
        .map_err(|e| AppError::Config(format!("Invalid JSON: {e}")))?;
    client.save_settings(settings).await
}
