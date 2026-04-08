use crate::client::llm::LlmClient;
use std::time::Instant;
use tracing::info;

const SPRITE_PROMPT_SYSTEM: &str = r#"You are an expert sprite sheet artist and prompt engineer for AI image generation.

Your task is to generate a detailed, frame-by-frame prompt for an AI image generation model (Google Gemini) to create a sprite animation grid.

Rules:
1. Describe each frame by its grid position (Row X, Col Y)
2. Use precise anatomical terms for poses (e.g., "left leg forward at 45 degrees", "right arm swings back")
3. EMPHASIZE consistency: same style, colors, proportions, and ALL accessories/weapons must appear in EVERY frame
4. Include technical constraints: white background per cell, thin black grid lines separating frames, equal frame sizes
5. Describe a logical animation sequence that loops seamlessly
6. Output ONLY the final prompt text, no explanations or preamble

The prompt you generate will be sent directly to Gemini's image editing API along with a reference character image."#;

pub async fn enhance_prompt(
    llm: &LlmClient,
    animation_type: &str,
    direction: &str,
    grid_size: (u32, u32),
    character_description: &str,
    style: Option<&str>,
) -> anyhow::Result<String> {
    let start = Instant::now();
    let (cols, rows) = grid_size;
    let total_frames = cols * rows;

    let mut user_prompt = format!(
        "Create a detailed sprite sheet generation prompt for this character and animation.\n\n\
Character description:\n{character_description}\n\n\
Animation requirements:\n\
- Animation type: {animation_type}\n\
- Facing direction: {direction}\n\
- Grid: {cols}x{rows} grid with {total_frames} total frames\n\
- The animation must read clearly from this direction and loop seamlessly.\n\
- Every frame must preserve the exact same character identity, costume, silhouette, colors, proportions, and props.\n\
- Use a clean white background in each cell, with thin black grid lines separating frames and equal frame dimensions.\n\
- Describe every frame in order by row and column."
    );

    if let Some(style) = style.filter(|value| !value.trim().is_empty()) {
        user_prompt.push_str(&format!("\n- Visual style: {style}"));
    }

    let enhanced_prompt = llm.chat(SPRITE_PROMPT_SYSTEM, &user_prompt, 2000).await?;
    info!(
        elapsed_ms = start.elapsed().as_millis(),
        prompt_len = enhanced_prompt.len(),
        "prompt enhancement completed"
    );
    Ok(enhanced_prompt)
}
