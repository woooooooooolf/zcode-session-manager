"""Generate the 1024x1024 app icon (rounded square + three white session bars)."""
import struct, zlib

SIZE = 1024
R = 200          # corner radius of the square
BG_TOP = (59, 130, 246)
BG_BOT = (29, 78, 216)

# bars: (y_top, height, x_left, x_right) rounded with radius h/2
BARS = [(300, 110, 230, 794), (460, 110, 230, 700), (620, 110, 230, 620)]
BAR_R = 55

def rounded_rect_alpha(px, py, x0, y0, x1, y1, r):
    """1.0 inside rounded rect else 0.0, with 2px AA edge."""
    if px < x0 or px > x1 or py < y0 or py > y1:
        return 0.0
    # distance to the nearest corner center
    cx = min(max(px, x0 + r), x1 - r)
    cy = min(max(py, y0 + r), y1 - r)
    d = ((px - cx) ** 2 + (py - cy) ** 2) ** 0.5
    if cx in (x0 + r, x1 - r) and cy in (y0 + r, y1 - r):
        if d > r:
            return 0.0
        return min(1.0, max(0.0, (r - d) / 2.0))
    return 1.0

rows = []
for y in range(SIZE):
    t = y / (SIZE - 1)
    bg = tuple(BG_TOP[i] + (BG_BOT[i] - BG_TOP[i]) * t for i in range(3))
    # corner alpha of the whole tile
    cx = min(max(0 + R, y * 0 + 0), 0)  # placeholder, computed below per-pixel
    row = bytearray([0])  # filter byte
    for x in range(SIZE):
        # square corner alpha
        ccx = min(max(x, R), SIZE - R)
        ccy = min(max(y, R), SIZE - R)
        d = ((x - ccx) ** 2 + (y - ccy) ** 2) ** 0.5
        if ccx in (R, SIZE - R) and ccy in (R, SIZE - R):
            a = 0 if d > R + 1 else 255 if d < R - 1 else int(255 * (R + 1 - d) / 2)
        else:
            a = 255
        r, g, b = (int(v) for v in bg)
        for (by, bh, bx0, bx1) in BARS:
            m = rounded_rect_alpha(x, y, bx0, by, bx1, by + bh, BAR_R)
            if m > 0:
                r = int(r * (1 - m) + 255 * m)
                g = int(g * (1 - m) + 255 * m)
                b = int(b * (1 - m) + 255 * m)
        row += bytes((r, g, b, a))
    rows.append(bytes(row))

raw = b"".join(rows)

def chunk(tag, data):
    c = struct.pack(">I", len(data)) + tag + data
    return c + struct.pack(">I", zlib.crc32(tag + data) & 0xFFFFFFFF)

png = b"\x89PNG\r\n\x1a\n"
png += chunk(b"IHDR", struct.pack(">IIBBBBB", SIZE, SIZE, 8, 6, 0, 0, 0))
png += chunk(b"IDAT", zlib.compress(raw, 9))
png += chunk(b"IEND", b"")

with open("app-icon.png", "wb") as f:
    f.write(png)
print("app-icon.png written,", len(png), "bytes")
