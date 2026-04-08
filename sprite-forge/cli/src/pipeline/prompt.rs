use crate::client::llm::LlmClient;
use std::time::Instant;
use tracing::info;

const SPRITE_PROMPT_SYSTEM: &str = r#"You are an expert sprite sheet artist and prompt engineer for AI image generation.

Your task is to generate a single, detailed prompt for an AI image generation model to create a sprite animation grid image.

Rules:
1. Start with a global description: the grid layout (e.g. "A 3x3 grid sprite sheet"), the character's full appearance (outfit, colors, proportions, accessories, weapons), the art style, and the animation type.
2. Then describe EACH cell individually by its grid position. For each cell, write a self-contained visual description like a storyboard shot:
   - "Row 1, Col 1: [character name/description] in [exact pose]. [specific anatomical details: limb positions, weight distribution, facial expression]. [any motion blur or action lines]."
   - Use precise anatomical terms (e.g., "left leg forward at 45 degrees, right arm swings back, torso tilted 10 degrees forward").
3. EMPHASIZE consistency: every cell must show the SAME character with identical outfit, colors, proportions, silhouette, and ALL accessories/weapons.
4. Technical constraints: clean white background in each cell, frames placed edge-to-edge with NO borders, NO grid lines, NO separators between frames. All cells must be equal size.
5. The animation sequence must loop seamlessly (last frame transitions naturally back to first frame).
6. Output ONLY the final prompt text. No JSON, no preamble, no explanations.

The prompt you generate will be sent directly to an image generation API as a single text prompt."#;

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
- Use a clean white background in each cell, with frames placed edge-to-edge (NO borders, NO grid lines, NO separators between frames) and equal frame dimensions.\n\
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
