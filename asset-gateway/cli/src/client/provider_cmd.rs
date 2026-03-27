use super::http::authenticated_client;
use super::ProviderCommands;
use crate::output;

pub async fn handle(cmd: ProviderCommands, gateway_url: &str) -> anyhow::Result<()> {
    let (client, gateway_url) = authenticated_client(gateway_url)?;

    match cmd {
        ProviderCommands::List => {
            let resp = client
                .get(format!("{}/api/providers", gateway_url))
                .send()
                .await?;
            let body: serde_json::Value = resp.json().await?;
            output::print_json(&body);
        }
        ProviderCommands::Health { name } => {
            let url = match name {
                Some(n) => format!("{}/api/providers/{}/health", gateway_url, n),
                None => format!("{}/api/providers/health", gateway_url),
            };
            let resp = client.get(&url).send().await?;
            let body: serde_json::Value = resp.json().await?;
            output::print_json(&body);
        }
    }
    Ok(())
}
