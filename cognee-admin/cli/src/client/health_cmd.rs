use serde_json::Value;

use crate::cognee_client::CogneeClient;
use crate::error::AppError;

pub async fn run(client: &CogneeClient, detailed: bool) -> Result<Value, AppError> {
    if detailed {
        client.health_detailed().await
    } else {
        client.health().await
    }
}
