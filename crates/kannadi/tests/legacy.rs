use kannadi::legacy::{decode_windows_1252, nudi_to_unicode, unicode_to_nudi, Options};

#[test]
fn government_pages_convert_cleanly() {
    let rows = include_str!("fixtures/nudi.tsv")
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'));
    let mut count = 0;
    for row in rows {
        let cols: Vec<&str> = row.split('\t').collect();
        let bytes: Vec<u8> = cols[0]
            .split_whitespace()
            .map(|h| u8::from_str_radix(h, 16).unwrap())
            .collect();
        let converted = nudi_to_unicode(&decode_windows_1252(&bytes), &Options::default());
        assert!(converted.is_clean(), "{converted:?}");
        assert_eq!(converted.text, cols[1], "source {}", cols[2]);
        count += 1;
    }
    assert!(count >= 10);
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
