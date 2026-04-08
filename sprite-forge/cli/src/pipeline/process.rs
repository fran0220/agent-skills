use anyhow::{Context, anyhow};
use image::{DynamicImage, GenericImage, GenericImageView, ImageBuffer, ImageFormat, RgbaImage};
use std::io::Cursor;
use std::time::Instant;
use tracing::info;

pub struct ProcessedSprite {
    pub grid_image: Vec<u8>,
    pub frames: Vec<Vec<u8>>,
    pub sprite_sheet: Vec<u8>,
}

/// 将网格图片切割为独立帧
pub fn split_grid(grid_bytes: &[u8], cols: u32, rows: u32) -> anyhow::Result<Vec<DynamicImage>> {
    if cols == 0 || rows == 0 {
        return Err(anyhow!("grid dimensions must be greater than zero"));
    }

    let grid_image = image::load_from_memory(grid_bytes).context("failed to decode grid image")?;
    let (img_width, img_height) = grid_image.dimensions();

    if img_width % cols != 0 || img_height % rows != 0 {
        return Err(anyhow!(
            "grid image dimensions {img_width}x{img_height} are not divisible by {cols}x{rows}"
        ));
    }

    let frame_width = img_width / cols;
    let frame_height = img_height / rows;
    let mut frames = Vec::with_capacity((cols * rows) as usize);

    for row in 0..rows {
        for col in 0..cols {
            let x = col * frame_width;
            let y = row * frame_height;
            frames.push(grid_image.crop_imm(x, y, frame_width, frame_height));
        }
    }

    Ok(frames)
}

/// 将帧合成为水平精灵条带
pub fn compose_sprite_sheet(frames: &[DynamicImage]) -> anyhow::Result<DynamicImage> {
    let first_frame = frames
        .first()
        .ok_or_else(|| anyhow!("cannot compose sprite sheet from empty frame list"))?;

    let frame_width = first_frame.width();
    let frame_height = first_frame.height();
    let total_width = frame_width
        .checked_mul(frames.len() as u32)
        .ok_or_else(|| anyhow!("sprite sheet width overflow"))?;

    let mut sheet: RgbaImage = ImageBuffer::new(total_width, frame_height);
    for (index, frame) in frames.iter().enumerate() {
        if frame.width() != frame_width || frame.height() != frame_height {
            return Err(anyhow!("all frames must share the same dimensions"));
        }

        let offset_x = frame_width * index as u32;
        sheet
            .copy_from(&frame.to_rgba8(), offset_x, 0)
            .map_err(|_| anyhow!("failed to copy frame {index} into sprite sheet"))?;
    }

    Ok(DynamicImage::ImageRgba8(sheet))
}

/// 将 DynamicImage 编码为 PNG bytes
fn encode_png(img: &DynamicImage) -> anyhow::Result<Vec<u8>> {
    let mut buf = Cursor::new(Vec::new());
    img.write_to(&mut buf, ImageFormat::Png)?;
    Ok(buf.into_inner())
}

/// 完整后处理流水线
pub fn process_grid(grid_bytes: &[u8], cols: u32, rows: u32) -> anyhow::Result<ProcessedSprite> {
    let start = Instant::now();
    let frames = split_grid(grid_bytes, cols, rows)?;
    let encoded_frames = frames
        .iter()
        .map(encode_png)
        .collect::<anyhow::Result<Vec<_>>>()?;
    let sprite_sheet = compose_sprite_sheet(&frames)?;
    let sprite_sheet_bytes = encode_png(&sprite_sheet)?;

    info!(
        elapsed_ms = start.elapsed().as_millis(),
        frame_count = encoded_frames.len(),
        sprite_sheet_bytes = sprite_sheet_bytes.len(),
        "grid post-processing completed"
    );

    Ok(ProcessedSprite {
        grid_image: grid_bytes.to_vec(),
        frames: encoded_frames,
        sprite_sheet: sprite_sheet_bytes,
    })
}
