use super::generate_cmd::{
    infer_extension, load_output_bytes, maybe_encode_local_inputs, parse_generate_response,
    write_output,
};
use super::http::authenticated_client;
use super::GenerateCommands;
use crate::output;
use anyhow::{bail, Context};
use serde_json::Value;
use std::path::{Path, PathBuf};

fn parse_frame_size(raw: &str) -> anyhow::Result<(u32, u32)> {
    let (width, height) = raw
        .split_once('x')
        .ok_or_else(|| anyhow::anyhow!("frame size must be formatted as WIDTHxHEIGHT"))?;
    Ok((
        width
            .trim()
            .parse()
            .context("failed to parse frame width")?,
        height
            .trim()
            .parse()
            .context("failed to parse frame height")?,
    ))
}

fn frame_output_path(output_dir: &Path, index: usize, asset_type: &str) -> PathBuf {
    output_dir.join(format!("frame_{index:03}.{}", infer_extension(asset_type)))
}

pub async fn handle(cmd: GenerateCommands, gateway_url: &str) -> anyhow::Result<()> {
    let GenerateCommands::Batch {
        asset_type,
        prompts,
        transparent,
        size,
        reference_images,
        compose,
        columns,
        frame_size,
        output_dir,
    } = cmd
    else {
        bail!("batch handler received a non-batch command")
    };

    let reference_images = maybe_encode_local_inputs(if reference_images.is_empty() {
        None
    } else {
        Some(reference_images)
    })
    .await?;

    let mut shared = serde_json::json!({});
    if transparent {
        shared["transparent"] = serde_json::json!(true);
    }
    if let Some(size) = size {
        shared["size"] = serde_json::json!(size);
    }
    if let Some(reference_images) = reference_images {
        shared["reference_images"] = serde_json::json!(reference_images);
    }

    let mut body = serde_json::json!({
        "asset_type": asset_type.clone(),
        "prompts": prompts,
        "shared": shared,
    });
    if let Some(direction) = compose {
        let mut compose_body = serde_json::json!({ "direction": direction });
        if let Some(columns) = columns {
            compose_body["columns"] = serde_json::json!(columns);
        }
        if let Some(frame_size) = frame_size {
            let (frame_width, frame_height) = parse_frame_size(&frame_size)?;
            compose_body["frame_width"] = serde_json::json!(frame_width);
            compose_body["frame_height"] = serde_json::json!(frame_height);
        }
        body["compose"] = compose_body;
    }

    let (client, gateway_url) = authenticated_client(gateway_url)?;
    let resp = client
        .post(format!("{}/api/generate/batch", gateway_url))
        .json(&body)
        .send()
        .await?;

    let mut result = parse_generate_response(resp).await?;
    let output_dir_path = Path::new(&output_dir);

    if let Some(frames) = result
        .pointer_mut("/data/frames")
        .and_then(Value::as_array_mut)
    {
        for frame in frames {
            let Some(frame_obj) = frame.as_object_mut() else {
                continue;
            };
            let index = frame_obj
                .get("index")
                .and_then(Value::as_u64)
                .unwrap_or_default() as usize;
            let bytes = load_output_bytes(
                &asset_type,
                frame_obj.get("output_data").and_then(Value::as_str),
                frame_obj.get("output_url").and_then(Value::as_str),
            )
            .await?;
            if let Some(bytes) = bytes {
                let path = frame_output_path(output_dir_path, index, &asset_type);
                write_output(&path, &bytes).await?;
                frame_obj.insert(
                    "local_path".into(),
                    Value::String(path.to_string_lossy().to_string()),
                );
            }
        }
    }

    if let Some(sprite) = result
        .pointer_mut("/data/spritesheet")
        .and_then(Value::as_object_mut)
    {
        let bytes = load_output_bytes(
            "image",
            sprite.get("output_data").and_then(Value::as_str),
            sprite.get("output_url").and_then(Value::as_str),
        )
        .await?;
        if let Some(bytes) = bytes {
            let path = output_dir_path.join("spritesheet.png");
            write_output(&path, &bytes).await?;
            sprite.insert(
                "local_path".into(),
                Value::String(path.to_string_lossy().to_string()),
            );
        }
    }

    output::print_json(&result);
    Ok(())
}
