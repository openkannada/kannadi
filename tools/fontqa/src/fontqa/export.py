"""The test strings in the formats of other font QA tools: FontBakery and Shaperglot."""

from __future__ import annotations

import json

from .kannada import RA, VIRAMA, Case

# Glyph names fonts give U+25CC; HarfBuzz inserts it before a broken cluster.
DOTTED_CIRCLE_NAMES = ("uni25CC", "dottedcircle", "dottedCircle")
ZWJ = "\u200d"


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
    """Shaperglot manual checks (TOML): conjuncts and the arkavattu must shape differently from
    the same text with a joiner that blocks them."""
    conjuncts = [(c.text, c.unjoined) for c in cases if c.unjoined]
    reph = RA + VIRAMA
    arkavattu = [
        (c.text, RA + ZWJ + VIRAMA + c.text[len(reph) :])
        for c in cases
        if c.text.startswith(reph) and c.unjoined
    ]
    checks = [
        (
            "Consonant clusters form conjuncts",
            "A cluster with a virama should shape differently from the same cluster with ZWNJ"
            " after the virama, which asks for an explicit virama.",
            conjuncts,
        ),
        (
            "Ra before a consonant forms the arkavattu",
            "Ra + virama + consonant should shape differently from ra + ZWJ + virama + consonant,"
            " which asks for ra with a visible virama.",
            arkavattu,
        ),
    ]
    return "\n".join(_shaperglot_check(language, *check) for check in checks)


def _shaperglot_check(language: str, name: str, description: str, pairs) -> str:
    rows = ",\n".join(f"    [{{ text = {_toml(a)} }}, {{ text = {_toml(b)} }}]" for a, b in pairs)
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
        "ignore_notdefs = false\n"
        f"pairs = [\n{rows},\n]\n"
    )


def _toml(text: str) -> str:
    """A TOML basic string; JSON escaping is a valid subset of it."""
    return json.dumps(text, ensure_ascii=True)
