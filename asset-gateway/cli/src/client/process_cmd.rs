use super::http::authenticated_client;
use super::ProcessCommands;
use crate::output;
use anyhow::Context;
use base64::engine::general_purpose::STANDARD;
use base64::Engine as _;
use serde_json::{json, Value};
use std::path::Path;

pub async fn handle(cmd: ProcessCommands, gateway_url: &str) -> anyhow::Result<()> {
    let (operations, input, output_dir) = match &cmd {
        ProcessCommands::RemoveBg {
            input,
            smart_crop,
            output_dir,
        } => {
            let mut ops = vec![json!({"op": "remove_bg"})];
            if *smart_crop {
                ops.push(json!({"op": "smart_crop", "mode": "power_of2"}));
            }
            (ops, input.clone(), output_dir.clone())
        }
        ProcessCommands::Crop {
            input,
            mode,
            output_dir,
        } => (
            vec![json!({"op": "smart_crop", "mode": mode})],
            input.clone(),
            output_dir.clone(),
        ),
        ProcessCommands::Resize {
            input,
            width,
            height,
            output_dir,
        } => (
            vec![json!({"op": "resize", "width": width, "height": height})],
            input.clone(),
            output_dir.clone(),
        ),
        ProcessCommands::Upscale {
            input,
            scale,
            output_dir,
        } => (
            vec![json!({"op": "upscale", "scale": scale})],
            input.clone(),
            output_dir.clone(),
        ),
    };

    // If input is a local file, read and base64 encode it
    let input_value = if Path::new(&input).exists() {
        let bytes = tokio::fs::read(&input)
            .await
            .context("failed to read input file")?;
        format!("data:image/png;base64,{}", STANDARD.encode(&bytes))
    } else {
        input
    };

    let body = json!({
        "input": input_value,
        "operations": operations,
    });

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
