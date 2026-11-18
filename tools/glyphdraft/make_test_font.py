"""Write a small synthetic Kannada font for testing the drafter: rings for the consonants,
ಅ and ಂ. Run with FontForge's Python: make_test_font.py OUT.sfd OUT.ttf
"""

import sys
from pathlib import Path

import fontforge

sys.path.insert(0, str(Path(__file__).parent))
from draft import CONSONANTS


def ring(glyph, width, height, stroke):
    pen = glyph.glyphPen()
    pen.moveTo((0, height / 2))
    pen.curveTo((0, height), (width, height), (width, height / 2))
    pen.curveTo((width, 0), (0, 0), (0, height / 2))
    pen.closePath()
    pen = None
    glyph.stroke("circular", stroke, "round", "round")
    glyph.removeOverlap()
    glyph.correctDirection()
    glyph.left_side_bearing = 60
    glyph.right_side_bearing = 60


def main(sfd, ttf):
    font = fontforge.font()
    font.em = 1000
    font.familyname = font.fontname = font.fullname = "DraftTest"
    for cp in CONSONANTS:
        ring(font.createChar(cp, f"uni{cp:04X}"), 440, 500, 70)
    ring(font.createChar(0x0C85, "uni0C85"), 700, 500, 70)
    ring(font.createChar(0x0C82, "uni0C82"), 440, 500, 70)
    font.save(sfd)
    font.generate(ttf)


if __name__ == "__main__":
    main(*sys.argv[1:])
