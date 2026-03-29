use serde_json::{json, Value};

use crate::cognee_client::CogneeClient;
use crate::error::AppError;

pub async fn run(
    client: &CogneeClient,
    query: &str,
    search_type: &str,
    top_k: u32,
    datasets: &[String],
    verbose: bool,
) -> Result<Value, AppError> {
    let result = client
        .search(query, Some(search_type), Some(top_k), Some(datasets))
        .await?;

    if verbose {
        Ok(json!({
            "request": {
                "query": query,
                "search_type": search_type,
                "top_k": top_k,
                "datasets": datasets,
            },
            "response": result,
        }))
    } else {
        Ok(result)
    }
}

pub async fn history(client: &CogneeClient) -> Result<Value, AppError> {
    client.search_history().await
}
