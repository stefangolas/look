"""Compose a labelled side-by-side renderer comparison.

The comparison images in `docs/images` are built with this script so they can
be regenerated rather than hand-assembled. Text is drawn with a TrueType font
resolved from the host; the bitmap fallback PIL ships cannot render the
punctuation used here and silently substitutes boxes.

Usage:
    python benchmarks/make-comparison.py \
        --left target/bench/pbr/DamagedHelmet-v3.png --left-label "look" \
        --left-note "602 ms median" \
        --right target/bench/pbr/DamagedHelmet-f3d.png --right-label "F3D 3.5" \
        --right-note "939 ms median" \
        --title "Khronos Damaged Helmet" \
        --footer "6 fresh launches each, 512x512, source PBR" \
        --output docs/images/damaged-helmet-look-vs-f3d.png
"""

import argparse
import sys
from pathlib import Path

from PIL import Image, ImageDraw, ImageFont

BACKGROUND = (24, 24, 24)
TITLE_RGB = (245, 245, 245)
LEFT_RGB = (122, 178, 255)
RIGHT_RGB = (245, 166, 87)
FOOTER_RGB = (170, 170, 170)
DIVIDER_RGB = (90, 90, 90)

FONT_CANDIDATES = [
    "C:/Windows/Fonts/segoeui.ttf",
    "C:/Windows/Fonts/arial.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans.ttf",
    "/System/Library/Fonts/Helvetica.ttc",
]
BOLD_CANDIDATES = [
    "C:/Windows/Fonts/segoeuib.ttf",
    "C:/Windows/Fonts/arialbd.ttf",
    "/usr/share/fonts/truetype/dejavu/DejaVuSans-Bold.ttf",
    "/System/Library/Fonts/Helvetica.ttc",
]


def load_font(candidates, size):
    for path in candidates:
        if Path(path).exists():
            return ImageFont.truetype(path, size)
    raise SystemExit(
        "no TrueType font found; pass one explicitly or install DejaVu.\n"
        "The bitmap fallback cannot render this text correctly."
    )


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--left", required=True)
    parser.add_argument("--right", required=True)
    parser.add_argument("--left-label", required=True)
    parser.add_argument("--right-label", required=True)
    parser.add_argument("--left-note", default="")
    parser.add_argument("--right-note", default="")
    parser.add_argument("--title", required=True)
    parser.add_argument("--footer", default="")
    parser.add_argument("--output", required=True)
    parser.add_argument("--gap", type=int, default=2)
    args = parser.parse_args()

    left = Image.open(args.left).convert("RGB")
    right = Image.open(args.right).convert("RGB")
    if left.size != right.size:
        # Unequal panels would misrepresent the comparison by cropping or
        # rescaling one renderer's output relative to the other.
        raise SystemExit(
            f"panels differ in size: {left.size} vs {right.size}; "
            "render both at the same resolution"
        )

    title_font = load_font(BOLD_CANDIDATES, 27)
    label_font = load_font(FONT_CANDIDATES, 21)
    footer_font = load_font(FONT_CANDIDATES, 17)

    header = 96
    footer_h = 42 if args.footer else 0
    width = left.width + args.gap + right.width
    height = header + left.height + footer_h

    canvas = Image.new("RGB", (width, height), BACKGROUND)
    canvas.paste(left, (0, header))
    canvas.paste(right, (left.width + args.gap, header))

    draw = ImageDraw.Draw(canvas)
    draw.text((22, 22), args.title, font=title_font, fill=TITLE_RGB)

    left_text = args.left_label + (f"   {args.left_note}" if args.left_note else "")
    right_text = args.right_label + (f"   {args.right_note}" if args.right_note else "")
    draw.text((22, 62), left_text, font=label_font, fill=LEFT_RGB)
    draw.text((left.width + args.gap + 22, 62), right_text, font=label_font, fill=RIGHT_RGB)

    if args.gap:
        divider = left.width
        draw.rectangle([divider, header, divider + args.gap - 1, header + left.height],
                       fill=DIVIDER_RGB)

    if args.footer:
        draw.text((22, header + left.height + 12), args.footer,
                  font=footer_font, fill=FOOTER_RGB)

    Path(args.output).parent.mkdir(parents=True, exist_ok=True)
    canvas.save(args.output, optimize=True)

    # Guard against the failure that produced the previous batch: characters the
    # font cannot draw are silently replaced, so verify every glyph resolves.
    missing = set()
    for text in (args.title, left_text, right_text, args.footer):
        for char in text:
            if char.strip() and title_font.getmask(char).getbbox() is None:
                missing.add(char)
    if missing:
        print(f"warning: font cannot render {sorted(missing)}", file=sys.stderr)

    print(f"wrote {args.output} ({canvas.width}x{canvas.height})")


if __name__ == "__main__":
    main()
