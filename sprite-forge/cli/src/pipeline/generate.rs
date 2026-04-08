use crate::client::gemini::ImageClient;
use std::time::Instant;
use tracing::info;

pub async fn generate_sprite_grid(
    image_client: &ImageClient,
    enhanced_prompt: &str,
    reference_image: Option<&[u8]>,
) -> anyhow::Result<Vec<u8>> {
    let start = Instant::now();
    let grid_bytes = image_client
        .generate_image(enhanced_prompt, reference_image)
        .await?;

    info!(
        elapsed_ms = start.elapsed().as_millis(),
        bytes = grid_bytes.len(),
        has_reference = reference_image.is_some(),
        "sprite grid generation completed"
    );

    Ok(grid_bytes)
}
