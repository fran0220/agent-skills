"""Shared data structures, config, and utility functions."""
from __future__ import annotations

import base64
import os
import sys
from dataclasses import dataclass, field
from io import BytesIO
from pathlib import Path
from typing import Optional

import requests
from PIL import Image
from google import genai
from google.genai import types
from pptx.enum.text import PP_ALIGN


# ═══════════════════════════════════════════════════════
#  Config
# ═══════════════════════════════════════════════════════

VISION_MODEL = "gemini-3.1-pro-preview"
IMAGE_MODEL = "gemini-3.1-flash-image-preview"

GENAI_TIMEOUT = 300  # seconds

SLIDE_WIDTH_INCHES = 13.333  # 16:9
SLIDE_HEIGHT_INCHES = 7.5
PPI = 96

BAIDU_TOKEN_URL = "https://aip.baidubce.com/oauth/2.0/token"
BAIDU_OCR_URL = "https://aip.baidubce.com/rest/2.0/ocr/v1/accurate"

ALIGN_MAP = {
    "left": PP_ALIGN.LEFT,
    "center": PP_ALIGN.CENTER,
    "right": PP_ALIGN.RIGHT,
}

DEFAULT_FONT_NAME_CJK = "Microsoft YaHei"
DEFAULT_FONT_NAME_LATIN = "Arial"

# Font paths for precise text measurement (tried in order)
FONT_SEARCH_PATHS = [
    # Bundled
    Path(__file__).parent.parent / "fonts" / "NotoSansSC-Regular.ttf",
    # macOS
    Path("/System/Library/Fonts/PingFang.ttc"),
    Path("/System/Library/Fonts/STHeiti Light.ttc"),
    Path.home() / "Library/Fonts/NotoSansSC-Regular.ttf",
    # Linux
    Path("/usr/share/fonts/truetype/noto/NotoSansCJK-Regular.ttc"),
    Path("/usr/share/fonts/noto-cjk/NotoSansCJK-Regular.ttc"),
    # Windows
    Path("C:/Windows/Fonts/msyh.ttc"),
]


# ═══════════════════════════════════════════════════════
#  Data Structures
# ═══════════════════════════════════════════════════════

@dataclass
class ColoredSegment:
    """A text segment with its own color."""
    text: str
    color: str = "#000000"  # hex
    is_latex: bool = False


@dataclass
class TextStyle:
    """Extracted style for a text region."""
    color: str = "#000000"
    bold: bool = False
    italic: bool = False
    underline: bool = False
    align: str = "left"
    colored_segments: list[ColoredSegment] = field(default_factory=list)


@dataclass
class TextRegion:
    """A detected text region with OCR data, crop, and style."""
    text: str
    left: int
    top: int
    width: int
    height: int
    element_type: str = "text"  # text/title/image/table
    probability: float = 1.0
    crop_path: Optional[str] = None
    style: Optional[TextStyle] = None


@dataclass
class SlideArtifacts:
    """All data needed to build one PPTX slide."""
    image_path: str
    image_size: tuple
    clean_bg_path: str
    regions: list[TextRegion]


# ═══════════════════════════════════════════════════════
#  Environment & Clients
# ═══════════════════════════════════════════════════════

def _apply_env_file(dotenv: Path) -> bool:
    """Load a .env file into os.environ (setdefault). Returns True if found."""
    if not dotenv.exists():
        return False
    for line in dotenv.read_text().splitlines():
        line = line.strip()
        if line and not line.startswith("#") and "=" in line:
            k, v = line.split("=", 1)
            os.environ.setdefault(k.strip(), v.strip())
    return True


def load_env(start_path: str) -> None:
    """Walk up from start_path to find .env; fall back to skill directory .env."""
    path = Path(start_path).resolve()
    for parent in [path] + list(path.parents):
        if _apply_env_file(parent / ".env"):
            return
    # Fallback: skill directory (.agents/skills/slide-deck/.env)
    skill_env = Path(__file__).resolve().parent.parent.parent / ".env"
    _apply_env_file(skill_env)


def get_genai_client() -> genai.Client:
    """Create Google GenAI client with optional base URL override."""
    api_key = os.environ.get("GOOGLE_API_KEY", "")
    if not api_key:
        print("错误：未设置 GOOGLE_API_KEY", file=sys.stderr)
        sys.exit(1)
    api_base = os.environ.get("GOOGLE_API_BASE", None)
    timeout_ms = int(GENAI_TIMEOUT * 1000)
    http_opts = types.HttpOptions(timeout=timeout_ms)
    if api_base:
        http_opts = types.HttpOptions(timeout=timeout_ms, base_url=api_base)
    return genai.Client(api_key=api_key, http_options=http_opts)


def get_baidu_access_token() -> str:
    """Get Baidu OCR access token."""
    api_key = os.environ.get("BAIDU_OCR_API_KEY", "")
    secret_key = os.environ.get("BAIDU_OCR_SECRET_KEY", "")
    if not api_key or not secret_key:
        print("错误：未设置 BAIDU_OCR_API_KEY 或 BAIDU_OCR_SECRET_KEY", file=sys.stderr)
        sys.exit(1)

    resp = requests.post(BAIDU_TOKEN_URL, params={
        "grant_type": "client_credentials",
        "client_id": api_key,
        "client_secret": secret_key,
    })
    resp.raise_for_status()
    token = resp.json().get("access_token")
    if not token:
        print(f"百度 access_token 获取失败: {resp.json()}", file=sys.stderr)
        sys.exit(1)
    return token


# ═══════════════════════════════════════════════════════
#  Utility Functions
# ═══════════════════════════════════════════════════════

def image_to_base64(image_path: str) -> str:
    with open(image_path, "rb") as f:
        return base64.b64encode(f.read()).decode("utf-8")


def load_pil_image(image_path: str) -> Image.Image:
    """Load an image as PIL Image (RGB)."""
    return Image.open(image_path).convert("RGB")


def get_image_size(image_path: str) -> tuple:
    """Return (width, height) in pixels."""
    with Image.open(image_path) as img:
        return img.size


def extract_response_image(response) -> Optional[Image.Image]:
    """Extract the last image from a Gemini generate_content response."""
    last_image = None
    if not response.parts:
        return None
    for part in response.parts:
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
    return last_image


def get_default_font_name(text: str) -> str:
    """Return CJK or Latin font name based on text content."""
    if any(ord(c) > 0x2E80 for c in text):
        return DEFAULT_FONT_NAME_CJK
    return DEFAULT_FONT_NAME_LATIN


def hex_to_rgb(hex_str: str) -> tuple:
    """Convert '#RRGGBB' to (r, g, b) tuple."""
    h = hex_str.lstrip("#")
    if len(h) != 6:
        return (0, 0, 0)
    try:
        return (int(h[0:2], 16), int(h[2:4], 16), int(h[4:6], 16))
    except ValueError:
        return (0, 0, 0)


def find_cjk_font_path() -> Optional[str]:
    """Find an available CJK font for text measurement."""
    for p in FONT_SEARCH_PATHS:
        if p.exists():
            return str(p)
    return None
