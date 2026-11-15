"""Per-face JSON results, and the CSV dataset across all faces."""

from __future__ import annotations

import csv
import json
import re
from collections import Counter
from pathlib import Path

from .checks import FAILURES, Result
from .fetch import Entry
from .kannada import Case
from .shaping import Face

SUMMARY_FIELDS = [
    "face_id", "font_id", "family", "style", "version", "license", "license_in_font", "source",
    "file", "face_index", "sha256", "variable", "outlines", "gsub_scripts", "kannada_codepoints",
    "glyph_count", "cases", "failing_cases", *FAILURES, "missing_codepoints",
]  # fmt: skip

FAILURE_FIELDS = [
    "face_id", "case_id", "group", "text", "codepoints", "failures", "glyphs", "detail",
]  # fmt: skip


def face_id(entry: Entry, face: Face) -> str:
    stem = re.sub(r"[^A-Za-z0-9]+", "-", face.path.stem).strip("-").lower()
    suffix = f"-{face.index}" if face.path.suffix.lower() in (".ttc", ".otc") else ""
    return f"{entry.id}--{stem}{suffix}"


def summarize(fid: str, entry: Entry, face: Face, results: list[Result]) -> dict:
    info = face.info
    counts = Counter(f for r in results for f in r.failures)
    missing = sorted({cp for r in results for cp in r.missing})
    return {
        "face_id": fid,
        "font_id": entry.id,
        "family": info["family"],
        "style": info["style"],
        "version": info["version"],
        "license": entry.license,
        "license_in_font": " ".join(info["license_name"].split())[:200],
        "source": entry.source,
        "file": face.path.name,
        "face_index": face.index,
        "sha256": face.sha256,
        "variable": info["variable"],
        "outlines": info["outlines"],
        "gsub_scripts": " ".join(info["script_tags"].get("GSUB", [])),
        "kannada_codepoints": info["kannada_codepoints"],
        "glyph_count": info["glyph_count"],
        "cases": len(results),
        "failing_cases": sum(1 for r in results if r.failures),
        **{k: counts.get(k, 0) for k in FAILURES},
        "missing_codepoints": " ".join(f"U+{cp:04X}" for cp in missing),
    }


def failure_row(fid: str, r: Result) -> dict:
    detail = "; ".join(
        [f"{c.a} x {c.b} {c.share:.0%}" for c in r.collisions]
        + [f"same ink as {' '.join(r.same_as)}"] * bool(r.same_as)
    )
    return {
        "face_id": fid,
        "case_id": r.case.id,
        "group": r.case.group,
        "text": r.case.text,
        "codepoints": r.case.codepoints,
        "failures": " ".join(r.failures),
        "glyphs": " ".join(g.name for g in r.glyphs),
        "detail": detail,
    }


def write_face_json(path: Path, summary: dict, face: Face, results: list[Result]) -> None:
    data = {
        "summary": summary,
        "font": face.info,
        "failures": [
            {k: v for k, v in failure_row(summary["face_id"], r).items() if k != "face_id"}
            for r in results
            if r.failures
        ],
    }
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text(json.dumps(data, ensure_ascii=False, indent=1) + "\n")


def write_csv(path: Path, fields: list[str], rows: list[dict]) -> None:
    path.parent.mkdir(parents=True, exist_ok=True)
    with open(path, "w", newline="", encoding="utf-8") as f:
        w = csv.DictWriter(f, fieldnames=fields, lineterminator="\n")
        w.writeheader()
        w.writerows(rows)


def write_cases(path: Path, cases: list[Case]) -> None:
    write_csv(
        path,
        ["case_id", "group", "text", "codepoints", "unjoined", "note"],
        [
            {
                "case_id": c.id,
                "group": c.group,
                "text": c.text,
                "codepoints": c.codepoints,
                "unjoined": " ".join(f"U+{ord(ch):04X}" for ch in c.unjoined or ""),
                "note": c.note,
            }
            for c in cases
        ],
    )
