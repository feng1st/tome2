#!/usr/bin/env python3
"""Generate the tileset atlases for the Bevy version of ToME.

Outputs (under bevy/assets/tiles/):
  - background.png : 16 solid-color tiles (one per ToME color), RGB, NO alpha.
                     Used for cell backgrounds.
  - foreground.png : printable ASCII glyphs (32..126) in white, RGBA WITH alpha.
                     Used for terrain glyphs / monsters / player, tinted at
                     runtime with the ToME color of the entity.
  - preview.png    : visual sanity check (not used by the game).

Tile layout: TILE x TILE pixels, COLS columns per row.
Foreground tile index = ascii_code - 32.
Background tile index = ToME color index (0..15).
"""
import os
from PIL import Image, ImageDraw, ImageFont

HERE = os.path.dirname(os.path.abspath(__file__))
ASSETS = os.path.normpath(os.path.join(HERE, "..", "assets"))
FONT_PATH = os.path.join(ASSETS, "fonts", "LiberationMono-Regular.ttf")
OUT_DIR = os.path.join(ASSETS, "tiles")

TILE = 16
COLS = 16

# ToME palette (from src/variable.cc angband_color_table), index -> (name, rgb)
PALETTE = [
    ("dark",      (0x00, 0x00, 0x00)),  # 0  'd'
    ("white",     (0xFF, 0xFF, 0xFF)),  # 1  'w'
    ("slate",     (0x80, 0x80, 0x80)),  # 2  's'
    ("orange",    (0xFF, 0x80, 0x00)),  # 3  'o'
    ("red",       (0xC0, 0x00, 0x00)),  # 4  'r'
    ("green",     (0x00, 0x80, 0x40)),  # 5  'g'
    ("blue",      (0x00, 0x00, 0xFF)),  # 6  'b'
    ("umber",     (0x80, 0x40, 0x00)),  # 7  'u'
    ("l_dark",    (0x40, 0x40, 0x40)),  # 8  'D'
    ("l_white",   (0xC0, 0xC0, 0xC0)),  # 9  'W'
    ("violet",    (0xFF, 0x00, 0xFF)),  # 10 'v'
    ("yellow",    (0xFF, 0xFF, 0x00)),  # 11 'y'
    ("l_red",     (0xFF, 0x00, 0x00)),  # 12 'R'
    ("l_green",   (0x00, 0xFF, 0x00)),  # 13 'G'
    ("l_blue",    (0x00, 0xFF, 0xFF)),  # 14 'B'
    ("l_umber",   (0xC0, 0x80, 0x40)),  # 15 'U'
]

FIRST_CHAR = 32
LAST_CHAR = 126


def gen_background():
    """Solid color tiles, RGB (no alpha channel)."""
    rows = 1
    img = Image.new("RGB", (COLS * TILE, rows * TILE))
    draw = ImageDraw.Draw(img)
    for i, (_name, rgb) in enumerate(PALETTE):
        x0 = (i % COLS) * TILE
        draw.rectangle([x0, 0, x0 + TILE - 1, TILE - 1], fill=rgb)
    out = os.path.join(OUT_DIR, "background.png")
    img.save(out)
    print(f"wrote {out} mode={img.mode} size={img.size}")
    return img


def gen_foreground():
    """ASCII glyphs in white on transparent background, RGBA."""
    n = LAST_CHAR - FIRST_CHAR + 1
    rows = (n + COLS - 1) // COLS
    img = Image.new("RGBA", (COLS * TILE, rows * TILE), (0, 0, 0, 0))
    draw = ImageDraw.Draw(img)
    font = ImageFont.truetype(FONT_PATH, TILE - 3)
    for code in range(FIRST_CHAR, LAST_CHAR + 1):
        ch = chr(code)
        idx = code - FIRST_CHAR
        x0 = (idx % COLS) * TILE
        y0 = (idx // COLS) * TILE
        bbox = draw.textbbox((0, 0), ch, font=font)
        w = bbox[2] - bbox[0]
        h = bbox[3] - bbox[1]
        tx = x0 + (TILE - w) // 2 - bbox[0]
        ty = y0 + (TILE - h) // 2 - bbox[1]
        draw.text((tx, ty), ch, font=font, fill=(255, 255, 255, 255))
    out = os.path.join(OUT_DIR, "foreground.png")
    img.save(out)
    print(f"wrote {out} mode={img.mode} size={img.size}")
    return img


def gen_preview(bg, fg):
    """Compose a small preview: palette strip + sample glyphs on black."""
    preview = Image.new("RGB", (COLS * TILE, 10 * TILE), (16, 16, 16))
    preview.paste(bg.crop((0, 0, COLS * TILE, TILE)), (0, 0))
    sample = "@#.<>+pPoOrRkKZzjJ"
    for i, ch in enumerate(sample):
        idx = ord(ch) - FIRST_CHAR
        tile = fg.crop(((idx % COLS) * TILE, (idx // COLS) * TILE,
                        (idx % COLS) * TILE + TILE, (idx // COLS) * TILE + TILE))
        x = (i % COLS) * TILE
        y = 2 * TILE + (i // COLS) * TILE
        preview.paste(tile, (x, y), tile)
    out = os.path.join(OUT_DIR, "preview.png")
    preview.save(out)
    print(f"wrote {out}")


if __name__ == "__main__":
    os.makedirs(OUT_DIR, exist_ok=True)
    bg = gen_background()
    fg = gen_foreground()
    gen_preview(bg, fg)
