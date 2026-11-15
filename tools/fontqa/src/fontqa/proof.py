"""Proof sheets: each failing string drawn in the tested font, beside a reference font."""

from __future__ import annotations

from pathlib import Path

import freetype
from PIL import Image, ImageChops, ImageDraw, ImageFont

from .checks import FAILURES, Result
from .shaping import Face

CELL_W, CELL_H = 260, 240
TEXT_PX, REF_PX = 80, 36
INK = (20, 20, 20)
RED = (200, 30, 30)


class Renderer:
    """Rasterizes HarfBuzz output with FreeType, at the default instance."""

    def __init__(self, face: Face):
        self.face = face
        self.ft = freetype.Face(str(face.path), index=face.index)

    def draw(self, canvas: Image.Image, text: str, origin: tuple[int, int], px: int, color=INK):
        """Draw text with its baseline starting at origin. Returns font units to pixels."""
        self.ft.set_char_size(px * 64)
        scale = px / self.face.upem
        glyphs = self.face.shape(text)
        width = round(sum(g.advance for g in glyphs) * scale) + 2 * px
        pad = 2 * px
        mask = Image.new("L", (width + 2 * pad, 4 * px), 0)
        baseline = (pad, 2 * px + px // 2)
        for g in glyphs:
            self.ft.load_glyph(g.gid, freetype.FT_LOAD_NO_HINTING | freetype.FT_LOAD_RENDER)
            slot = self.ft.glyph
            bm = slot.bitmap
            if bm.width == 0 or bm.rows == 0:
                continue
            img = Image.frombytes("L", (bm.width, bm.rows), bytes(bm.buffer), "raw", "L", bm.pitch)
            x = baseline[0] + round(g.x * scale) + slot.bitmap_left
            y = baseline[1] - round(g.y * scale) - slot.bitmap_top
            layer = Image.new("L", mask.size, 0)
            layer.paste(img, (x, y))
            mask = ImageChops.lighter(mask, layer)
        top_left = (origin[0] - baseline[0], origin[1] - baseline[1])
        canvas.paste(Image.new("RGB", mask.size, color), top_left, mask)
        return scale


def sheet(face: Face, results: list[Result], out: Path, reference: Face | None, per_kind: int = 40):
    """Write 1 PNG with up to per_kind failing cases of each failure kind."""
    picked: list[tuple[str, Result]] = []
    for kind in FAILURES:
        hits = [r for r in results if kind in r.failures]
        picked += [(kind, r) for r in hits[:per_kind]]
    if not picked:
        return None
    cols = 6
    rows = (len(picked) + cols - 1) // cols
    header = 70
    canvas = Image.new("RGB", (cols * CELL_W, header + rows * CELL_H), "white")
    draw = ImageDraw.Draw(canvas)
    small = ImageFont.load_default(size=14)
    title = ImageFont.load_default(size=22)
    info = face.info
    draw.text((12, 10), f"{info['full_name']}  {info['version']}", fill=INK, font=title)
    draw.text(
        (12, 42),
        "Large: the tested font. Small: the reference font. Red box: the ink overlap.",
        fill=INK,
        font=small,
    )
    main = Renderer(face)
    ref = Renderer(reference) if reference else None
    for i, (kind, r) in enumerate(picked):
        x0 = (i % cols) * CELL_W
        y0 = header + (i // cols) * CELL_H
        draw.rectangle([x0, y0, x0 + CELL_W - 1, y0 + CELL_H - 1], outline=(210, 210, 210))
        base = (x0 + 24, y0 + 115)
        scale = main.draw(canvas, r.case.text, base, TEXT_PX)
        for c in r.collisions:
            x_min, y_min, x_max, y_max = c.box
            draw.rectangle(
                [
                    base[0] + x_min * scale,
                    base[1] - y_max * scale,
                    base[0] + x_max * scale,
                    base[1] - y_min * scale,
                ],
                outline=RED,
                width=2,
            )
        if ref:
            ref.draw(canvas, r.case.text, (x0 + 24, y0 + 175), REF_PX)
        cps = r.case.codepoints.split()
        label = " ".join(cps[:4]) + ("\n" + " ".join(cps[4:]) if len(cps) > 4 else "")
        draw.text((x0 + 8, y0 + 186), kind, fill=RED, font=small)
        draw.text((x0 + 8, y0 + 202), label, fill=INK, font=small, spacing=2)
    out.parent.mkdir(parents=True, exist_ok=True)
    canvas.save(out, optimize=True)
    return out


def comparison(faces: list[tuple[str, Face]], texts: list[str], out: Path, px: int = 64) -> Path:
    """1 row per face, 1 column per string: a before and after sheet for a fix."""
    label_w, cell_w, cell_h = 260, max(px * 3, 160), px * 2 + 20
    canvas = Image.new("RGB", (label_w + cell_w * len(texts), cell_h * len(faces) + 10), "white")
    draw = ImageDraw.Draw(canvas)
    small = ImageFont.load_default(size=16)
    for row, (label, face) in enumerate(faces):
        y = row * cell_h + 10
        draw.text((10, y + cell_h // 2 - 10), label, fill=INK, font=small)
        renderer = Renderer(face)
        for col, text in enumerate(texts):
            x = label_w + col * cell_w
            draw.line([x, y, x, y + cell_h], fill=(220, 220, 220))
            renderer.draw(canvas, text, (x + 16, y + int(px * 1.25)), px)
        draw.line([0, y + cell_h - 1, canvas.width, y + cell_h - 1], fill=(220, 220, 220))
    out.parent.mkdir(parents=True, exist_ok=True)
    canvas.save(out, optimize=True)
    return out
