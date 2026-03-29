use serde_json::{json, Value};
use std::time::Duration;

use crate::cognee_client::CogneeClient;
use crate::error::AppError;
use crate::output;

pub async fn run(
    client: &CogneeClient,
    detailed: bool,
    watch: bool,
    interval: u64,
    human: bool,
) -> Result<Value, AppError> {
    if interval == 0 {
        return Err(AppError::Config(
            "--interval must be at least 1 second".into(),
        ));
    }

    if !watch {
        return fetch_health(client, detailed).await;
    }

    loop {
        let data = fetch_health(client, detailed).await?;
        output::print_result("health", Ok(data), human);

        tokio::select! {
            _ = tokio::signal::ctrl_c() => break,
            _ = tokio::time::sleep(Duration::from_secs(interval)) => {}
        }
    }

    Ok(json!({
        "watch_stopped": true,
        "interval_secs": interval,
    }))
}

async fn fetch_health(client: &CogneeClient, detailed: bool) -> Result<Value, AppError> {
    let data = if detailed {
        client.health_detailed().await?
    } else {
        client.health().await?
    };

    client.record_health_snapshot(&data).await;
    Ok(data)
}
