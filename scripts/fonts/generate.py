#!/usr/bin/env python3
"""Rasterize the firmware's 1-bpp bitmap fonts from open-source font files.

Downloads pinned font sources, renders every strike with FreeType monochrome
hinting and rewrites the Rust atlases listed in OUTPUTS. Needs Pillow and
fontTools; the `fonts` GitHub workflow installs them, runs this script from the
repository root and uploads the regenerated files.
"""

import io
import sys
import urllib.request
import zipfile
from pathlib import Path

from fontTools.ttLib import TTFont
from PIL import Image, ImageDraw, ImageFont

GOOGLE_FONTS = (
    "https://raw.githubusercontent.com/google/fonts/"
    "9710da1eacb3be272583c3224dcb70f9da6eadbb/ofl/"
)
DEJAVU_ZIP = (
    "https://github.com/dejavu-fonts/dejavu-fonts/releases/download/"
    "version_2_37/dejavu-fonts-ttf-2.37.zip"
)
SOURCES = {
    "inter": GOOGLE_FONTS + "inter/Inter%5Bopsz%2Cwght%5D.ttf",
    "atkinson-regular": GOOGLE_FONTS + "atkinsonhyperlegible/AtkinsonHyperlegible-Regular.ttf",
    "atkinson-bold": GOOGLE_FONTS + "atkinsonhyperlegible/AtkinsonHyperlegible-Bold.ttf",
    "atkinson-next": GOOGLE_FONTS
    + "atkinsonhyperlegiblenext/AtkinsonHyperlegibleNext%5Bwght%5D.ttf",
    "literata": GOOGLE_FONTS + "literata/Literata%5Bopsz%2Cwght%5D.ttf",
    "dejavu-serif": (DEJAVU_ZIP, "dejavu-fonts-ttf-2.37/ttf/DejaVuSerif.ttf"),
}

# Glyph order shared with src/charset.rs: printable ASCII, Latin-1, extras.
EXTRAS = "\u2013\u2014\u2018\u2019\u201a\u201c\u201d\u201e\u2022\u2026\u2039\u203a\u20ac\u2122\u2212"
CHARSET = (
    [chr(code) for code in range(0x20, 0x7F)]
    + [chr(code) for code in range(0xA0, 0x100)]
    + list(EXTRAS)
)

MEDIUM = {"wght": 500, "opsz": 14}
SEMIBOLD = {"wght": 600, "opsz": 14}


def ui_strikes(prefix, regular, bold, sizes):
    """UI roles: Detail and Body use the regular weight, Heading and Large the bold one."""
    strikes = []
    for profile, roles in sizes.items():
        for role, (pixels, line_height) in roles.items():
            source, axes = regular if role in ("DETAIL", "BODY") else bold
            name = f"{prefix}_{profile}_{role}"
            strikes.append((name, source, axes, ("px", pixels), line_height))
    return strikes


# Pixel sizes and line heights match the previous ASCII-only atlases.
UI_FONTS = ui_strikes(
    "INTER",
    ("inter", MEDIUM),
    ("inter", SEMIBOLD),
    {
        "COMPACT": {"DETAIL": (11, 16), "BODY": (13, 18), "HEADING": (18, 24), "LARGE": (22, 29)},
        "STANDARD": {"DETAIL": (12, 17), "BODY": (15, 20), "HEADING": (20, 26), "LARGE": (24, 31)},
        "LARGE": {"DETAIL": (13, 18), "BODY": (17, 23), "HEADING": (22, 29), "LARGE": (26, 34)},
    },
) + ui_strikes(
    "ATKINSON",
    ("atkinson-regular", {}),
    ("atkinson-bold", {}),
    {
        "COMPACT": {"DETAIL": (11, 13), "BODY": (13, 15), "HEADING": (18, 20), "LARGE": (22, 24)},
        "STANDARD": {"DETAIL": (12, 13), "BODY": (15, 17), "HEADING": (20, 22), "LARGE": (24, 26)},
        "LARGE": {"DETAIL": (13, 15), "BODY": (17, 20), "HEADING": (22, 24), "LARGE": (26, 28)},
    },
)


def reader_strikes(prefix, source, axes, caps):
    """Reader sizes are chosen by capital height to keep the previous page layout."""
    sizes = ("SMALL", "MEDIUM", "LARGE", "XLARGE")
    line_heights = (20, 24, 28, 33)
    return [
        (f"{prefix}_{size}", source, axes, ("cap", cap), line_height)
        for size, cap, line_height in zip(sizes, caps, line_heights)
    ]


OUTPUTS = [
    (
        "src/app/typography/assets.rs",
        "UI glyph atlases: Inter (Medium, SemiBold) and Atkinson Hyperlegible (Regular, Bold).",
        "use super::{BitmapFont, Glyph};",
        UI_FONTS,
    ),
    (
        "src/app/reader_atkinson_next_assets.rs",
        "Reader glyph atlases: Atkinson Hyperlegible Next Medium.",
        "use super::typography::{BitmapFont, Glyph};",
        reader_strikes("ATKINSON_NEXT", "atkinson-next", {"wght": 500}, (12, 13, 16, 18)),
    ),
    (
        "src/app/reader_literata_assets.rs",
        "Reader glyph atlases: Literata Medium.",
        "use super::typography::{BitmapFont, Glyph};",
        reader_strikes("LITERATA", "literata", {"wght": 500, "opsz": "size"}, (12, 15, 16, 19)),
    ),
    (
        "src/app/reader_serif_assets.rs",
        "Reader glyph atlases: DejaVu Serif.",
        "use super::typography::{BitmapFont, Glyph};",
        reader_strikes("SERIF", "dejavu-serif", {}, (12, 14, 16, 19)),
    ),
]


def download(source):
    if isinstance(source, tuple):
        url, member = source
        with urllib.request.urlopen(url) as response:
            archive = zipfile.ZipFile(io.BytesIO(response.read()))
        return archive.read(member)
    with urllib.request.urlopen(source) as response:
        return response.read()


def load_font(data, pixels, axes):
    font = ImageFont.truetype(io.BytesIO(data), pixels)
    if axes:
        values = []
        for axis in TTFont(io.BytesIO(data))["fvar"].axes:
            value = axes.get(axis.axisTag, axis.defaultValue)
            if value == "size":
                value = pixels
            values.append(min(max(value, axis.minValue), axis.maxValue))
        font.set_variation_by_axes(values)
    return font


def ink_box(font, text):
    left, top, right, bottom = font.getbbox(text, mode="1", anchor="ls")
    return left, top, right, bottom


def size_for_cap(data, axes, cap):
    """Smallest pixel size whose capital H is `cap` pixels tall."""
    best = None
    for pixels in range(8, 64):
        _, top, _, bottom = ink_box(load_font(data, pixels, axes), "H")
        height = bottom - top
        if height == cap:
            return pixels
        if best is None or abs(height - cap) < abs(best[1] - cap):
            best = (pixels, height)
    return best[0]


def rasterize(font, character):
    advance = int(round(font.getlength(character, mode="1")))
    left, top, right, bottom = ink_box(font, character)
    if right <= left or bottom <= top:
        return (0, 0, advance, 0, 0, b"")
    image = Image.new("1", (right - left, bottom - top), 0)
    draw = ImageDraw.Draw(image)
    draw.fontmode = "1"
    draw.text((-left, -top), character, font=font, fill=1, anchor="ls")
    box = image.getbbox()
    if box is None:
        return (0, 0, advance, 0, 0, b"")
    image = image.crop(box)
    width, height = image.size
    stride = (width + 7) // 8
    rows = bytearray()
    for y in range(height):
        for column in range(stride):
            value = 0
            for bit in range(8):
                x = column * 8 + bit
                if x < width and image.getpixel((x, y)):
                    value |= 0x80 >> bit
            rows.append(value)
    return (width, height, advance, left + box[0], top + box[1], bytes(rows))


def build_strike(name, data, axes, size, line_height, coverage):
    kind, value = size
    pixels = value if kind == "px" else size_for_cap(data, axes, value)
    font = load_font(data, pixels, axes)
    fallback = rasterize(font, "?")
    glyphs, bitmap, missing = [], bytearray(), []
    for character in CHARSET:
        if ord(character) in coverage or character == " ":
            glyph = rasterize(font, character)
        else:
            glyph = fallback
            missing.append(character)
        width, height, advance, left, top, rows = glyph
        glyphs.append((len(bitmap), width, height, advance, left, top))
        bitmap.extend(rows)
    cap = ink_box(font, "H")
    summary = f"{name}: {pixels} px, cap {cap[3] - cap[1]}, line height {line_height}"
    if missing:
        summary += f", missing {''.join(missing)!r}"
    return summary, glyphs, bytes(bitmap), pixels


def emit(name, glyphs, bitmap, line_height, description):
    lines = [f"// {description}", f"pub static {name}: BitmapFont = BitmapFont {{", "    glyphs: &["]
    for offset, width, height, advance, left, top in glyphs:
        lines.append(f"        Glyph::new({offset}, {width}, {height}, {advance}, {left}, {top}),")
    lines.append("    ],")
    lines.append("    bitmap: &[")
    for start in range(0, len(bitmap), 20):
        chunk = ", ".join(f"0x{byte:02X}" for byte in bitmap[start : start + 20])
        lines.append(f"        {chunk},")
    lines.append("    ],")
    lines.append(f"    line_height: {line_height},")
    lines.append("};")
    return "\n".join(lines)


def main():
    root = Path(sys.argv[1]) if len(sys.argv) > 1 else Path(".")
    cache = {}
    for path, title, imports, strikes in OUTPUTS:
        blocks = []
        for name, source, axes, size, line_height in strikes:
            if source not in cache:
                data = download(SOURCES[source])
                cache[source] = (data, set(TTFont(io.BytesIO(data)).getBestCmap()))
            data, coverage = cache[source]
            summary, glyphs, bitmap, pixels = build_strike(
                name, data, axes, size, line_height, coverage
            )
            print(summary)
            blocks.append(emit(name, glyphs, bitmap, line_height, summary))
        header = (
            f"//! {title}\n"
            "//!\n"
            "//! Generated by scripts/fonts/generate.py; do not edit by hand. Glyph order\n"
            "//! follows src/charset.rs. Source fonts are not distributed with this\n"
            "//! repository; see docs/licenses/FONT_NOTICES.md.\n\n"
            f"{imports}\n\n"
        )
        target = root / path
        target.parent.mkdir(parents=True, exist_ok=True)
        target.write_text(header + "\n\n".join(blocks) + "\n")


if __name__ == "__main__":
    main()
