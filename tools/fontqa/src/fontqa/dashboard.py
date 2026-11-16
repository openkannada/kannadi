"""A static coverage dashboard: 1 self-contained HTML page built from the results folder."""

from __future__ import annotations

import csv
import json
from collections import defaultdict
from html import escape
from pathlib import Path

from .checks import FAILURES

NEW_CHARACTERS = {"U+0CDC": "೜", "U+0CDD": "ೝ", "U+0CF3": "ೳ"}
EXAMPLES_PER_KIND = 8
TEMPLATE = Path(__file__).with_name("dashboard.html")


def _label(kind: str) -> str:
    return escape(kind.replace("_", " "))


def build(results: Path, out: Path) -> Path:
    """Write out/index.html from results/summary.csv and results/fonts/*.json."""
    with (results / "summary.csv").open(encoding="utf-8") as f:
        faces = list(csv.DictReader(f))
    faces.sort(key=lambda r: (int(r["failing_cases"]), r["family"], r["style"]))
    rows = "\n".join(
        _row(face, _examples(results / "fonts" / f"{face['face_id']}.json")) for face in faces
    )
    total = faces[0]["cases"] if faces else "0"
    html = TEMPLATE.read_text(encoding="utf-8").format(
        faces=len(faces),
        families=len({f["font_id"] for f in faces}),
        cases=f"{int(total):,}",
        clean=sum(1 for f in faces if f["failing_cases"] == "0"),
        heads="".join(
            f"<th scope=col data-num><button>{_label(k)}</button></th>" for k in FAILURES
        ),
        rows=rows,
    )
    out.mkdir(parents=True, exist_ok=True)
    page = out / "index.html"
    page.write_text(html, encoding="utf-8")
    return page


def _examples(path: Path) -> dict[str, list[str]]:
    data = json.loads(path.read_text(encoding="utf-8"))
    by_kind: dict[str, list[str]] = defaultdict(list)
    for failure in data["failures"]:
        for kind in failure["failures"].split():
            if len(by_kind[kind]) < EXAMPLES_PER_KIND:
                by_kind[kind].append(failure["text"])
    return by_kind


def _row(face: dict, examples: dict[str, list[str]]) -> str:
    missing = set(face["missing_codepoints"].split())
    new = "".join(
        f'<span class="{"no" if cp in missing else "yes"}" title="{cp}">{ch}'
        f'<span class="sr"> {cp} {"missing" if cp in missing else "present"}</span></span>'
        for cp, ch in NEW_CHARACTERS.items()
    )
    knd2 = "yes" if "knd2" in face["gsub_scripts"].split() else "no"
    counts = "".join(
        f'<td data-num class="{"bad" if face[k] != "0" else ""}">{int(face[k]):,}</td>'
        for k in FAILURES
    )
    detail = "".join(
        f"<p><b>{_label(k)}</b> "
        + " ".join(f'<span lang="kn">{escape(t)}</span>' for t in examples[k])
        + "</p>"
        for k in FAILURES
        if examples.get(k)
    )
    name = f"{face['family']} {face['style']}".strip()
    search = escape(f"{name} {face['license']}".lower(), quote=True)
    return (
        f'<tr data-search="{search}"><th scope=row><details><summary>{escape(name)}</summary>'
        f"{detail or '<p>No failures.</p>'}</details></th>"
        f"<td>{escape(face['version'])}</td><td>{escape(face['license'])}</td>"
        f'<td data-num class="{"bad" if face["failing_cases"] != "0" else ""}">'
        f"{int(face['failing_cases']):,}</td>{counts}"
        f'<td class="new" lang="kn">{new}</td><td>{knd2}</td></tr>'
    )
