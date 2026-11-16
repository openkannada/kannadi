import csv
import json

from fontqa.dashboard import build
from fontqa.report import SUMMARY_FIELDS


def write_results(root, rows):
    (root / "fonts").mkdir(parents=True)
    with (root / "summary.csv").open("w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=SUMMARY_FIELDS)
        w.writeheader()
        for row in rows:
            w.writerow({k: row.get(k, "0") for k in SUMMARY_FIELDS})
            failure = {"text": "<x>", "failures": "missing_glyph"}
            failures = [failure] * int(row["failing_cases"])
            (root / "fonts" / f"{row['face_id']}.json").write_text(
                json.dumps({"summary": {}, "font": {}, "failures": failures})
            )


def test_dashboard_lists_every_face_worst_last_and_escapes_text(tmp_path):
    base = {"cases": "10", "gsub_scripts": "knd2 knda", "missing_codepoints": "", "style": "R"}
    rows = [
        {**base, "face_id": "b", "font_id": "b", "family": "Bad <i>", "failing_cases": "3",
         "missing_glyph": "3", "missing_codepoints": "U+0CDC", "version": "1", "license": "OFL"},
        {**base, "face_id": "g", "font_id": "g", "family": "Good", "failing_cases": "0",
         "version": "2", "license": "OFL"},
    ]  # fmt: skip
    write_results(tmp_path / "results", rows)
    page = build(tmp_path / "results", tmp_path / "out").read_text(encoding="utf-8")
    assert page.index("Good") < page.index("Bad &lt;i&gt;")
    assert "<i>" not in page.split("<tbody>")[1]
    assert '<span class="no" title="U+0CDC">' in page
    assert "&lt;x&gt;" in page
