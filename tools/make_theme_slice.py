"""Compose a three-theme "sliced puzzle" screenshot.

The three input shots must be pixel-aligned (same window, same data, only
the theme differs). Two modes:

    --mode vertical  three equal vertical stripes (default)
    --mode diagonal  two diagonal bands, x/w + y/h = c1 / c2

A soft separator line marks each seam.

Usage:
    python tools/make_theme_slice.py FIRST SECOND THIRD --out OUT
        [--mode vertical|diagonal] [--c1 0.9] [--c2 1.7] [--sep 2]
"""

import argparse
from PIL import Image, ImageDraw


def diagonal_seam(c, w, h):
    """The two points where the line x/w + y/h = c crosses the image rect."""
    pts = []
    for x, y in ((c * w, 0), (0, c * h), (w, (c - 1) * h), ((c - 1) * w, h)):
        if 0 <= x <= w and 0 <= y <= h:
            pts.append((x, y))
    return pts[:2]


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("first", help="image for the first band")
    ap.add_argument("second", help="image for the second band")
    ap.add_argument("third", help="image for the third band")
    ap.add_argument("--out", required=True)
    ap.add_argument("--mode", choices=["vertical", "diagonal"], default="vertical")
    ap.add_argument("--c1", type=float, default=0.9, help="diagonal: first seam, x/w + y/h = c1")
    ap.add_argument("--c2", type=float, default=1.7, help="diagonal: second seam, x/w + y/h = c2")
    ap.add_argument("--sep", type=int, default=2, help="separator width in px")
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
    seams = []

    if args.mode == "vertical":
        x1, x2 = round(w / 3), round(w * 2 / 3)
        mask = Image.new("L", (w, h), 0)
        ImageDraw.Draw(mask).rectangle((x1, 0, x2 - 1, h), fill=255)
        out.paste(b, (0, 0), mask)
        mask = Image.new("L", (w, h), 0)
        ImageDraw.Draw(mask).rectangle((x2, 0, w, h), fill=255)
        out.paste(c, (0, 0), mask)
        seams = [(x1, 0), (x1, h)], [(x2, 0), (x2, h)]
    else:
        poly2 = [
            (args.c1 * w, 0),
            (w, 0),
            (w, h * (args.c2 - 1)),
            ((args.c2 - 1) * w, h),
            (0, args.c1 * h),
        ]
        poly3 = [(w, h * (args.c2 - 1)), (w, h), ((args.c2 - 1) * w, h)]
        mask = Image.new("L", (w, h), 0)
        ImageDraw.Draw(mask).polygon(poly2, fill=255)
        out.paste(b, (0, 0), mask)
        mask = Image.new("L", (w, h), 0)
        ImageDraw.Draw(mask).polygon(poly3, fill=255)
        out.paste(c, (0, 0), mask)
        seams = diagonal_seam(args.c1, w, h), diagonal_seam(args.c2, w, h)

    overlay = Image.new("RGBA", (w, h), (0, 0, 0, 0))
    od = ImageDraw.Draw(overlay)
    for p1, p2 in seams:
        od.line([p1, p2], fill=(255, 255, 255, 130), width=args.sep)
    out = Image.alpha_composite(out, overlay).convert("RGB")
    out.save(args.out)
    print(f"composed {args.out} ({w} x {h}, mode={args.mode})")


if __name__ == "__main__":
    main()

