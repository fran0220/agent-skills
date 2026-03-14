"""Dual-path text style extraction: local crop → color, global image → bold/align."""
from __future__ import annotations

import json
import re
import sys
from concurrent.futures import ThreadPoolExecutor, as_completed
from typing import Optional

from .common import (
    TextRegion, TextStyle, ColoredSegment,
    VISION_MODEL, image_to_base64, hex_to_rgb,
)


# ═══════════════════════════════════════════════════════
#  Local Path: per-element crop → precise color
# ═══════════════════════════════════════════════════════

_DEFAULT_COLOR_RESULT = {"color": "#000000", "colored_segments": []}


def _strip_code_block(text: str) -> str:
    """Remove markdown code fences from VLM output."""
    text = text.strip()
    text = re.sub(r"^```(?:json)?\s*\n?", "", text)
    text = re.sub(r"\n?```\s*$", "", text)
    return text.strip()


def _extract_local_color(client, region: TextRegion) -> dict:
    """Send cropped element image to VLM for precise color extraction."""
    if region.crop_path is None:
        return dict(_DEFAULT_COLOR_RESULT)

    prompt = (
        f'你的任务是精确识别这张图片中的文字内容和样式，返回JSON格式的结果。\n\n'
        f'图片中的文字内容是: "{region.text}"\n\n'
        f'## 核心任务\n'
        f'请仔细观察图片，精确识别：\n'
        f'1. **颜色** - 每个字/词的实际颜色\n'
        f'2. **颜色分割** - 一行文字可能有多种颜色，按颜色分割成片段\n\n'
        f'## 注意事项\n'
        f'- 相邻相同颜色的文字应合并为一个片段\n'
        f'- 一般只有1-2种颜色\n\n'
        f'## 输出格式\n'
        f'返回JSON对象：\n'
        f'{{\n'
        f'    "colored_segments": [\n'
        f'        {{"text": "示例文字", "color": "#000000"}},\n'
        f'        {{"text": "重点", "color": "#FF0000"}}\n'
        f'    ]\n'
        f'}}\n'
        f'只返回JSON对象，不要包含其他文字或markdown代码块。'
    )

    try:
        b64 = image_to_base64(region.crop_path)
        resp = client.chat.completions.create(
            model=VISION_MODEL,
            messages=[{
                "role": "user",
                "content": [
                    {"type": "image_url", "image_url": {"url": f"data:image/png;base64,{b64}"}},
                    {"type": "text", "text": prompt},
                ],
            }],
            max_tokens=2048,
            temperature=0.1,
        )
        raw = resp.choices[0].message.content
        data = json.loads(_strip_code_block(raw))
        segments = data.get("colored_segments", [])
        color = segments[0]["color"] if segments else "#000000"
        return {"color": color, "colored_segments": segments}
    except Exception as e:
        print(f"  ⚠ 局部颜色提取失败 [{region.text[:20]}]: {e}", file=sys.stderr)
        return dict(_DEFAULT_COLOR_RESULT)


# ═══════════════════════════════════════════════════════
#  Global Path: full slide image → bold / italic / align
# ═══════════════════════════════════════════════════════

def _extract_global_layout(client, image_path: str, regions: list[TextRegion]) -> list[dict]:
    """Send full slide image + all region bboxes to VLM for bold/italic/alignment."""
    defaults = [{"bold": False, "italic": False, "align": "left"} for _ in regions]
    if not regions:
        return defaults

    elements = []
    for i, r in enumerate(regions):
        elements.append({
            "index": i,
            "bbox": [r.left, r.top, r.left + r.width, r.top + r.height],
            "content": r.text,
        })
    elements_json = json.dumps(elements, ensure_ascii=False, indent=2)

    prompt = (
        f'你是一位专业的 PPT 排版分析专家。请分析这张图片中所有标注的文字区域的样式属性。\n\n'
        f'以下是已提取的文字元素及其位置信息：\n{elements_json}\n\n'
        f'请仔细观察图片，对比每个文字区域在图片中的实际视觉效果，为每个元素分析：\n'
        f'1. is_bold: 是否为粗体 (true/false) - 观察笔画粗细，标题通常是粗体\n'
        f'2. is_italic: 是否为斜体 (true/false)\n'
        f'3. text_alignment: 文字对齐方式 ("left"/"center"/"right")\n\n'
        f'返回 JSON 数组，每个对象包含: index, is_bold, is_italic, text_alignment\n'
        f'只返回JSON数组，不要包含其他文字或markdown代码块。'
    )

    try:
        b64 = image_to_base64(image_path)
        resp = client.chat.completions.create(
            model=VISION_MODEL,
            messages=[{
                "role": "user",
                "content": [
                    {"type": "image_url", "image_url": {"url": f"data:image/png;base64,{b64}"}},
                    {"type": "text", "text": prompt},
                ],
            }],
            max_tokens=4096,
            temperature=0.1,
        )
        raw = resp.choices[0].message.content
        items = json.loads(_strip_code_block(raw))

        results = list(defaults)
        for item in items:
            idx = item.get("index")
            if idx is not None and 0 <= idx < len(regions):
                results[idx] = {
                    "bold": bool(item.get("is_bold", False)),
                    "italic": bool(item.get("is_italic", False)),
                    "align": item.get("text_alignment", "left"),
                }
        return results
    except Exception as e:
        print(f"  ⚠ 全局布局提取失败: {e}", file=sys.stderr)
        return defaults


# ═══════════════════════════════════════════════════════
#  Hybrid Merge
# ═══════════════════════════════════════════════════════

def extract_styles_hybrid(client, image_path: str, regions: list[TextRegion]) -> None:
    """Main entry: local color + global layout in parallel, merge into region.style."""
    if not regions:
        return

    local_results: list[dict] = []
    global_results: list[dict] = []

    with ThreadPoolExecutor(max_workers=2) as pool:
        # Global path as one future
        global_future = pool.submit(_extract_global_layout, client, image_path, regions)

        # Local path: sequential per-region color extraction
        def _run_local():
            return [_extract_local_color(client, r) for r in regions]

        local_future = pool.submit(_run_local)

        global_results = global_future.result()
        local_results = local_future.result()

    # Merge
    for i, region in enumerate(regions):
        lc = local_results[i]
        gl = global_results[i]

        colored_segments = [ColoredSegment(**s) for s in lc.get("colored_segments", [])]

        region.style = TextStyle(
            color=lc.get("color", "#000000"),
            bold=gl.get("bold", False),
            italic=gl.get("italic", False),
            align=gl.get("align", "left"),
            colored_segments=colored_segments,
        )

    # Summary
    bold_count = sum(1 for r in regions if r.style and r.style.bold)
    colored_count = sum(1 for r in regions if r.style and r.style.colored_segments)
    print(f"  ✓ 样式提取完成: {len(regions)} 个区域, {bold_count} 个粗体, {colored_count} 个多色")


# ═══════════════════════════════════════════════════════
#  Simple Fallback (single VLM call)
# ═══════════════════════════════════════════════════════

def extract_styles_simple(client, image_path: str, regions: list[TextRegion]) -> None:
    """Fallback: single VLM call for all styles when crops aren't available."""
    if not regions:
        return

    block_lines = []
    for i, r in enumerate(regions):
        block_lines.append(f'{i}: "{r.text}" (位置: left={r.left}, top={r.top})')
    blocks_desc = "\n".join(block_lines)

    prompt = (
        f'你是一位专业的 PPT 排版分析专家。请分析这张图片中所有文字区域的样式。\n\n'
        f'文字区域列表：\n{blocks_desc}\n\n'
        f'请为每个文字区域分析以下属性：\n'
        f'1. color: 文字颜色 (十六进制，如 "#000000")\n'
        f'2. is_bold: 是否为粗体 (true/false)\n'
        f'3. text_alignment: 对齐方式 ("left"/"center"/"right")\n\n'
        f'返回 JSON 数组，每个对象包含: index, color, is_bold, text_alignment\n'
        f'只返回JSON数组，不要包含其他文字或markdown代码块。'
    )

    try:
        b64 = image_to_base64(image_path)
        resp = client.chat.completions.create(
            model=VISION_MODEL,
            messages=[{
                "role": "user",
                "content": [
                    {"type": "image_url", "image_url": {"url": f"data:image/png;base64,{b64}"}},
                    {"type": "text", "text": prompt},
                ],
            }],
            max_tokens=4096,
            temperature=0.1,
        )
        raw = resp.choices[0].message.content
        items = json.loads(_strip_code_block(raw))

        for item in items:
            idx = item.get("index")
            if idx is not None and 0 <= idx < len(regions):
                regions[idx].style = TextStyle(
                    color=item.get("color", "#000000"),
                    bold=bool(item.get("is_bold", False)),
                    align=item.get("text_alignment", "left"),
                )

        # Fill any regions that weren't covered
        for r in regions:
            if r.style is None:
                r.style = TextStyle()

        print(f"  ✓ 简单样式提取完成: {len(regions)} 个区域")

    except Exception as e:
        print(f"  ⚠ 简单样式提取失败: {e}", file=sys.stderr)
        for r in regions:
            if r.style is None:
                r.style = TextStyle()
