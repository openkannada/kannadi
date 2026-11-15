import pytest
from fontTools.feaLib.builder import addOpenTypeFeaturesFromString
from fontTools.fontBuilder import FontBuilder
from fontTools.pens.ttGlyphPen import TTGlyphPen
from fontTools.ttLib import TTFont
from fontTools.ttLib.tables.DefaultTable import DefaultTable

from fontqa import compose, fixes
from fontqa.checks import run_case
from fontqa.shaping import Face, area

from .synth import KA, KHA, OLD_ORDER_FEA, VIRAMA, build
from .test_checks import case

# Gubbi's layout: blwf only under knda, reached through a contextual lookup.
KNDA_ONLY_FEA = """
languagesystem knd2 dflt;
languagesystem knda dflt;
lookup LIG { sub ka virama by ka.blwf; sub kha virama by kha.blwf; } LIG;
feature rphf { script knda; sub ra virama by reph; script knd2; sub ra virama by reph; } rphf;
feature blwf { script knda; sub [ka kha]' lookup LIG virama'; } blwf;
"""


def _fixed(path, fea):
    font = TTFont(str(build(path, fea=fea)))
    added = fixes.add_new_order_blwf(font)
    font.save(str(path))
    return added, Face(path)


@pytest.mark.parametrize("fea", [OLD_ORDER_FEA, KNDA_ONLY_FEA], ids=["old-order", "knda-only"])
def test_new_order_blwf_forms_the_conjunct(tmp_path, fea):
    before = Face(build(tmp_path / "before.ttf", fea=fea))
    assert run_case(before, case(KA, VIRAMA, KHA)).failures == ["no_conjunct"]
    added, after = _fixed(tmp_path / "after.ttf", fea)
    assert added == 2
    assert run_case(after, case(KA, VIRAMA, KHA)).failures == []


def test_feature_records_stay_sorted(tmp_path):
    _, face = _fixed(tmp_path / "after.ttf", KNDA_ONLY_FEA)
    tags = [r.FeatureTag for r in face.tt["GSUB"].table.FeatureList.FeatureRecord]
    assert tags == sorted(tags)


def test_drop_aat_removes_only_aat_tables(tmp_path):
    font = TTFont(str(build(tmp_path / "f.ttf")))
    font["morx"] = DefaultTable("morx")
    font["morx"].data = b"\0" * 8
    assert fixes.drop_aat(font) == ["morx"]
    assert "morx" not in font and "GSUB" in font


U, UU, R, LETTER_L = 0x0CC1, 0x0CC2, 0x0CC3, 0x0C8C
LA, KA_CP = 0x0CB2, 0x0C95


def _pen(*rects):
    pen = TTGlyphPen(None)
    for x0, y0, x1, y1 in rects:
        pen.moveTo((x0, y0))
        pen.lineTo((x0, y1))
        pen.lineTo((x1, y1))
        pen.lineTo((x1, y0))
        pen.closePath()
    return pen.glyph()


def _sign_font(path, merged_tail: bool):
    """ು is 1 bar, ೂ adds a tail bar to its right, ೃ hangs lower, ಲ has a subscript form."""
    tail = (260, 0, 500, 80)
    uu = [(0, 0, 330, 300)] if merged_tail else [(0, 0, 250, 300), tail]
    glyphs = {
        ".notdef": _pen((0, 0, 400, 700)),
        "ka": _pen((50, 0, 450, 500)),
        "virama": _pen((-300, 550, -100, 650)),
        "u": _pen((0, 0, 250, 300)),
        "uu": _pen(*uu),
        "r": _pen((0, -300, 230, 200)),
        "la": _pen((50, 0, 450, 500)),
        "la.blwf": _pen((-400, -300, -100, -100), (-400, -300, -340, -100)),
        "ll": _pen((40, 0, 760, 120), (40, 0, 160, 600), (640, 0, 760, 600)),
    }
    cmap = {KA_CP: "ka", VIRAMA: "virama", U: "u", UU: "uu", R: "r", LA: "la", LETTER_L: "ll"}
    advance = {"u": 250, "uu": 500, "r": 230, "la.blwf": 0, "virama": 0, "ll": 800}
    fb = FontBuilder(1000, isTTF=True)
    fb.setupGlyphOrder(list(glyphs))
    fb.setupCharacterMap(cmap)
    fb.setupGlyf(glyphs)
    glyf = fb.font["glyf"]
    fb.setupHorizontalMetrics({n: (advance.get(n, 500), glyf[n].xMin) for n in glyphs})
    fb.setupHorizontalHeader(ascent=800, descent=-300)
    fb.setupNameTable({"familyName": "Signs", "styleName": "Regular"})
    fb.setupOS2()
    fb.setupPost()
    addOpenTypeFeaturesFromString(
        fb.font,
        "languagesystem knd2 dflt;\n"
        "@BASE = [ka la];\n@MARK = [la.blwf];\n"
        "table GDEF { GlyphClassDef @BASE, , @MARK, ; } GDEF;\n"
        "feature blwf { script knd2; sub virama la by la.blwf; } blwf;\n"
        "markClass la.blwf <anchor -250 -100> @BOTTOM;\n"
        "feature blwm { script knd2; pos base ka <anchor 250 0> mark @BOTTOM; } blwm;",
    )
    fb.save(str(path))
    return TTFont(str(path))


@pytest.mark.parametrize("merged", [False, True], ids=["apart", "merged"])
def test_rr_sign_is_r_sign_plus_the_long_tail(tmp_path, merged):
    font = _sign_font(tmp_path / "f.ttf", merged_tail=merged)
    name = compose.compose_rr_sign(font)
    font.save(str(tmp_path / "out.ttf"))
    face = Face(tmp_path / "out.ttf")
    assert font.getBestCmap()[0x0CC4] == name
    assert font["hmtx"][name][0] == 230 + 500 - 250
    gid = face.font.get_nominal_glyph(0x0CC4)
    r_ink = area(face.outline(face.font.get_nominal_glyph(R)))
    assert area(face.outline(gid)) > r_ink * 1.05
    uu_right = 330 if merged else 500
    assert face.outline(gid).bounds[2] == pytest.approx(uu_right + 230 - 250)


def test_l_sign_takes_the_subscript_box_and_mark_class(tmp_path):
    font = _sign_font(tmp_path / "f.ttf", merged_tail=False)
    (name,) = compose.compose_l_signs(font)
    font.save(str(tmp_path / "out.ttf"))
    out = TTFont(str(tmp_path / "out.ttf"))
    glyph = out["glyf"][name]
    assert glyph.yMin == pytest.approx(-300, abs=6)
    assert glyph.yMax == pytest.approx(-100, abs=6)
    assert out["GDEF"].table.GlyphClassDef.classDefs[name] == 3
    assert out["hmtx"][name][0] == 0
    marks = out["GPOS"].table.LookupList.Lookup[0].SubTable[0].MarkCoverage.glyphs
    assert name in marks
    face = Face(tmp_path / "out.ttf")
    new = face.outline(face.font.get_nominal_glyph(0x0CE2))
    model = face.outline(face.font.get_glyph_from_name("la.blwf"))
    assert compose.stem(new) == pytest.approx(compose.stem(model), rel=0.2)


def test_compose_refuses_a_font_without_the_parts(tmp_path):
    font = _sign_font(tmp_path / "f.ttf", merged_tail=False)
    for table in font["cmap"].tables:
        table.cmap.pop(LETTER_L, None)
    with pytest.raises(compose.ComposeError, match="U\\+0C8C"):
        compose.compose_l_signs(font)


def test_stem_of_a_bar_is_its_width():
    bar = compose.to_path([(("moveTo", ((0, 0),)), ("lineTo", ((0, 1000),)),
                            ("lineTo", ((80, 1000),)), ("lineTo", ((80, 0),)),
                            ("closePath", ()))])  # fmt: skip
    assert compose.stem(bar) == pytest.approx(80, rel=0.1)


MIXED_FEA = """
languagesystem knd2 dflt;
feature blwf { script knd2; sub virama ka by ka.blwf; sub kha virama by kha.blwf; } blwf;
"""


def test_existing_new_order_rules_are_kept(tmp_path):
    before = Face(build(tmp_path / "before.ttf", fea=MIXED_FEA))
    assert run_case(before, case(KHA, VIRAMA, KA)).failures == []
    assert run_case(before, case(KA, VIRAMA, KHA)).failures == ["no_conjunct"]
    _, after = _fixed(tmp_path / "after.ttf", MIXED_FEA)
    assert run_case(after, case(KHA, VIRAMA, KA)).failures == []
    assert run_case(after, case(KA, VIRAMA, KHA)).failures == []


def test_a_font_without_knd2_is_refused(tmp_path):
    fea = "languagesystem knda dflt;\nfeature blwf { script knda; sub ka virama by ka.blwf; } blwf;"
    font = TTFont(str(build(tmp_path / "f.ttf", fea=fea)))
    with pytest.raises(ValueError, match="no knd2"):
        fixes.add_new_order_blwf(font)


def test_composing_drops_per_glyph_device_tables(tmp_path):
    font = _sign_font(tmp_path / "f.ttf", merged_tail=False)
    font["hdmx"] = DefaultTable("hdmx")
    font["hdmx"].data = b"\0" * 8
    compose.compose_rr_sign(font)
    assert "hdmx" not in font


def test_a_sign_that_already_has_ink_is_left_alone(tmp_path):
    font = _sign_font(tmp_path / "f.ttf", merged_tail=False)
    compose.compose_rr_sign(font)
    with pytest.raises(compose.ComposeError, match="already has a glyph"):
        compose.compose_rr_sign(font)


SHARED_FEA = """
languagesystem knda dflt;
languagesystem knd2 dflt;
feature blwf { sub ka virama by ka.blwf; sub kha virama by kha.blwf; } blwf;
"""


def test_a_feature_record_shared_with_knda_is_left_alone(tmp_path):
    font = TTFont(str(build(tmp_path / "f.ttf", fea=SHARED_FEA)))
    gsub = font["GSUB"].table

    def blwf_lookups(script):
        (sr,) = [r for r in gsub.ScriptList.ScriptRecord if r.ScriptTag == script]
        records = gsub.FeatureList.FeatureRecord
        ids = sr.Script.DefaultLangSys.FeatureIndex
        return [records[i].Feature.LookupListIndex for i in ids if records[i].FeatureTag == "blwf"]

    before = blwf_lookups("knda")
    fixes.add_new_order_blwf(font)
    assert blwf_lookups("knda") == before
    assert len(blwf_lookups("knd2")[0]) == len(before[0]) + 1
