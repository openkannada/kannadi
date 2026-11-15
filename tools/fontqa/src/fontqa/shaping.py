"""HarfBuzz shaping of 1 font face, with glyph outlines and metadata from fontTools."""

from __future__ import annotations

import hashlib
from dataclasses import dataclass
from functools import cached_property
from pathlib import Path

import pathops
import uharfbuzz as hb
from fontTools.ttLib import TTCollection, TTFont

DOTTED_CIRCLE = 0x25CC


@dataclass(frozen=True)
class Glyph:
    gid: int
    name: str
    cluster: int
    x: int
    y: int
    advance: int


class Face:
    """1 face of a font file, shaped at its default instance."""

    def __init__(self, path: Path, index: int = 0):
        self.path = Path(path)
        self.index = index
        self.blob = hb.Blob.from_file_path(str(self.path))
        self.hb_face = hb.Face(self.blob, index)
        self.font = hb.Font(self.hb_face)
        self.upem = self.hb_face.upem
        self.num_glyphs = self.hb_face.glyph_count
        # HarfBuzz inserts U+25CC only when the font maps it. A subfont that maps it to an
        # id past the last glyph makes every inserted dotted circle visible to the checks.
        self.sentinel = self.num_glyphs
        self._probe = hb.Font(self.font)
        funcs = hb.FontFuncs()
        funcs.set_nominal_glyph_func(self._nominal, None)
        self._probe.funcs = funcs
        self._paths: dict[int, pathops.Path] = {}

    def _nominal(self, _font, codepoint, _data):
        if codepoint == DOTTED_CIRCLE:
            return self.sentinel
        return self.font.get_nominal_glyph(codepoint) or 0

    @cached_property
    def tt(self) -> TTFont:
        if self.path.suffix.lower() in (".ttc", ".otc"):
            return TTCollection(str(self.path), lazy=True).fonts[self.index]
        return TTFont(str(self.path), lazy=True)

    @cached_property
    def cmap(self) -> dict[int, str]:
        return self.tt.getBestCmap() or {}

    @cached_property
    def mark_glyphs(self) -> set[int]:
        if "GDEF" not in self.tt or self.tt["GDEF"].table.GlyphClassDef is None:
            return set()
        order = self.tt.getGlyphOrder()
        ids = {n: i for i, n in enumerate(order)}
        classes = self.tt["GDEF"].table.GlyphClassDef.classDefs
        return {ids[n] for n, c in classes.items() if c == 3 and n in ids}

    @cached_property
    def aat(self) -> bool:
        """HarfBuzz shapes with the AAT morx table when a font has one, in place of GSUB."""
        return "morx" in self.tt or "mort" in self.tt

    @cached_property
    def sha256(self) -> str:
        return hashlib.sha256(self.path.read_bytes()).hexdigest()

    @cached_property
    def info(self) -> dict:
        tt = self.tt
        name = tt["name"]

        def n(i: int) -> str:
            rec = name.getDebugName(i)
            return rec.strip() if rec else ""

        scripts: dict[str, list[str]] = {}
        for tag in ("GSUB", "GPOS"):
            if tag in tt and tt[tag].table.ScriptList:
                scripts[tag] = sorted(r.ScriptTag for r in tt[tag].table.ScriptList.ScriptRecord)
        return {
            "family": n(16) or n(1),
            "style": n(17) or n(2),
            "full_name": n(4),
            "version": n(5),
            "license_name": n(13),
            "license_url": n(14),
            "copyright": n(0),
            "designer": n(9),
            "vendor": tt["OS/2"].achVendID.strip("\x00 ") if "OS/2" in tt else "",
            "glyph_count": self.num_glyphs,
            "units_per_em": self.upem,
            "variable": "fvar" in tt,
            "axes": [a.axisTag for a in tt["fvar"].axes] if "fvar" in tt else [],
            "outlines": "CFF" if ("CFF " in tt or "CFF2" in tt) else "TrueType",
            "script_tags": scripts,
            "aat_morx": self.aat,
            "kannada_codepoints": sum(1 for cp in self.cmap if 0x0C80 <= cp <= 0x0CFF),
        }

    def shape(self, text: str, probe: bool = False, features: dict | None = None) -> list[Glyph]:
        buf = hb.Buffer()
        buf.add_str(text)
        buf.guess_segment_properties()
        buf.flags = hb.BufferFlags.REMOVE_DEFAULT_IGNORABLES
        font = self._probe if probe else self.font
        hb.shape(font, buf, features)
        out, x = [], 0
        for info, pos in zip(buf.glyph_infos, buf.glyph_positions, strict=True):
            gid = info.codepoint
            name = "<dotted circle>" if gid == self.sentinel else self.glyph_name(gid)
            out.append(
                Glyph(gid, name, info.cluster, x + pos.x_offset, pos.y_offset, pos.x_advance)
            )
            x += pos.x_advance
        return out

    def glyph_name(self, gid: int) -> str:
        return self.font.get_glyph_name(gid) or f"gid{gid}"

    def is_mark(self, glyph: Glyph) -> bool:
        return glyph.gid in self.mark_glyphs or glyph.advance == 0

    def outline(self, gid: int) -> pathops.Path:
        """The glyph's ink at the default instance, with overlaps removed."""
        if gid not in self._paths:
            path = pathops.Path()
            if gid < self.num_glyphs:
                self.font.draw_glyph_with_pen(gid, path.getPen())
                path.simplify(fix_winding=True)
            self._paths[gid] = path
        return self._paths[gid]

    def outline_key(self, gid: int) -> tuple:
        """Equal for 2 glyphs exactly when their ink is the same."""
        return tuple(self.outline(gid).segments)


def area(path: pathops.Path) -> float:
    return abs(path.area)


def faces_in(path: Path) -> list[Face]:
    if path.suffix.lower() in (".ttc", ".otc"):
        count = len(TTCollection(str(path), lazy=True).fonts)
        return [Face(path, i) for i in range(count)]
    return [Face(path, 0)]
