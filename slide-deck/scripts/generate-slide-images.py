#!/usr/bin/env python3
"""
Generate slide images from an outline file using Gemini image model.

Usage:
  python3 generate-slide-images.py <outline_path> --slides 6,9,10
  python3 generate-slide-images.py <outline_path> --all
  python3 generate-slide-images.py <outline_path> --slides 6,9,10 --parallel 3

Requires:
  - GOOGLE_API_KEY environment variable (or .env file)
  - Optional: GOOGLE_API_BASE for proxy
  - Python 3.9+ with google-genai, Pillow
"""
from __future__ import annotations

import argparse
import os
import re
import sys
import time
from concurrent.futures import ThreadPoolExecutor, as_completed
from io import BytesIO
from pathlib import Path

from google import genai
from google.genai import types
from PIL import Image

# ---------------------------------------------------------------------------
# Config
# ---------------------------------------------------------------------------
IMAGE_MODEL = "gemini-3.1-flash-image-preview"
GENAI_TIMEOUT = 180  # seconds per request
MAX_RETRIES = 1

# Base prompt (inline, matching skill's references/base-prompt.md)
BASE_PROMPT = """Create a presentation slide image following these guidelines:

## Image Specifications
- **Type**: Presentation slide
- **Aspect Ratio**: 16:9 (landscape)
- **Style**: Professional slide deck

## Core Principles
- Hand-drawn quality throughout - NO realistic or photographic elements
- If content involves sensitive or copyrighted figures, create stylistically similar alternatives
- NO slide numbers, page numbers, footers, headers, or logos
- Clean, uncluttered layouts with clear visual hierarchy
- Each slide conveys ONE clear message

## Text Style (CRITICAL)
- **ALL text MUST match the designated style exactly**
- Title text: Large, bold, immediately readable
- Body text: Clear, legible, appropriate sizing
- Max 3-4 text elements per slide
- Font rendering must match the style aesthetic

## Layout Principles
- Visual Hierarchy: Most important element gets most visual weight
- Breathing Room: Generous margins and spacing between elements
- Alignment: Consistent alignment creates professional feel
- Balance: Distribute visual weight evenly
- Focal Point: One clear area draws the eye first

## Language
- Use the same language as the content provided below for all text elements
- Match punctuation style to the content language
- Write in direct, confident language
"""


# ---------------------------------------------------------------------------
# Env loading
# ---------------------------------------------------------------------------
def _apply_env_file(env_file: Path) -> bool:
    if not env_file.exists():
        return False
    for line in env_file.read_text().splitlines():
        line = line.strip()
        if line and not line.startswith("#") and "=" in line:
            k, v = line.split("=", 1)
            os.environ.setdefault(k.strip(), v.strip())
    return True


def load_env(outline_path: str) -> None:
    """Load .env from project root (walk up from outline); fall back to skill dir."""
    p = Path(outline_path).resolve().parent
    for _ in range(10):
        if _apply_env_file(p / ".env"):
            return
        if p.parent == p:
            break
        p = p.parent
    # Fallback: skill directory
    _apply_env_file(Path(__file__).resolve().parent.parent / ".env")


def make_client() -> genai.Client:
    """Create Google GenAI client."""
    api_key = os.environ.get("GOOGLE_API_KEY", "")
    if not api_key:
        print("Error: GOOGLE_API_KEY must be set", file=sys.stderr)
        sys.exit(1)
    api_base = os.environ.get("GOOGLE_API_BASE", None)
    timeout_ms = int(GENAI_TIMEOUT * 1000)
    http_opts = types.HttpOptions(timeout=timeout_ms)
    if api_base:
        http_opts = types.HttpOptions(timeout=timeout_ms, base_url=api_base)
    return genai.Client(api_key=api_key, http_options=http_opts)


# ---------------------------------------------------------------------------
# Outline parsing
# ---------------------------------------------------------------------------
def extract_style_instructions(text: str) -> str:
    m = re.search(r"<STYLE_INSTRUCTIONS>(.*?)</STYLE_INSTRUCTIONS>", text, re.DOTALL)
    return m.group(0) if m else ""


def extract_slide_content(text: str, slide_num: int) -> str | None:
    """Extract a single slide block by its number."""
    pattern = rf"## Slide {slide_num} of \d+\n(.*?)(?=\n---|\Z)"
    m = re.search(pattern, text, re.DOTALL)
    return m.group(0) if m else None


def extract_filename(slide_block: str) -> str | None:
    m = re.search(r"\*\*Filename\*\*:\s*(.+\.png)", slide_block)
    return m.group(1).strip() if m else None


def list_all_slide_nums(text: str) -> list[int]:
    return [int(n) for n in re.findall(r"## Slide (\d+) of \d+", text)]


# ---------------------------------------------------------------------------
# Image generation
# ---------------------------------------------------------------------------
def generate_image(client: genai.Client, prompt: str, output_path: Path) -> bool:
    """Call Gemini image model and save result as PNG."""
    resp = client.models.generate_content(
        model=IMAGE_MODEL,
        contents=prompt,
        config=types.GenerateContentConfig(
            response_modalities=['TEXT', 'IMAGE'],
            image_config=types.ImageConfig(
                aspect_ratio='16:9',
            ),
        ),
    )

    # Extract the last image from response parts
    last_image = None
    if resp.parts:
        for part in resp.parts:
            if part.text is not None:
                continue
            try:
                image = part.as_image()
                if isinstance(image, Image.Image):
                    last_image = image
                elif hasattr(image, 'image_bytes') and image.image_bytes:
                    last_image = Image.open(BytesIO(image.image_bytes))
                elif hasattr(image, '_pil_image') and image._pil_image:
                    last_image = image._pil_image
            except Exception:
                continue

    if last_image is None:
        return False

    last_image.save(str(output_path), format="PNG")
    return True


# Global client instance (set in main, shared across threads)
_client: genai.Client | None = None


def process_slide(
    slide_num: int,
    outline_text: str,
    style_block: str,
    output_dir: Path,
) -> tuple[int, str, bool, str]:
    """Generate one slide. Returns (num, filename, success, message)."""
    content = extract_slide_content(outline_text, slide_num)
    if not content:
        return slide_num, "", False, "Slide not found in outline"

    filename = extract_filename(content)
    if not filename:
        return slide_num, "", False, "No filename in slide block"

    prompt = f"{BASE_PROMPT}\n\n{style_block}\n\n## SLIDE CONTENT\n\n{content}"
    output_path = output_dir / filename

    for attempt in range(1 + MAX_RETRIES):
        try:
            ok = generate_image(_client, prompt, output_path)
            if ok:
                size_kb = output_path.stat().st_size / 1024
                return slide_num, filename, True, f"{size_kb:.0f} KB"
            else:
                msg = "No image in response"
        except Exception as e:
            msg = str(e)
        if attempt < MAX_RETRIES:
            time.sleep(2)

    return slide_num, filename, False, msg


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
def main() -> None:
    global _client

    parser = argparse.ArgumentParser(description="Generate slide images from outline")
    parser.add_argument("outline", help="Path to outline.md")
    parser.add_argument("--slides", help="Comma-separated slide numbers (e.g. 6,9,10)")
    parser.add_argument("--all", action="store_true", help="Generate all slides")
    parser.add_argument("--parallel", type=int, default=2, help="Max parallel requests (default 2)")
    args = parser.parse_args()

    outline_path = Path(args.outline).resolve()
    if not outline_path.exists():
        print(f"Error: {outline_path} not found")
        sys.exit(1)

    load_env(str(outline_path))

    _client = make_client()

    outline_text = outline_path.read_text()
    style_block = extract_style_instructions(outline_text)
    output_dir = outline_path.parent

    if args.all:
        slide_nums = list_all_slide_nums(outline_text)
    elif args.slides:
        slide_nums = [int(n.strip()) for n in args.slides.split(",")]
    else:
        print("Error: specify --slides or --all")
        sys.exit(1)

    print(f"Outline:  {outline_path}")
    print(f"Output:   {output_dir}")
    print(f"Model:    {IMAGE_MODEL}")
    print(f"Slides:   {slide_nums}")
    print(f"Parallel: {args.parallel}")
    print(f"{'─' * 50}")

    results = []
    with ThreadPoolExecutor(max_workers=args.parallel) as pool:
        futures = {
            pool.submit(process_slide, num, outline_text, style_block, output_dir): num
            for num in slide_nums
        }
        for future in as_completed(futures):
            num, filename, ok, msg = future.result()
            status = "✓" if ok else "✗"
            print(f"  {status} Slide {num:2d}  {filename or '???':<40s}  {msg}")
            results.append((num, ok))

    # Summary
    ok_count = sum(1 for _, ok in results if ok)
    print(f"{'─' * 50}")
    print(f"Done: {ok_count}/{len(results)} slides generated")

    if ok_count < len(results):
        sys.exit(1)


if __name__ == "__main__":
    main()
