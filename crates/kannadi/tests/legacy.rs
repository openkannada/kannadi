use kannadi::legacy::{
    decode_windows_1252, from_unicode, nudi_to_unicode, to_unicode, unicode_to_nudi, Layout,
    Options,
};

/// The golden fixture of each layout, with the fewest rows it must have.
const FIXTURES: [(Layout, &str, usize); 2] = [
    (Layout::Nudi, include_str!("fixtures/nudi.tsv"), 10),
    (Layout::Akruti, include_str!("fixtures/akruti.tsv"), 100),
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
    let rows = include_str!("fixtures/nudi.tsv")
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'));
    let mut count = 0;
    for row in rows {
        let bytes: Vec<u8> = row
            .split('\t')
            .next()
            .unwrap()
            .split_whitespace()
            .map(|h| u8::from_str_radix(h, 16).unwrap())
            .collect();
        for typed in decode_windows_1252(&bytes).split(' ') {
            let unicode = nudi_to_unicode(typed, &Options::default());
            if typed.contains('0') || !unicode.is_clean() {
                continue;
            }
            assert_eq!(
                unicode_to_nudi(&unicode.text).text,
                typed,
                "{}",
                unicode.text
            );
            count += 1;
        }
    }
    assert!(count >= 80);
}
