"""Layout extraction: Baidu OCR + element cropping."""
from __future__ import annotations

import re
from pathlib import Path
from typing import Optional

import requests
from PIL import Image

from .common import TextRegion, BAIDU_OCR_URL, image_to_base64

# OCR merge params
OCR_MERGE_Y_TOLERANCE_PX = 14
OCR_MERGE_MAX_GAP_PX = 28


def merge_ocr_blocks(blocks: list[dict]) -> list[dict]:
    """Merge OCR blocks on the same line to reduce fragmentation."""
    if not blocks:
        return blocks

    blocks = sorted(blocks, key=lambda b: (b["top"], b["left"]))
    merged = [dict(blocks[0])]

    for block in blocks[1:]:
        block = dict(block)
        prev = merged[-1]

        prev_cy = prev["top"] + prev["height"] / 2
        curr_cy = block["top"] + block["height"] / 2
        gap_x = block["left"] - (prev["left"] + prev["width"])

        similar_y = abs(prev_cy - curr_cy) <= max(
            OCR_MERGE_Y_TOLERANCE_PX,
            min(prev["height"], block["height"]) * 0.35,
        )
        height_ratio = max(prev["height"], block["height"]) / max(
            1, min(prev["height"], block["height"])
        )
        similar_height = height_ratio <= 1.6
        close_x = gap_x <= max(
            OCR_MERGE_MAX_GAP_PX,
            min(prev["height"], block["height"]) * 0.6,
        )

        if similar_y and similar_height and close_x:
            new_left = min(prev["left"], block["left"])
            new_top = min(prev["top"], block["top"])
            new_right = max(
                prev["left"] + prev["width"], block["left"] + block["width"]
            )
            new_bottom = max(
                prev["top"] + prev["height"], block["top"] + block["height"]
            )

            prev["text"] = f'{prev["text"]}{block["text"]}'
            prev["left"] = new_left
            prev["top"] = new_top
            prev["width"] = new_right - new_left
            prev["height"] = new_bottom - new_top
            prev["probability"] = min(
                prev.get("probability", 1), block.get("probability", 1)
            )
        else:
            merged.append(block)

    if len(merged) != len(blocks):
        print(f"  OCR 合并: {len(blocks)} -> {len(merged)} blocks")
    return merged


def ocr_extract_text_positions(
    image_path: str, access_token: str
) -> list[TextRegion]:
    """Call Baidu accurate OCR API to extract text positions.

    Returns list of TextRegion with pixel-level bounding boxes.
    """
    b64 = image_to_base64(image_path)

    resp = requests.post(
        BAIDU_OCR_URL,
        params={"access_token": access_token},
        headers={"Content-Type": "application/x-www-form-urlencoded"},
        data={
            "image": b64,
            "recognize_granularity": "big",
            "detect_direction": "true",
            "paragraph": "true",
            "probability": "true",
        },
    )
    resp.raise_for_status()
    result = resp.json()

    if "error_code" in result:
        raise RuntimeError(
            f"百度 OCR 错误: {result['error_code']} - {result.get('error_msg', '')}"
        )

    words = result.get("words_result", [])
    blocks = []
    for w in words:
        loc = w.get("location", {})
        blocks.append({
            "text": w.get("words", ""),
            "left": loc.get("left", 0),
            "top": loc.get("top", 0),
            "width": loc.get("width", 0),
            "height": loc.get("height", 0),
            "probability": w.get("probability", {}).get("average", 0),
        })

    blocks = merge_ocr_blocks(blocks)
    print(f"  百度 OCR 识别到 {len(blocks)} 个文本块")

    regions = [
        TextRegion(
            text=b["text"],
            left=b["left"],
            top=b["top"],
            width=b["width"],
            height=b["height"],
            probability=b.get("probability", 1.0),
        )
        for b in blocks
    ]
    return regions


def crop_elements(
    image_path: str, regions: list[TextRegion], output_dir: str
) -> None:
    """Crop each text region from the source image and save as individual PNG.

    This is the banana-slides pattern for accurate per-element style extraction.
    """
    out = Path(output_dir)
    out.mkdir(parents=True, exist_ok=True)

    img = Image.open(image_path)
    img_w, img_h = img.size

    for i, region in enumerate(regions):
        pad = max(4, int(region.height * 0.15))
        x0 = max(0, region.left - pad)
        y0 = max(0, region.top - pad)
        x1 = min(img_w, region.left + region.width + pad)
        y1 = min(img_h, region.top + region.height + pad)

        cropped = img.crop((x0, y0, x1, y1))
        filename = f"{i:03d}_{region.element_type}.png"
        save_path = out / filename
        cropped.save(str(save_path))
        region.crop_path = str(save_path)

    print(f"  裁切了 {len(regions)} 个元素到 {output_dir}")
