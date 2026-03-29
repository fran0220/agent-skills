use serde_json::Value;

use crate::cognee_client::CogneeClient;
use crate::config::AuthConfig;
use crate::error::AppError;

pub async fn run(client: &CogneeClient, username: &str, password: &str) -> Result<Value, AppError> {
    let result = client.login(username, password).await?;

    let token = result
        .get("access_token")
        .and_then(|t| t.as_str())
        .ok_or_else(|| AppError::CogneeApi("No access_token in login response".to_string()))?;

    let mut auth = AuthConfig::load();
    auth.cognee_jwt = Some(token.to_string());
    auth.save()
        .map_err(|e| AppError::Config(format!("Failed to save auth config: {e}")))?;

    Ok(result)
}
