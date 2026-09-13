"""Compose a diagonal three-theme "sliced puzzle" screenshot.

The three input shots must be pixel-aligned (same window, same data, only
the theme differs). The image is split by two diagonal lines running from
top-left to bottom-right — parameterized as x/W + y/H = c — and each band
is filled from one of the inputs, with a soft separator line on the seams.

Usage:
    python tools/make_theme_slice.py FIRST SECOND THIRD --out OUT
        [--c1 0.72] [--c2 1.38] [--sep 3]
"""

import argparse
from PIL import Image, ImageDraw


def edge_points(c, w, h):
    """The two points where the line x/w + y/h = c crosses the image rect."""
    pts = []
    for x, y in ((c * w, 0), (0, c * h), (w, (c - 1) * h), ((c - 1) * w, h)):
        if 0 <= x <= w and 0 <= y <= h:
            pts.append((x, y))
    return pts[:2]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("first", help="image for the top-left band")
    ap.add_argument("second", help="image for the middle band")
    ap.add_argument("third", help="image for the bottom-right band")
    ap.add_argument("--out", required=True)
    ap.add_argument("--c1", type=float, default=0.72, help="first seam, x/w + y/h = c1")
    ap.add_argument("--c2", type=float, default=1.38, help="second seam, x/w + y/h = c2")
    ap.add_argument("--sep", type=int, default=3, help="separator width in px")
    args = ap.parse_args()

    a = Image.open(args.first).convert("RGB")
    b = Image.open(args.second).convert("RGB")
    c = Image.open(args.third).convert("RGB")
    w, h = a.size
    for img in (b, c):
        if img.size != (w, h):
            raise SystemExit(f"input size mismatch: {img.size} != {(w, h)}")

    out = Image.new("RGBA", (w, h))
    out.paste(a, (0, 0))

    poly2 = [(args.c1 * w, 0), (w, 0), (w, h * (args.c2 - 1)), ((args.c2 - 1) * w, h), (0, args.c1 * h)]
    poly3 = [(w, h * (args.c2 - 1)), (w, h), ((args.c2 - 1) * w, h)]
    mask = Image.new("L", (w, h), 0)
    ImageDraw.Draw(mask).polygon(poly2, fill=255)
    out.paste(b, (0, 0), mask)
    mask = Image.new("L", (w, h), 0)
    ImageDraw.Draw(mask).polygon(poly3, fill=255)
    out.paste(c, (0, 0), mask)

    overlay = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    od = ImageDraw.Draw(overlay)
    for cline in (args.c1, args.c2):
        pts = edge_points(cline, w, h)
        if len(pts) == 2:
            od.line([pts[0], pts[1]], fill=(255, 255, 255, 150), width=args.sep)
    out = Image.alpha_composite(out, overlay).convert("RGB")
    out.save(args.out)
    print(f"composed {args.out} ({w} x {h}, seams at {args.c1}/{args.c2})")


if __name__ == "__main__":
    main()
