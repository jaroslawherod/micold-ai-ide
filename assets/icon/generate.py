#!/usr/bin/env python3
"""Generate the Micold AI IDE raster app icons from the same design as icon.svg.

Reproduce with:  uv run --with pillow python assets/icon/generate.py

Produces (under assets/icon/):
  png/icon-<n>.png   Linux/desktop PNGs (16..1024)
  icon.ico           Windows multi-size icon
  icon.icns          macOS icon
  icon-64.rgba       raw 64x64 RGBA for the iced window icon (include_bytes! in main.rs)

Design: a terminal prompt (caret ">" + cursor block) in white on the brand primary
tile (#005DB8, from tokens.rs). Drawn 4x-supersampled then downscaled (LANCZOS).
"""
from pathlib import Path
from PIL import Image, ImageDraw

PRIMARY = (0, 93, 184, 255)   # #005DB8
FG = (255, 255, 255, 255)
S = 4                          # supersample factor
BASE = 1024 * S

OUT = Path(__file__).parent
(OUT / "png").mkdir(exist_ok=True)


def sc(v: float) -> float:
    return v * S


def render_master() -> Image.Image:
    img = Image.new("RGBA", (BASE, BASE), (0, 0, 0, 0))
    d = ImageDraw.Draw(img)

    # Rounded tile.
    d.rounded_rectangle([0, 0, BASE - 1, BASE - 1], radius=sc(220), fill=PRIMARY)

    # Prompt caret ">": polyline with rounded joins + rounded end caps.
    pts = [(340, 376), (532, 512), (340, 648)]
    w = int(sc(64))
    d.line([(sc(x), sc(y)) for x, y in pts], fill=FG, width=w, joint="curve")
    r = w // 2
    for (x, y) in (pts[0], pts[-1]):
        cx, cy = sc(x), sc(y)
        d.ellipse([cx - r, cy - r, cx + r, cy + r], fill=FG)

    # Cursor block.
    d.rounded_rectangle([sc(576), sc(404), sc(680), sc(620)], radius=sc(20), fill=FG)

    return img.resize((1024, 1024), Image.LANCZOS)


def main() -> None:
    master = render_master()

    sizes = [16, 32, 48, 64, 128, 256, 512, 1024]
    for n in sizes:
        master.resize((n, n), Image.LANCZOS).save(OUT / "png" / f"icon-{n}.png")

    # Windows .ico (embeds several sizes).
    master.save(OUT / "icon.ico", sizes=[(s, s) for s in (16, 32, 48, 64, 128, 256)])

    # macOS .icns.
    master.save(OUT / "icon.icns", format="ICNS")

    # Raw RGBA for the iced window icon (no runtime image decoder needed).
    (OUT / "icon-64.rgba").write_bytes(master.resize((64, 64), Image.LANCZOS).tobytes())

    print("icons written to", OUT)


if __name__ == "__main__":
    main()
