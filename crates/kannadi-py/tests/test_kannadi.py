import kannadi
import pytest

# Karnataka RDPR page, Dec 2002, BRH Kannada (the same line as web/test/smoke.mjs).
NUDI = "¸ÀPÁðj ¥ÁæxÀ«ÄPÀ ±Á¯ÉUÀ¼À°è ²PÀëPÀjUÉ vÀgÀ¨ÉÃw ¤ÃqÀÄªÀÅzÀÄ."
UNICODE = "ಸರ್ಕಾರಿ ಪ್ರಾಥಮಿಕ ಶಾಲೆಗಳಲ್ಲಿ ಶಿಕ್ಷಕರಿಗೆ ತರಬೇತಿ ನೀಡುವುದು."


def test_converts_a_real_line():
    c = kannadi.to_unicode(NUDI)
    assert c.text == UNICODE
    assert c.is_clean and c.unknown == [] and c.malformed == []


def test_round_trips_through_nudi():
    e = kannadi.from_unicode(UNICODE)
    assert e.is_clean
    assert kannadi.to_unicode(e.text).text == UNICODE


def test_unknown_glyphs_are_reported_at_python_indices():
    text = "PÀ£ÀßqÀ õ ¥Àæ õ"
    c = kannadi.to_unicode(text)
    assert not c.is_clean
    assert [g for _, g in c.unknown] == ["õ", "õ"]
    for at, glyph in c.unknown:
        assert text[at] == glyph


def test_malformed_ranges_index_the_output():
    c = kannadi.to_unicode("Ã")
    assert c.malformed == [(0, len(c.text))]
    assert not c.is_clean


def test_reprs_read_as_python():
    assert repr(kannadi.to_unicode("PÀ£ÀßqÀ")) == "Conversion(text='ಕನ್ನಡ', unknown=[], malformed=[])"
    assert repr(kannadi.from_unicode("ಕ")) == "Encoding(text='PÀ', unencodable=[], malformed=[])"


def test_unencodable_characters_are_reported_at_python_indices():
    text = "ಕನ್ನಡ 😀 ಕ"
    e = kannadi.from_unicode(text)
    for at, ch in e.unencodable:
        assert text[at] == ch


def test_layouts_fonts_and_options():
    assert kannadi.best_layout(NUDI) == "nudi"
    assert kannadi.font_layout("BRH Kannada") == "nudi"
    assert kannadi.font_layout("Noto Sans Kannada") is None
    assert kannadi.is_legacy_font("BRH Kannada")
    assert kannadi.to_unicode("12", kannada_digits=True).text == "೧೨"
    assert kannadi.to_unicode("12", "baraha").text == "12"
    with pytest.raises(ValueError, match="unknown layout"):
        kannadi.to_unicode("x", "devanagari")


def test_text_tools():
    assert kannadi.aksharas("ಕನ್ನಡ") == ["ಕ", "ನ್ನ", "ಡ"]
    assert kannadi.count_aksharas("ಕನ್ನಡ") == 3
    assert kannadi.transliterate("ಕನ್ನಡ ನಾಡು", "kannada", "iso15919") == "kannaḍa nāḍu"
    assert kannadi.transliterate("kannaḍa", "iso", "kn") == "ಕನ್ನಡ"
    assert kannadi.modernize("ೞ ಱ") == "ಳ ರ"
    assert kannadi.normalize(UNICODE) == UNICODE
    with pytest.raises(ValueError):
        kannadi.normalize("x", "nfc")
    with pytest.raises(ValueError):
        kannadi.transliterate("x", "kannada", "hunterian")


def test_version():
    assert kannadi.__version__ == "0.1.0"
