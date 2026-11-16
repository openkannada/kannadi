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


def test_shaperglot_checks_parse_as_toml_pairs():
    checks = tomllib.loads(shaperglot_checks(all_cases()))["kn_Knda"]
    conjuncts, arkavattu = (c["implementations"][0] for c in checks)
    assert conjuncts["type"] == "ShapingDiffers"
    joined, unjoined = conjuncts["pairs"][0]
    assert unjoined["text"] == joined["text"].replace(VIRAMA, VIRAMA + "\u200c", 1)
    assert all(a["text"].startswith(RA + VIRAMA) for a, _ in arkavattu["pairs"])
    assert all(b["text"].startswith(RA + "\u200d" + VIRAMA) for _, b in arkavattu["pairs"])


def test_export_command_writes_the_files(tmp_path):
    assert main(["export", "--out", str(tmp_path)]) == 0
    json.loads((tmp_path / "fontbakery" / "kannada-fontqa-forbidden.json").read_text())
    assert "test_directory" in (tmp_path / "fontbakery" / "fontbakery.yml").read_text()
    tomllib.loads((tmp_path / "shaperglot" / "kn_Knda.toml").read_text())
