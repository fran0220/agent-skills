use anyhow::{bail, Context};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde::Serialize;
use serde_json::{json, Value};
use std::path::Path;

use super::http::authenticated_client;
use super::{VoiceCommands, VoiceType};
use crate::output;

#[derive(Serialize)]
struct VoiceListQuery<'a> {
    #[serde(rename = "type")]
    voice_type: &'a str,
    page: u32,
    page_size: u32,
}

#[derive(Serialize)]
struct VoiceTypeQuery<'a> {
    #[serde(rename = "type")]
    voice_type: &'a str,
}

fn response_snippet(body: &str) -> String {
    let trimmed = body.trim();
    if trimmed.is_empty() {
        return "<empty response body>".to_string();
    }

    const LIMIT: usize = 400;
    let snippet: String = trimmed.chars().take(LIMIT).collect();
    if trimmed.chars().count() > LIMIT {
        format!("{snippet}...")
    } else {
        snippet
    }
}

async fn parse_voice_response(resp: reqwest::Response, action: &str) -> anyhow::Result<Value> {
    let status = resp.status();
    let body = resp
        .text()
        .await
        .with_context(|| format!("failed to read {action} response body"))?;

    match serde_json::from_str::<Value>(&body) {
        Ok(json) if status.is_success() => Ok(json),
        Ok(json) => {
            let message = json
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or_else(|| body.trim());
            bail!("{action} request failed with {}: {}", status, message);
        }
        Err(_) if status.is_success() => bail!(
            "{action} request returned {} with a non-JSON body: {}",
            status,
            response_snippet(&body)
        ),
        Err(_) => bail!(
            "{action} request failed with {}: {}",
            status,
            response_snippet(&body)
        ),
    }
}

fn infer_audio_mime(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|ext| ext.to_str())
        .map(|ext| ext.to_ascii_lowercase())
        .as_deref()
    {
        Some("mp3") => "audio/mpeg",
        Some("wav") => "audio/wav",
        Some("flac") => "audio/flac",
        Some("m4a") => "audio/mp4",
        Some("aac") => "audio/aac",
        Some("ogg") | Some("opus") => "audio/ogg",
        _ => "application/octet-stream",
    }
}

fn voice_type_value(value: VoiceType) -> &'static str {
    value.as_str()
}

pub async fn handle(cmd: VoiceCommands, gateway_url: &str) -> anyhow::Result<()> {
    let (client, gateway_url) = authenticated_client(gateway_url)?;

    let result = match cmd {
        VoiceCommands::Clone {
            audio,
            name,
            target_model,
            mime,
        } => {
            let audio_path = Path::new(&audio);
            let audio_bytes = tokio::fs::read(audio_path)
                .await
                .with_context(|| format!("failed to read audio file {}", audio_path.display()))?;
            let audio_mime = mime.unwrap_or_else(|| infer_audio_mime(audio_path).to_string());

            let resp = client
                .post(format!("{}/api/voice/clone", gateway_url))
                .json(&json!({
                    "audio_base64": STANDARD.encode(audio_bytes),
                    "audio_mime": audio_mime,
                    "name": name,
                    "target_model": target_model,
                }))
                .send()
                .await?;
            parse_voice_response(resp, "voice clone").await?
        }
        VoiceCommands::Design {
            prompt,
            preview_text,
            name,
            target_model,
            language,
        } => {
            let resp = client
                .post(format!("{}/api/voice/design", gateway_url))
                .json(&json!({
                    "voice_prompt": prompt,
                    "preview_text": preview_text,
                    "name": name,
                    "target_model": target_model,
                    "language": language,
                }))
                .send()
                .await?;
            parse_voice_response(resp, "voice design").await?
        }
        VoiceCommands::List {
            r#type,
            page,
            page_size,
        } => {
            let query = VoiceListQuery {
                voice_type: voice_type_value(r#type),
                page,
                page_size,
            };
            let resp = client
                .get(format!("{}/api/voice/list", gateway_url))
                .query(&query)
                .send()
                .await?;
            parse_voice_response(resp, "voice list").await?
        }
        VoiceCommands::Delete { voice_id, r#type } => {
            let query = VoiceTypeQuery {
                voice_type: voice_type_value(r#type),
            };
            let resp = client
                .delete(format!("{}/api/voice/{}", gateway_url, voice_id))
                .query(&query)
                .send()
                .await?;
            parse_voice_response(resp, "voice delete").await?
        }
    };

    output::print_json(&result);
    Ok(())
}
