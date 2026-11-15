"""The Kannada test inventory: which strings every font is shaped against."""

from __future__ import annotations

from dataclasses import dataclass

VIRAMA = "\u0ccd"
ZWNJ = "\u200c"
RA = "\u0cb0"

# U+0C95..U+0CB9 without the unassigned U+0CA9 and U+0CB4, then U+0CDE ೞ.
CONSONANTS = tuple(chr(cp) for cp in [*range(0x0C95, 0x0CBA), 0x0CDE] if cp not in (0x0CA9, 0x0CB4))

# The 13 dependent vowel signs, the vocalic L signs U+0CE2 and U+0CE3, and the virama.
VOWEL_SIGNS = tuple(
    chr(cp)
    for cp in [
        0x0CBE, 0x0CBF, 0x0CC0, 0x0CC1, 0x0CC2, 0x0CC3, 0x0CC4,
        0x0CC6, 0x0CC7, 0x0CC8, 0x0CCA, 0x0CCB, 0x0CCC,
        0x0CE2, 0x0CE3, 0x0CCD,
    ]
)  # fmt: skip

# Vowel signs, then anusvara ಂ U+0C82 and visarga ಃ U+0C83.
SIGNS = (*VOWEL_SIGNS, "\u0c82", "\u0c83")

# ಱ U+0CB1 and ೞ U+0CDE are Old Kannada letters; clusters with them are their own group.
OLD = ("\u0cb1", "\u0cde")

# U+0C85..U+0C94 without the unassigned U+0C8D and U+0C91, then ೠ U+0CE0 and ೡ U+0CE1.
VOWELS = tuple(
    chr(cp) for cp in [*range(0x0C85, 0x0C95), 0x0CE0, 0x0CE1] if cp not in (0x0C8D, 0x0C91)
)

# ೄ, ೢ and ೣ on a cluster do not occur in text, so they are left out of the signed clusters.
CLUSTER_SIGNS = tuple(s for s in SIGNS if s not in ("\u0cc4", "\u0ce2", "\u0ce3"))

# Common clusters that carry a vowel sign in running text, where marks meet subscripts.
SIGNED_CLUSTERS = ("ಕ್ಕ", "ಸ್ತ", "ಕ್ಷ", "ನ್ನ", "ಲ್ಲ", "ತ್ರ", "ದ್ಯ", "ಸ್ವ", "ಪ್ಪ", "ಮ್ಮ")

THREE_CONSONANTS = ("ಸ್ತ್ರ", "ಷ್ಟ್ರ", "ನ್ತ್ಯ", "ಕ್ಷ್ಮ")

# Rare characters, each shown in a context where it is valid text.
RARE = (
    ("0CDD", "ಅವನೝ", "NAKAARA POLLU ೝ after a word"),
    ("0CF3", "ಕೳ", "COMBINING ANUSVARA ABOVE RIGHT ೳ on ಕ"),
    ("0CDC", "\u0cdc", "ARCHAIC SHRII, Unicode 17.0"),
    ("0CDC-word", "\u0cdcಗಣೇಶ", "ARCHAIC SHRII before a word"),
    ("0CF1", "ಅೱಕ", "JIHVAMULIYA ೱ"),
    ("0CF2", "ಅೲಪ", "UPADHMANIYA ೲ"),
    ("0CB1", "ಱ", "LETTER RRA ಱ"),
    ("0CDE", "ಬಾೞ್", "LETTER LLLA ೞ in ಬಾೞ್"),
    ("0CBC-ja", "ಜ಼", "NUKTA on ಜ, for z"),
    ("0CBC-pha", "ಫ಼", "NUKTA on ಫ, for f"),
    ("0CBD", "ಸೋಽಹಂ", "AVAGRAHA ಽ"),
)


@dataclass(frozen=True)
class Case:
    """1 string to shape, with what a correct font must do with it."""

    id: str
    group: str
    text: str
    # A cluster must shape differently from this string, which has a ZWNJ after the virama.
    unjoined: str | None = None
    note: str = ""

    @property
    def codepoints(self) -> str:
        return " ".join(f"U+{ord(c):04X}" for c in self.text)


def _hex(text: str) -> str:
    return "-".join(f"{ord(c):04X}" for c in text)


def syllables() -> list[Case]:
    return [Case(f"syl-{_hex(c + v)}", "syllable", c + v) for c in CONSONANTS for v in SIGNS]


def vowels() -> list[Case]:
    return [Case(f"vow-{_hex(v)}", "vowel", v) for v in VOWELS]


def _joined(group: str, text: str, prefix: str = "clu") -> Case:
    """A cluster that must shape differently from the same text with ZWNJ after its 1st virama."""
    i = text.index(VIRAMA) + 1
    return Case(f"{prefix}-{_hex(text)}", group, text, unjoined=text[:i] + ZWNJ + text[i:])


def clusters() -> list[Case]:
    """Every consonant + virama + consonant, grouped as arkavattu, old or modern.

    ರ್ರ is a modern cluster: fonts draw it with a subscript ರ as often as with the arkavattu.
    """
    out = []
    for a in CONSONANTS:
        for b in CONSONANTS:
            if {a, b} & set(OLD):
                group = "old_cluster"
            elif a == RA and b != RA:
                group = "arkavattu"
            else:
                group = "cluster"
            out.append(_joined(group, a + VIRAMA + b))
    return out


def signed_clusters() -> list[Case]:
    """Common and 3-consonant clusters with each sign, where marks and subscripts meet."""
    out = [_joined("signed_cluster", c + s, "sig") for c in SIGNED_CLUSTERS for s in CLUSTER_SIGNS]
    return out + [_joined("signed_cluster", c, "sig") for c in THREE_CONSONANTS]


def arkavattu_extra() -> list[Case]:
    """Arkavattu with each sign, and on 3-consonant clusters."""
    ka = "ಕ"
    tails = [ka + s for s in SIGNS] + ["ಕ್ಷ", "ತ್ಯ", "ಧ್ವ"]
    return [_joined("arkavattu", RA + VIRAMA + t, "ark") for t in tails]


def rare() -> list[Case]:
    return [Case(f"rare-{key}", "rare", text, note=note) for key, text, note in RARE]


def all_cases() -> list[Case]:
    return syllables() + vowels() + clusters() + signed_clusters() + arkavattu_extra() + rare()
