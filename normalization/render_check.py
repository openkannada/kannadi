# SPDX-FileCopyrightText: 2026 The Open Kannada Authors
# SPDX-License-Identifier: MIT OR Apache-2.0
"""Shape the sequence pairs behind the canonical joiner rules with HarfBuzz, and print
whether each pair draws the same ink in each font.

    uv run --no-project --with uharfbuzz==0.57.0 python normalization/render_check.py FONT...

Ink is compared as glyph ids with outlines at their pen positions, so joiners, which
shape to empty glyphs, do not count.
"""

from __future__ import annotations

import sys
from pathlib import Path

import uharfbuzz as hb

V, N, J = "್", "‌", "‍"

# Rule, what is rewritten, what it becomes.
PAIRS = [
    ("C5", "ಮ" + N + V + "ಯ", "ಮ" + V + "ಯ"),
    ("C5", "ಪ" + J + V + "ಲ", "ಪ" + V + "ಲ"),
    ("C5", "ರ" + N + V + "ಯ", "ರ" + J + V + "ಯ"),
    ("C6", "ರ" + V + J + "ಲ", "ರ" + J + V + "ಲ"),
    ("C7", "ಜ" + V + J + "ಕ", "ಜ" + V + N + "ಕ"),
    ("C7", "ನ" + V + N + N + "ನ", "ನ" + V + N + "ನ"),
    ("C7", "ನ" + V + J + N + "ನ", "ನ" + V + N + "ನ"),
    ("C7", "ನ" + V + N + J + "ನ", "ನ" + V + N + "ನ"),
    ("C8", "ಆಫ" + V + N + " ", "ಆಫ" + V + " "),
    ("C8", "ಆಫ" + V + J + " ", "ಆಫ" + V + " "),
    ("C8", "ಸಿ" + N + "ಇ", "ಸಿಇ"),
    ("C8", "ರ" + N + "ಲಾ", "ರಲಾ"),
    ("C8", " " + N + "ಕನ", " ಕನ"),
    ("kept", "ಮ" + N + "ೂಲ", "ಮೂಲ"),
    ("kept", "ರ" + V + N + "ಲ", "ರ" + J + V + "ಲ"),
]


def ink(font: hb.Font, text: str) -> tuple:
    buf = hb.Buffer()
    buf.add_str(text)
    buf.guess_segment_properties()
    hb.shape(font, buf)
    out, x = [], 0
    for info, pos in zip(buf.glyph_infos, buf.glyph_positions, strict=True):
        extents = font.get_glyph_extents(info.codepoint)
        if extents and (extents.width or extents.height):
            out.append((info.codepoint, x + pos.x_offset, pos.y_offset))
        x += pos.x_advance
    return tuple(out)


def show(text: str) -> str:
    return text.replace(N, "<ZWNJ>").replace(J, "<ZWJ>").replace(V, "್").strip()


def main(paths: list[str]) -> int:
    fonts = [(Path(p).stem, hb.Font(hb.Face(hb.Blob.from_file_path(p)))) for p in paths]
    print("| Rule | Written | Becomes | " + " | ".join(name for name, _ in fonts) + " |")
    print("|---" * (3 + len(fonts)) + "|")
    for rule, before, after in PAIRS:
        same = ["same" if ink(f, before) == ink(f, after) else "differs" for _, f in fonts]
        print(f"| {rule} | {show(before)} | {show(after)} | " + " | ".join(same) + " |")
    print(f"\nHarfBuzz {hb.version_string()}")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
