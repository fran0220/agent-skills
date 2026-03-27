use anyhow::{anyhow, Context};
use reqwest::header::{HeaderMap, HeaderValue, AUTHORIZATION};

use super::config::{load_auth, normalize_gateway_url};

pub fn authenticated_client(gateway_url: &str) -> anyhow::Result<(reqwest::Client, String)> {
    let normalized_gateway = normalize_gateway_url(gateway_url);
    let auth = load_auth(&normalized_gateway)?.ok_or_else(|| {
        anyhow!(
            "no saved auth token for {}. run `asset-gateway auth login --url {}` first",
            normalized_gateway,
            normalized_gateway
        )
    })?;

    let mut headers = HeaderMap::new();
    let auth_header = HeaderValue::from_str(&format!("Bearer {}", auth.token))
        .context("saved token is invalid, run `asset-gateway auth login` again")?;
    headers.insert(AUTHORIZATION, auth_header);

    let client = reqwest::Client::builder()
        .default_headers(headers)
        .build()
        .context("failed to build authenticated HTTP client")?;

    Ok((client, normalized_gateway))
}
