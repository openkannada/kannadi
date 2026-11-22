use super::build::{char_style, compound, font, Doc, Piece, Style};
use super::*;
use crate::flow::Run;

fn runs(doc: &Document, story: usize) -> Vec<Vec<(Option<&str>, &str)>> {
    doc.stories[story]
        .paragraphs
        .iter()
        .map(|p| {
            p.iter()
                .map(|Run { font, text }| {
                    let name = font.and_then(|f| doc.fonts.get(f)).map(|f| f.name.as_str());
                    (name, text.as_str())
                })
                .collect()
        })
        .collect()
}

fn normal(grpprl: Vec<u8>) -> Style {
    Style {
        kind: 1,
        base: 0x0FFF,
        grpprl,
    }
}

fn fonts() -> Vec<&'static str> {
    vec!["Times New Roman", "Nudi 01 e", "BRH Kannada"]
}

#[test]
fn runs_keep_their_fonts_across_compressed_and_unicode_pieces() {
    let doc = Doc {
        fonts: fonts(),
        styles: vec![normal(Vec::new())],
        pieces: vec![
            Piece::new(true, vec![(font(1), "PÀ"), (font(0), " Office\r")]),
            Piece::new(false, vec![(font(0), "ಕನ್ನಡ "), (font(2), "PÀ\r")]),
        ],
        ..Doc::default()
    };
    let read = read(&doc.bytes()).unwrap();
    assert_eq!(read.stories.len(), 1);
    assert_eq!(read.stories[0].name, "body");
    assert_eq!(
        runs(&read, 0),
        [
            vec![
                (Some("Nudi 01 e"), "PÀ"),
                (Some("Times New Roman"), " Office")
            ],
            vec![
                (Some("Times New Roman"), "ಕನ್ನಡ "),
                (Some("BRH Kannada"), "PÀ")
            ],
        ]
    );
}

#[test]
fn fonts_come_from_the_paragraph_style_its_base_and_the_character_style() {
    let doc = Doc {
        fonts: fonts(),
        styles: vec![
            normal(Vec::new()),
            Style {
                kind: 1,
                base: 2,
                grpprl: Vec::new(),
            },
            normal(font(1)),
            Style {
                kind: 2,
                base: 0x0FFF,
                grpprl: font(0),
            },
        ],
        paragraph_style: 1,
        default_fonts: (0, 0),
        pieces: vec![Piece::new(
            true,
            vec![(Vec::new(), "PÀ"), (char_style(3), " Office\r")],
        )],
        ..Doc::default()
    };
    let read = read(&doc.bytes()).unwrap();
    assert_eq!(
        runs(&read, 0),
        [vec![
            (Some("Nudi 01 e"), "PÀ"),
            (Some("Times New Roman"), " Office")
        ]]
    );
}

#[test]
fn the_stylesheet_defaults_apply_when_no_style_sets_a_font() {
    let doc = Doc {
        fonts: fonts(),
        styles: vec![normal(Vec::new())],
        default_fonts: (1, 1),
        pieces: vec![Piece::new(true, vec![(Vec::new(), "PÀ\r")])],
        ..Doc::default()
    };
    assert_eq!(
        runs(&read(&doc.bytes()).unwrap(), 0),
        [vec![(Some("Nudi 01 e"), "PÀ")]]
    );
}

#[test]
fn the_ascii_and_other_slots_are_chosen_by_character() {
    let mixed = [
        super::build::FONT_ASCII,
        [1, 0],
        super::build::FONT_OTHER,
        [0, 0],
    ]
    .concat();
    let doc = Doc {
        fonts: fonts(),
        styles: vec![normal(Vec::new())],
        pieces: vec![Piece::new(true, vec![(mixed, "PÀ\r")])],
        ..Doc::default()
    };
    assert_eq!(
        runs(&read(&doc.bytes()).unwrap(), 0),
        [vec![
            (Some("Nudi 01 e"), "P"),
            (Some("Times New Roman"), "À")
        ]]
    );
}

#[test]
fn a_piece_applies_its_own_properties_last() {
    let mut piece = Piece::new(true, vec![(font(0), "PÀ\r")]);
    piece.grpprl = Some(font(1));
    let doc = Doc {
        fonts: fonts(),
        styles: vec![normal(Vec::new())],
        pieces: vec![piece],
        ..Doc::default()
    };
    assert_eq!(
        runs(&read(&doc.bytes()).unwrap(), 0),
        [vec![(Some("Nudi 01 e"), "PÀ")]]
    );
}

#[test]
fn field_instructions_are_hidden_and_results_kept() {
    let doc = Doc {
        fonts: fonts(),
        styles: vec![normal(font(0))],
        pieces: vec![Piece::new(
            true,
            vec![(
                Vec::new(),
                "a\u{13} PAGE \u{14}1\u{15}b\u{13}HYPERLINK \u{13}x\u{14}y\u{15}\u{14}c\u{15}\r\u{13}open\rd\r",
            )],
        )],
        ..Doc::default()
    };
    let read = read(&doc.bytes()).unwrap();
    let text: Vec<String> = read.stories[0]
        .paragraphs
        .iter()
        .map(|p| p.iter().map(|r| r.text.as_str()).collect())
        .collect();
    assert_eq!(text, ["a1bc", "", "d"]);
}

#[test]
fn tracked_deletions_are_left_out() {
    let doc = Doc {
        fonts: fonts(),
        styles: vec![normal(font(1))],
        pieces: vec![Piece::new(
            true,
            vec![
                (Vec::new(), "PÀ"),
                (vec![0x00, 0x08, 0x01], "old "),
                (Vec::new(), "PÀ\r"),
            ],
        )],
        ..Doc::default()
    };
    assert_eq!(
        runs(&read(&doc.bytes()).unwrap(), 0),
        [vec![(Some("Nudi 01 e"), "PÀPÀ")]]
    );
}

fn texts(doc: &Document, story: usize) -> Vec<String> {
    doc.stories[story]
        .paragraphs
        .iter()
        .map(|p| p.iter().map(|r| r.text.as_str()).collect())
        .collect()
}

#[test]
fn row_end_marks_add_no_line_in_either_form() {
    // An inner table's row ends with a paragraph mark and sprmPFInnerTtp; with
    // sprmPFInTable its properties take an even length, the long PapxInFkp form.
    let doc = Doc {
        fonts: fonts(),
        styles: vec![normal(font(0))],
        pieces: vec![Piece::new(
            true,
            vec![(Vec::new(), "a\rb\rc\u{07}\u{07}d\r")],
        )],
        paragraphs: vec![
            Vec::new(),
            vec![0x16, 0x24, 0x01, 0x4C, 0x24, 0x01],
            Vec::new(),
            vec![0x17, 0x24, 0x01],
        ],
        ..Doc::default()
    };
    assert_eq!(texts(&read(&doc.bytes()).unwrap(), 0), ["a", "bc", "d"]);
}

#[test]
fn styles_read_the_same_with_the_longer_stdf_of_word_2000() {
    let doc = Doc {
        fonts: fonts(),
        styles: vec![
            normal(font(1)),
            Style {
                kind: 2,
                base: 0x0FFF,
                grpprl: font(0),
            },
        ],
        std_base_size: Some(18),
        pieces: vec![Piece::new(
            true,
            vec![(Vec::new(), "PÀ"), (char_style(1), "A\r")],
        )],
        ..Doc::default()
    };
    assert_eq!(
        runs(&read(&doc.bytes()).unwrap(), 0),
        [vec![
            (Some("Nudi 01 e"), "PÀ"),
            (Some("Times New Roman"), "A")
        ]]
    );
}

#[test]
fn header_parts_are_split_by_the_plcfhdd_and_repeats_dropped() {
    let doc = Doc {
        fonts: fonts(),
        styles: vec![normal(font(1))],
        pieces: vec![Piece::new(true, vec![(Vec::new(), "PÀ\rA\rB\rA\rC\rZ\r")])],
        ccp: [3, 0, 8, 0, 0, 2, 0],
        // The last part's end lies past the story, and stops at it.
        hdd: vec![0, 2, 4, 6, 99],
        ..Doc::default()
    };
    let read = read(&doc.bytes()).unwrap();
    assert_eq!(read.stories[1].name, "headers and footers");
    assert_eq!(texts(&read, 1), ["A", "B", "C"]);
    assert_eq!(texts(&read, 2), ["Z"]);
}

#[test]
fn an_inserted_symbol_reads_as_its_character_in_its_font() {
    let symbol = vec![0x09, 0x6A, 0x02, 0x00, 0x50, 0xF0];
    let doc = Doc {
        fonts: fonts(),
        styles: vec![normal(font(0))],
        pieces: vec![Piece::new(
            true,
            vec![(Vec::new(), "a"), (symbol, "("), (Vec::new(), "\r")],
        )],
        ..Doc::default()
    };
    assert_eq!(
        runs(&read(&doc.bytes()).unwrap(), 0),
        [vec![
            (Some("Times New Roman"), "a"),
            (Some("BRH Kannada"), "\u{F050}")
        ]]
    );
}

#[test]
fn more_characters_than_the_text_stream_holds_are_refused() {
    let doc = Doc {
        fonts: fonts(),
        styles: vec![normal(Vec::new())],
        pieces: vec![Piece::new(true, vec![(Vec::new(), "a\r")])],
        ccp: [1 << 20, 0, 0, 0, 0, 0, 0],
        ..Doc::default()
    };
    let err = read(&doc.bytes()).unwrap_err().to_string();
    assert!(err.contains("more than its text can"), "{err}");
}

#[test]
fn a_deleted_paragraph_mark_or_break_joins_the_lines() {
    let deleted = vec![0x00, 0x08, 0x01];
    let doc = Doc {
        fonts: fonts(),
        styles: vec![normal(font(0))],
        pieces: vec![Piece::new(
            true,
            vec![
                (Vec::new(), "a"),
                (deleted.clone(), "\r"),
                (Vec::new(), "b"),
                (deleted, "\u{0B}"),
                (Vec::new(), "c\rd\r"),
            ],
        )],
        ..Doc::default()
    };
    assert_eq!(texts(&read(&doc.bytes()).unwrap(), 0), ["abc", "d"]);
}

#[test]
fn breaks_start_lines_and_marks_and_pictures_are_dropped() {
    let doc = Doc {
        fonts: fonts(),
        styles: vec![normal(font(0))],
        pieces: vec![Piece::new(
            true,
            vec![(Vec::new(), "a\u{0B}b\u{01}\u{1F}\u{1E}c\u{07}d\te\r")],
        )],
        ..Doc::default()
    };
    let read = read(&doc.bytes()).unwrap();
    let text: Vec<String> = read.stories[0]
        .paragraphs
        .iter()
        .map(|p| p.iter().map(|r| r.text.as_str()).collect())
        .collect();
    assert_eq!(text, ["a", "b-c", "d\te"]);
}

#[test]
fn other_stories_follow_the_body_in_order_and_empty_ones_are_left_out() {
    let doc = Doc {
        fonts: fonts(),
        styles: vec![normal(font(1))],
        pieces: vec![Piece::new(true, vec![(Vec::new(), "PÀ\rnote\r\rbox\r")])],
        ccp: [3, 5, 0, 0, 1, 4, 0],
        ..Doc::default()
    };
    let read = read(&doc.bytes()).unwrap();
    let names: Vec<&str> = read.stories.iter().map(|s| s.name).collect();
    assert_eq!(names, ["body", "footnotes", "text boxes"]);
    assert_eq!(runs(&read, 2), [vec![(Some("Nudi 01 e"), "box")]]);
}

#[test]
fn surrogate_pairs_and_lone_surrogates_decode() {
    let units = vec![(0, 0xD83D), (2, 0xDE00), (4, 0xD800), (6, 0x0041)];
    assert_eq!(
        decode(units, false),
        [('\u{1F600}', 0), ('\u{FFFD}', 4), ('A', 6)]
    );
}

#[test]
fn files_that_are_not_word_97_are_refused_with_a_reason() {
    let err = |bytes: &[u8]| read(bytes).unwrap_err().to_string();
    assert!(err(b"plain text").contains("not a Word document"));
    assert!(err(&compound(&[("/Other", b"x")])).contains("WordDocument"));
    let doc = |n_fib, encrypted| Doc {
        fonts: fonts(),
        styles: vec![normal(Vec::new())],
        pieces: vec![Piece::new(true, vec![(Vec::new(), "a\r")])],
        n_fib,
        encrypted,
        ..Doc::default()
    };
    assert!(err(&doc(Some(0x0065), false).bytes()).contains("Word 6 or Word 95"));
    assert!(err(&doc(None, true).bytes()).contains("password"));
    assert!(err(&compound(&[("/WordDocument", &[0xEC, 0xA5])])).contains("too short"));
}

#[test]
fn a_stream_over_the_limit_is_refused() {
    let big = vec![0u8; (MAX_STREAM + 1) as usize];
    let err = read(&compound(&[("/WordDocument", &big)]))
        .unwrap_err()
        .to_string();
    assert!(err.contains("larger than this tool reads (64 MB)"), "{err}");
}

#[test]
fn broken_tables_give_errors_and_never_panic() {
    let good = Doc {
        fonts: fonts(),
        styles: vec![normal(font(1))],
        pieces: vec![Piece::new(true, vec![(Vec::new(), "PÀ\r")])],
        ..Doc::default()
    }
    .bytes();
    for len in [0, 10, 33, 100, 200, 400, 900] {
        let word = vec![0xEC, 0xA5, 0xC1, 0x00]
            .into_iter()
            .chain(std::iter::repeat(0xFF))
            .take(len.max(4))
            .collect::<Vec<u8>>();
        let _ = read(&compound(&[
            ("/WordDocument", &word),
            ("/0Table", &[0xFF; 64]),
        ]));
    }
    assert!(read(&good).is_ok());
}
