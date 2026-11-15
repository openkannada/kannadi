"""The checks run on each shaped test string."""

from __future__ import annotations

from dataclasses import dataclass, field
from itertools import combinations

import pathops

from .kannada import Case
from .shaping import Face, Glyph, area

IGNORED = {0x200C, 0x200D}

# Share of the smaller glyph's ink that 2 glyphs must overlap by to count as a collision.
# Below this, overlaps are joins a designer drew on purpose.
COLLISION_SHARE = 0.03

FAILURES = (
    "missing_glyph", "notdef", "dotted_circle", "no_conjunct", "no_arkavattu", "collision",
    "indistinct",
)  # fmt: skip


@dataclass
class Collision:
    a: str
    b: str
    share: float
    box: tuple[float, float, float, float]


@dataclass
class Result:
    case: Case
    glyphs: list[Glyph]
    missing: list[int] = field(default_factory=list)
    notdef: bool = False
    dotted_circle: bool = False
    unjoined: bool = False
    collisions: list[Collision] = field(default_factory=list)
    same_as: list[str] = field(default_factory=list)

    @property
    def failures(self) -> list[str]:
        out = []
        if self.missing:
            out.append("missing_glyph")
        if self.notdef:
            out.append("notdef")
        if self.dotted_circle:
            out.append("dotted_circle")
        if self.unjoined:
            out.append("no_arkavattu" if self.case.group == "arkavattu" else "no_conjunct")
        if self.collisions:
            out.append("collision")
        if self.same_as:
            out.append("indistinct")
        return out


def run_case(face: Face, case: Case) -> Result:
    glyphs = face.shape(case.text, probe=True)
    result = Result(case, glyphs)
    result.missing = [ord(c) for c in case.text if ord(c) not in IGNORED and not has_ink(face, c)]
    result.notdef = not result.missing and any(g.gid == 0 for g in glyphs)
    result.dotted_circle = any(g.gid == face.sentinel for g in glyphs)
    if case.unjoined and not result.missing:
        ids = [g.gid for g in glyphs]
        result.unjoined = ids == [g.gid for g in face.shape(case.unjoined, probe=True)]
        # OpenType feature switches do not reach AAT tables, so rphf cannot be turned off there.
        if case.group == "arkavattu" and not result.unjoined and not face.aat:
            no_reph = face.shape(case.text, probe=True, features={"rphf": False})
            result.unjoined = ids == [g.gid for g in no_reph]
    if not result.missing and not result.dotted_circle:
        result.collisions = collisions(face, glyphs)
    return result


def has_ink(face: Face, char: str) -> bool:
    """True when the font maps the character to a glyph that draws something."""
    gid = face.font.get_nominal_glyph(ord(char))
    return bool(gid) and area(face.outline(gid)) > 0


def collisions(face: Face, glyphs: list[Glyph]) -> list[Collision]:
    """Ink overlaps between the glyphs that sit on the base, where at least 1 is a mark.

    The base is the first spacing glyph. Marks overlap their base by design, so the base
    is left out.
    """
    base = next((i for i, g in enumerate(glyphs) if not face.is_mark(g)), None)
    attached = [g for i, g in enumerate(glyphs) if i != base and g.gid != face.sentinel]
    found = []
    for a, b in combinations(attached, 2):
        if not (face.is_mark(a) or face.is_mark(b)):
            continue
        pa = face.outline(a.gid).transform(1, 0, 0, 1, a.x, a.y)
        pb = face.outline(b.gid).transform(1, 0, 0, 1, b.x, b.y)
        if not _boxes_meet(pa.bounds, pb.bounds):
            continue
        smaller = min(area(pa), area(pb))
        if smaller == 0:
            continue
        overlap = pathops.op(pa, pb, pathops.PathOp.INTERSECTION)
        share = area(overlap) / smaller
        if share > COLLISION_SHARE:
            found.append(Collision(a.name, b.name, round(share, 3), overlap.bounds))
    return found


def _boxes_meet(a, b) -> bool:
    return a[0] < b[2] and b[0] < a[2] and a[1] < b[3] and b[1] < a[3]


def mark_indistinct(face: Face, results: list[Result]) -> None:
    """Flag strings that render with exactly the same ink as a different string, like ೄ drawn as ೃ.

    Strings with a missing glyph are left out: they already fail, and all draw .notdef.
    """
    groups: dict[tuple, list[Result]] = {}
    for r in results:
        if r.missing or r.notdef:
            continue
        key = tuple((face.outline_key(g.gid), g.x, g.y) for g in r.glyphs)
        groups.setdefault(key, []).append(r)
    for same in groups.values():
        for r in same:
            r.same_as = sorted({o.case.text for o in same} - {r.case.text})
