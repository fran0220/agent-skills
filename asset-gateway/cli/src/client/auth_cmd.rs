use super::config::{clear_auth, load_auth, normalize_gateway_url, save_auth};
use super::AuthCommands;
use crate::output;
use anyhow::anyhow;
use serde_json::Value;

fn resolve_token(value: Option<String>) -> anyhow::Result<String> {
    if let Some(token) = value {
        let trimmed = token.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    if let Ok(from_env) = std::env::var("ASSET_GATEWAY_TOKEN") {
        let trimmed = from_env.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }
    Err(anyhow!(
        "missing token: pass --token or set ASSET_GATEWAY_TOKEN"
    ))
}

pub async fn handle(cmd: AuthCommands, gateway_url: &str) -> anyhow::Result<()> {
    match cmd {
        AuthCommands::Login { url, token } => {
            let target = normalize_gateway_url(url.as_deref().unwrap_or(gateway_url));
            let token = resolve_token(token)?;

            let client = reqwest::Client::new();
            let resp = client
                .post(format!("{}/auth/login", target))
                .json(&serde_json::json!({ "token": token }))
                .send()
                .await?;

            let body: Value = resp.json().await?;

            if body.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                let returned_token = body
                    .pointer("/data/token")
                    .and_then(Value::as_str)
                    .unwrap_or(&token);
                let expires_at = body.pointer("/data/expires_at").and_then(Value::as_str);
                let api_key = body.pointer("/data/api_key").and_then(Value::as_str);
                let login_username = body
                    .pointer("/data/user/username")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown");
                save_auth(&target, returned_token, expires_at, api_key, login_username)?;
            }

            output::print_json(&body);
        }
        AuthCommands::Logout => {
            let removed = clear_auth(gateway_url)?;
            output::print_json(&output::success(
                "auth.logout",
                serde_json::json!({
                    "gateway_url": gateway_url,
                    "removed": removed,
                    "message": if removed { "logged out" } else { "no saved login" },
                }),
            ));
        }
        AuthCommands::Whoami => {
            let payload = match load_auth(gateway_url)? {
                Some(auth) => serde_json::json!({
                    "logged_in": true,
                    "gateway_url": auth.gateway_url,
                    "username": auth.username,
                    "expires_at": auth.expires_at,
                    "api_key": auth.api_key,
                }),
                None => serde_json::json!({
                    "logged_in": false,
                    "gateway_url": gateway_url,
                    "message": "not logged in",
                }),
            };
            output::print_json(&output::success("auth.whoami", payload));
        }
    }
    Ok(())
}
