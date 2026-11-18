"""Command line: fetch the fonts, run the tests, check font files, gate drafted glyphs."""

from __future__ import annotations

import argparse
import json
import sys
from collections import Counter
from pathlib import Path

from fontTools.ttLib import TTFont

from . import compose, dashboard, export, fixes
from . import fetch as fetching
from .checks import FAILURES, mark_indistinct, run_case
from .gates import DRAFT_TARGETS, gate_glyph, measure
from .kannada import all_cases
from .proof import sheet
from .report import (
    FAILURE_FIELDS,
    SUMMARY_FIELDS,
    face_id,
    failure_row,
    summarize,
    write_cases,
    write_csv,
    write_face_json,
)
from .shaping import Face, faces_in

HERE = Path(__file__).resolve().parents[2]
DEFAULT_MANIFEST = HERE / "fonts.toml"
DEFAULT_LOCK = HERE / "fonts.lock.json"
DEFAULT_CACHE = Path.home() / ".cache" / "kannadi-fontqa"
REFERENCE_FONT = "noto-sans-kannada"

# Faces with fewer Kannada characters than this are other scripts in a shared collection file.
MIN_KANNADA = 40


def main(argv: list[str] | None = None) -> int:
    p = argparse.ArgumentParser(prog="fontqa", description=__doc__)
    p.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    p.add_argument("--lock", type=Path, default=DEFAULT_LOCK)
    p.add_argument("--cache", type=Path, default=DEFAULT_CACHE)
    sub = p.add_subparsers(dest="cmd", required=True)

    f = sub.add_parser("fetch", help="download the fonts in the manifest")
    f.add_argument("--write-lock", action="store_true", help="record the sha256 of every file")

    r = sub.add_parser("run", help="test every font in the manifest")
    r.add_argument("--out", type=Path, default=HERE / "results")
    r.add_argument("--proofs", type=Path, help="write a proof sheet per face into this folder")
    r.add_argument("--only", nargs="*", help="font ids to test")

    c = sub.add_parser("check", help="test font files and print a summary")
    c.add_argument("fonts", nargs="+", type=Path)
    c.add_argument("--proofs", type=Path)
    c.add_argument("--fail-on", nargs="*", default=[], choices=FAILURES)

    x = sub.add_parser("fix", help="apply engineering fixes to a font file")
    x.add_argument("font", type=Path)
    x.add_argument("out", type=Path)
    x.add_argument("--drop-aat", action="store_true", help="remove morx, mort, feat and prop")
    x.add_argument("--knd2-blwf", action="store_true", help="add new-order blwf rules to knd2")
    x.add_argument("--compose-rr", action="store_true", help="add U+0CC4 from U+0CC3 and U+0CC2")
    x.add_argument("--compose-l", action="store_true", help="add U+0CE2 and U+0CE3 as subscripts")

    m = sub.add_parser("measure", help="print a font's stems and edges as JSON")
    m.add_argument("font", type=Path)

    g = sub.add_parser("gate", help="run the outline gates on drafted glyphs")
    g.add_argument("font", type=Path)
    g.add_argument(
        "--chars",
        nargs="+",
        default=[f"{cp:04X}" for cp in DRAFT_TARGETS],
        help="hex codepoints, from: " + " ".join(f"{cp:04X}" for cp in DRAFT_TARGETS),
    )

    e = sub.add_parser("export", help="write the test strings for FontBakery and Shaperglot")
    e.add_argument("--out", type=Path, default=HERE / "exports")

    d = sub.add_parser("dashboard", help="build the static coverage page from the results")
    d.add_argument("--results", type=Path, default=HERE / "results")
    d.add_argument("--out", type=Path, default=HERE / "results" / "dashboard")

    sub.add_parser("cases", help="print the test strings as CSV").add_argument(
        "--out", type=Path, default=Path("/dev/stdout")
    )

    args = p.parse_args(argv)
    return {
        "fetch": cmd_fetch,
        "run": cmd_run,
        "check": cmd_check,
        "fix": cmd_fix,
        "measure": cmd_measure,
        "gate": cmd_gate,
        "export": cmd_export,
        "dashboard": cmd_dashboard,
        "cases": cmd_cases,
    }[args.cmd](args)


def _fetch_all(args, only=None) -> dict[str, tuple[fetching.Entry, list[Path]]]:
    lock = fetching.load_lock(args.lock)
    out = {}
    for entry in fetching.load_manifest(args.manifest):
        if only and entry.id not in only:
            continue
        try:
            out[entry.id] = (entry, fetching.fetch(entry, args.cache, lock))
        except Exception as e:
            print(f"{entry.id}: fetch failed: {e}", file=sys.stderr)
    return out


def cmd_fetch(args) -> int:
    got = _fetch_all(args)
    for fid, (_, paths) in got.items():
        print(f"{fid}: {len(paths)} files")
    if args.write_lock:
        known = {e.id for e in fetching.load_manifest(args.manifest)}
        fetching.write_lock(args.lock, {k: v for k, (e, v) in got.items() if not e.local}, known)
    return 0


def _reference(args) -> Face | None:
    lock = fetching.load_lock(args.lock)
    for entry in fetching.load_manifest(args.manifest):
        if entry.id == REFERENCE_FONT:
            return Face(fetching.fetch(entry, args.cache, lock)[0])
    return None


def cmd_run(args) -> int:
    cases = all_cases()
    write_cases(args.out / "cases.csv", cases)
    reference = _reference(args) if args.proofs else None
    summaries, failures, seen = [], [], {}
    for entry, paths in _fetch_all(args, args.only).values():
        for path in paths:
            try:
                faces = faces_in(path)
            except Exception as e:
                print(f"{entry.id}/{path.name}: not a readable font: {e}", file=sys.stderr)
                continue
            for face in faces:
                if face.info["kannada_codepoints"] < MIN_KANNADA:
                    print(f"{entry.id}/{path.name}#{face.index}: not a Kannada face, skipped")
                    continue
                fid = face_id(entry, face)
                key = (face.sha256, face.index)
                if key in seen:
                    print(f"{fid}: same file as {seen[key]}, skipped")
                    continue
                seen[key] = fid
                results = [run_case(face, case) for case in cases]
                mark_indistinct(face, results)
                summary = summarize(fid, entry, face, results)
                summaries.append(summary)
                failures += [failure_row(fid, r) for r in results if r.failures]
                write_face_json(args.out / "fonts" / f"{fid}.json", summary, face, results)
                if args.proofs:
                    sheet(face, results, args.proofs / f"{fid}.png", reference)
                print(f"{fid}: {summary['failing_cases']} of {len(cases)} cases fail")
    write_csv(args.out / "summary.csv", SUMMARY_FIELDS, summaries)
    write_csv(args.out / "failures.csv", FAILURE_FIELDS, failures)
    return 0


def cmd_check(args) -> int:
    cases = all_cases()
    reference = None
    if args.proofs:
        try:
            reference = _reference(args)
        except (OSError, ValueError) as e:
            print(f"no reference font for the proof sheets: {e}", file=sys.stderr)
    bad = 0
    for path in args.fonts:
        for face in faces_in(path):
            results = [run_case(face, case) for case in cases]
            mark_indistinct(face, results)
            counts = Counter(f for r in results for f in r.failures)
            line = ", ".join(f"{k} {counts[k]}" for k in FAILURES if counts[k]) or "all pass"
            print(f"{path.name}#{face.index}: {line}")
            bad += sum(counts[k] for k in args.fail_on)
            if args.proofs:
                sheet(face, results, args.proofs / f"{path.stem}-{face.index}.png", reference)
    return 1 if bad else 0


def cmd_fix(args) -> int:
    font = TTFont(str(args.font))
    if args.drop_aat:
        print(f"removed: {' '.join(fixes.drop_aat(font)) or 'nothing'}")
    if args.knd2_blwf:
        print(f"knd2 blwf ligatures added: {fixes.add_new_order_blwf(font)}")
    if args.compose_rr:
        print(f"added: {compose.compose_rr_sign(font)}")
    if args.compose_l:
        print(f"added: {' '.join(compose.compose_l_signs(font))}")
    font.save(str(args.out))
    return 0


def cmd_measure(args) -> int:
    ref = measure(Face(args.font))
    print(json.dumps({k: round(v, 1) for k, v in vars(ref).items()}))
    return 0


def cmd_gate(args) -> int:
    face = Face(args.font)
    ref = measure(face)
    failed = 0
    for code in args.chars:
        cp = int(code, 16)
        if cp not in DRAFT_TARGETS:
            print(f"U+{cp:04X}: no gate target", file=sys.stderr)
            return 2
        for result in gate_glyph(face, cp, ref):
            failed += not result.passed
            verdict = "pass" if result.passed else "FAIL"
            print(f"U+{cp:04X} {result.name}: {verdict}: {result.detail}")
    return 1 if failed else 0


def cmd_export(args) -> int:
    cases = all_cases()
    bakery = args.out / "fontbakery"
    bakery.mkdir(parents=True, exist_ok=True)
    common = [c for c in cases if c.group != "rare"]
    rare = [c for c in cases if c.group == "rare"]
    for name, data in (
        ("kannada-fontqa-forbidden.json", export.fontbakery_forbidden(common)),
        ("kannada-fontqa-forbidden-rare.json", export.fontbakery_forbidden(rare)),
        ("kannada-fontqa-collisions.json", export.fontbakery_collisions(cases)),
    ):
        (bakery / name).write_text(json.dumps(data, ensure_ascii=False, indent=1) + "\n")
    (bakery / "fontbakery.yml").write_text("shaping:\n  test_directory: .\n")
    glot = args.out / "shaperglot"
    glot.mkdir(parents=True, exist_ok=True)
    (glot / "kn_Knda.toml").write_text(export.shaperglot_checks(cases))
    print(f"wrote {bakery} and {glot}")
    return 0


def cmd_dashboard(args) -> int:
    print(f"wrote {dashboard.build(args.results, args.out)}")
    return 0


def cmd_cases(args) -> int:
    write_cases(args.out, all_cases())
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
