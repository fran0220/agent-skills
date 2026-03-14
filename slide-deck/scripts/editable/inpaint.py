"""Background text removal: simple fill for uniform backgrounds + Gemini inpainting for complex."""
from __future__ import annotations

import base64
import io
import re
import sys
from typing import Optional

from PIL import Image, ImageDraw

from .common import TextRegion, IMAGE_MODEL, image_to_base64

MASK_EXPAND_PX = 12
UNIFORM_FILL_EXTRA_PX = 4


def _expanded_rect(left, top, width, height, img_w, img_h, pad_px):
    """Expand a rect by pad_px pixels, clamping to image bounds."""
    return (
        max(0, int(left - pad_px)),
        max(0, int(top - pad_px)),
        min(img_w, int(left + width + pad_px)),
        min(img_h, int(top + height + pad_px)),
    )


def _sample_border_color(img, x0, y0, x1, y1, sample_depth=3):
    """
    采样 bbox 边框外围像素，推断背景色。
    从四条边外侧取样，返回出现最多的颜色的中位数。
    """
    w, h = img.size
    pixels = []

    for d in range(1, sample_depth + 1):
        # 上边
        for x in range(max(0, x0), min(w, x1)):
            if y0 - d >= 0:
                pixels.append(img.getpixel((x, y0 - d)))
        # 下边
        for x in range(max(0, x0), min(w, x1)):
            if y1 + d - 1 < h:
                pixels.append(img.getpixel((x, y1 + d - 1)))
        # 左边
        for y in range(max(0, y0), min(h, y1)):
            if x0 - d >= 0:
                pixels.append(img.getpixel((x0 - d, y)))
        # 右边
        for y in range(max(0, y0), min(h, y1)):
            if x1 + d - 1 < w:
                pixels.append(img.getpixel((x1 + d - 1, y)))

    if not pixels:
        return (255, 255, 255)

    # 按颜色排序取中位数
    pixels.sort(key=lambda p: (p[0], p[1], p[2]))
    return pixels[len(pixels) // 2]


def _is_uniform_background(img, x0, y0, x1, y1, threshold=40):
    """检查 bbox 周围背景是否相对均匀（低方差 = 纯色/渐变）。"""
    w, h = img.size
    pixels = []
    sample_depth = 5

    for d in range(1, sample_depth + 1):
        for x in range(max(0, x0), min(w, x1), 3):  # 每3像素采一个
            if y0 - d >= 0:
                pixels.append(img.getpixel((x, y0 - d)))
            if y1 + d - 1 < h:
                pixels.append(img.getpixel((x, y1 + d - 1)))

    if len(pixels) < 4:
        return True, (255, 255, 255)

    # 计算 RGB 各通道标准差
    rs = [p[0] for p in pixels]
    gs = [p[1] for p in pixels]
    bs = [p[2] for p in pixels]

    def stdev(vals):
        mean = sum(vals) / len(vals)
        return (sum((v - mean) ** 2 for v in vals) / len(vals)) ** 0.5

    max_std = max(stdev(rs), stdev(gs), stdev(bs))
    median_color = _sample_border_color(img, x0, y0, x1, y1)

    return max_std < threshold, median_color


def _extract_image_from_response(msg) -> Optional[str]:
    """从 Gemini 图像生成响应中提取 base64 图片数据。"""
    # 方式 1: msg.images
    images = getattr(msg, 'images', None)
    if images and len(images) > 0:
        img_item = images[0]
        if isinstance(img_item, dict):
            url = img_item.get("image_url", {}).get("url") or img_item.get("url", "")
        else:
            url = getattr(getattr(img_item, 'image_url', None), 'url', '') or getattr(img_item, 'url', '')
        if url.startswith("data:"):
            return url.split(",", 1)[1]
        elif url:
            return url

    # 方式 2: content 中的 base64
    content = msg.content or ""
    b64_match = re.search(r'data:image/[^;]+;base64,([A-Za-z0-9+/=\n]+)', content)
    if b64_match:
        return b64_match.group(1).replace("\n", "")

    # 方式 3: model_extra
    raw_dict = getattr(msg, 'model_extra', None) or {}
    for key in ['images', 'image']:
        if key in raw_dict:
            val = raw_dict[key]
            if isinstance(val, list) and val:
                val = val[0]
            if isinstance(val, dict):
                url = val.get("image_url", {}).get("url") or val.get("url", "")
            elif isinstance(val, str):
                url = val
            else:
                continue
            if url.startswith("data:"):
                return url.split(",", 1)[1]
            elif url:
                return url

    return None


def remove_text_from_image(
    client,
    image_path: str,
    regions: list[TextRegion],
    output_path: str,
) -> str:
    """
    混合策略去文字：
    - 简单背景（纯色/渐变）：直接用周围背景色填充（100% 可靠）
    - 复杂背景（图案/图片）：调用 Gemini inpainting

    最终用 mask composite 确保非文字区域严格保留原图。
    """
    img = Image.open(image_path).convert("RGB")
    w, h = img.size
    result = img.copy()
    result_draw = ImageDraw.Draw(result)

    complex_blocks = []  # 需要 AI inpainting 的区域

    print("  3a. 分析每个文字块的背景...")
    for i, region in enumerate(regions):
        x0, y0, x1, y1 = _expanded_rect(
            region.left, region.top, region.width, region.height,
            w, h, MASK_EXPAND_PX
        )

        is_uniform, bg_color = _is_uniform_background(img, x0, y0, x1, y1)

        if is_uniform:
            # 简单背景：用更大 pad 填充确保覆盖抗锯齿边缘
            fx0, fy0, fx1, fy1 = _expanded_rect(
                region.left, region.top, region.width, region.height,
                w, h, MASK_EXPAND_PX + UNIFORM_FILL_EXTRA_PX
            )
            result_draw.rectangle([fx0, fy0, fx1, fy1], fill=bg_color)
        else:
            # 复杂背景：记录，稍后 AI 处理
            complex_blocks.append((i, x0, y0, x1, y1))

    simple_count = len(regions) - len(complex_blocks)
    print(f"  简单填充: {simple_count} 块, 需 AI inpainting: {len(complex_blocks)} 块")

    # 如果有复杂背景区域，调用 Gemini inpainting
    if complex_blocks and client:
        print("  3b. Gemini inpainting 处理复杂背景区域...")

        # 生成仅包含复杂区域的 mask
        mask = Image.new("L", (w, h), 0)
        mask_draw = ImageDraw.Draw(mask)

        # 在已经简单填充过的 result 上标注复杂区域
        marked = result.copy()
        marked_draw = ImageDraw.Draw(marked)

        for _, x0, y0, x1, y1 in complex_blocks:
            mask_draw.rectangle([x0, y0, x1, y1], fill=255)
            marked_draw.rectangle([x0, y0, x1, y1], fill=(0, 0, 0))

        # 发双图给 Gemini
        buf_orig = io.BytesIO()
        img.save(buf_orig, format="PNG")
        original_b64 = base64.b64encode(buf_orig.getvalue()).decode()

        buf_marked = io.BytesIO()
        marked.save(buf_marked, format="PNG")
        marked_b64 = base64.b64encode(buf_marked.getvalue()).decode()

        resp = client.chat.completions.create(
            model=IMAGE_MODEL,
            messages=[
                {
                    "role": "user",
                    "content": [
                        {
                            "type": "text",
                            "text": (
                                "You are a professional image inpainting expert. "
                                "I provide two images:\n"
                                "1. The original presentation slide\n"
                                "2. The same slide with black rectangles covering areas to be filled\n\n"
                                "Redraw ONLY the black rectangle areas. Remove any text/numbers in those areas "
                                "and fill them seamlessly with the surrounding background pattern and colors. "
                                "Do NOT add any new text. Do NOT modify areas outside the black rectangles. "
                                "Output the complete image."
                            ),
                        },
                        {
                            "type": "image_url",
                            "image_url": {"url": f"data:image/png;base64,{original_b64}"},
                        },
                        {
                            "type": "image_url",
                            "image_url": {"url": f"data:image/png;base64,{marked_b64}"},
                        },
                    ],
                }
            ],
            max_tokens=2000,
        )

        ai_b64 = _extract_image_from_response(resp.choices[0].message)
        if ai_b64:
            ai_data = base64.b64decode(ai_b64)
            ai_img = Image.open(io.BytesIO(ai_data)).convert("RGB")
            if ai_img.size != img.size:
                ai_img = ai_img.resize(img.size, Image.LANCZOS)

            # mask composite: 复杂区域用 AI 结果，其余用已处理的 result
            result = Image.composite(ai_img, result, mask)
            print(f"  AI inpainting 完成，合成 {len(complex_blocks)} 个复杂区域")
        else:
            print("  警告：Gemini 未返回图片，复杂区域用背景色近似填充", file=sys.stderr)
            # 回退：对复杂区域也用边缘色填充
            for _, x0, y0, x1, y1 in complex_blocks:
                bg_color = _sample_border_color(img, x0, y0, x1, y1)
                result_draw.rectangle([x0, y0, x1, y1], fill=bg_color)

    result.save(output_path)
    print(f"  去文字背景图已保存: {output_path}")
    return output_path
