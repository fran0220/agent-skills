use super::http::authenticated_client;
use super::JobCommands;
use crate::output;

#[derive(serde::Serialize)]
struct ListJobsQuery<'a> {
    limit: u32,
    #[serde(skip_serializing_if = "Option::is_none")]
    status: Option<&'a str>,
}

pub async fn handle(cmd: JobCommands, gateway_url: &str) -> anyhow::Result<()> {
    let (client, gateway_url) = authenticated_client(gateway_url)?;

    match cmd {
        JobCommands::List { status, limit } => {
            let query = ListJobsQuery {
                limit,
                status: status.as_deref(),
            };
            let resp = client
                .get(format!("{}/api/jobs", gateway_url))
                .query(&query)
                .send()
                .await?;
            let body: serde_json::Value = resp.json().await?;
            output::print_json(&body);
        }
        JobCommands::Status { id } => {
            let resp = client
                .get(format!("{}/api/jobs/{}", gateway_url, id))
                .send()
                .await?;
            let body: serde_json::Value = resp.json().await?;
            output::print_json(&body);
        }
        JobCommands::Cancel { id } => {
            let resp = client
                .post(format!("{}/api/jobs/{}/cancel", gateway_url, id))
                .send()
                .await?;
            let body: serde_json::Value = resp.json().await?;
            output::print_json(&body);
        }
    }
    Ok(())
}
