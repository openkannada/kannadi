"""Automatic gates for drafted glyphs: proportions, contours, master compatibility, shaping."""

from __future__ import annotations

import statistics
from collections import Counter
from dataclasses import dataclass

import pathops
from fontTools.pens.areaPen import AreaPen
from fontTools.pens.recordingPen import RecordingPen, replayRecording
from fontTools.ttLib import TTFont

from .checks import mark_indistinct, run_case
from .kannada import Case
from .outline import polygons, split_contours, to_path
from .shaping import Face

# Curves are cut into this many straight pieces for scanline measurement.
FLATTEN_STEPS = 24


@dataclass(frozen=True)
class GateResult:
    name: str
    passed: bool
    detail: str


@dataclass(frozen=True)
class Strokes:
    """Median ink run widths: vertical stems across rows, horizontal strokes down columns."""

    vstem: float
    hstem: float
    bottom: float
    top: float

    @property
    def contrast(self) -> float:
        return self.vstem / self.hstem if self.hstem else 0.0


def ink_runs(path: pathops.Path, axis: str, at: float) -> list[tuple[float, float]]:
    """Ink intervals where the line y=at (axis "y") or x=at (axis "x") crosses the outline."""
    clean = pathops.Path(path)
    clean.simplify(fix_winding=True)
    i, j = (1, 0) if axis == "y" else (0, 1)
    hits = []
    for poly in polygons(clean, FLATTEN_STEPS):
        for a, b in zip(poly, poly[1:] + poly[:1], strict=True):
            if (a[i] <= at < b[i]) or (b[i] <= at < a[i]):
                t = (at - a[i]) / (b[i] - a[i])
                hits.append(a[j] + t * (b[j] - a[j]))
    hits.sort()
    runs: list[tuple[float, float]] = []
    for a, b in zip(hits[0::2], hits[1::2], strict=False):
        # A line through a vertex where 2 edges meet gives 2 hits at 1 point; that is no gap.
        if runs and a - runs[-1][1] < 1e-6:
            runs[-1] = (runs[-1][0], b)
        else:
            runs.append((a, b))
    return runs


def strokes(path: pathops.Path, lines: int = 21) -> Strokes:
    """Median stem and stroke widths, and the most common bottom and top ink edges."""
    x0, y0, x1, y1 = path.bounds
    rows = [y0 + (y1 - y0) * (k + 1) / (lines + 1) for k in range(lines)]
    cols = [x0 + (x1 - x0) * (k + 1) / (lines + 1) for k in range(lines)]
    v = [b - a for y in rows for a, b in ink_runs(path, "y", y)]
    h, bottoms, tops = [], [], []
    for x in cols:
        runs = ink_runs(path, "x", x)
        h += [b - a for a, b in runs]
        if runs:
            bottoms.append(runs[0][0])
            tops.append(runs[-1][1])
    return Strokes(
        vstem=statistics.median(v) if v else 0.0,
        hstem=statistics.median(h) if h else 0.0,
        bottom=_common_edge(bottoms),
        top=_common_edge(tops),
    )


def _common_edge(values: list[float]) -> float:
    """The median of the edges in the most common 10-unit band: the flat edge, not the curves."""
    if not values:
        return 0.0
    band = Counter(round(v, -1) for v in values).most_common(1)[0][0]
    return statistics.median(v for v in values if round(v, -1) == band)


def reference(paths: list[pathops.Path]) -> Strokes:
    """The font's typical strokes and edges, as medians over reference glyphs."""
    s = [strokes(p) for p in paths]
    return Strokes(
        vstem=statistics.median(x.vstem for x in s),
        hstem=statistics.median(x.hstem for x in s),
        bottom=statistics.median(x.bottom for x in s),
        top=statistics.median(x.top for x in s),
    )


def gate_strokes(draft: pathops.Path, ref: Strokes, tolerance: float = 0.15) -> GateResult:
    """Stem width, stroke width and contrast within a share of the reference."""
    d = strokes(draft)
    off = {
        "vstem": _rel(d.vstem, ref.vstem),
        "hstem": _rel(d.hstem, ref.hstem),
        "contrast": _rel(d.contrast, ref.contrast),
    }
    bad = {k: v for k, v in off.items() if v > tolerance}
    detail = ", ".join(
        f"{k} {getattr(d, k):.0f} vs {getattr(ref, k):.0f}" for k in ("vstem", "hstem")
    )
    detail += f", contrast {d.contrast:.2f} vs {ref.contrast:.2f}"
    return GateResult("strokes", not bad, detail)


def gate_vertical(draft: pathops.Path, ref: Strokes, tolerance: float = 20) -> GateResult:
    """The glyph body sits on the baseline and reaches the x-height, as the reference glyphs do."""
    d = strokes(draft)
    ok = abs(d.bottom - ref.bottom) <= tolerance and abs(d.top - ref.top) <= tolerance
    detail = f"bottom {d.bottom:.0f} vs {ref.bottom:.0f}, top {d.top:.0f} vs {ref.top:.0f}"
    return GateResult("vertical", ok, detail)


def _rel(a: float, b: float) -> float:
    return abs(a - b) / b if b else float("inf")


def _area(contour: list) -> float:
    pen = AreaPen()
    replayRecording(contour, pen)
    return pen.value


def outer_sign(font: TTFont) -> int:
    """The area sign of outer contours: TrueType draws them clockwise, CFF counter-clockwise."""
    return -1 if "glyf" in font else 1


def gate_contours(
    recording: list,
    *,
    outer_sign: int,
    expected_holes: int | None = None,
    min_area: float = 200.0,
    min_segment: float = 2.0,
) -> GateResult:
    """Contours are closed, not degenerate, drawn in the font's direction, and union cleanly.

    outer_sign is the sign of an outer contour's area in the font: see outer_sign().
    """
    contours = split_contours(recording)
    problems: list[str] = []
    if any(c[-1][0] != "closePath" for c in contours):
        return GateResult("contours", False, "open contour")
    for k, c in enumerate(contours):
        if abs(_area(c)) < min_area:
            problems.append(f"contour {k} has area {abs(_area(c)):.0f}")
        on = [args[-1] for op, args in c if args]
        for a, b in zip(on, on[1:], strict=False):
            if abs(a[0] - b[0]) + abs(a[1] - b[1]) < min_segment:
                problems.append(f"contour {k} has a segment shorter than {min_segment}")
                break
    if contours:
        largest = max(contours, key=lambda c: abs(_area(c)))
        if (_area(largest) > 0) != (outer_sign > 0):
            problems.append("outer contour runs the wrong way")
    union = to_path(recording)
    union.simplify(fix_winding=True)
    pieces = [c for c in split_contours(_record(union)) if c]
    slivers = [c for c in pieces if abs(_area(c)) < min_area]
    if slivers:
        problems.append(f"{len(slivers)} slivers after overlap removal")
    signs = [_area(c) for c in pieces]
    outer = max(signs, key=abs) if signs else 0
    holes = sum(1 for s in signs if (s > 0) != (outer > 0))
    if expected_holes is not None and holes != expected_holes:
        problems.append(f"{holes} counters after overlap removal, expected {expected_holes}")
    detail = "; ".join(problems) or f"{len(contours)} contours, {holes} counters"
    return GateResult("contours", not problems, detail)


def _record(path: pathops.Path) -> list:
    pen = RecordingPen()
    path.draw(pen)
    return pen.value


def gate_compatible(masters: dict[str, list]) -> GateResult:
    """Every master has the same contours, segment types, point counts and directions."""
    names = list(masters)
    if not names:
        return GateResult("compatible", False, "no masters")

    def shape(rec):
        ops = [(op, len(args)) for op, args in rec]
        return ops, [_area(c) > 0 for c in split_contours(rec)]

    first = shape(masters[names[0]])
    bad = [n for n in names[1:] if shape(masters[n]) != first]
    detail = f"{len(names)} masters" + (
        f"; differ from {names[0]}: {', '.join(bad)}" if bad else ""
    )
    return GateResult("compatible", not bad, detail)


def gate_shaping(face: Face, cases: list[Case], allowed: set[str] | None = None) -> GateResult:
    """No case fails, apart from case ids listed in allowed (failures the font had before)."""
    allowed = allowed or set()
    results = [run_case(face, c) for c in cases]
    mark_indistinct(face, results)
    failing = [r.case.id for r in results if r.failures and r.case.id not in allowed]
    detail = f"{len(cases)} cases" + (f"; failing: {', '.join(failing[:10])}" if failing else "")
    return GateResult("shaping", not failing, detail)
