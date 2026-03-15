#!/usr/bin/env python3
"""
Merge slide images into a single PDF.

Usage:
  python3 merge-to-pdf.py <slide-deck-dir> [--output filename.pdf]

Each image becomes a PDF page at its native resolution.
Finds slides matching NN-slide-*.{png,jpg,jpeg}, sorted by number.

Requires:
  - Pillow (PIL)
"""
from __future__ import annotations

import argparse
import re
import sys
from pathlib import Path

from PIL import Image


# ---------------------------------------------------------------------------
# Slide discovery
# ---------------------------------------------------------------------------
SLIDE_PATTERN = re.compile(r"^(\d+)-slide-.*\.(png|jpg|jpeg)$", re.IGNORECASE)


def find_slide_images(directory: Path) -> list[tuple[int, Path]]:
    """Return sorted list of (index, path) for slide images in directory."""
    if not directory.is_dir():
        print(f"Error: Directory not found: {directory}", file=sys.stderr)
        sys.exit(1)

    slides: list[tuple[int, Path]] = []
    for f in directory.iterdir():
        m = SLIDE_PATTERN.match(f.name)
        if m:
            slides.append((int(m.group(1)), f))

    slides.sort(key=lambda s: s[0])

    if not slides:
        print(f"Error: No slide images found in: {directory}", file=sys.stderr)
        print("Expected format: 01-slide-*.png, 02-slide-*.png, etc.", file=sys.stderr)
        sys.exit(1)

    return slides


# ---------------------------------------------------------------------------
# PDF creation
# ---------------------------------------------------------------------------
def create_pdf(slides: list[tuple[int, Path]], output_path: Path) -> None:
    """Create a PDF from slide images using Pillow."""
    images: list[Image.Image] = []

    for idx, path in slides:
        img = Image.open(path)
        if img.mode == "RGBA":
            bg = Image.new("RGB", img.size, (255, 255, 255))
            bg.paste(img, mask=img.split()[3])
            img = bg
        elif img.mode != "RGB":
            img = img.convert("RGB")
        images.append(img)
        print(f"  Added: {path.name}  ({img.width}x{img.height})")

    if not images:
        return

    first, rest = images[0], images[1:]
    first.save(
        output_path,
        "PDF",
        save_all=True,
        append_images=rest,
        author="slide-deck",
    )

    print(f"\nCreated: {output_path}")
    print(f"Total pages: {len(images)}")


# ---------------------------------------------------------------------------
# Main
# ---------------------------------------------------------------------------
def main() -> None:
    parser = argparse.ArgumentParser(description="Merge slide images into a PDF")
    parser.add_argument("dir", help="Directory containing slide images")
    parser.add_argument("--output", "-o", help="Output PDF filename (default: <dirName>.pdf)")
    args = parser.parse_args()

    directory = Path(args.dir).resolve()
    slides = find_slide_images(directory)

    dir_name = directory.parent.name if directory.name == "slide-deck" else directory.name
    output_path = Path(args.output) if args.output else directory / f"{dir_name}.pdf"

    print(f"Found {len(slides)} slides in: {directory}\n")

    create_pdf(slides, output_path)


if __name__ == "__main__":
    main()
