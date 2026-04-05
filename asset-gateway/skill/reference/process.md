# Process Commands

Image and video post-processing pipeline. All operations run server-side.

## Crop

Smart crop: trim transparent borders from an image.

```bash
# Tightest crop (remove all transparent padding)
asset-gateway process crop --input ./sprite.png --mode tightest --output-dir ./out

# Power-of-2 crop (trim then extend canvas to nearest power of 2)
asset-gateway process crop --input ./sprite.png --mode power_of2 --output-dir ./out
```

## Resize

Exact pixel resize (ignores aspect ratio).

```bash
asset-gateway process resize --input ./poster.png --width 1024 --height 1024 --output-dir ./out
```

## Compose

Combine multiple images into a sprite sheet. Supports horizontal, vertical, and grid layouts.

```bash
# Horizontal strip
asset-gateway process compose --input frame1.png frame2.png frame3.png frame4.png --direction horizontal --output-dir ./out

# Vertical strip
asset-gateway process compose --input frame*.png --direction vertical --output-dir ./out

# Grid layout (auto or explicit columns)
asset-gateway process compose --input frame*.png --direction grid --columns 4 --output-dir ./out

# With frame normalization (auto-crop + resize each frame before composing)
asset-gateway process compose --input frame*.png --direction horizontal --frame-width 64 --frame-height 64 --output-dir ./out

# With padding between frames
asset-gateway process compose --input frame*.png --direction grid --columns 4 --padding 2 --frame-width 64 --frame-height 64 --output-dir ./out
```

**Response** returns a single composed image with `local_path`, `width`, `height`.

## Extract Frames

Extract evenly-spaced frames from a video using ffmpeg. Returns multiple PNG files.

```bash
# Extract 8 frames (default)
asset-gateway process extract-frames --input ./video.mp4 --output-dir ./frames

# Extract specific count
asset-gateway process extract-frames --input ./video.mp4 --count 12 --output-dir ./frames

# From a URL
asset-gateway process extract-frames --input ./video_output.mp4 --count 8 --output-dir ./frames
```

**Response** returns multiple files with `local_paths[]` array:
```json
{
  "ok": true,
  "data": {
    "local_paths": ["./frames/frame_..._0000.png", "..."],
    "width": 1280,
    "height": 720,
    "operations_applied": ["extract_frames:8"]
  }
}
```

## Remove Background

AI-powered background removal using bgsweep API. Works on single or multiple images.

```bash
# Single image
asset-gateway process remove-bg --input ./frame.png --output-dir ./nobg

# Multiple images
asset-gateway process remove-bg --input frame1.png frame2.png frame3.png --output-dir ./nobg
```

**Response** for single input returns `local_path`; for multiple inputs returns `local_paths[]`.

> The bgsweep URL is configurable via `BGSWEEP_URL` environment variable (default: `https://bgsweep.com/api/remove-background`).

## Pipeline Chaining

Operations can be chained server-side in a single API call. The pipeline processes operations sequentially, passing output of one step as input to the next.

Common chains:
- `extract_frames → remove_bg → compose` — video to transparent sprite sheet
- `smart_crop → resize` — trim then scale
- `remove_bg → smart_crop → resize` — clean up, trim, scale

For the full sprite animation pipeline using these commands, see **reference/sprite-workflow.md**.
