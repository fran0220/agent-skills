use super::http::authenticated_client;
use super::GenerateCommands;
use crate::output;
use anyhow::{bail, Context};
use base64::{engine::general_purpose::STANDARD, Engine as _};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn infer_extension(asset_type: &str) -> &'static str {
    match asset_type {
        "image" => "png",
        "audio" => "mp3",
        "video" => "mp4",
        "model3d" => "glb",
        "text" => "txt",
        _ => "bin",
    }
}

fn default_output_path(job_id: Option<&str>, asset_type: &str) -> PathBuf {
    let stem = job_id
        .map(str::trim)
        .filter(|v| !v.is_empty())
        .unwrap_or("output");
    PathBuf::from(format!("{}.{}", stem, infer_extension(asset_type)))
}

fn resolve_output_path(output_dir: &Path, job_id: Option<&str>, asset_type: &str) -> PathBuf {
    output_dir.join(default_output_path(job_id, asset_type))
}

async fn write_output(path: &Path, bytes: &[u8]) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent).await.with_context(|| {
                format!("failed to create output directory {}", parent.display())
            })?;
        }
    }

    tokio::fs::write(path, bytes)
        .await
        .with_context(|| format!("failed to write output file {}", path.display()))
}

fn decode_output_data(asset_type: &str, output_data: &str) -> anyhow::Result<Vec<u8>> {
    if asset_type == "text" {
        return Ok(output_data.as_bytes().to_vec());
    }

    // Strip data URI prefix (e.g. "data:image/png;base64,")
    let raw = if let Some(pos) = output_data.find(";base64,") {
        &output_data[pos + 8..]
    } else {
        output_data
    };

    STANDARD
        .decode(raw)
        .with_context(|| format!("failed to decode base64 output for asset type {asset_type}"))
}

async fn save_generated_file(
    result: &mut Value,
    asset_type: &str,
    output_dir: String,
) -> anyhow::Result<()> {
    if !result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return Ok(());
    }

    let Some(data) = result.get_mut("data").and_then(Value::as_object_mut) else {
        return Ok(());
    };

    let output_data = data
        .get("output_data")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .map(str::to_string);

    let output_url = data
        .get("output_url")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|url| !url.is_empty())
        .map(str::to_string);

    if output_data.is_none() && output_url.is_none() {
        return Ok(());
    }

    let job_id = data.get("job_id").and_then(Value::as_str);
    let output_path = resolve_output_path(Path::new(&output_dir), job_id, asset_type);

    let bytes = if let Some(raw_data) = output_data {
        decode_output_data(asset_type, &raw_data)?
    } else if let Some(url) = output_url {
        let resp = reqwest::Client::new()
            .get(&url)
            .send()
            .await
            .with_context(|| format!("failed to download output from {url}"))?;
        resp.bytes()
            .await
            .with_context(|| format!("failed to read bytes from {url}"))?
            .to_vec()
    } else {
        Vec::new()
    };

    write_output(&output_path, &bytes).await?;
    let output_path_string = output_path.to_string_lossy().to_string();
    data.insert(
        "output_path".into(),
        Value::String(output_path_string.clone()),
    );
    data.insert("local_path".into(), Value::String(output_path_string));

    Ok(())
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

async fn parse_generate_response(resp: reqwest::Response) -> anyhow::Result<Value> {
    let status = resp.status();
    let body = resp.text().await.context("failed to read generate response body")?;

    match serde_json::from_str::<Value>(&body) {
        Ok(json) if status.is_success() => Ok(json),
        Ok(json) => {
            let message = json
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or_else(|| body.trim());
            bail!("generate request failed with {}: {}", status, message);
        }
        Err(_) if status.is_success() => bail!(
            "generate request returned {} with a non-JSON body: {}",
            status,
            response_snippet(&body)
        ),
        Err(_) => bail!(
            "generate request failed with {}: {}",
            status,
            response_snippet(&body)
        ),
    }
}

pub async fn handle(cmd: GenerateCommands, gateway_url: &str) -> anyhow::Result<()> {
    let (asset_type, body, output_dir) = match cmd {
        GenerateCommands::Image {
            prompt,
            provider,
            transparent,
            model,
            size,
            output_dir,
        } => (
            "image",
            serde_json::json!({
                "asset_type": "image",
                "prompt": prompt,
                "provider": provider,
                "model": model,
                "params": { "size": size, "transparent": transparent },
            }),
            output_dir,
        ),
        GenerateCommands::Video {
            prompt,
            provider,
            output_dir,
        } => (
            "video",
            serde_json::json!({
                "asset_type": "video",
                "prompt": prompt,
                "provider": provider,
            }),
            output_dir,
        ),
        GenerateCommands::Audio {
            prompt,
            r#type,
            duration,
            output_dir,
        } => (
            "audio",
            serde_json::json!({
                "asset_type": "audio",
                "prompt": prompt,
                "params": { "type": r#type, "duration_seconds": duration },
            }),
            output_dir,
        ),
        GenerateCommands::Model {
            image,
            prompt,
            output_dir,
        } => (
            "model3d",
            serde_json::json!({
                "asset_type": "model3d",
                "prompt": prompt,
                "input_file": image,
            }),
            output_dir,
        ),
        GenerateCommands::Text {
            prompt,
            model,
            max_tokens,
            output_dir,
        } => (
            "text",
            serde_json::json!({
                "asset_type": "text",
                "prompt": prompt,
                "model": model,
                "params": { "max_tokens": max_tokens.unwrap_or(4096) },
            }),
            output_dir,
        ),
    };

    let (client, gateway_url) = authenticated_client(gateway_url)?;
    let resp = client
        .post(format!("{}/api/generate", gateway_url))
        .json(&body)
        .send()
        .await?;

    let mut result = parse_generate_response(resp).await?;
    save_generated_file(&mut result, asset_type, output_dir).await?;
    output::print_json(&result);

    Ok(())
}
