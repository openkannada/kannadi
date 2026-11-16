import json
import tomllib

from fontqa.cli import main
from fontqa.export import fontbakery_collisions, fontbakery_forbidden, shaperglot_checks
from fontqa.kannada import RA, VIRAMA, all_cases


def test_fontbakery_forbidden_covers_every_case():
    data = fontbakery_forbidden(all_cases())
    assert ".notdef" in data["configuration"]["forbidden_glyphs"]
    assert len(data["tests"]) == len(all_cases())
    assert all(t["input"] for t in data["tests"])


def test_fontbakery_collisions_skip_strings_without_marks():
    groups = {c.id: c.group for c in all_cases()}
    tests = fontbakery_collisions(all_cases())["tests"]
    assert tests and all(groups[t["comment"]] not in ("vowel", "rare") for t in tests)


def test_shaperglot_pairs_compare_1_text_with_its_conjunct_features_off():
    checks = tomllib.loads(shaperglot_checks(all_cases()))["kn_Knda"]
    conjuncts, arkavattu = (c["implementations"][0] for c in checks)
    for impl in (conjuncts, arkavattu):
        assert impl["type"] == "ShapingDiffers" and impl["ignore_notdefs"] is True
        for on, off in impl["pairs"]:
            assert set(on) == set(off) == {"text", "features"}
            assert on["text"] == off["text"] and on["features"] == []
    assert "-blwf" in conjuncts["pairs"][0][1]["features"]
    assert arkavattu["pairs"][0][1]["features"] == ["-rphf"]
    assert all(on["text"].startswith(RA + VIRAMA) for on, _ in arkavattu["pairs"])
    assert RA + VIRAMA + RA not in [on["text"] for on, _ in arkavattu["pairs"]]


def test_export_command_writes_the_files(tmp_path):
    assert main(["export", "--out", str(tmp_path)]) == 0
    common = json.loads((tmp_path / "fontbakery" / "kannada-fontqa-forbidden.json").read_text())
    rare = json.loads((tmp_path / "fontbakery" / "kannada-fontqa-forbidden-rare.json").read_text())
    assert len(common["tests"]) + len(rare["tests"]) == len(all_cases())
    assert "test_directory" in (tmp_path / "fontbakery" / "fontbakery.yml").read_text()
    tomllib.loads((tmp_path / "shaperglot" / "kn_Knda.toml").read_text())
