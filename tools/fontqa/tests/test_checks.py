from fontqa.checks import mark_indistinct, run_case
from fontqa.kannada import Case
from fontqa.shaping import Face

from .synth import ANUSVARA, E_SIGN, KA, KHA, NO_REPH_FEA, OLD_ORDER_FEA, RA, VIRAMA, build

ZWNJ = 0x200C


def case(*cps: int, group: str = "cluster", joined: bool = True) -> Case:
    text = "".join(map(chr, cps))
    unjoined = None
    if joined:
        i = cps.index(VIRAMA) + 1
        unjoined = "".join(map(chr, cps[:i] + (ZWNJ,) + cps[i:]))
    return Case("t", group, text, unjoined=unjoined)


def test_a_correct_font_passes(tmp_path):
    face = Face(build(tmp_path / "good.ttf"))
    assert run_case(face, case(KA, VIRAMA, KHA)).failures == []
    assert run_case(face, case(RA, VIRAMA, KA, group="arkavattu")).failures == []
    assert run_case(face, case(KA, E_SIGN, ANUSVARA, joined=False)).failures == []


def test_old_order_below_base_rule_leaves_the_cluster_unjoined(tmp_path):
    face = Face(build(tmp_path / "old.ttf", fea=OLD_ORDER_FEA))
    assert run_case(face, case(KA, VIRAMA, KHA)).failures == ["no_conjunct"]


def test_a_font_without_reph_misses_the_arkavattu(tmp_path):
    face = Face(build(tmp_path / "noreph.ttf", fea=NO_REPH_FEA))
    result = run_case(face, case(RA, VIRAMA, KA, group="arkavattu"))
    assert result.failures == ["no_arkavattu"]


def test_unmapped_and_empty_characters_are_missing(tmp_path):
    unmapped = Face(build(tmp_path / "unmapped.ttf", drop=(KHA,)))
    assert run_case(unmapped, case(KA, VIRAMA, KHA)).missing == [KHA]
    empty = Face(build(tmp_path / "empty.ttf", empty=(KHA,)))
    assert run_case(empty, case(KA, VIRAMA, KHA)).missing == [KHA]


def test_missing_characters_skip_the_conjunct_check(tmp_path):
    face = Face(build(tmp_path / "unmapped.ttf", drop=(KHA,)))
    assert "no_conjunct" not in run_case(face, case(KA, VIRAMA, KHA)).failures


def test_overlapping_marks_collide(tmp_path):
    face = Face(build(tmp_path / "collide.ttf", marks_collide=True))
    result = run_case(face, case(KA, E_SIGN, ANUSVARA, joined=False))
    assert result.failures == ["collision"]
    (hit,) = result.collisions
    assert {hit.a, hit.b} == {"esign", "anusvara"}
    assert hit.share == 1.0


def test_a_mark_touching_its_base_is_not_a_collision(tmp_path):
    face = Face(build(tmp_path / "onbase.ttf", mark_on_base=True))
    assert run_case(face, case(KA, E_SIGN, joined=False)).collisions == []


def test_dotted_circle_is_seen_when_the_font_lacks_one(tmp_path):
    face = Face(build(tmp_path / "good.ttf"))
    assert face.font.get_nominal_glyph(0x25CC) is None
    result = run_case(face, Case("t", "rare", chr(E_SIGN)))
    assert result.dotted_circle
    assert "dotted_circle" in result.failures


def test_2_strings_with_the_same_ink_are_indistinct(tmp_path):
    face = Face(build(tmp_path / "good.ttf"))
    ka_e = Case("a", "syllable", chr(KA) + chr(E_SIGN))
    ka_ee = Case("b", "syllable", chr(KA) + chr(E_SIGN))
    ka_m = Case("c", "syllable", chr(KA) + chr(ANUSVARA))
    results = [run_case(face, c) for c in (ka_e, ka_m)]
    mark_indistinct(face, results)
    assert all(r.same_as == [] for r in results)
    twin = Face(build(tmp_path / "twin.ttf", anusvara_like_esign=True))
    results = [run_case(twin, c) for c in (ka_e, ka_ee, ka_m)]
    mark_indistinct(twin, results)
    assert results[0].same_as == [ka_m.text]
    assert results[2].failures == ["indistinct"]
