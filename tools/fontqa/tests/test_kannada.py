import unicodedata
from collections import Counter

from fontqa.kannada import CONSONANTS, SIGNS, VOWEL_SIGNS, VOWELS, all_cases, clusters, syllables


def test_inventory_sizes():
    assert len(CONSONANTS) == 36
    assert len(VOWEL_SIGNS) == 16
    assert len(SIGNS) == 18
    assert len(VOWELS) == 16
    assert len(syllables()) == 648
    groups = Counter(c.group for c in clusters())
    assert groups == {"cluster": 33 * 34 + 1, "old_cluster": 36 * 36 - 34 * 34, "arkavattu": 33}


def test_every_string_is_tested_once():
    texts = [c.text for c in all_cases()]
    assert len(texts) == len(set(texts))


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
