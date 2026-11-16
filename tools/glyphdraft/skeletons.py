"""Skeletons for drafted Kannada glyphs, as stroke centerlines.

Coordinates are in units of the skeleton height: y 0 is the center of the bottom stroke and
y 1 the center of the top stroke. x uses the same unit, measured in a design whose ಅ (U+0C85)
has the ink aspect ASPECT_A, so fonts with wider or narrower letters get wider or narrower
drafts. Each piece is stroked on its own; pieces meet at sharp corners.
"""

ASPECT_A = 1.372

# U+0CDC KANNADA ARCHAIC SHRII, traced from the representative glyph in L2/22-006 (image pixels:
# top 133, bottom 407, left 111) and revised after a second opinion from Codex. 2 pieces, split
# at the sharp waist of the inner 3.
ARCHAIC_SHRII = (
    (
        (0.328, 0.544),
        (0.339, 0.664),
        (0.533, 0.650),
        (0.555, 0.478),
        (0.427, 0.420),
    ),
    (
        (0.427, 0.420),
        (0.628, 0.303),
        (0.569, 0.051),
        (0.292, 0.004),
        (0.066, 0.190),
        (0.000, 0.496),
        (0.142, 0.869),
        (0.551, 1.000),
        (0.905, 0.964),
        (1.161, 0.788),
        (1.277, 0.427),
        (1.182, 0.066),
        (1.047, 0.000),
        (0.927, 0.080),
        (0.843, 0.398),
        (0.891, 0.726),
        (1.040, 0.927),
        (1.237, 1.055),
        (1.412, 1.099),
    ),
)

# The inner 3 is drawn lighter than the bowl so its 2 small recesses stay open.
ARCHAIC_SHRII_WEIGHTS = (0.7, 1.0)

# U+0CDD KANNADA LETTER NAKAARA POLLU, after the Unicode code chart: an epsilon whose top
# stroke runs on to the right. 2 pieces, split at the cusp that points right at mid height.
NAKAARA_POLLU = (
    (
        (1.000, 1.000),
        (0.560, 1.000),
        (0.240, 0.990),
        (0.060, 0.860),
        (0.070, 0.650),
        (0.240, 0.540),
        (0.500, 0.520),
    ),
    (
        (0.500, 0.520),
        (0.230, 0.490),
        (0.030, 0.350),
        (0.050, 0.120),
        (0.250, 0.000),
        (0.560, 0.000),
        (0.760, 0.060),
    ),
)
NAKAARA_POLLU_WEIGHTS = (1.0, 1.0)

# U+0CF3 KANNADA SIGN COMBINING ANUSVARA ABOVE RIGHT is built from the font's own ಂ (U+0C82):
# scaled by RING_SCALE, with its bottom at RING_BOTTOM of the x-height and a mean stroke width
# RING_WEIGHT of the anusvara's. Noto Sans Kannada Regular's ring is 0.85 of its ಂ and 0.88 of
# its consonants.
RING_SCALE = 0.49
RING_BOTTOM = 0.83
RING_WEIGHT = 0.9
