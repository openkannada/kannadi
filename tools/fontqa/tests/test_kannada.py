import unicodedata

from fontqa.kannada import CONSONANTS, VOWEL_SIGNS, all_cases, clusters, syllables


def test_inventory_sizes():
    assert len(CONSONANTS) == 36
    assert len(VOWEL_SIGNS) == 16
    assert len(syllables()) == 576
    assert len(clusters()) == 1296
    assert sum(c.group == "arkavattu" for c in clusters()) == 36


def test_every_character_is_assigned():
    for case in all_cases():
        for ch in case.text:
            assert unicodedata.name(ch, None) or ord(ch) == 0x0CDC, f"U+{ord(ch):04X}"


def test_case_ids_are_unique():
    ids = [c.id for c in all_cases()]
    assert len(ids) == len(set(ids))


def test_unjoined_strings_differ_only_by_zwnj():
    for case in all_cases():
        if case.unjoined:
            assert case.unjoined.replace("‌", "") == case.text
