#!/usr/bin/env python3
"""Draws the Baddel app icon from one description of its geometry.

Outputs, in design/icon/:
  0-background.svg, 1-keycap-back.svg, 2-keycap-front.svg   full-bleed 1024 layers, no mask,
                                                             letters as outlines (Icon Composer input)
  icon-flat.svg                                              the three layers composed
  icon-macos-1024.png                                        macOS-shaped, with margin and shadow
                                                             (input for `tauri icon`)
  preview.png                                                the icon at 256…16 px, light and dark

Needs Pillow, fontTools and Cairo Bold (~/Library/Fonts/Cairo-Bold.ttf or $CAIRO_BOLD).
"""
import os
from pathlib import Path

from fontTools.pens.boundsPen import BoundsPen
from fontTools.pens.svgPathPen import SVGPathPen
from fontTools.pens.transformPen import TransformPen
from fontTools.ttLib import TTFont
from PIL import Image, ImageDraw, ImageFilter, ImageFont

OUT = Path(__file__).resolve().parent.parent / "design" / "icon"
FONT = os.environ.get("CAIRO_BOLD", str(Path.home() / "Library/Fonts/Cairo-Bold.ttf"))
N = 1024

BG_TOP, BG_BOTTOM = "#4A6485", "#35485F"
GAP = "#3F5673"          # background tone, drawn around the front key to separate it from the back one
BACK_FACE, BACK_EDGE = "#E4DED2", "#C9C1B2"
FRONT_FACE, FRONT_EDGE = "#FBFAF7", "#D8D2C6"
INK = "#2B3B4E"

KEY, RADIUS, EDGE, GAP_W = 372, 80, 16, 14
BACK = (242, 214)        # top-left of each key face; the pair is centred on the canvas
FRONT = (410, 422)
# The front key covers the back key's lower-right corner, so the "A" sits in the part that
# stays visible rather than in the middle of its key.
VISIBLE = FRONT[0] - GAP_W - BACK[0]
KEYS = [
    # name, origin, face, edge, letter, letter height in px, box the letter is centred in (w, h)
    ("1-keycap-back", BACK, BACK_FACE, BACK_EDGE, "A", 150, (VISIBLE + 60, VISIBLE + 60)),
    ("2-keycap-front", FRONT, FRONT_FACE, FRONT_EDGE, "\u0639", 212, (KEY, KEY)),
]

font = TTFont(FONT)
glyphs, cmap = font.getGlyphSet(), font.getBestCmap()


def letter(char, height, box):
    """SVG path data and pixel bounds for `char`, `height` px tall, centred in `box` (x, y, w, h)."""
    glyph = glyphs[cmap[ord(char)]]
    bounds = BoundsPen(glyphs)
    glyph.draw(bounds)
    x0, y0, x1, y1 = bounds.bounds
    scale = height / (y1 - y0)
    bx, by, bw, bh = box
    dx = bx + (bw - (x1 - x0) * scale) / 2 - x0 * scale
    dy = by + (bh - height) / 2 + y1 * scale          # font y points up, canvas y points down
    pen = SVGPathPen(glyphs, ntos=lambda v: f"{v:.1f}")
    glyph.draw(TransformPen(pen, (scale, 0, 0, -scale, dx, dy)))
    return pen.getCommands()


def rrect(x, y, w, h, r, fill):
    return f'<rect x="{x}" y="{y}" width="{w}" height="{h}" rx="{r}" fill="{fill}"/>'


def key_svg(origin, face, edge, char, height, box, gap):
    x, y = origin
    parts = []
    if gap:
        g = GAP_W
        parts.append(rrect(x - g, y - g, KEY + 2 * g, KEY + EDGE + 2 * g, RADIUS + g, GAP))
    parts.append(rrect(x, y + EDGE, KEY, KEY, RADIUS, edge))        # the key's lower edge, flat
    parts.append(rrect(x, y, KEY, KEY, RADIUS, face))
    parts.append(f'<path d="{letter(char, height, (x, y, *box))}" fill="{INK}"/>')
    return "\n  ".join(parts)


def svg(body):
    return f'<svg xmlns="http://www.w3.org/2000/svg" width="{N}" height="{N}" viewBox="0 0 {N} {N}">\n  {body}\n</svg>\n'


background = (
    f'<defs><linearGradient id="bg" x1="0" y1="0" x2="0" y2="1"><stop offset="0" stop-color="{BG_TOP}"/>'
    f'<stop offset="1" stop-color="{BG_BOTTOM}"/></linearGradient></defs>\n  '
    f'<rect width="{N}" height="{N}" fill="url(#bg)"/>'
)
layers = {"0-background": background}
for name, origin, face, edge, char, height, box in KEYS:
    layers[name] = key_svg(origin, face, edge, char, height, box, gap=name.endswith("front"))

OUT.mkdir(parents=True, exist_ok=True)
for name, body in layers.items():
    (OUT / f"{name}.svg").write_text(svg(body), encoding="utf-8")
(OUT / "icon-flat.svg").write_text(svg("\n  ".join(layers.values())), encoding="utf-8")


# ── Raster: the same geometry through Pillow, supersampled ───────────────────────────────────
S = 4


def rgb(h):
    return tuple(int(h[i : i + 2], 16) for i in (1, 3, 5))


def draw_flat():
    img = Image.new("RGB", (N * S, N * S))
    d = ImageDraw.Draw(img)
    top, bottom = rgb(BG_TOP), rgb(BG_BOTTOM)
    for y in range(N * S):
        t = y / (N * S - 1)
        d.line([(0, y), (N * S, y)], fill=tuple(round(a + (b - a) * t) for a, b in zip(top, bottom)))

    def rr(x, y, w, h, r, fill):
        d.rounded_rectangle([x * S, y * S, (x + w) * S, (y + h) * S], radius=r * S, fill=rgb(fill))

    for name, (x, y), face, edge, char, height, (bw, bh) in KEYS:
        if name.endswith("front"):
            g = GAP_W
            rr(x - g, y - g, KEY + 2 * g, KEY + EDGE + 2 * g, RADIUS + g, GAP)
        rr(x, y + EDGE, KEY, KEY, RADIUS, edge)
        rr(x, y, KEY, KEY, RADIUS, face)
        # Same sizing rule as the SVG: scale so the glyph's ink is `height` px tall, centre its ink box.
        probe = ImageFont.truetype(FONT, 1000)
        l, t, r_, b = probe.getbbox(char)
        f = ImageFont.truetype(FONT, round(1000 * height * S / (b - t)))
        l, t, r_, b = f.getbbox(char)
        d.text((x * S + (bw * S - (r_ - l)) / 2 - l, y * S + (bh * S - (b - t)) / 2 - t), char, font=f, fill=rgb(INK))
    return img.resize((N, N), Image.LANCZOS)


flat = draw_flat()

# macOS shape: 824px body on the 1024 canvas, continuous-ish corners, soft shadow below.
BODY, MARGIN, CORNER = 824, 100, 186
mask = Image.new("L", (N * S, N * S), 0)
ImageDraw.Draw(mask).rounded_rectangle(
    [MARGIN * S, MARGIN * S, (MARGIN + BODY) * S, (MARGIN + BODY) * S], radius=CORNER * S, fill=255
)
mask = mask.resize((N, N), Image.LANCZOS)
body = flat.resize((BODY, BODY), Image.LANCZOS)
icon = Image.new("RGBA", (N, N), (0, 0, 0, 0))
shadow = Image.new("RGBA", (N, N), (0, 0, 0, 0))
shadow.paste((0, 0, 0, 90), (0, 12), mask)
icon = Image.alpha_composite(icon, shadow.filter(ImageFilter.GaussianBlur(14)))
placed = Image.new("RGBA", (N, N), (0, 0, 0, 0))
placed.paste(body, (MARGIN, MARGIN))
placed.putalpha(mask)
icon = Image.alpha_composite(icon, placed)
icon.save(OUT / "icon-macos-1024.png")

sizes = [256, 128, 64, 32, 16]
strip_w = sum(sizes) + 40 * (len(sizes) + 1)
preview = Image.new("RGB", (strip_w, 2 * 336), "#F6F4EF")
preview.paste("#1B1C1F", (0, 336, strip_w, 672))
for row in range(2):
    x = 40
    for s in sizes:
        small = icon.resize((s, s), Image.LANCZOS)
        preview.paste(small, (x, row * 336 + (336 - s) // 2), small)
        x += s + 40
preview.save(OUT / "preview.png")
print("wrote", *sorted(p.name for p in OUT.iterdir()))
