pub mod generate;
pub mod process;
pub mod prompt;

use crate::client::{gemini::ImageClient, llm::LlmClient};
use crate::types::{AnimationType, Direction, SpriteRequest};
use anyhow::{Context, anyhow};
use std::time::Instant;
use tokio::fs;
use tracing::info;

pub struct PipelineResult {
    pub enhanced_prompt: String,
    pub grid_image: Vec<u8>,
    pub frames: Vec<Vec<u8>>,
    pub sprite_sheet: Vec<u8>,
}

pub async fn run_pipeline(
    llm: &LlmClient,
    image_client: &ImageClient,
    request: &SpriteRequest,
) -> anyhow::Result<PipelineResult> {
    let start = Instant::now();
    let grid_size = (request.grid_size.cols(), request.grid_size.rows());
    let character_desc = request
        .character_description
        .as_deref()
        .unwrap_or("the character shown in the reference image");
    let animation_type = animation_type_str(&request.animation_type);
    let direction = direction_str(&request.direction);

    info!(
        animation_type,
        direction,
        cols = grid_size.0,
        rows = grid_size.1,
        has_reference = request.character_image.is_some(),
        "step 1/3: enhancing prompt"
    );
    let enhanced_prompt = prompt::enhance_prompt(
        llm,
        animation_type,
        direction,
        grid_size,
        character_desc,
        request.style.as_deref(),
    )
    .await?;

    info!("step 2/3: loading reference image");
    let reference_image = load_reference_image(request.character_image.as_deref()).await?;

    info!("step 2/3: generating sprite grid");
    let grid_image =
        generate::generate_sprite_grid(image_client, &enhanced_prompt, reference_image.as_deref())
            .await?;

    info!("step 3/3: post-processing");
    let processed = process::process_grid(&grid_image, grid_size.0, grid_size.1)?;

    info!(
        elapsed_ms = start.elapsed().as_millis(),
        frames = processed.frames.len(),
        "pipeline completed"
    );

    Ok(PipelineResult {
        enhanced_prompt,
        grid_image: processed.grid_image,
        frames: processed.frames,
        sprite_sheet: processed.sprite_sheet,
    })
}

fn animation_type_str(animation_type: &AnimationType) -> &str {
    match animation_type {
        AnimationType::Walk => "walk",
        AnimationType::Run => "run",
        AnimationType::Attack => "attack",
        AnimationType::Idle => "idle",
        AnimationType::Death => "death",
        AnimationType::Custom(custom) => custom.as_str(),
    }
}

fn direction_str(direction: &Direction) -> &str {
    match direction {
        Direction::Right => "right",
        Direction::Left => "left",
        Direction::Front => "front",
        Direction::Back => "back",
    }
}

async fn load_reference_image(character_image: Option<&str>) -> anyhow::Result<Option<Vec<u8>>> {
    let Some(character_image) = character_image.filter(|value| !value.trim().is_empty()) else {
        return Ok(None);
    };

    if is_url(character_image) {
        let response = reqwest::get(character_image).await.with_context(|| {
            format!("failed to download reference image from {character_image}")
        })?;
        let status = response.status();
        if !status.is_success() {
            return Err(anyhow!(
                "failed to download reference image from {character_image}: http {status}"
            ));
        }

        let bytes = response.bytes().await.with_context(|| {
            format!("failed to read downloaded reference image from {character_image}")
        })?;
        return Ok(Some(bytes.to_vec()));
    }

    let bytes = fs::read(character_image)
        .await
        .with_context(|| format!("failed to read reference image from path {character_image}"))?;
    Ok(Some(bytes))
}

fn is_url(value: &str) -> bool {
    value.starts_with("http://") || value.starts_with("https://")
}
