use serde_json::Value;

use crate::cognee_client::CogneeClient;
use crate::error::AppError;

pub async fn run(client: &CogneeClient, query: &str, search_type: &str, top_k: u32) -> Result<Value, AppError> {
    client.search(query, Some(search_type), Some(top_k)).await
}
