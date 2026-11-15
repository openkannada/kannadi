"""Build missing vowel signs from outlines the font already has.

ೄ (U+0CC4) is ೃ (U+0CC3) with the length tail that turns ು (U+0CC1) into ೂ (U+0CC2).
ೢ (U+0CE2) and ೣ (U+0CE3) are ಌ (U+0C8C) and ೡ (U+0CE1) drawn as subscripts, in the box
and stroke weight of the font's own subscript ಲ.
"""

from __future__ import annotations

import io

import pathops
import uharfbuzz as hb
from fontTools.pens.cu2quPen import Cu2QuPen
from fontTools.pens.recordingPen import DecomposingRecordingPen
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont

U_SIGN, UU_SIGN, R_SIGN, RR_SIGN = 0x0CC1, 0x0CC2, 0x0CC3, 0x0CC4
L_SIGN, LL_SIGN = 0x0CE2, 0x0CE3
LETTER_R, LETTER_RR, LETTER_L, LETTER_LL = 0x0C8B, 0x0CE0, 0x0C8C, 0x0CE1
SUBSCRIPT_MODELS = ("ಕ್ಲ", "ಕ್ಳ", "ಕ್ವ")


class ComposeError(ValueError):
    pass


def contours(font: TTFont, name: str) -> list[tuple]:
    """The glyph's contours with components decomposed, each as a tuple of pen calls."""
    pen = DecomposingRecordingPen(font.getGlyphSet())
    font.getGlyphSet()[name].draw(pen)
    out, current = [], []
    for op, args in pen.value:
        current.append((op, tuple(args)))
        if op in ("closePath", "endPath"):
            out.append(tuple(current))
            current = []
    return out


def to_path(cs) -> pathops.Path:
    path = pathops.Path()
    pen = path.getPen()
    for contour in cs:
        for op, args in contour:
            getattr(pen, op)(*args)
    return path


def tail(font: TTFont, short: str, long: str) -> pathops.Path:
    """The length tail: what long adds to the right of short.

    Contours of long that lie mostly outside short are the tail when it is drawn apart. When
    the tail is merged into 1 outline, it is the part of long right of short's advance, less
    1 stem width so it overlaps whatever it is joined to.
    """
    base = to_path(contours(font, short))
    base.simplify(fix_winding=True)
    apart = []
    for contour in contours(font, long):
        piece = to_path([contour])
        piece.simplify(fix_winding=True)
        inside = pathops.op(piece, base, pathops.PathOp.INTERSECTION).area
        if piece.area and inside / piece.area < 0.5:
            apart.append(contour)
    if apart and len(apart) < len(contours(font, long)) and _closed(font, long, apart):
        return to_path(apart)
    whole = to_path(contours(font, long))
    whole.simplify(fix_winding=True)
    x0, y0, x1, y1 = whole.bounds
    cut = font["hmtx"][short][0] - stem(base)
    if cut >= x1:
        raise ComposeError(f"{long} has nothing right of {short}")
    box = pathops.Path()
    pen = box.getPen()
    pen.moveTo((cut, y0 - 1))
    pen.lineTo((cut, y1 + 1))
    pen.lineTo((x1 + 1, y1 + 1))
    pen.lineTo((x1 + 1, y0 - 1))
    pen.closePath()
    return pathops.op(whole, box, pathops.PathOp.INTERSECTION)


def _closed(font: TTFont, long: str, apart: list[tuple]) -> bool:
    """True when the apart contours hold ink of their own, not just counters of the rest."""
    rest = to_path([c for c in contours(font, long) if c not in apart])
    rest.simplify(fix_winding=True)
    piece = to_path(apart)
    piece.simplify(fix_winding=True)
    return pathops.op(piece, rest, pathops.PathOp.DIFFERENCE).area > 0.5 * piece.area


def compose_rr_sign(font: TTFont) -> str:
    """Add ೄ as ೃ plus the ೂ tail, shifted by the difference of the 2 base widths."""
    cmap = font.getBestCmap()
    _require(cmap, R_SIGN, U_SIGN, UU_SIGN)
    r, u, uu = cmap[R_SIGN], cmap[U_SIGN], cmap[UU_SIGN]
    hmtx = font["hmtx"]
    dx = hmtx[r][0] - hmtx[u][0]
    path = to_path(contours(font, r))
    path.simplify(fix_winding=True)
    path = pathops.op(path, tail(font, u, uu).transform(1, 0, 0, 1, dx, 0), pathops.PathOp.UNION)
    advance = hmtx[r][0] + hmtx[uu][0] - hmtx[u][0]
    return _add_glyph(font, "uni0CC4", RR_SIGN, path, advance, like=r)


def compose_l_signs(font: TTFont) -> list[str]:
    """Add ೢ and, where the font has ೡ or ೠ, ೣ, as subscript forms of the letters."""
    cmap = font.getBestCmap()
    _require(cmap, LETTER_L)
    model = _subscript_model(font)
    added = [
        _add_subscript(font, "uni0CE2", L_SIGN, to_path(contours(font, cmap[LETTER_L])), model)
    ]
    if LETTER_LL in cmap:
        letter = to_path(contours(font, cmap[LETTER_LL]))
    elif LETTER_RR in cmap and LETTER_R in cmap:
        hmtx = font["hmtx"]
        dx = hmtx[cmap[LETTER_L]][0] - hmtx[cmap[LETTER_R]][0]
        letter = to_path(contours(font, cmap[LETTER_L]))
        letter.simplify(fix_winding=True)
        extra = tail(font, cmap[LETTER_R], cmap[LETTER_RR]).transform(1, 0, 0, 1, dx, 0)
        letter = pathops.op(letter, extra, pathops.PathOp.UNION)
    else:
        return added
    added.append(_add_subscript(font, "uni0CE3", LL_SIGN, letter, model))
    return added


def stem(path: pathops.Path) -> float:
    """Mean stroke width of an outline: 2 x area / perimeter, close for long thin strokes."""
    flat = pathops.Path()
    flat.addPath(path)
    flat.convertConicsToQuads()
    perimeter = 0.0
    for contour in _polylines(flat):
        perimeter += sum(
            ((x1 - x0) ** 2 + (y1 - y0) ** 2) ** 0.5
            for (x0, y0), (x1, y1) in zip(contour, contour[1:] + contour[:1], strict=True)
        )
    return 2 * abs(flat.area) / perimeter if perimeter else 0.0


def _polylines(path: pathops.Path, steps: int = 8) -> list[list[tuple[float, float]]]:
    out, current, last = [], [], (0.0, 0.0)
    for verb, pts in path.segments:
        if verb == "moveTo":
            current, last = [pts[0]], pts[0]
        elif verb == "lineTo":
            current.append(pts[0])
            last = pts[0]
        elif verb in ("qCurveTo", "curveTo"):
            ctrl = [last, *pts]
            for i in range(1, steps + 1):
                current.append(_bezier(ctrl, i / steps))
            last = pts[-1]
        elif verb in ("closePath", "endPath"):
            if current:
                out.append(current)
            current = []
    return out


def _bezier(ctrl, t):
    pts = list(ctrl)
    while len(pts) > 1:
        pts = [
            ((1 - t) * a[0] + t * b[0], (1 - t) * a[1] + t * b[1])
            for a, b in zip(pts, pts[1:], strict=False)
        ]
    return pts[0]


def _require(cmap, *cps):
    missing = [f"U+{cp:04X}" for cp in cps if cp not in cmap]
    if missing:
        raise ComposeError(f"the font lacks {' '.join(missing)}")


def _shape(font: TTFont, text: str) -> list[str]:
    buf = io.BytesIO()
    font.save(buf)
    hb_font = hb.Font(hb.Face(hb.Blob(buf.getvalue())))
    b = hb.Buffer()
    b.add_str(text)
    b.guess_segment_properties()
    hb.shape(hb_font, b)
    order = font.getGlyphOrder()
    return [order[i.codepoint] for i in b.glyph_infos]


def _subscript_model(font: TTFont) -> str:
    """The font's own subscript glyph for ಲ, ಳ or ವ under ಕ."""
    cmap = font.getBestCmap()
    for text in SUBSCRIPT_MODELS:
        if not all(ord(c) in cmap for c in text):
            continue
        names = _shape(font, text)
        if len(names) == 2 and names[0] == cmap[ord(text[0])] and names[1] != cmap[ord(text[2])]:
            return names[1]
    raise ComposeError("no subscript consonant to take the size and weight from")


def _add_subscript(font, name, cp, letter: pathops.Path, model: str) -> str:
    """Fit letter into the model subscript's box, thicken it to the model's stem, add it."""
    target = to_path(contours(font, model))
    target.simplify(fix_winding=True)
    want = stem(target)
    gap = 0.0
    for _ in range(3):
        path = _fit(letter, target.bounds, inset=gap / 2)
        gap = max(0.0, want - stem(path))
    if gap > 1:
        stroke = pathops.Path()
        stroke.addPath(path)
        stroke.stroke(gap, pathops.LineCap.ROUND_CAP, pathops.LineJoin.ROUND_JOIN, 4)
        stroke.convertConicsToQuads()
        path = pathops.op(path, stroke, pathops.PathOp.UNION, fix_winding=True)
    return _add_glyph(font, name, cp, path, font["hmtx"][model][0], like=model)


def _fit(letter: pathops.Path, box, inset: float) -> pathops.Path:
    """Scale letter to the box height less the inset, centered across and resting on its floor."""
    bx0, by0, bx1, by1 = box
    lx0, ly0, lx1, ly1 = letter.bounds
    scale = (by1 - by0 - 2 * inset) / (ly1 - ly0)
    dx = (bx0 + bx1) / 2 - (lx0 + lx1) / 2 * scale
    path = letter.transform(scale, 0, 0, scale, dx, by0 + inset - ly0 * scale)
    path.simplify(fix_winding=True)
    return path


def _add_glyph(font: TTFont, name: str, cp: int, path: pathops.Path, advance: int, like: str):
    """Add a TrueType glyph that copies cmap, GDEF class and mark positioning from like."""
    if "glyf" not in font or "gvar" in font:
        raise ComposeError("only static TrueType fonts are supported")
    if name in font.getGlyphOrder():
        name = f"{name}.composed"
    path.simplify(fix_winding=True, clockwise=True)
    path.convertConicsToQuads()
    pen = TTGlyphPen(None)
    path.draw(Cu2QuPen(pen, max_err=1.0))
    glyph = pen.glyph()
    glyph.recalcBounds(font["glyf"])
    font.setGlyphOrder([*font.getGlyphOrder(), name])
    font["glyf"][name] = glyph
    font["hmtx"][name] = (round(advance), getattr(glyph, "xMin", 0))
    for table in font["cmap"].tables:
        if table.isUnicode() and (cp <= 0xFFFF or table.format in (12, 13)):
            table.cmap[cp] = name
    if "GDEF" in font and font["GDEF"].table.GlyphClassDef:
        classes = font["GDEF"].table.GlyphClassDef.classDefs
        if like in classes:
            classes[name] = classes[like]
    if "GPOS" in font:
        _copy_mark_records(font["GPOS"].table, like, name)
    font["maxp"].numGlyphs = len(font.getGlyphOrder())
    return name


def _copy_mark_records(gpos, like: str, name: str) -> None:
    for lookup in gpos.LookupList.Lookup:
        for sub in lookup.SubTable:
            if lookup.LookupType == 9:
                sub = sub.ExtSubTable
            coverage = getattr(sub, "MarkCoverage", None) or getattr(sub, "Mark1Coverage", None)
            records = getattr(sub, "MarkArray", None) or getattr(sub, "Mark1Array", None)
            if coverage is None or like not in coverage.glyphs:
                continue
            record = records.MarkRecord[coverage.glyphs.index(like)]
            # The new glyph has the highest id, so appending keeps the coverage sorted.
            coverage.glyphs.append(name)
            records.MarkRecord.append(record)
            records.MarkCount = len(records.MarkRecord)
