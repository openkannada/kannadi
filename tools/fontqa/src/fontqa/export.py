"""The test strings in the formats of other font QA tools: FontBakery and Shaperglot."""

from __future__ import annotations

import json

from .kannada import RA, VIRAMA, Case

# Glyph names fonts give U+25CC; HarfBuzz inserts it before a broken cluster.
DOTTED_CIRCLE_NAMES = ("uni25CC", "dottedcircle", "dottedCircle")
# Features that form conjuncts; Shaperglot turns them off with a minus sign.
CONJUNCT_FEATURES = ("-akhn", "-rphf", "-blwf", "-half", "-pstf", "-cjct")


def fontbakery_forbidden(cases: list[Case]) -> dict:
    """A FontBakery shaping test file: no string may shape to .notdef or a dotted circle."""
    return {
        "configuration": {
            "comment": "Kannada strings from kannadi fontqa. None may produce these glyphs.",
            "forbidden_glyphs": [".notdef", *DOTTED_CIRCLE_NAMES],
        },
        "tests": [{"input": c.text, "comment": c.id} for c in cases],
    }


def fontbakery_collisions(cases: list[Case]) -> dict:
    """A FontBakery shaping test file that runs collidoscope on strings with marks."""
    marked = [c for c in cases if c.group not in ("vowel", "rare")]
    return {
        "configuration": {
            "comment": "Kannada strings from kannadi fontqa, checked for mark collisions.",
            "collidoscope": {"bases": False, "marks": True, "faraway": False, "area": 3},
        },
        "tests": [{"input": c.text, "comment": c.id} for c in marked],
    }


def shaperglot_checks(cases: list[Case], language: str = "kn_Knda") -> str:
    """Shaperglot manual checks (TOML): each cluster must shape differently with its conjunct
    features on and off. Shaperglot compares the whole output, so a joiner would always differ."""
    clusters = [c.text for c in cases if c.unjoined]
    # ರ್ರ forms a subscript ra in many fonts, not a reph, so it is a conjunct case only.
    arkavattu = [t for t in clusters if t.startswith(RA + VIRAMA) and t != RA + VIRAMA + RA]
    checks = [
        (
            "Consonant clusters form conjuncts",
            "A cluster with a virama should shape differently with the features that form"
            " conjuncts turned off.",
            clusters,
            CONJUNCT_FEATURES,
        ),
        (
            "Ra before a consonant forms the arkavattu",
            "Ra + virama + consonant should shape differently with rphf turned off.",
            arkavattu,
            ("-rphf",),
        ),
    ]
    return "\n".join(_shaperglot_check(language, *check) for check in checks)


def _shaperglot_check(language, name, description, texts, features_off) -> str:
    off = "[" + ", ".join(_toml(f) for f in features_off) + "]"
    rows = ",\n".join(
        f"    [{{ text = {_toml(t)}, features = [] }}, {{ text = {_toml(t)}, features = {off} }}]"
        for t in texts
    )
    return (
        f"[[{language}]]\n"
        f"name = {_toml(name)}\n"
        'severity = "Fail"\n'
        f"description = {_toml(description)}\n"
        'scoring_strategy = "Continuous"\n'
        "weight = 20\n\n"
        f"[[{language}.implementations]]\n"
        'type = "ShapingDiffers"\n'
        "features_optional = false\n"
        "ignore_notdefs = true\n"
        f"pairs = [\n{rows},\n]\n"
    )


def _toml(text: str) -> str:
    """A TOML basic string; JSON escaping is a valid subset of it."""
    return json.dumps(text, ensure_ascii=True)
