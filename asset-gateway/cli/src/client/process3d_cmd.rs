use std::path::{Path, PathBuf};

use anyhow::{bail, Context};
use serde_json::{json, Value};

use super::http::authenticated_client;
use super::Process3dCommands;
use crate::output;

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

async fn parse_process3d_response(resp: reqwest::Response) -> anyhow::Result<Value> {
    let status = resp.status();
    let body = resp
        .text()
        .await
        .context("failed to read process3d response body")?;

    match serde_json::from_str::<Value>(&body) {
        Ok(json) if status.is_success() => Ok(json),
        Ok(json) => {
            let message = json
                .pointer("/error/message")
                .and_then(Value::as_str)
                .unwrap_or_else(|| body.trim());
            bail!("process3d request failed with {}: {}", status, message);
        }
        Err(_) if status.is_success() => bail!(
            "process3d request returned {} with a non-JSON body: {}",
            status,
            response_snippet(&body)
        ),
        Err(_) => bail!(
            "process3d request failed with {}: {}",
            status,
            response_snippet(&body)
        ),
    }
}

fn infer_extension(output_url: Option<&str>, format_hint: Option<&str>) -> String {
    if let Some(output_url) = output_url {
        let path = output_url.split('?').next().unwrap_or(output_url);
        if let Some(ext) = Path::new(path).extension().and_then(|value| value.to_str()) {
            let ext = ext.trim().trim_start_matches('.');
            if !ext.is_empty() {
                return ext.to_ascii_lowercase();
            }
        }
    }

    format_hint
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.trim_start_matches('.').to_ascii_lowercase())
        .unwrap_or_else(|| "glb".to_string())
}

fn default_output_path(job_id: Option<&str>, extension: &str) -> PathBuf {
    let stem = job_id
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("process3d_output");
    PathBuf::from(format!("{}.{}", stem, extension))
}

async fn save_output_file(
    result: &mut Value,
    output_dir: &str,
    format_hint: Option<&str>,
) -> anyhow::Result<()> {
    if !result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        return Ok(());
    }

    let Some(data) = result.get_mut("data").and_then(Value::as_object_mut) else {
        return Ok(());
    };

    let output_url = data
        .get("output_url")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_string);
    let Some(output_url) = output_url else {
        return Ok(());
    };

    let bytes = reqwest::Client::new()
        .get(&output_url)
        .send()
        .await
        .with_context(|| format!("failed to download output from {output_url}"))?
        .bytes()
        .await
        .with_context(|| format!("failed to read bytes from {output_url}"))?;

    let extension = infer_extension(Some(&output_url), format_hint);
    let job_id = data.get("job_id").and_then(Value::as_str);
    let output_path = Path::new(output_dir).join(default_output_path(job_id, &extension));
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            tokio::fs::create_dir_all(parent).await?;
        }
    }
    tokio::fs::write(&output_path, &bytes).await?;

    let output_path_string = output_path.to_string_lossy().to_string();
    data.insert(
        "output_path".into(),
        Value::String(output_path_string.clone()),
    );
    data.insert("local_path".into(), Value::String(output_path_string));

    Ok(())
}

pub async fn handle(cmd: Process3dCommands, gateway_url: &str) -> anyhow::Result<()> {
    let (body, output_dir, format_hint) = match cmd {
        Process3dCommands::Convert {
            task_id,
            format,
            quad,
            face_limit,
            pack_uv,
            bake,
            texture_format,
            force_symmetry,
            output_dir,
        } => {
            let mut params = json!({
                "format": format,
                "quad": quad,
                "face_limit": face_limit,
            });
            if pack_uv {
                params["pack_uv"] = json!(true);
            }
            if bake {
                params["bake"] = json!(true);
            }
            if force_symmetry {
                params["force_symmetry"] = json!(true);
            }
            if let Some(texture_format) = texture_format {
                params["texture_format"] = json!(texture_format);
            }
            (
                json!({
                    "task_id": task_id,
                    "operation": "convert",
                    "params": params,
                }),
                Some(output_dir),
                Some(format),
            )
        }
        Process3dCommands::Texture {
            task_id,
            prompt,
            style_image,
            pbr,
            quality,
            texture_alignment,
            bake,
            texture_version,
            output_dir,
        } => {
            let mut params = json!({});
            if let Some(prompt) = prompt {
                params["prompt"] = json!(prompt);
            }
            if let Some(style_image) = style_image {
                params["style_image"] = json!(style_image);
            }
            if pbr {
                params["pbr"] = json!(true);
            }
            if let Some(quality) = quality {
                params["texture_quality"] = json!(quality);
            }
            if let Some(texture_alignment) = texture_alignment {
                params["texture_alignment"] = json!(texture_alignment);
            }
            if bake {
                params["bake"] = json!(true);
            }
            if let Some(texture_version) = texture_version {
                params["model_version"] = json!(texture_version);
            }
            (
                json!({
                    "task_id": task_id,
                    "operation": "texture",
                    "params": params,
                }),
                Some(output_dir),
                Some("glb".to_string()),
            )
        }
        Process3dCommands::Rig {
            task_id,
            format,
            spec,
            rig_type,
            output_dir,
        } => {
            let mut params = json!({
                "out_format": format,
                "spec": spec,
            });
            if let Some(rig_type) = rig_type {
                params["rig_type"] = json!(rig_type);
            }
            (
                json!({
                    "task_id": task_id,
                    "operation": "rig",
                    "params": params,
                }),
                Some(output_dir),
                Some(format),
            )
        }
        Process3dCommands::Animate {
            task_id,
            animation,
            format,
            output_dir,
        } => (
            json!({
                "task_id": task_id,
                "operation": "animate",
                "params": {
                    "animation": animation,
                    "out_format": format,
                },
            }),
            Some(output_dir),
            Some(format),
        ),
        Process3dCommands::Reduce {
            task_id,
            face_limit,
            quad,
            output_dir,
        } => (
            json!({
                "task_id": task_id,
                "operation": "reduce",
                "params": {
                    "face_limit": face_limit,
                    "quad": quad,
                },
            }),
            Some(output_dir),
            Some("glb".to_string()),
        ),
        Process3dCommands::Stylize {
            task_id,
            style,
            output_dir,
        } => (
            json!({
                "task_id": task_id,
                "operation": "stylize",
                "params": {
                    "style": style,
                },
            }),
            Some(output_dir),
            Some("glb".to_string()),
        ),
        Process3dCommands::Segment { task_id } => (
            json!({
                "task_id": task_id,
                "operation": "segment",
                "params": {},
            }),
            None,
            None,
        ),
        Process3dCommands::Prerigcheck { task_id } => (
            json!({
                "task_id": task_id,
                "operation": "prerigcheck",
                "params": {},
            }),
            None,
            None,
        ),
        Process3dCommands::Refine {
            task_id,
            output_dir,
        } => (
            json!({
                "task_id": task_id,
                "operation": "refine",
                "params": {},
            }),
            Some(output_dir),
            Some("glb".to_string()),
        ),
        Process3dCommands::Import {
            file_url,
            file_path,
            output_dir,
        } => {
            let mut params = json!({});
            if let Some(url) = file_url {
                params["file_url"] = json!(url);
            }
            if let Some(path) = file_path {
                params["file_path"] = json!(path);
            }
            (
                json!({
                    "task_id": "",
                    "operation": "import",
                    "params": params,
                }),
                Some(output_dir),
                Some("glb".to_string()),
            )
        }
    };

    let (client, gateway_url) = authenticated_client(gateway_url)?;
    let resp = client
        .post(format!("{}/api/process3d", gateway_url))
        .json(&body)
        .send()
        .await?;

    let mut result = parse_process3d_response(resp).await?;
    if let Some(output_dir) = output_dir.as_deref() {
        save_output_file(&mut result, output_dir, format_hint.as_deref()).await?;
    }
    output::print_json(&result);

    Ok(())
}
