"""Compose the three theme screenshots into an Aero-task-view style cascade.

The front window (first input) sits centered; the other two stack behind it,
stepping up-left, with rounded corners and soft shadows. The backdrop is a
blurred, darkened copy of the front shot so the composition feels like a
task-view scene rather than a collage.

Usage:
    python tools/make_theme_cascade.py FRONT MID BACK --out OUT
        [--step-x 76] [--step-y 58] [--radius 14] [--blur 28]
"""

import argparse
from PIL import Image, ImageDraw, ImageFilter, ImageEnhance


def rounded(img, radius):
    mask = Image.new("L", img.size, 0)
    d = ImageDraw.Draw(mask)
    d.rounded_rectangle((0, 0, img.size[0] - 1, img.size[1] - 1), radius=radius, fill=255)
    out = img.convert("RGBA")
    out.putalpha(mask)
    return out


def shadow_layer_for(img, radius, blur, alpha, dy):
    """A canvas (img.height + dy tall) holding the blurred drop shadow."""
    w, h = img.size
    mask = Image.new("L", (w, h), 0)
    ImageDraw.Draw(mask).rounded_rectangle((0, 0, w - 1, h - 1), radius=radius, fill=alpha)
    mask = mask.filter(ImageFilter.GaussianBlur(blur))
    layer = Image.new("RGBA", (w, h + dy), (0, 0, 0, 0))
    layer.paste(Image.new("RGBA", (w, h), (0, 0, 0, 255)), (0, dy), mask)
    return layer


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("front", help="front window (e.g. dark theme)")
    ap.add_argument("mid", help="middle window (e.g. light theme)")
    ap.add_argument("back", help="back window (e.g. high contrast)")
    ap.add_argument("--out", required=True)
    ap.add_argument("--step-x", type=int, default=76)
    ap.add_argument("--step-y", type=int, default=58)
    ap.add_argument("--radius", type=int, default=14)
    ap.add_argument("--blur", type=int, default=28)
    args = ap.parse_args()

    front = rounded(Image.open(args.front).convert("RGB"), args.radius)
    mid = rounded(Image.open(args.mid).convert("RGB"), args.radius)
    back = rounded(Image.open(args.back).convert("RGB"), args.radius)
    w, h = front.size

    mx, my = args.step_x * 2 + 40, args.step_y * 2 + 46
    canvas_w, canvas_h = w + mx, h + my

    # backdrop: blurred + darkened copy of the front shot
    bg = front.convert("RGB").resize((canvas_w, canvas_h))
    bg = bg.filter(ImageFilter.GaussianBlur(args.blur))
    bg = ImageEnhance.Brightness(bg).enhance(0.42)
    canvas = bg.convert("RGBA")

    def paste_window(img, pos):
        canvas.alpha_composite(shadow_layer_for(img, args.radius, 14, 120, 16), pos)
        canvas.alpha_composite(img, pos)

    paste_window(back, (0, 0))
    paste_window(mid, (args.step_x, args.step_y))
    paste_window(front, (args.step_x * 2, args.step_y * 2))

    canvas.convert("RGB").save(args.out)
    print(f"composed {args.out} ({canvas_w} x {canvas_h}, cascade step {args.step_x}x{args.step_y})")


if __name__ == "__main__":
    main()
