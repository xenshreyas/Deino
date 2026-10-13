#!/usr/bin/env python3
"""Deino's brand assets, made from the owner's two images (BRAND-01).

Run from anywhere with the system Python and Pillow only:

    python3 -I scripts/brand/make-brand-assets.py [--previews DIR]
    python3 -I scripts/brand/make-brand-assets.py --import WORDMARK.png MARK.webp

`--import` first copies the owner's originals into `assets/brand/source/`
(the webp converted to PNG with `dwebp`); without it the committed originals
are used, so the outputs are reproducible from the repository alone.

Every source is a dark shape on a white background. Its alpha is the pixel's
distance from the background along the background-to-ink axis, so edges stay
anti-aliased; each output is then filled with the one sampled ink colour, so
no white fringe survives. The script writes:

- `assets/brand/mark.png`           the mark in its blue, 1024 px long side
- `assets/brand/mark-mask.png`      alpha only, for CSS masks (256 px long side)
- `assets/brand/wordmark-mask.png`  alpha only, at the source resolution
- `assets/brand/app-icon.png`       the 1024 px macOS icon, input to `tauri icon`
- `frontend/src/ui/brand-masks/*`   the two masks, where Vite can import them
- `frontend/src-tauri/icons/tray/*` the menu-bar template icon, @1x and @2x
- `frontend/public/favicon-*.png`   the favicon at 32 and 64 px

The app icon set itself comes from `pnpm tauri icon assets/brand/app-icon.png`
(see `docs/changes/BRAND-01.md`).
"""

from __future__ import annotations

import argparse
import math
import shutil
import subprocess
from dataclasses import dataclass
from pathlib import Path

from PIL import Image, ImageChops, ImageDraw, ImageFilter, ImageMath, ImageStat

REPO = Path(__file__).resolve().parents[2]
BRAND = REPO / "assets" / "brand"
ORIGINALS = BRAND / "source"
UI_MASKS = REPO / "frontend" / "src" / "ui" / "brand-masks"
TRAY = REPO / "frontend" / "src-tauri" / "icons" / "tray"
PUBLIC = REPO / "frontend" / "public"

# Alpha below LOW is background noise; above HIGH it is solid ink.
LOW, HIGH = 0.04, 0.96
# Supersampling for the drawn shapes (icon body, shadow).
SS = 4

Rgb = tuple[int, int, int]


@dataclass(frozen=True)
class Shape:
    """A source reduced to one ink colour and a cropped alpha mask."""

    ink: Rgb
    alpha: Image.Image  # mode "L", cropped to its bounds


# --- Masks -----------------------------------------------------------------


def flatten(path: Path) -> Image.Image:
    """The image as RGB, any transparency composited onto white."""
    image = Image.open(path).convert("RGBA")
    white = Image.new("RGBA", image.size, (255, 255, 255, 255))
    return Image.alpha_composite(white, image).convert("RGB")


def border_mask(size: tuple[int, int], width: int = 4) -> Image.Image:
    """A mask of the image's outer `width` pixels: the background sample."""
    mask = Image.new("L", size, 255)
    w, h = size
    ImageDraw.Draw(mask).rectangle((width, width, w - 1 - width, h - 1 - width), fill=0)
    return mask


def median_rgb(image: Image.Image, mask: Image.Image) -> Rgb:
    """The per-channel median of `image` where `mask` is set."""
    r, g, b = (round(v) for v in ImageStat.Stat(image, mask).median)
    return (r, g, b)


def ink_mask(image: Image.Image, background: Rgb) -> Image.Image:
    """The inked pixels: at least 60% as far from the background as the farthest."""
    flat = Image.new("RGB", image.size, background)
    distance = ImageChops.difference(image, flat).convert("L")
    far = distance.getextrema()[1]
    return distance.point(lambda v: 255 if v >= 0.6 * far else 0)


def unmix_alpha(image: Image.Image, background: Rgb, ink: Rgb) -> Image.Image:
    """Alpha as the projection onto background→ink, cleaned and in 0..255."""
    axis = [bg - fg for bg, fg in zip(background, ink)]
    norm = sum(k * k for k in axis)
    r, g, b = (channel.convert("F") for channel in image.split())
    raw = ImageMath.lambda_eval(
        lambda a: (
            (background[0] - a["r"]) * axis[0]
            + (background[1] - a["g"]) * axis[1]
            + (background[2] - a["b"]) * axis[2]
        )
        * (255.0 / norm),
        r=r,
        g=g,
        b=b,
    )
    scale = 1.0 / (HIGH - LOW)
    return raw.point(lambda v: (v - LOW * 255.0) * scale).convert("L")


def extract(path: Path) -> Shape:
    """One source as its ink colour and its alpha, cropped to the shape."""
    image = flatten(path)
    background = median_rgb(image, border_mask(image.size))
    ink = median_rgb(image, ink_mask(image, background))
    alpha = unmix_alpha(image, background, ink)
    bounds = alpha.getbbox()
    if bounds is None:
        raise SystemExit(f"{path.name}: no shape found on the background")
    return Shape(ink=ink, alpha=alpha.crop(bounds))


# --- Output helpers ----------------------------------------------------------


def fit(alpha: Image.Image, long_side: int) -> Image.Image:
    """`alpha` scaled so its longer side is `long_side` pixels."""
    w, h = alpha.size
    ratio = long_side / max(w, h)
    size = (max(1, round(w * ratio)), max(1, round(h * ratio)))
    return alpha.resize(size, Image.Resampling.LANCZOS)


def solid(alpha: Image.Image, colour: Rgb) -> Image.Image:
    """An RGBA image of one colour, shaped by `alpha`."""
    image = Image.new("RGBA", alpha.size, colour + (255,))
    image.putalpha(alpha)
    return image


def mask_only(alpha: Image.Image) -> Image.Image:
    """Black plus alpha (mode LA): what `mask-image` reads."""
    return Image.merge("LA", (Image.new("L", alpha.size, 0), alpha))


def centred(content: Image.Image, size: tuple[int, int]) -> Image.Image:
    """`content` pasted at the centre of a transparent canvas of `size`."""
    canvas = Image.new("RGBA", size, (0, 0, 0, 0))
    x = (size[0] - content.width) // 2
    y = (size[1] - content.height) // 2
    canvas.alpha_composite(content.convert("RGBA"), (x, y))
    return canvas


def hex_of(colour: Rgb) -> str:
    return "#{:02X}{:02X}{:02X}".format(*colour)


def save(image: Image.Image, path: Path) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    image.save(path, optimize=True)
    shown = path.relative_to(REPO) if path.is_relative_to(REPO) else path
    print(f"  wrote {shown} {image.width}x{image.height}")


# --- App icon ------------------------------------------------------------------

CANVAS = 1024
BODY = 824
# A 185 px corner as a continuous curve: a superellipse quarter spread over
# 1.528 × the radius (Apple's smoothing), straight edges in between.
CORNER = round(185 * 1.528)
CORNER_EXPONENT = 3.26
MARK_SHARE = 0.62  # the mark's longer side, as a share of the body


def corner_points(centre: tuple[float, float], signs: tuple[int, int], radius: float) -> list[tuple[float, float]]:
    """One continuous corner from the vertical edge to the horizontal one."""
    (cx, cy), (sx, sy) = centre, signs
    points = []
    for step in range(65):
        t = step / 64 * math.pi / 2
        dx = math.cos(t) ** (2 / CORNER_EXPONENT)
        dy = math.sin(t) ** (2 / CORNER_EXPONENT)
        points.append((cx + sx * dx * radius, cy + sy * dy * radius))
    return points


def squircle(size: int, inset: float, radius: float) -> Image.Image:
    """The icon body as an L mask on a `size` canvas, `inset` from each edge."""
    lo, hi = inset + radius, size - inset - radius
    corners = [(hi, hi, 1, 1), (lo, hi, -1, 1), (lo, lo, -1, -1), (hi, lo, 1, -1)]
    outline: list[tuple[float, float]] = []
    for cx, cy, sx, sy in corners:
        arc = corner_points((cx, cy), (sx, sy), radius)
        outline.extend(arc if sx * sy > 0 else arc[::-1])
    mask = Image.new("L", (size, size), 0)
    ImageDraw.Draw(mask).polygon(outline, fill=255)
    return mask


def body_masks() -> tuple[Image.Image, Image.Image]:
    """The body's mask and its 2 px inner edge, drawn at SS× and reduced."""
    big = CANVAS * SS
    inset = (CANVAS - BODY) / 2 * SS
    outer = squircle(big, inset, CORNER * SS)
    inner = squircle(big, inset + 2 * SS, (CORNER - 2) * SS)
    edge = ImageChops.subtract(outer, inner)
    size = (CANVAS, CANVAS)
    return (
        outer.resize(size, Image.Resampling.LANCZOS),
        edge.resize(size, Image.Resampling.LANCZOS),
    )


def vertical_gradient(top: Rgb, bottom: Rgb) -> Image.Image:
    """A full-canvas top-to-bottom gradient."""
    column = Image.new("RGB", (1, CANVAS))
    for y in range(CANVAS):
        t = y / (CANVAS - 1)
        column.putpixel((0, y), tuple(round(a + (b - a) * t) for a, b in zip(top, bottom)))
    return column.resize((CANVAS, CANVAS))


def shadow(body: Image.Image) -> Image.Image:
    """The soft drop shadow under the body (offset 12, blur 14, 26% black)."""
    alpha = body.point(lambda v: v * 0.26)
    shifted = Image.new("L", body.size, 0)
    shifted.paste(alpha, (0, 12))
    blurred = shifted.filter(ImageFilter.GaussianBlur(14))
    return solid(blurred, (0, 0, 0))


def optical_offset(alpha: Image.Image) -> tuple[int, int]:
    """Half the way from the bounds' centre to the shape's centroid."""
    small = alpha.resize((alpha.width // 4 or 1, alpha.height // 4 or 1))
    total = sx = sy = 0.0
    for index, value in enumerate(small.getdata()):
        x, y = index % small.width, index // small.width
        total += value
        sx += value * x
        sy += value * y
    cx, cy = sx / total * 4, sy / total * 4
    return (round((alpha.width / 2 - cx) / 2), round((alpha.height / 2 - cy) / 2))


def app_icon(mark: Shape) -> Image.Image:
    """The 1024 px icon: a light squircle with a hairline edge, the mark inside."""
    body, edge = body_masks()
    icon = shadow(body)
    icon.alpha_composite(solid(body, (0, 0, 0)).convert("RGBA"))
    fill = vertical_gradient((255, 255, 255), (241, 243, 247)).convert("RGBA")
    fill.putalpha(body)
    icon = Image.alpha_composite(icon, fill)
    icon.alpha_composite(solid(edge.point(lambda v: v * 0.10), (0, 0, 0)))
    glyph = fit(mark.alpha, round(BODY * MARK_SHARE))
    dx, dy = optical_offset(glyph)
    x = (CANVAS - glyph.width) // 2 + dx
    y = (CANVAS - glyph.height) // 2 + dy
    icon.alpha_composite(solid(glyph, mark.ink), (x, y))
    return icon


# --- Small sizes -------------------------------------------------------------------


def tray_icon(mark: Shape, scale: int) -> Image.Image:
    """The menu-bar template: black plus alpha, an 18 pt tall canvas, 16 pt glyph."""
    glyph = fit(mark.alpha, 16 * scale)
    width = glyph.width + 2 * scale
    return centred(solid(glyph, (0, 0, 0)), (width, 18 * scale))


def favicon(mark: Shape, size: int) -> Image.Image:
    """The mark in its blue on transparent, with 1/16 padding."""
    glyph = fit(mark.alpha, size - 2 * max(1, size // 16))
    return centred(solid(glyph, mark.ink), (size, size))


def previews(icon: Image.Image, tray: Image.Image, folder: Path) -> None:
    """Icon previews at 1024/128/32/16 and the tray on light and dark bars."""
    for size in (1024, 128, 32, 16):
        save(icon.resize((size, size), Image.Resampling.LANCZOS), folder / f"brand-icon-{size}.png")
    bars = Image.new("RGBA", (tray.width * 2 + 48, tray.height + 16), (0, 0, 0, 0))
    for index, (bar, ink) in enumerate((((236, 236, 236), 0), ((40, 40, 40), 255))):
        tile = Image.new("RGBA", (tray.width + 24, tray.height + 16), bar + (255,))
        glyph = solid(tray.getchannel("A"), (ink, ink, ink))
        tile.alpha_composite(glyph, (12, 8))
        bars.alpha_composite(tile, (index * (tray.width + 24), 0))
    save(bars.resize((bars.width * 4, bars.height * 4), Image.Resampling.NEAREST), folder / "brand-tray.png")


# --- Main ------------------------------------------------------------------------


def import_originals(wordmark: Path, mark: Path) -> None:
    """Copies the owner's originals into the repository, the webp as PNG."""
    ORIGINALS.mkdir(parents=True, exist_ok=True)
    shutil.copyfile(wordmark, ORIGINALS / "wordmark.png")
    subprocess.run(["dwebp", "-quiet", str(mark), "-o", str(ORIGINALS / "mark.png")], check=True)
    print(f"  imported originals into {ORIGINALS.relative_to(REPO)}")


def write_masks(mark: Shape, wordmark: Shape) -> None:
    mark_mask = mask_only(fit(mark.alpha, 256))
    wordmark_mask = mask_only(wordmark.alpha)
    save(solid(fit(mark.alpha, 1024), mark.ink), BRAND / "mark.png")
    save(mark_mask, BRAND / "mark-mask.png")
    save(wordmark_mask, BRAND / "wordmark-mask.png")
    save(mark_mask, UI_MASKS / "mark.png")
    save(wordmark_mask, UI_MASKS / "wordmark.png")


def main() -> None:
    parser = argparse.ArgumentParser(description=__doc__.splitlines()[0])
    parser.add_argument("--import", dest="originals", nargs=2, type=Path, metavar=("WORDMARK", "MARK"))
    parser.add_argument("--previews", type=Path, help="also write icon and tray previews here")
    args = parser.parse_args()
    if args.originals:
        import_originals(*args.originals)
    mark = extract(ORIGINALS / "mark.png")
    wordmark = extract(ORIGINALS / "wordmark.png")
    print(f"mark blue {hex_of(mark.ink)}, wordmark navy {hex_of(wordmark.ink)}")
    write_masks(mark, wordmark)
    icon = app_icon(mark)
    save(icon, BRAND / "app-icon.png")
    tray = tray_icon(mark, 2)
    save(tray_icon(mark, 1), TRAY / "tray-icon.png")
    save(tray, TRAY / "tray-icon@2x.png")
    for size in (32, 64):
        save(favicon(mark, size), PUBLIC / f"favicon-{size}.png")
    if args.previews:
        previews(icon, tray, args.previews)


if __name__ == "__main__":
    main()
