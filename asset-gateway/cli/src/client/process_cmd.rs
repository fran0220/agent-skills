use super::http::authenticated_client;
use super::ProcessCommands;
use crate::output;
use anyhow::Context;
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use serde_json::{json, Value};
use std::path::Path;

async fn encode_input(input: &str) -> anyhow::Result<String> {
    if Path::new(input).exists() {
        let bytes = tokio::fs::read(input)
            .await
            .context("failed to read input file")?;
        Ok(format!("data:image/png;base64,{}", STANDARD.encode(&bytes)))
    } else {
        Ok(input.to_string())
    }
}

pub async fn handle(cmd: ProcessCommands, gateway_url: &str) -> anyhow::Result<()> {
    let (body, output_dir) = match &cmd {
        ProcessCommands::Crop {
            input,
            mode,
            output_dir,
        } => (
            json!({
                "input": encode_input(input).await?,
                "operations": [{"op": "smart_crop", "mode": mode}],
            }),
            output_dir.clone(),
        ),
        ProcessCommands::Resize {
            input,
            width,
            height,
            output_dir,
        } => (
            json!({
                "input": encode_input(input).await?,
                "operations": [{"op": "resize", "width": width, "height": height}],
            }),
            output_dir.clone(),
        ),
        ProcessCommands::Compose {
            inputs,
            direction,
            columns,
            padding,
            frame_width,
            frame_height,
            output_dir,
        } => {
            let mut encoded_inputs = Vec::with_capacity(inputs.len());
            for input in inputs {
                encoded_inputs.push(encode_input(input).await?);
            }

            (
                json!({
                    "inputs": encoded_inputs,
                    "operations": [{
                        "op": "compose",
                        "direction": direction,
                        "columns": columns,
                        "padding": padding,
                        "frame_width": frame_width,
                        "frame_height": frame_height,
                    }],
                }),
                output_dir.clone(),
            )
        }
        ProcessCommands::ExtractFrames {
            input,
            count,
            output_dir,
        } => (
            json!({
                "input": encode_input(input).await?,
                "operations": [{"op": "extract_frames", "count": count}],
            }),
            output_dir.clone(),
        ),
        ProcessCommands::RemoveBg {
            inputs,
            bg_color,
            output_dir,
        } => {
            let mut encoded_inputs = Vec::with_capacity(inputs.len());
            for input in inputs {
                encoded_inputs.push(encode_input(input).await?);
            }

            let mut op = json!({"op": "remove_bg"});
            if let Some(color) = bg_color {
                op["bg_color"] = json!(color);
            }

            (
                json!({
                    "inputs": encoded_inputs,
                    "operations": [op],
                }),
                output_dir.clone(),
            )
        }
    };

    let (client, gateway_url) = authenticated_client(gateway_url)?;
    let resp = client
        .post(format!("{}/api/process", gateway_url))
        .json(&body)
        .send()
        .await?;

    let status = resp.status();
    let text = resp
        .text()
        .await
        .context("failed to read process response")?;

    if !status.is_success() {
        anyhow::bail!("process request failed with {}: {}", status, text);
    }

    let mut result: Value = serde_json::from_str(&text)?;

    // Save output file
    if let Some(data) = result.pointer("/data/output_data").and_then(Value::as_str) {
        let bytes = STANDARD.decode(data).context("failed to decode output")?;
        let stem = chrono::Utc::now().format("%Y%m%d_%H%M%S");
        let out_path = Path::new(&output_dir).join(format!("processed_{}.png", stem));
        if let Some(parent) = out_path.parent() {
            if !parent.as_os_str().is_empty() {
                tokio::fs::create_dir_all(parent).await?;
            }
        }
        tokio::fs::write(&out_path, &bytes).await?;

        if let Some(obj) = result.pointer_mut("/data").and_then(Value::as_object_mut) {
            obj.insert(
                "local_path".into(),
                Value::String(out_path.to_string_lossy().to_string()),
            );
            obj.remove("output_data");
        }
    }

    output::print_json(&result);
    Ok(())
}
