use super::config::{clear_auth, load_auth, normalize_gateway_url, save_auth};
use super::AuthCommands;
use crate::output;
use anyhow::anyhow;
use serde_json::Value;

fn resolve_login_field(
    value: Option<String>,
    env_name: &str,
    flag_name: &str,
) -> anyhow::Result<String> {
    if let Some(flag) = value {
        let trimmed = flag.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    if let Ok(from_env) = std::env::var(env_name) {
        let trimmed = from_env.trim();
        if !trimmed.is_empty() {
            return Ok(trimmed.to_string());
        }
    }

    Err(anyhow!(
        "missing {flag_name}: pass --{flag_name} or set {env_name}"
    ))
}

pub async fn handle(cmd: AuthCommands, gateway_url: &str) -> anyhow::Result<()> {
    match cmd {
        AuthCommands::Login {
            url,
            username,
            password,
        } => {
            let target = normalize_gateway_url(url.as_deref().unwrap_or(gateway_url));
            let username = resolve_login_field(username, "ASSET_GATEWAY_USERNAME", "username")?;
            let password = resolve_login_field(password, "ASSET_GATEWAY_PASSWORD", "password")?;

            let client = reqwest::Client::new();
            let resp = client
                .post(format!("{}/auth/login", target))
                .json(&serde_json::json!({
                    "username": username.clone(),
                    "password": password,
                }))
                .send()
                .await?;

            let body: Value = resp.json().await?;

            if body.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                let token = body
                    .pointer("/data/token")
                    .and_then(Value::as_str)
                    .ok_or_else(|| anyhow!("login response missing data.token"))?;
                let expires_at = body.pointer("/data/expires_at").and_then(Value::as_str);
                let api_key = body.pointer("/data/api_key").and_then(Value::as_str);
                let login_username = body
                    .pointer("/data/user/username")
                    .and_then(Value::as_str)
                    .unwrap_or(&username);
                save_auth(&target, token, expires_at, api_key, login_username)?;
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
