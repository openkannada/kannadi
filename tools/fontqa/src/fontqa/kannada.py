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

# Rare characters, each shown in a context where it is valid text.
RARE = (
    ("0CDD", "ಅವನೝ", "NAKAARA POLLU ೝ after a word"),
    ("0CF3", "ಕೳ", "COMBINING ANUSVARA ABOVE RIGHT ೳ on ಕ"),
    ("0CDC", "\u0cdc", "ARCHAIC SHRII, Unicode 17.0"),
    ("0CDC-word", "\u0cdcಗಣೇಶ", "ARCHAIC SHRII before a word"),
    ("0CF1", "ಅೱಕ", "JIHVAMULIYA ೱ"),
    ("0CF2", "ಅೲಪ", "UPADHMANIYA ೲ"),
    ("0CE2", "ಕೢ", "VOWEL SIGN VOCALIC L ೢ"),
    ("0CE3", "ಕೣ", "VOWEL SIGN VOCALIC LL ೣ"),
    ("0C8C", "ಌ", "LETTER VOCALIC L ಌ"),
    ("0CE1", "ೡ", "LETTER VOCALIC LL ೡ"),
    ("0CC4", "ಕೄ", "VOWEL SIGN VOCALIC RR ೄ"),
    ("0CE0", "ೠ", "LETTER VOCALIC RR ೠ"),
    ("0CB1", "ಱಿ", "LETTER RRA ಱ with ಿ"),
    ("0CDE", "ಬಾೞ್", "LETTER LLLA ೞ in ಬಾೞ್"),
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
    return [Case(f"syl-{_hex(c + v)}", "syllable", c + v) for c in CONSONANTS for v in VOWEL_SIGNS]


def clusters() -> list[Case]:
    """Every consonant + virama + consonant. A first ರ is an arkavattu, so it goes in that group."""
    return [
        Case(
            f"clu-{_hex(a + VIRAMA + b)}",
            "arkavattu" if a == RA else "cluster",
            a + VIRAMA + b,
            unjoined=a + VIRAMA + ZWNJ + b,
        )
        for a in CONSONANTS
        for b in CONSONANTS
    ]


def arkavattu_extra() -> list[Case]:
    """Arkavattu with each vowel sign, and on 3-consonant clusters."""
    ka = "ಕ"
    cases = [
        Case(
            f"ark-{_hex(RA + VIRAMA + ka + v)}",
            "arkavattu",
            RA + VIRAMA + ka + v,
            unjoined=RA + VIRAMA + ZWNJ + ka + v,
        )
        for v in VOWEL_SIGNS
    ]
    for tail in ("ಕ್ಷ", "ತ್ಯ", "ಧ್ವ"):
        cases.append(
            Case(
                f"ark-{_hex(RA + VIRAMA + tail)}",
                "arkavattu",
                RA + VIRAMA + tail,
                unjoined=RA + VIRAMA + ZWNJ + tail,
            )
        )
    return cases


def rare() -> list[Case]:
    return [Case(f"rare-{key}", "rare", text, note=note) for key, text, note in RARE]


def all_cases() -> list[Case]:
    return syllables() + clusters() + arkavattu_extra() + rare()
