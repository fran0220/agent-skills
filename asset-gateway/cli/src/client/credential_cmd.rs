use super::http::authenticated_client;
use super::CredentialCommands;
use crate::output;

pub async fn handle(cmd: CredentialCommands, gateway_url: &str) -> anyhow::Result<()> {
    let (client, gateway_url) = authenticated_client(gateway_url)?;

    match cmd {
        CredentialCommands::Set {
            key,
            value,
            provider,
        } => {
            let resp = client
                .put(format!("{}/api/credentials", gateway_url))
                .json(&serde_json::json!({
                    "key": key,
                    "value": value,
                    "provider_id": provider,
                }))
                .send()
                .await?;
            let body: serde_json::Value = resp.json().await?;
            output::print_json(&body);
        }
        CredentialCommands::List => {
            let resp = client
                .get(format!("{}/api/credentials", gateway_url))
                .send()
                .await?;
            let body: serde_json::Value = resp.json().await?;
            output::print_json(&body);
        }
        CredentialCommands::Delete { key } => {
            let resp = client
                .delete(format!("{}/api/credentials/{}", gateway_url, key))
                .send()
                .await?;
            let body: serde_json::Value = resp.json().await?;
            output::print_json(&body);
        }
    }
    Ok(())
}
