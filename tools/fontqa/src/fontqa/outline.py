"""Outline geometry shared by the composer and the gates."""

from __future__ import annotations

import pathops
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
        elif op in ("curveTo", "qCurveTo"):
            ctrl = [last, *pts]
            current += [_bezier(ctrl, i / steps) for i in range(1, steps + 1)]
            last = pts[-1]
        elif op in ("closePath", "endPath"):
            if len(current) > 2:
                polys.append(current)
            current = []
    return polys


def _bezier(ctrl, t):
    pts = list(ctrl)
    while len(pts) > 1:
        pairs = zip(pts, pts[1:], strict=False)
        pts = [(a[0] + (b[0] - a[0]) * t, a[1] + (b[1] - a[1]) * t) for a, b in pairs]
    return pts[0]
