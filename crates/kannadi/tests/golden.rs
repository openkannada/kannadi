use kannadi::{from_latin, to_latin, Scheme};

const SCHEMES: [Scheme; 3] = [Scheme::Iso15919, Scheme::Iast, Scheme::Baraha];

fn rows() -> Vec<Vec<String>> {
    include_str!("fixtures/transliteration.tsv")
        .lines()
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(|l| l.split('\t').map(str::to_string).collect())
        .collect()
}

#[test]
fn fixtures_have_four_columns() {
    for row in rows() {
        assert_eq!(row.len(), 4, "{row:?}");
    }
}

#[test]
fn kannada_to_latin_matches_fixtures() {
    for row in rows() {
        for (i, scheme) in SCHEMES.iter().enumerate() {
            assert_eq!(
                to_latin(&row[0], *scheme),
                row[i + 1],
                "{} to {scheme}",
                row[0]
            );
        }
    }
}

#[test]
fn latin_to_kannada_matches_fixtures() {
    for row in rows() {
        for (i, scheme) in SCHEMES.iter().enumerate() {
            assert_eq!(
                from_latin(&row[i + 1], *scheme),
                row[0],
                "{} from {scheme}",
                row[i + 1]
            );
        }
    }
}
