"""Tiny Kannada test fonts built in memory, each with a known defect or none."""

from __future__ import annotations

from pathlib import Path

from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen

KA, KHA, RA, VIRAMA, E_SIGN, ANUSVARA = 0x0C95, 0x0C96, 0x0CB0, 0x0CCD, 0x0CC6, 0x0C82

GOOD_FEA = """
languagesystem knd2 dflt;
feature rphf { script knd2; sub ra virama by reph; } rphf;
feature blwf { script knd2; sub virama ka by ka.blwf; sub virama kha by kha.blwf; } blwf;
"""

# Gubbi's defect: the below-base rule written in the old Indic order, consonant before virama.
OLD_ORDER_FEA = """
languagesystem knd2 dflt;
feature rphf { script knd2; sub ra virama by reph; } rphf;
feature blwf { script knd2; sub ka virama by ka.blwf; sub kha virama by kha.blwf; } blwf;
"""

NO_REPH_FEA = """
languagesystem knd2 dflt;
feature blwf { script knd2; sub virama ka by ka.blwf; sub virama kha by kha.blwf; } blwf;
"""


def _rect(x0, y0, x1, y1):
    pen = TTGlyphPen(None)
    pen.moveTo((x0, y0))
    pen.lineTo((x0, y1))
    pen.lineTo((x1, y1))
    pen.lineTo((x1, y0))
    pen.closePath()
    return pen.glyph()


def build(
    path: Path,
    fea: str = GOOD_FEA,
    marks_collide: bool = False,
    mark_on_base: bool = False,
    anusvara_like_esign: bool = False,
    drop: tuple[int, ...] = (),
    empty: tuple[int, ...] = (),
) -> Path:
    """Write a font whose top marks overlap each other or the base, as asked."""
    names = {
        KA: "ka",
        KHA: "kha",
        RA: "ra",
        VIRAMA: "virama",
        E_SIGN: "esign",
        ANUSVARA: "anusvara",
    }
    order = [".notdef", "ka", "kha", "ra", "virama", "esign", "anusvara", "ka.blwf", "kha.blwf",
             "reph"]  # fmt: skip
    anusvara_x = (-450, -250) if marks_collide else (-200, -50)
    glyphs = {
        ".notdef": _rect(50, 0, 450, 700),
        "ka": _rect(50, 0, 450, 500),
        "kha": _rect(50, 0, 450, 520),
        "ra": _rect(50, 0, 450, 480),
        "virama": _rect(-300, 550, -100, 650),
        "esign": _rect(-450, 300, -250, 800) if mark_on_base else _rect(-450, 600, -250, 800),
        "anusvara": _rect(anusvara_x[0], 600, anusvara_x[1], 800),
        "ka.blwf": _rect(-450, -300, -50, -50),
        "kha.blwf": _rect(-450, -320, -50, -50),
        "reph": _rect(20, 500, 180, 700),
    }
    if anusvara_like_esign:
        glyphs["anusvara"] = _rect(-450, 600, -250, 800)
    for cp in empty:
        glyphs[names[cp]] = TTGlyphPen(None).glyph()
    advance = {n: 0 for n in ("virama", "esign", "anusvara", "ka.blwf", "kha.blwf")}
    fb = FontBuilder(1000, isTTF=True)
    fb.setupGlyphOrder(order)
    fb.setupCharacterMap({cp: n for cp, n in names.items() if cp not in drop})
    fb.setupGlyf(glyphs)
    glyf = fb.font["glyf"]
    lsb = {n: getattr(glyf[n], "xMin", 0) for n in order}
    fb.setupHorizontalMetrics({n: (advance.get(n, 500), lsb[n]) for n in order})
    fb.setupHorizontalHeader(ascent=800, descent=-300)
    fb.setupNameTable({"familyName": "Synth", "styleName": "Regular"})
    fb.setupOS2()
    fb.setupPost()
    addOpenTypeFeaturesFromString(fb.font, fea)
    fb.save(str(path))
    return path
