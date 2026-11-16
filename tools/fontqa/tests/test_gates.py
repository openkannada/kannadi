from fontTools.fontBuilder import FontBuilder
from fontTools.pens.recordingPen import replayRecording
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont

from fontqa.cli import main
from fontqa.gates import (
    gate_compatible,
    gate_contours,
    gate_glyph,
    gate_shaping,
    gate_strokes,
    gate_vertical,
    gate_weight,
    ink_runs,
    measure,
    outer_sign,
    reference,
    strokes,
    to_path,
)
from fontqa.kannada import ZWNJ, Case
from fontqa.outline import mean_stroke_width, polygons, split_contours
from fontqa.shaping import Face

from .synth import ANUSVARA, E_SIGN, KA, KHA, OLD_ORDER_FEA, VIRAMA, build


def box(x0, y0, x1, y1, ccw=True):
    pts = [(x0, y0), (x1, y0), (x1, y1), (x0, y1)]
    if not ccw:
        pts.reverse()
    return [("moveTo", (pts[0],)), *[("lineTo", (p,)) for p in pts[1:]], ("closePath", ())]


def ring(x=0, y=0, stem=60, stroke=50, w=400, h=500):
    """A rectangular O: vertical stems stem wide, horizontal strokes stroke thick."""
    return box(x, y, x + w, y + h) + box(
        x + stem, y + stroke, x + w - stem, y + h - stroke, ccw=False
    )


def test_strokes_measures_stems_and_edges():
    s = strokes(to_path(ring()))
    assert round(s.vstem) == 60
    assert round(s.hstem) == 50
    assert (s.bottom, s.top) == (0, 500)


def test_stroke_gate_accepts_a_match_and_rejects_a_heavier_draft():
    ref = reference([to_path(ring()), to_path(ring(w=500))])
    assert gate_strokes(to_path(ring(w=450)), ref).passed
    assert not gate_strokes(to_path(ring(stem=80)), ref).passed


def test_stroke_gate_rejects_wrong_contrast():
    # Stem and stroke are each within 15%, but their ratio is 27% off.
    ref = reference([to_path(ring(stem=60, stroke=50))])
    assert not gate_strokes(to_path(ring(stem=67, stroke=44)), ref).passed


def test_vertical_gate_wants_baseline_and_x_height():
    ref = reference([to_path(ring())])
    assert gate_vertical(to_path(ring(x=30)), ref).passed
    assert not gate_vertical(to_path(ring(y=60)), ref).passed
    assert not gate_vertical(to_path(ring(h=600)), ref).passed


def test_contour_gate_passes_a_clean_ring():
    result = gate_contours(ring(), outer_sign=1, expected_holes=1)
    assert result.passed, result.detail


def test_contour_gate_catches_a_reversed_outline():
    reversed_ring = box(0, 0, 400, 500, ccw=False) + box(60, 50, 340, 450)
    assert "wrong way" in gate_contours(reversed_ring, outer_sign=1).detail


def test_contour_gate_catches_tiny_and_open_contours():
    assert "area" in gate_contours(ring() + box(500, 0, 505, 5), outer_sign=1).detail
    open_contour = box(0, 0, 400, 500)[:-1] + [("endPath", ())]
    assert "open contour" in gate_contours(open_contour, outer_sign=1).detail


def test_contour_gate_catches_short_segments():
    rec = box(0, 0, 400, 500)
    rec.insert(2, ("lineTo", ((400.5, 0.5),)))
    assert "shorter" in gate_contours(rec, outer_sign=1).detail


def test_contour_gate_counts_counters_after_overlap_removal():
    # 4 overlapping bars make a frame with 1 counter; asking for 0 must fail.
    frame = box(0, 0, 400, 60) + box(0, 440, 400, 500) + box(0, 0, 60, 500) + box(340, 0, 400, 500)
    assert gate_contours(frame, outer_sign=1, expected_holes=1).passed
    wrong = gate_contours(frame, outer_sign=1, expected_holes=0)
    assert not wrong.passed
    assert "expected 0" in wrong.detail


def test_contour_gate_catches_slivers_after_overlap_removal():
    # 2 boxes that nearly abut leave a 1-unit gap: the union keeps a sliver of ink apart.
    rec = box(0, 0, 400, 500) + box(401, 0, 402, 500) + box(403, 0, 800, 500)
    assert "slivers" in gate_contours(rec, outer_sign=1, min_area=600).detail


def test_compatible_gate():
    assert gate_compatible({"Light": ring(stem=20), "Bold": ring(stem=90)}).passed
    extra = ring()
    extra.insert(1, ("lineTo", ((200, 0),)))
    result = gate_compatible({"Light": ring(), "Bold": extra})
    assert not result.passed
    assert "Bold" in result.detail


def test_shaping_gate(tmp_path):
    case = Case(
        "c",
        "cluster",
        chr(KA) + chr(VIRAMA) + chr(KHA),
        unjoined=chr(KA) + chr(VIRAMA) + ZWNJ + chr(KHA),
    )
    good = Face(build(tmp_path / "good.ttf"))
    old = Face(build(tmp_path / "old.ttf", fea=OLD_ORDER_FEA))
    assert gate_shaping(good, [case]).passed
    assert not gate_shaping(old, [case]).passed
    assert gate_shaping(old, [case], allowed={"c"}).passed


def test_outer_sign_follows_the_outline_format(tmp_path):
    font = TTFont(str(build(tmp_path / "f.ttf")))
    assert outer_sign(font) == -1
    clockwise = box(0, 0, 400, 500, ccw=False) + box(60, 50, 340, 450)
    assert gate_contours(clockwise, outer_sign=-1, expected_holes=1).passed


def test_a_scanline_through_a_vertex_keeps_the_run_whole():
    diamond = [
        ("moveTo", ((200, 60),)),
        ("lineTo", ((100, 220),)),
        ("lineTo", ((200, 380),)),
        ("lineTo", ((300, 220),)),
        ("closePath", ()),
    ]
    assert ink_runs(to_path(box(0, 0, 400, 440) + diamond), "y", 60) == [(0, 400)]


def test_compatible_gate_catches_a_reversed_contour():
    flipped = box(0, 0, 400, 500, ccw=False) + box(60, 50, 340, 450, ccw=False)
    assert not gate_compatible({"Light": ring(), "Bold": flipped}).passed


def test_shaping_gate_catches_a_sign_drawn_like_another(tmp_path):
    twin = Face(build(tmp_path / "twin.ttf", anusvara_like_esign=True))
    cases = [
        Case("e", "syllable", chr(KA) + chr(E_SIGN)),
        Case("m", "syllable", chr(KA) + chr(ANUSVARA)),
    ]
    assert not gate_shaping(twin, cases).passed


def test_vertical_gate_measures_edges_before_rounding():
    ref = reference([to_path(ring())])
    assert not gate_vertical(to_path(ring(y=25)), ref).passed


def test_off_curve_only_contour_is_flattened_as_a_closed_curve():
    # 4 off-curve points, no on-curve point: TrueType's implied-point circle of radius 100.
    circle = [("qCurveTo", ((100, 0), (0, 100), (-100, 0), (0, -100), None)), ("closePath", ())]
    (poly,) = polygons(to_path(circle), steps=16)
    radii = [(x * x + y * y) ** 0.5 for x, y in poly]
    assert min(radii) > 49 and max(radii) < 101


def test_quadratic_with_implied_points_is_split_into_quadratics():
    # 2 off-curve points imply an on-curve point at their midpoint (100, 100).
    rec = [
        ("moveTo", ((0, 0),)),
        ("qCurveTo", ((0, 100), (200, 100), (200, 0))),
        ("closePath", ()),
    ]
    (poly,) = polygons(to_path(rec), steps=4)
    assert (100, 100) in [(round(x), round(y)) for x, y in poly]


def test_weight_gate_compares_mean_stroke_width():
    ref = reference([to_path(ring())])
    assert gate_weight(to_path(ring(w=300)), ref).passed
    assert not gate_weight(to_path(ring(stem=90, stroke=80)), ref).passed
    assert mean_stroke_width(to_path(ring())) > 0


def _rec_glyph(rec):
    pen = TTGlyphPen(None)
    replayRecording(rec, pen)
    return pen.glyph()


def drafted_font(path, cf3=None, cdd=None):
    """Ring consonants, ಅ and ಂ, a raised ring U+0CF3 and a C-shaped U+0CDD, unless replaced."""
    c_shape = box(0, 0, 400, 50) + box(0, 450, 400, 500) + box(0, 0, 60, 500)
    glyphs = {
        ".notdef": box(0, 0, 400, 500),
        **{name: ring() for name in ("ka", "kha", "ga")},
        "a": ring(w=600),
        "anusvara": ring(),
        "uni0CF3": cf3 if cf3 is not None else ring(y=415, w=200, h=245),
        "uni0CDD": cdd if cdd is not None else c_shape,
    }
    cmap = {0x0C95: "ka", 0x0C96: "kha", 0x0C97: "ga", 0x0C85: "a", 0x0C82: "anusvara"}
    cmap |= {0x0CF3: "uni0CF3", 0x0CDD: "uni0CDD"}
    fb = FontBuilder(1000, isTTF=True)
    fb.setupGlyphOrder(list(glyphs))
    fb.setupCharacterMap(cmap)
    fb.setupGlyf({n: _rec_glyph(_cw(r)) for n, r in glyphs.items()})
    fb.setupHorizontalMetrics({n: (700, 0) for n in glyphs})
    fb.setupHorizontalHeader(ascent=800, descent=-200)
    fb.setupNameTable({"familyName": "Drafts", "styleName": "Regular"})
    fb.setupOS2()
    fb.setupPost()
    fb.save(str(path))
    return path


def _cw(rec):
    """TrueType outer contours run clockwise: reverse every contour of a counter-clockwise box."""
    out = []
    for contour in split_contours(rec):
        pts = [args[0] for op, args in contour if args]
        out += [("moveTo", (pts[0],))] + [("lineTo", (p,)) for p in reversed(pts[1:])]
        out.append(("closePath", ()))
    return out


def test_measure_reads_the_consonants(tmp_path):
    ref = measure(Face(drafted_font(tmp_path / "d.ttf")))
    assert (round(ref.vstem), round(ref.hstem), ref.bottom, ref.top) == (60, 50, 0, 500)


def test_gate_glyph_passes_matching_drafts(tmp_path):
    face = Face(drafted_font(tmp_path / "d.ttf"))
    ref = measure(face)
    for cp in (0x0CF3, 0x0CDD):
        results = gate_glyph(face, cp, ref)
        assert all(r.passed for r in results), [r.detail for r in results]


def test_gate_glyph_catches_a_full_size_ring_on_the_baseline(tmp_path):
    face = Face(drafted_font(tmp_path / "d.ttf", cf3=ring()))
    failed = {r.name for r in gate_glyph(face, 0x0CF3, measure(face)) if not r.passed}
    assert "ring" in failed


def test_gate_glyph_catches_a_draft_3_times_too_wide(tmp_path):
    wide = box(0, 0, 1200, 50) + box(0, 450, 1200, 500) + box(0, 0, 60, 500)
    face = Face(drafted_font(tmp_path / "d.ttf", cdd=wide))
    failed = {r.name for r in gate_glyph(face, 0x0CDD, measure(face)) if not r.passed}
    assert "aspect" in failed


def test_gate_glyph_catches_a_filled_ring_and_a_missing_glyph(tmp_path):
    face = Face(drafted_font(tmp_path / "d.ttf", cf3=box(0, 415, 200, 660)))
    ref = measure(face)
    failed = {r.name for r in gate_glyph(face, 0x0CF3, ref) if not r.passed}
    assert {"weight", "contours"} <= failed
    assert [r.name for r in gate_glyph(face, 0x0CDC, ref)] == ["present"]


def test_gate_command_exit_codes(tmp_path, capsys):
    good = drafted_font(tmp_path / "good.ttf")
    assert main(["gate", str(good), "--chars", "0CF3", "0CDD"]) == 0
    bad = drafted_font(tmp_path / "bad.ttf", cdd=box(0, 0, 400, 500))
    assert main(["gate", str(bad), "--chars", "0CDD"]) == 1
    assert main(["gate", str(good), "--chars", "0C95"]) == 2
    assert main(["measure", str(good)]) == 0
    assert '"vstem": 60.0' in capsys.readouterr().out
