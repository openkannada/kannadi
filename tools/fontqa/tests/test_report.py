import csv
import json

from fontqa.checks import mark_indistinct, run_case
from fontqa.fetch import Entry
from fontqa.kannada import Case
from fontqa.proof import sheet
from fontqa.report import (
    FAILURE_FIELDS,
    SUMMARY_FIELDS,
    face_id,
    failure_row,
    summarize,
    write_csv,
    write_face_json,
)
from fontqa.shaping import Face

from .synth import ANUSVARA, E_SIGN, KA, KHA, OLD_ORDER_FEA, VIRAMA, build
from .test_checks import case


def _results(tmp_path):
    face = Face(build(tmp_path / "Synth-Regular.ttf", fea=OLD_ORDER_FEA, marks_collide=True))
    cases = [
        case(KA, VIRAMA, KHA),
        case(KA, E_SIGN, ANUSVARA, joined=False),
        Case("ok", "syllable", chr(KA)),
    ]
    results = [run_case(face, c) for c in cases]
    mark_indistinct(face, results)
    return face, results


def test_summary_and_failure_rows_round_trip_through_csv(tmp_path):
    face, results = _results(tmp_path)
    entry = Entry("synth", "Synth", "tests", "OFL-1.1", [])
    fid = face_id(entry, face)
    assert fid == "synth--synth-regular"
    summary = summarize(fid, entry, face, results)
    assert (summary["cases"], summary["failing_cases"]) == (3, 2)
    assert (summary["no_conjunct"], summary["collision"]) == (1, 1)
    rows = [failure_row(fid, r) for r in results if r.failures]
    write_csv(tmp_path / "s.csv", SUMMARY_FIELDS, [summary])
    write_csv(tmp_path / "f.csv", FAILURE_FIELDS, rows)
    back = list(csv.DictReader((tmp_path / "f.csv").read_text(encoding="utf-8").splitlines()))
    assert [r["failures"] for r in back] == ["no_conjunct", "collision"]
    assert "esign x anusvara 100%" in back[1]["detail"]
    (row,) = csv.DictReader((tmp_path / "s.csv").read_text(encoding="utf-8").splitlines())
    assert row["license"] == "OFL-1.1"


def test_face_json_lists_only_failures(tmp_path):
    face, results = _results(tmp_path)
    entry = Entry("synth", "Synth", "tests", "OFL-1.1", [])
    summary = summarize("x", entry, face, results)
    write_face_json(tmp_path / "x.json", summary, face, results)
    data = json.loads((tmp_path / "x.json").read_text())
    assert len(data["failures"]) == 2
    assert data["font"]["family"] == "Synth"


def test_proof_sheet_is_written_only_when_something_fails(tmp_path):
    face, results = _results(tmp_path)
    assert sheet(face, results, tmp_path / "p.png", reference=face).stat().st_size > 0
    passing = [r for r in results if not r.failures]
    assert sheet(face, passing, tmp_path / "q.png", reference=None) is None
