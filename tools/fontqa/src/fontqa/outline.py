"""Outline geometry shared by the composer and the gates."""

from __future__ import annotations

import pathops
from fontTools.pens.basePen import decomposeQuadraticSegment, decomposeSuperBezierSegment
from fontTools.pens.recordingPen import replayRecording


def to_path(recording: list) -> pathops.Path:
    """A pathops Path from pen operations, as a RecordingPen holds them."""
    path = pathops.Path()
    replayRecording(recording, path.getPen())
    return path


def split_contours(recording: list) -> list[list]:
    """Pen operations grouped into 1 list per contour."""
    out, current = [], []
    for op, args in recording:
        current.append((op, args))
        if op in ("closePath", "endPath"):
            out.append(current)
            current = []
    return out


def polygons(path: pathops.Path, steps: int) -> list[list[tuple[float, float]]]:
    """Each contour as points, with every curve cut into steps straight pieces."""
    flat = pathops.Path(path)
    flat.convertConicsToQuads()
    polys, current, last = [], [], (0.0, 0.0)
    for op, pts in flat.segments:
        if op == "moveTo":
            current, last = [pts[0]], pts[0]
        elif op == "lineTo":
            current.append(pts[0])
            last = pts[0]
        elif op == "qCurveTo":
            if pts[-1] is None:
                # A TrueType contour with no on-curve point starts between its last and first.
                last = _mid(pts[-2], pts[0])
                current, pts = [last], (*pts[:-1], last)
            for ctrl, end in decomposeQuadraticSegment(pts):
                current += _flatten((last, ctrl, end), steps)
                last = end
        elif op == "curveTo":
            for c1, c2, end in decomposeSuperBezierSegment(pts):
                current += _flatten((last, c1, c2, end), steps)
                last = end
        elif op in ("closePath", "endPath"):
            if len(current) > 2:
                polys.append(current)
            current = []
    return polys


def mean_stroke_width(path: pathops.Path) -> float:
    """Mean stroke width of an outline: 2 x area / perimeter, close for long thin strokes."""
    flat = pathops.Path(path)
    flat.convertConicsToQuads()
    perimeter = 0.0
    for contour in polygons(flat, steps=8):
        perimeter += sum(
            ((x1 - x0) ** 2 + (y1 - y0) ** 2) ** 0.5
            for (x0, y0), (x1, y1) in zip(contour, contour[1:] + contour[:1], strict=True)
        )
    return 2 * abs(flat.area) / perimeter if perimeter else 0.0


def _mid(a, b):
    return ((a[0] + b[0]) / 2, (a[1] + b[1]) / 2)


def _flatten(ctrl, steps):
    return [_bezier(ctrl, i / steps) for i in range(1, steps + 1)]


def _bezier(ctrl, t):
    pts = list(ctrl)
    while len(pts) > 1:
        pairs = zip(pts, pts[1:], strict=False)
        pts = [(a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t) for a, b in pairs]
    return pts[0]
