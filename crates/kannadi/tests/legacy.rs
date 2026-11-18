use kannadi::legacy::{decode_windows_1252, from_unicode, to_unicode, Layout, Options};

/// The golden fixture of each layout, with the fewest rows it must have.
const FIXTURES: [(Layout, &str, usize); 5] = [
    (Layout::Nudi, include_str!("fixtures/nudi.tsv"), 10),
    (Layout::Akruti, include_str!("fixtures/akruti.tsv"), 100),
    (
        Layout::ShreeLipi,
        include_str!("fixtures/shree-lipi.tsv"),
        100,
    ),
    (Layout::NudiB, include_str!("fixtures/nudi-b.tsv"), 100),
    (Layout::KanEng, include_str!("fixtures/kan-eng.tsv"), 100),
];

/// The legacy text, expected Unicode and source of each fixture row.
fn rows(fixture: &str) -> impl Iterator<Item = (String, &str, &str)> {
    fixture
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|row| {
            let cols: Vec<&str> = row.split('\t').collect();
            let bytes: Vec<u8> = cols[0]
                .split_whitespace()
                .map(|h| u8::from_str_radix(h, 16).unwrap())
                .collect();
            (decode_windows_1252(&bytes), cols[1], cols[2])
        })
}

#[test]
fn documents_convert_cleanly() {
    for (layout, fixture, least) in FIXTURES {
        let mut count = 0;
        for (legacy, expected, source) in rows(fixture) {
            let converted = to_unicode(&legacy, layout, &Options::default());
            assert!(converted.is_clean(), "{layout}: {converted:?}");
            assert_eq!(converted.text, expected, "{layout}: source {source}");
            count += 1;
        }
        assert!(count >= least, "{layout}: {count} rows");
    }
}

/// Text written in a layout reads back as the same Unicode, for every word of the
/// fixtures that converts cleanly.
#[test]
fn document_words_round_trip() {
    for (layout, fixture, _) in FIXTURES {
        for (legacy, _, _) in rows(fixture) {
            for typed in legacy.split(' ') {
                let unicode = to_unicode(typed, layout, &Options::default());
                if typed.contains('0') || !unicode.is_clean() {
                    continue;
                }
                let encoded = from_unicode(&unicode.text, layout);
                assert!(encoded.is_clean(), "{layout}: {}", unicode.text);
                let back = to_unicode(&encoded.text, layout, &Options::default());
                assert_eq!(back.text, unicode.text, "{layout}");
            }
        }
    }
}

/// Each fixture word encodes back to the bytes it was typed with. Words with a
/// digit zero are skipped: typists used 0 for ಂ, which the encoder writes as ಂ.
#[test]
fn government_pages_encode_as_typed() {
    let (same, total) = typed_back(Layout::Nudi);
    assert_eq!(same, total);
    assert!(total >= 80);
}

/// Akruti typists stored some letters in more than 1 way, such as the arkavattu
/// before or after ೕ, so a few fixture words encode differently from how they were typed.
#[test]
fn most_akruti_words_encode_as_typed() {
    let (same, total) = typed_back(Layout::Akruti);
    assert!(total >= 500, "{total}");
    assert!(same * 100 >= total * 95, "{same} of {total}");
}

/// Shree-Lipi typists also stored some clusters in more than 1 way, such as ್ಟ್ರ as
/// 1 glyph or 2, so at least 95% of fixture words encode as typed.
#[test]
fn most_shree_lipi_words_encode_as_typed() {
    let (same, total) = typed_back(Layout::ShreeLipi);
    assert!(total >= 700, "{total}");
    assert!(same * 100 >= total * 95, "{same} of {total}");
}

#[test]
fn most_nudi_b_words_encode_as_typed() {
    let (same, total) = typed_back(Layout::NudiB);
    assert!(total >= 400, "{total}");
    assert!(same * 100 >= total * 95, "{same} of {total}");
}

/// The budget papers type many glyphs at 2 codes, such as ಂ at 8C and F9, and the
/// encoder writes 1 of them.
#[test]
fn most_kan_eng_words_encode_as_typed() {
    let (same, total) = typed_back(Layout::KanEng);
    assert!(total >= 300, "{total}");
    assert!(same * 100 >= total * 90, "{same} of {total}");
}

/// How many clean fixture words encode back to the bytes typed, of how many.
fn typed_back(layout: Layout) -> (usize, usize) {
    let fixture = FIXTURES.iter().find(|f| f.0 == layout).unwrap().1;
    let (mut same, mut total) = (0, 0);
    for (legacy, _, _) in rows(fixture) {
        for typed in legacy.split(' ') {
            let unicode = to_unicode(typed, layout, &Options::default());
            if typed.is_empty() || typed.contains('0') || !unicode.is_clean() {
                continue;
            }
            total += 1;
            same += usize::from(from_unicode(&unicode.text, layout).text == typed);
        }
    }
    (same, total)
}
