#!/usr/bin/env python3
"""
修复已有 PPTX 中的文字重叠和背景残影问题。

针对已手动编辑过的 PPTX（不需要源图片），直接从 PPTX 内嵌的背景图
采样颜色，为每个透明文本框添加实心背景填充，遮住底图残留文字。

同时检测并标记重叠的文本框。

Usage:
  python3 fix-existing-pptx.py <input.pptx>
  python3 fix-existing-pptx.py <input.pptx> -o <output.pptx>
  python3 fix-existing-pptx.py <input.pptx> --skip-empty  # 跳过空文本框
  python3 fix-existing-pptx.py <input.pptx> --report-only  # 只报告不修改

依赖: python-pptx, Pillow
"""
from __future__ import annotations

import argparse
import io
import os
import sys
from pathlib import Path

from PIL import Image
from pptx import Presentation
from pptx.util import Emu
from pptx.dml.color import RGBColor
from pptx.enum.shapes import MSO_SHAPE_TYPE
from pptx.enum.dml import MSO_FILL

DEFAULT_FONT_NAME_CJK = "Microsoft YaHei"
DEFAULT_FONT_NAME_LATIN = "Arial"

# Slide dimensions in EMU (standard 16:9)
SLIDE_W_EMU = 12192000
SLIDE_H_EMU = 6858000


def extract_bg_image(slide) -> Image.Image | None:
    """Extract the first full-size picture from slide as background."""
    for shape in slide.shapes:
        if shape.shape_type == MSO_SHAPE_TYPE.PICTURE:
            try:
                blob = shape.image.blob
                img = Image.open(io.BytesIO(blob)).convert("RGB")
                return img
            except Exception:
                continue
    return None


def sample_bg_color_for_box(
    bg_img: Image.Image,
    slide_w_emu: int,
    slide_h_emu: int,
    left_emu: int,
    top_emu: int,
    width_emu: int,
    height_emu: int,
    sample_depth: int = 5,
) -> tuple[int, int, int]:
    """
    Sample background image color around a textbox's position.
    Maps EMU coordinates to image pixel coordinates, samples border pixels.
    """
    img_w, img_h = bg_img.size

    # EMU → pixel coordinate mapping
    scale_x = img_w / slide_w_emu
    scale_y = img_h / slide_h_emu

    x0 = int(left_emu * scale_x)
    y0 = int(top_emu * scale_y)
    x1 = int((left_emu + width_emu) * scale_x)
    y1 = int((top_emu + height_emu) * scale_y)

    # Clamp
    x0 = max(0, min(x0, img_w - 1))
    y0 = max(0, min(y0, img_h - 1))
    x1 = max(x0 + 1, min(x1, img_w))
    y1 = max(y0 + 1, min(y1, img_h))

    pixels = []
    for d in range(1, sample_depth + 1):
        # Top edge
        for x in range(x0, x1, max(1, (x1 - x0) // 20)):
            if y0 - d >= 0:
                pixels.append(bg_img.getpixel((x, y0 - d)))
        # Bottom edge
        for x in range(x0, x1, max(1, (x1 - x0) // 20)):
            if y1 + d - 1 < img_h:
                pixels.append(bg_img.getpixel((x, y1 + d - 1)))
        # Left edge
        for y in range(y0, y1, max(1, (y1 - y0) // 10)):
            if x0 - d >= 0:
                pixels.append(bg_img.getpixel((x0 - d, y)))
        # Right edge
        for y in range(y0, y1, max(1, (y1 - y0) // 10)):
            if x1 + d - 1 < img_w:
                pixels.append(bg_img.getpixel((x1 + d - 1, y)))

    if not pixels:
        return (255, 255, 255)

    pixels.sort(key=lambda p: (p[0], p[1], p[2]))
    return pixels[len(pixels) // 2]


def detect_overlapping_textboxes(textboxes: list) -> list[tuple]:
    """Detect pairs of overlapping text boxes."""
    overlaps = []
    for i in range(len(textboxes)):
        for j in range(i + 1, len(textboxes)):
            a, b = textboxes[i], textboxes[j]
            a_l, a_t, a_r, a_b = a.left, a.top, a.left + a.width, a.top + a.height
            b_l, b_t, b_r, b_b = b.left, b.top, b.left + b.width, b.top + b.height

            # Check intersection
            if a_l < b_r and a_r > b_l and a_t < b_b and a_b > b_t:
                # Calculate overlap area
                overlap_w = min(a_r, b_r) - max(a_l, b_l)
                overlap_h = min(a_b, b_b) - max(a_t, b_t)
                overlap_area = overlap_w * overlap_h
                min_area = min(a.width * a.height, b.width * b.height)
                overlap_ratio = overlap_area / max(1, min_area)
                if overlap_ratio > 0.15:  # >15% overlap
                    overlaps.append((i, j, overlap_ratio))
    return overlaps


def has_cjk(text: str) -> bool:
    return any(ord(c) > 0x2E80 for c in text)


def fix_pptx(
    input_path: str,
    output_path: str,
    skip_empty: bool = False,
    report_only: bool = False,
    fix_fonts: bool = True,
) -> None:
    prs = Presentation(input_path)

    slide_w = prs.slide_width
    slide_h = prs.slide_height

    total_fixed = 0
    total_overlaps = 0

    for si, slide in enumerate(prs.slides):
        slide_num = si + 1
        bg_img = extract_bg_image(slide)

        if bg_img is None:
            print(f"  Slide {slide_num}: 无背景图，跳过")
            continue

        textboxes = []
        for shape in slide.shapes:
            if shape.has_text_frame and shape.shape_type != MSO_SHAPE_TYPE.PICTURE:
                textboxes.append(shape)

        # Detect overlaps
        overlaps = detect_overlapping_textboxes(textboxes)
        if overlaps:
            total_overlaps += len(overlaps)
            for i, j, ratio in overlaps:
                txt_i = textboxes[i].text_frame.text[:30]
                txt_j = textboxes[j].text_frame.text[:30]
                print(f"  Slide {slide_num}: ⚠ 重叠 ({ratio:.0%}): \"{txt_i}\" ↔ \"{txt_j}\"")

        # Fix textbox fills
        fixed_count = 0
        for shape in textboxes:
            text = shape.text_frame.text.strip()
            if skip_empty and not text:
                continue

            # Check current fill
            needs_fix = False
            try:
                fill_type = shape.fill.type
                if fill_type != MSO_FILL.SOLID:
                    needs_fix = True
                else:
                    # Already solid, check if it's a reasonable color
                    try:
                        existing = shape.fill.fore_color.rgb
                    except:
                        needs_fix = True
            except:
                needs_fix = True

            if needs_fix and not report_only:
                bg_color = sample_bg_color_for_box(
                    bg_img, slide_w, slide_h,
                    shape.left, shape.top, shape.width, shape.height,
                )
                shape.fill.solid()
                shape.fill.fore_color.rgb = RGBColor(*bg_color)
                # Remove border
                shape.line.fill.background()
                fixed_count += 1

            # Fix fonts
            if fix_fonts and not report_only:
                for para in shape.text_frame.paragraphs:
                    para_text = para.text or text
                    font_name = DEFAULT_FONT_NAME_CJK if has_cjk(para_text) else DEFAULT_FONT_NAME_LATIN
                    if para.font.name != font_name:
                        para.font.name = font_name
                    # Also fix runs
                    for run in para.runs:
                        run_text = run.text or para_text
                        run_font = DEFAULT_FONT_NAME_CJK if has_cjk(run_text) else DEFAULT_FONT_NAME_LATIN
                        if run.font.name != run_font:
                            run.font.name = run_font

        total_fixed += fixed_count
        status = "报告" if report_only else "修复"
        if fixed_count > 0 or overlaps:
            print(f"  Slide {slide_num}: {status} {fixed_count} 个文本框填充, {len(overlaps)} 处重叠")

    if not report_only:
        prs.save(output_path)

    print(f"\n{'─' * 50}")
    print(f"总计: {total_fixed} 个文本框填充已修复, {total_overlaps} 处重叠检测")
    if not report_only:
        print(f"已保存: {output_path}")
    else:
        print("(报告模式，未修改文件)")


def main():
    parser = argparse.ArgumentParser(description="修复已有 PPTX 的文本框背景和重叠")
    parser.add_argument("input", help="输入 PPTX 路径")
    parser.add_argument("-o", "--output", help="输出路径（默认覆盖原文件）")
    parser.add_argument("--skip-empty", action="store_true", help="跳过空文本框")
    parser.add_argument("--report-only", action="store_true", help="只报告不修改")
    parser.add_argument("--no-fix-fonts", action="store_true", help="不修复字体")
    args = parser.parse_args()

    input_path = os.path.abspath(args.input)
    if not os.path.exists(input_path):
        print(f"错误：文件不存在: {input_path}", file=sys.stderr)
        sys.exit(1)

    output_path = args.output or input_path

    print(f"输入: {input_path}")
    print(f"输出: {output_path}")
    print(f"{'─' * 50}")

    fix_pptx(
        input_path,
        output_path,
        skip_empty=args.skip_empty,
        report_only=args.report_only,
        fix_fonts=not args.no_fix_fonts,
    )


if __name__ == "__main__":
    main()
