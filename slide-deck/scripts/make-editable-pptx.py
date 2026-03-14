#!/usr/bin/env python3
"""
将幻灯片图片转换为可编辑 PPTX（单页或整套 deck）。

Usage:
  python3 make-editable-pptx.py slide <image.png> [options]
  python3 make-editable-pptx.py deck  <deck-dir>  [options]
  python3 make-editable-pptx.py clean <deck-dir>

Options:
  --output, -o         输出 PPTX 路径
  --slides 6,9,10      指定 slide 编号（deck 模式）
  --parallel N          并行处理数（deck 模式，默认 1）
  --skip-inpaint       跳过去文字（调试用）
  --skip-styles        跳过样式提取（调试用）
  --skip-crop          跳过裁切，使用简单样式提取
  --force              忽略缓存，强制重新处理
"""
import sys
import os

# Add scripts/ to path so editable package is importable
sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))

import argparse
import json
import re
import time
from pathlib import Path
from concurrent.futures import ThreadPoolExecutor, as_completed

from editable.common import (
    load_env, get_llm_client, get_baidu_access_token,
    get_image_size, TextRegion, SlideArtifacts,
)
from editable.extractors import ocr_extract_text_positions, crop_elements
from editable.inpaint import remove_text_from_image
from editable.styles import extract_styles_hybrid, extract_styles_simple
from editable.builder import build_presentation


# ═══════════════════════════════════════════════════════
#  Slide Pattern
# ═══════════════════════════════════════════════════════

SLIDE_PATTERN = re.compile(r"^(\d{2})-slide-.+\.png$")
EXCLUDE_SUFFIXES = ("-clean-bg.png", "-mask.png", "-marked.png")


def find_slide_images(deck_dir: str, slide_nums: list[int] | None = None) -> list[Path]:
    """Find slide images in deck directory, sorted by number."""
    images = []
    for f in sorted(Path(deck_dir).iterdir()):
        if not f.is_file():
            continue
        if any(f.name.endswith(s) for s in EXCLUDE_SUFFIXES):
            continue
        m = SLIDE_PATTERN.match(f.name)
        if m:
            num = int(m.group(1))
            if slide_nums is None or num in slide_nums:
                images.append(f)
    return images


# ═══════════════════════════════════════════════════════
#  Core Pipeline: process one slide
# ═══════════════════════════════════════════════════════

def process_slide(
    image_path: str,
    access_token: str,
    llm_client,
    skip_inpaint: bool = False,
    skip_styles: bool = False,
    skip_crop: bool = False,
    force: bool = False,
) -> SlideArtifacts:
    """Process one slide image through the full pipeline."""
    base = str(Path(image_path).with_suffix(""))
    ocr_cache = Path(f"{base}-ocr.json")
    styles_cache = Path(f"{base}-styles.json")
    bg_path = Path(f"{base}-clean-bg.png")
    crops_dir = Path(f"{base}-crops")
    slide_name = Path(image_path).name
    image_size = get_image_size(image_path)

    # Force mode: clear caches
    if force:
        for p in [ocr_cache, styles_cache, bg_path]:
            if p.exists():
                p.unlink()
        if crops_dir.exists():
            import shutil
            shutil.rmtree(crops_dir)

    # ── Stage 1: OCR ──
    if ocr_cache.exists() and not force:
        raw = json.loads(ocr_cache.read_text(encoding="utf-8"))
        regions = [TextRegion(**r) for r in raw]
        print(f"  [OCR]     {slide_name}  (cached, {len(regions)} blocks)")
    else:
        regions = ocr_extract_text_positions(image_path, access_token)
        # Save cache
        cache_data = [
            {"text": r.text, "left": r.left, "top": r.top,
             "width": r.width, "height": r.height,
             "element_type": r.element_type, "probability": r.probability}
            for r in regions
        ]
        ocr_cache.write_text(json.dumps(cache_data, ensure_ascii=False, indent=2), encoding="utf-8")
        print(f"  [OCR]     {slide_name}  ({len(regions)} blocks)")

    # ── Stage 2: Crop elements ──
    if not skip_crop and not skip_styles:
        if crops_dir.exists() and not force:
            # Restore crop paths from disk
            for i, r in enumerate(regions):
                p = crops_dir / f"{i:03d}_{r.element_type}.png"
                if p.exists():
                    r.crop_path = str(p)
            print(f"  [CROP]    {slide_name}  (cached)")
        else:
            crop_elements(image_path, regions, str(crops_dir))

    # ── Stage 3: Inpaint ──
    if skip_inpaint:
        final_bg = image_path
        print(f"  [INPAINT] {slide_name}  (skipped)")
    elif bg_path.exists() and not force:
        final_bg = str(bg_path)
        print(f"  [INPAINT] {slide_name}  (cached)")
    else:
        remove_text_from_image(llm_client, image_path, regions, str(bg_path))
        final_bg = str(bg_path)
        # Clean temp files
        for suffix in ("-mask.png", "-marked.png"):
            tmp = Path(f"{base}{suffix}")
            if tmp.exists():
                tmp.unlink()
        print(f"  [INPAINT] {slide_name}  (done)")

    # ── Stage 4: Style extraction ──
    if skip_styles:
        from editable.common import TextStyle
        for r in regions:
            r.style = TextStyle()
        print(f"  [STYLE]   {slide_name}  (skipped)")
    elif styles_cache.exists() and not force:
        raw = json.loads(styles_cache.read_text(encoding="utf-8"))
        from editable.common import TextStyle, ColoredSegment
        for i, r in enumerate(regions):
            if i < len(raw):
                s = raw[i]
                segs = [ColoredSegment(**seg) for seg in s.get("colored_segments", [])]
                r.style = TextStyle(
                    color=s.get("color", "#000000"),
                    bold=s.get("bold", False),
                    italic=s.get("italic", False),
                    align=s.get("align", "left"),
                    colored_segments=segs,
                )
            else:
                r.style = TextStyle()
        print(f"  [STYLE]   {slide_name}  (cached)")
    else:
        if skip_crop or not any(r.crop_path for r in regions):
            extract_styles_simple(llm_client, image_path, regions)
        else:
            extract_styles_hybrid(llm_client, image_path, regions)
        # Save cache
        styles_data = []
        for r in regions:
            s = r.style or TextStyle()
            styles_data.append({
                "color": s.color,
                "bold": s.bold,
                "italic": s.italic,
                "align": s.align,
                "colored_segments": [
                    {"text": seg.text, "color": seg.color, "is_latex": seg.is_latex}
                    for seg in s.colored_segments
                ],
            })
        styles_cache.write_text(json.dumps(styles_data, ensure_ascii=False, indent=2), encoding="utf-8")

    return SlideArtifacts(
        image_path=image_path,
        image_size=image_size,
        clean_bg_path=final_bg,
        regions=regions,
    )


# ═══════════════════════════════════════════════════════
#  Subcommand: slide
# ═══════════════════════════════════════════════════════

def cmd_slide(args):
    """Process a single slide image."""
    load_env(args.target)
    image_path = os.path.abspath(args.target)
    if not os.path.exists(image_path):
        print(f"错误：图片不存在: {image_path}", file=sys.stderr)
        sys.exit(1)

    base = os.path.splitext(image_path)[0]
    output_path = args.output or f"{base}-editable.pptx"

    print(f"图片:  {image_path}")
    print(f"输出:  {output_path}")
    print(f"尺寸:  {get_image_size(image_path)}")
    print(f"{'─' * 56}")

    access_token = get_baidu_access_token()
    llm_client = None
    if not args.skip_styles or not args.skip_inpaint:
        llm_client = get_llm_client()

    data = process_slide(
        image_path, access_token, llm_client,
        skip_inpaint=args.skip_inpaint,
        skip_styles=args.skip_styles,
        skip_crop=args.skip_crop,
        force=args.force,
    )

    print(f"{'─' * 56}")
    build_presentation([data], output_path)


# ═══════════════════════════════════════════════════════
#  Subcommand: deck
# ═══════════════════════════════════════════════════════

def cmd_deck(args):
    """Process entire deck directory."""
    deck_dir = os.path.abspath(args.target)
    if not os.path.isdir(deck_dir):
        print(f"错误：目录不存在: {deck_dir}", file=sys.stderr)
        sys.exit(1)

    load_env(deck_dir)

    slide_nums = None
    if args.slides:
        slide_nums = [int(n.strip()) for n in args.slides.split(",")]

    images = find_slide_images(deck_dir, slide_nums)
    if not images:
        print("未找到幻灯片图片 (格式: NN-slide-*.png)")
        sys.exit(1)

    deck_slug = Path(deck_dir).name
    output_path = args.output or os.path.join(deck_dir, f"{deck_slug}-editable.pptx")

    print(f"目录:     {deck_dir}")
    print(f"输出:     {output_path}")
    print(f"Slides:   {len(images)} 张")
    if slide_nums:
        print(f"指定编号: {slide_nums}")
    print(f"并行数:   {args.parallel}")
    print(f"{'─' * 56}")

    access_token = get_baidu_access_token()
    llm_client = None
    if not args.skip_styles or not args.skip_inpaint:
        llm_client = get_llm_client()

    start_time = time.time()
    results: list[tuple[int, SlideArtifacts | None, str]] = []

    process_kwargs = dict(
        access_token=access_token,
        llm_client=llm_client,
        skip_inpaint=args.skip_inpaint,
        skip_styles=args.skip_styles,
        skip_crop=args.skip_crop,
        force=args.force,
    )

    if args.parallel > 1:
        with ThreadPoolExecutor(max_workers=args.parallel) as pool:
            futures = {}
            for idx, img in enumerate(images):
                f = pool.submit(process_slide, str(img), **process_kwargs)
                futures[f] = idx
            for future in as_completed(futures):
                idx = futures[future]
                try:
                    data = future.result()
                    results.append((idx, data, ""))
                except Exception as e:
                    results.append((idx, None, str(e)))
                    print(f"  ✗ {images[idx].name}: {e}")
    else:
        for idx, img in enumerate(images):
            try:
                data = process_slide(str(img), **process_kwargs)
                results.append((idx, data, ""))
            except Exception as e:
                results.append((idx, None, str(e)))
                print(f"  ✗ {img.name}: {e}")

    results.sort(key=lambda x: x[0])
    slides_data = [data for _, data, _ in results if data is not None]
    failed = [(images[idx].name, err) for idx, data, err in results if data is None]

    elapsed = time.time() - start_time
    print(f"{'─' * 56}")
    print(f"处理完成: {len(slides_data)}/{len(images)} 张 ({elapsed:.1f}s)")

    if failed:
        print(f"\n失败 ({len(failed)} 张):")
        for name, err in failed:
            print(f"  ✗ {name}: {err}")

    if not slides_data:
        print("没有可用的 slide 数据")
        sys.exit(1)

    # If --slides was specified, also include non-targeted slides from cache
    if slide_nums:
        all_images = find_slide_images(deck_dir)
        processed_paths = {d.image_path for d in slides_data}
        all_data = []
        for img in all_images:
            img_str = str(img)
            match = [d for d in slides_data if d.image_path == img_str]
            if match:
                all_data.append(match[0])
            else:
                # Use image as-is (no text boxes)
                all_data.append(SlideArtifacts(
                    image_path=img_str,
                    image_size=get_image_size(img_str),
                    clean_bg_path=img_str,
                    regions=[],
                ))
        slides_data = all_data

    print(f"\n正在生成可编辑 PPTX ({len(slides_data)} slides)...")
    build_presentation(slides_data, output_path)


# ═══════════════════════════════════════════════════════
#  Subcommand: clean
# ═══════════════════════════════════════════════════════

def cmd_clean(args):
    """Remove all intermediate files."""
    deck_dir = os.path.abspath(args.target)
    if not os.path.isdir(deck_dir):
        print(f"错误：目录不存在: {deck_dir}", file=sys.stderr)
        sys.exit(1)

    import shutil
    count = 0
    for f in Path(deck_dir).iterdir():
        if f.name.endswith(("-ocr.json", "-styles.json", "-clean-bg.png")):
            f.unlink()
            count += 1
        if f.is_dir() and f.name.endswith("-crops"):
            shutil.rmtree(f)
            count += 1
    print(f"已清除 {count} 个中间文件/目录")


# ═══════════════════════════════════════════════════════
#  Main
# ═══════════════════════════════════════════════════════

def main():
    parser = argparse.ArgumentParser(
        description="将幻灯片图片转换为可编辑 PPTX"
    )
    parser.add_argument("command", choices=["slide", "deck", "clean"],
                        help="模式: slide=单张, deck=整套, clean=清理缓存")
    parser.add_argument("target", help="图片路径(slide) 或目录路径(deck/clean)")
    parser.add_argument("--output", "-o", help="输出 PPTX 路径")
    parser.add_argument("--slides", help="指定 slide 编号，逗号分隔 (如 6,9,10)")
    parser.add_argument("--parallel", type=int, default=1,
                        help="并行处理数 (默认 1)")
    parser.add_argument("--skip-inpaint", action="store_true",
                        help="跳过去文字（调试用）")
    parser.add_argument("--skip-styles", action="store_true",
                        help="跳过样式提取（调试用）")
    parser.add_argument("--skip-crop", action="store_true",
                        help="跳过裁切，使用简单样式提取")
    parser.add_argument("--force", action="store_true",
                        help="忽略缓存，强制重新处理")
    args = parser.parse_args()

    if args.command == "slide":
        cmd_slide(args)
    elif args.command == "deck":
        cmd_deck(args)
    elif args.command == "clean":
        cmd_clean(args)


if __name__ == "__main__":
    main()
