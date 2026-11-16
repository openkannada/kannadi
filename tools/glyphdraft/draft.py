"""Draft missing Kannada glyphs into a FontForge source from skeletons and the font's own parts.

Run with FontForge's Python:

    fontforge -lang=py -script tools/glyphdraft/draft.py FONT.sfd MEASURE.json [CODEPOINT ...]

MEASURE.json is the output of `fontqa measure` for a build of the same font. The default
codepoints are U+0CDC, U+0CDD and U+0CF3. A glyph that already has ink is left alone.
"""

from __future__ import annotations

import json
import statistics
import sys
from pathlib import Path

import fontforge

sys.path.insert(0, str(Path(__file__).parent))
import skeletons  # noqa: E402

CONSONANTS = [cp for cp in range(0x0C95, 0x0CBA) if cp not in (0x0CA9, 0x0CB4)]
LETTER_A = 0x0C85
LETTER_KA = 0x0C95
ANUSVARA = 0x0C82
DEFAULT = (0x0CDC, 0x0CDD, 0x0CF3)


def catmull_rom(points):
    """Cubic segments (c1, c2, end) through the points."""
    segments = []
    for i in range(len(points) - 1):
        p0 = points[max(i - 1, 0)]
        p1, p2 = points[i], points[i + 1]
        p3 = points[min(i + 2, len(points) - 1)]
        c1 = (p1[0] + (p2[0] - p0[0]) / 6, p1[1] + (p2[1] - p0[1]) / 6)
        c2 = (p2[0] - (p3[0] - p1[0]) / 6, p2[1] - (p3[1] - p1[1]) / 6)
        segments.append((c1, c2, p2))
    return segments


class InkPen:
    """Area and perimeter of what a layer draws, with curves cut into straight pieces."""

    STEPS = 8

    def __init__(self):
        self.area = 0.0
        self.perimeter = 0.0
        self._points = []

    def moveTo(self, p):  # noqa: N802
        self._points = [p]

    def lineTo(self, p):  # noqa: N802
        self._points.append(p)

    def curveTo(self, *pts):  # noqa: N802
        start = self._points[-1]
        for c1, c2, end in zip(pts[0::3], pts[1::3], pts[2::3], strict=True):
            self._curve((start, c1, c2, end))
            start = end

    def qCurveTo(self, *pts):  # noqa: N802
        if pts[-1] is None:
            start = _mid(pts[-2], pts[0])
            self._points = [start]
            pts = (*pts[:-1], start)
        start = self._points[-1]
        offs, end = pts[:-1], pts[-1]
        for i, ctrl in enumerate(offs):
            stop = end if i == len(offs) - 1 else _mid(ctrl, offs[i + 1])
            self._curve((start, ctrl, stop))
            start = stop

    def _curve(self, ctrl):
        self._points += [_bezier(ctrl, i / self.STEPS) for i in range(1, self.STEPS + 1)]

    def closePath(self):  # noqa: N802
        pts = self._points
        for (x0, y0), (x1, y1) in zip(pts, pts[1:] + pts[:1], strict=True):
            self.area += (x0 * y1 - x1 * y0) / 2
            self.perimeter += ((x1 - x0) ** 2 + (y1 - y0) ** 2) ** 0.5
        self._points = []

    endPath = closePath  # noqa: N815


def _mid(a, b):
    return ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)


def _bezier(ctrl, t):
    pts = list(ctrl)
    while len(pts) > 1:
        pts = [
            (a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t)
            for a, b in zip(pts, pts[1:], strict=False)
        ]
    return pts[0]


def mean_stroke_width(layer):
    """2 x area / perimeter of the layer's ink, overlaps removed."""
    clean = layer.dup()
    clean.removeOverlap()
    clean.correctDirection()
    pen = InkPen()
    clean.draw(pen)
    return 2 * abs(pen.area) / pen.perimeter if pen.perimeter else 0.0


def without_short_segments(contour, min_length):
    """The contour without on-curve points closer than min_length to the previous one, and
    without their handles."""
    keep, last = [], None
    for p in contour:
        if p.on_curve and last is not None and abs(p.x - last.x) + abs(p.y - last.y) < min_length:
            while keep and not keep[-1].on_curve:
                keep.pop()
            continue
        keep.append(p)
        if p.on_curve:
            last = p
    out = fontforge.contour()
    out.is_quadratic = contour.is_quadratic
    for p in keep:
        out += p
    out.closed = contour.closed
    return out


def glyph_for(font, codepoint):
    for glyph in font.glyphs():
        if glyph.unicode == codepoint:
            return glyph
    return None


def has_ink(glyph):
    return glyph is not None and len(glyph.foreground) > 0


def bounds(glyph):
    xmin, ymin, xmax, ymax = glyph.boundingBox()
    return xmin, ymin, xmax, ymax


class Drafter:
    """Draws glyphs that match 1 font's stems, heights, proportions and spacing."""

    def __init__(self, font, measure):
        self.font = font
        self.vstem = measure["vstem"]
        self.hstem = measure["hstem"]
        self.bottom = measure["bottom"]
        self.top = measure["top"]
        a = bounds(glyph_for(font, LETTER_A))
        self.aspect = (a[2] - a[0]) / (a[3] - a[1])
        ka = glyph_for(font, LETTER_KA)
        self.lsb = ka.left_side_bearing
        self.rsb = ka.width - bounds(ka)[2]
        self.weight = statistics.median(
            mean_stroke_width(g.foreground)
            for g in (glyph_for(font, cp) for cp in CONSONANTS)
            if has_ink(g)
        )

    def stroked(self, glyph, pieces, weights, rounds=4):
        """Stroke skeleton pieces with an elliptical nib in the stems' proportion, scaled over a
        few rounds until the mean stroke width matches the consonants'."""
        scale = 1.0
        for _ in range(rounds):
            self.stroke_once(glyph, pieces, weights, scale)
            scale *= self.weight / mean_stroke_width(glyph.foreground)
        self.stroke_once(glyph, pieces, weights, scale)

    def stroke_once(self, glyph, pieces, weights, scale):
        nib_height = self.hstem * scale
        sy = (self.top - self.bottom) - nib_height
        sx = sy * self.aspect / skeletons.ASPECT_A
        base = self.bottom + nib_height / 2
        ink = fontforge.layer()
        for piece, weight in zip(pieces, weights, strict=True):
            points = [(x * sx, base + y * sy) for x, y in piece]
            contour = fontforge.contour()
            contour.moveTo(*points[0])
            for c1, c2, end in catmull_rom(points):
                contour.cubicTo(c1, c2, end)
            line = fontforge.layer()
            line += contour
            line.stroke("elliptical", nib_height * weight, self.vstem * scale * weight, 90)
            ink += line
        glyph.foreground = ink
        self.clean(glyph)

    def ring(self, glyph, rounds=4):
        """The font's anusvara, scaled down and raised, emboldened over a few rounds until its
        mean stroke width is RING_WEIGHT of the anusvara's."""
        source = glyph_for(self.font, ANUSVARA)
        target = mean_stroke_width(source.foreground) * skeletons.RING_WEIGHT
        s = skeletons.RING_SCALE
        added = 0.0
        for _ in range(rounds + 1):
            glyph.foreground = source.foreground.dup()
            glyph.transform((s, 0, 0, s, 0, 0))
            if added > 0:
                glyph.changeWeight(added, "auto", 0, 0, "squish")
            self.clean(glyph)
            added += target - mean_stroke_width(glyph.foreground)
        _, ymin, _, _ = bounds(glyph)
        glyph.transform((1, 0, 0, 1, 0, self.top * skeletons.RING_BOTTOM - ymin))
        glyph.glyphclass = source.glyphclass

    @staticmethod
    def clean(glyph, min_segment=3):
        glyph.removeOverlap()
        glyph.addExtrema()
        glyph.simplify(1.0, ("mergelines", "removesingletonpoints"))
        glyph.round()
        cleaned = fontforge.layer()
        cleaned.is_quadratic = glyph.foreground.is_quadratic
        for contour in glyph.foreground:
            cleaned += without_short_segments(contour, min_segment)
        glyph.foreground = cleaned
        glyph.correctDirection()

    def space(self, glyph, lsb, rsb):
        xmin, _, xmax, _ = bounds(glyph)
        glyph.transform((1, 0, 0, 1, lsb - xmin, 0))
        glyph.width = round(lsb + (xmax - xmin) + rsb)

    def draft(self, codepoint):
        glyph = self.font.createChar(codepoint, f"uni{codepoint:04X}")
        if codepoint == 0x0CF3:
            self.ring(glyph)
            source = glyph_for(self.font, ANUSVARA)
            s = skeletons.RING_SCALE
            self.space(glyph, source.left_side_bearing * s, self.ring_rsb(source) * s)
            return glyph
        pieces, weights = {
            0x0CDC: (skeletons.ARCHAIC_SHRII, skeletons.ARCHAIC_SHRII_WEIGHTS),
            0x0CDD: (skeletons.NAKAARA_POLLU, skeletons.NAKAARA_POLLU_WEIGHTS),
        }[codepoint]
        self.stroked(glyph, pieces, weights)
        glyph.glyphclass = "baseglyph"
        self.space(glyph, self.lsb, self.rsb)
        return glyph

    @staticmethod
    def ring_rsb(source):
        return source.width - bounds(source)[2]


def main(argv):
    sfd, measure_path, *codes = argv
    codepoints = [int(c, 16) for c in codes] or list(DEFAULT)
    font = fontforge.open(sfd)
    drafter = Drafter(font, json.loads(Path(measure_path).read_text()))
    for cp in codepoints:
        if has_ink(glyph_for(font, cp)):
            print(f"U+{cp:04X}: already has ink, left alone")
            continue
        glyph = drafter.draft(cp)
        print(f"U+{cp:04X}: drafted as {glyph.glyphname}, width {glyph.width}")
    font.save(sfd)


if __name__ == "__main__":
    main(sys.argv[1:])
