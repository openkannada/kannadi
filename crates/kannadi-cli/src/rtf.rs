//! RTF files, read for their text and each run's font. Bytes written as `\'hh` and
//! 8-bit text are read as Windows-1252, which keeps legacy glyph codes as they are;
//! `\uN` gives a Unicode character, and the `\uc` fallback after it is skipped.
//! Destinations that hold no document text are skipped, as are unknown ones marked
//! with `\*`. Footnotes, headers and footers, comments and text boxes become
//! stories of their own.

use std::collections::HashMap;

use anyhow::{bail, Result};
use kannadi::legacy::windows_1252_char;

use crate::flow::{Document, Font, Story};

/// Groups nested deeper than this are skipped.
const MAX_DEPTH: usize = 512;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Token<'a> {
    Open,
    Close,
    Word(&'a [u8], Option<i32>),
    Symbol(u8),
    /// A byte written as `\'hh`.
    Hex(u8),
    Text(u8),
}

struct Lexer<'a> {
    bytes: &'a [u8],
    at: usize,
}

impl<'a> Iterator for Lexer<'a> {
    type Item = Token<'a>;

    fn next(&mut self) -> Option<Token<'a>> {
        loop {
            let b = *self.bytes.get(self.at)?;
            self.at += 1;
            return Some(match b {
                b'{' => Token::Open,
                b'}' => Token::Close,
                b'\\' => match self.control() {
                    Some(token) => token,
                    None => continue,
                },
                b'\r' | b'\n' | 0 => continue,
                _ => Token::Text(b),
            });
        }
    }
}

impl<'a> Lexer<'a> {
    /// The control word or symbol after a backslash; `None` after `\bin` and its
    /// data, or at the end.
    fn control(&mut self) -> Option<Token<'a>> {
        let b = *self.bytes.get(self.at)?;
        if b == b'\'' {
            let hex = self.bytes.get(self.at + 1..self.at + 3);
            let value = hex
                .and_then(|h| std::str::from_utf8(h).ok())
                .and_then(|h| u8::from_str_radix(h, 16).ok());
            self.at += hex.map_or(1, |_| 3);
            return Some(value.map_or(Token::Symbol(b'\''), Token::Hex));
        }
        if !b.is_ascii_alphabetic() {
            self.at += 1;
            return Some(Token::Symbol(b));
        }
        let start = self.at;
        while self.bytes.get(self.at).is_some_and(u8::is_ascii_alphabetic) && self.at - start < 32 {
            self.at += 1;
        }
        let word = &self.bytes[start..self.at];
        let digits_start = self.at;
        if self.bytes.get(self.at) == Some(&b'-') {
            self.at += 1;
        }
        while self.bytes.get(self.at).is_some_and(u8::is_ascii_digit) {
            self.at += 1;
        }
        let param = std::str::from_utf8(&self.bytes[digits_start..self.at])
            .ok()
            .and_then(|d| d.parse::<i64>().ok())
            .map(|n| n.clamp(i64::from(i32::MIN), i64::from(i32::MAX)) as i32);
        if self.bytes.get(self.at) == Some(&b' ') {
            self.at += 1;
        }
        if word == b"bin" {
            // Binary data: its length is the parameter.
            let len = param.unwrap_or(0).max(0) as usize;
            self.at = self.at.saturating_add(len).min(self.bytes.len());
            return None;
        }
        Some(Token::Word(word, param))
    }
}

/// Where a group's text goes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Dest {
    Text(usize),
    FontTable,
    /// `\upr`: only its `\*\ud` group, the Unicode copy, is read, into the story
    /// given or else the font table.
    Upr(Option<usize>),
    Skip,
}

#[derive(Clone, Copy, Debug)]
struct State {
    dest: Dest,
    /// The font number set with `\f`, or `None` for the default font.
    font: Option<i32>,
    /// Characters to skip after `\uN`.
    uc: u32,
    /// Inside a tracked deletion (`\deleted`), which is not part of the text.
    deleted: bool,
}

const STORIES: [&str; 5] = [
    "body",
    "footnotes",
    "headers and footers",
    "comments",
    "text boxes",
];

/// Destinations that hold no document text.
const SKIPPED: &[&str] = &[
    "colortbl",
    "stylesheet",
    "info",
    "pict",
    "listtable",
    "listoverridetable",
    "revtbl",
    "rsidtbl",
    "xmlnstbl",
    "themedata",
    "colorschememapping",
    "latentstyles",
    "datastore",
    "generator",
    "fldinst",
    "sp",
    "shprslt",
    "nonshppict",
    "filetbl",
    "pgdsctbl",
    "userprops",
    "docvar",
    "template",
    "mmathPr",
    "pntxta",
    "pntxtb",
    // List numbers and bullets, which a .doc file does not hold as text either.
    "listtext",
    "pntext",
    "falt",
    "panose",
    "objdata",
    "objclass",
    "bkmkstart",
    "bkmkend",
];

struct Parser {
    stories: Vec<Story>,
    fonts: Vec<Font>,
    /// Font number to index in `fonts`.
    numbers: HashMap<i32, usize>,
    default_font: Option<i32>,
    /// The font table entry being read, and whether its name has ended.
    entry: Option<(usize, bool)>,
    stack: Vec<State>,
    /// Groups opened past [`MAX_DEPTH`].
    too_deep: usize,
    state: State,
    /// A group has just opened, and `\*` may have marked it.
    fresh: bool,
    starred: bool,
    /// Where a `\*\ud` group opening now reads to: the `\upr` it is in.
    ud: Option<Dest>,
    skip: u32,
    /// A UTF-16 high surrogate from `\uN`, waiting for its low half.
    high: Option<u16>,
}

/// Reads the stories of an RTF file.
pub fn read(bytes: &[u8]) -> Result<Document> {
    if !bytes.starts_with(b"{\\rtf") {
        bail!("not an RTF file");
    }
    let mut p = Parser {
        stories: STORIES.iter().map(|&name| Story::new(name)).collect(),
        fonts: Vec::new(),
        numbers: HashMap::new(),
        default_font: None,
        entry: None,
        stack: Vec::new(),
        too_deep: 0,
        state: State {
            dest: Dest::Text(0),
            font: None,
            uc: 1,
            deleted: false,
        },
        fresh: false,
        starred: false,
        ud: None,
        skip: 0,
        high: None,
    };
    for token in (Lexer { bytes, at: 0 }) {
        p.token(token);
    }
    let mut stories = std::mem::take(&mut p.stories);
    for story in &mut stories {
        story.finish();
    }
    stories.retain(|s| !s.is_empty());
    Ok(Document {
        fonts: p.fonts,
        stories,
    })
}

impl Parser {
    fn token(&mut self, token: Token) {
        match token {
            Token::Open => return self.open(),
            Token::Close => return self.close(),
            _ => {}
        }
        if self.too_deep > 0 {
            return;
        }
        if self.skip > 0 {
            self.skip -= 1;
            return;
        }
        let fresh = std::mem::take(&mut self.fresh);
        if fresh && token == Token::Symbol(b'*') {
            self.fresh = true;
            self.starred = true;
            return;
        }
        let ud = self.ud.take();
        match token {
            Token::Word(word, param) => {
                let word = std::str::from_utf8(word).unwrap_or_default();
                if fresh && self.destination(word, ud) {
                    return;
                }
                self.word(word, param);
            }
            Token::Symbol(s) => self.symbol(s),
            Token::Hex(b) | Token::Text(b) => self.char(windows_1252_char(b)),
            Token::Open | Token::Close => {}
        }
    }

    fn open(&mut self) {
        self.skip = 0;
        self.lone_surrogate();
        if self.too_deep > 0 || self.stack.len() >= MAX_DEPTH {
            self.too_deep += 1;
            return;
        }
        self.stack.push(self.state);
        self.fresh = true;
        self.starred = false;
        self.ud = None;
        if let Dest::Upr(inside) = self.state.dest {
            self.ud = Some(inside.map_or(Dest::FontTable, Dest::Text));
            self.state.dest = Dest::Skip;
        }
    }

    fn close(&mut self) {
        self.skip = 0;
        self.lone_surrogate();
        self.fresh = false;
        if self.too_deep > 0 {
            self.too_deep -= 1;
            return;
        }
        let Some(parent) = self.stack.pop() else {
            return;
        };
        let outer = match parent.dest {
            Dest::Text(story) | Dest::Upr(Some(story)) => Some(story),
            _ => None,
        };
        if let Dest::Text(inner) = self.state.dest {
            let story = &mut self.stories[inner];
            if outer != Some(inner) && story.paragraphs.last().is_some_and(|p| !p.is_empty()) {
                story.end_paragraph();
            }
        }
        self.state = parent;
    }

    /// Sets the group's destination from its first control word; true if `word`
    /// named one. `ud` is where a `\ud` group reads to, inside `\upr`.
    fn destination(&mut self, word: &str, ud: Option<Dest>) -> bool {
        let starred = std::mem::take(&mut self.starred);
        let story = match self.state.dest {
            Dest::Text(story) => Some(story),
            _ => None,
        };
        let dest = match (word, story, ud) {
            ("ud", _, Some(dest)) => dest,
            _ if self.state.dest == Dest::Skip => return false,
            ("fonttbl", ..) => {
                self.entry = None;
                Dest::FontTable
            }
            ("upr", ..) if matches!(self.state.dest, Dest::Text(_) | Dest::FontTable) => {
                Dest::Upr(story)
            }
            ("footnote", Some(_), _) => Dest::Text(1),
            (
                "header" | "headerl" | "headerr" | "headerf" | "footer" | "footerl" | "footerr"
                | "footerf",
                Some(_),
                _,
            ) => Dest::Text(2),
            ("annotation", Some(_), _) => Dest::Text(3),
            ("shptxt", Some(_), _) => Dest::Text(4),
            ("shpinst", Some(story), _) => Dest::Text(story),
            _ if SKIPPED.contains(&word) || starred => Dest::Skip,
            _ => return false,
        };
        self.state.dest = dest;
        true
    }

    fn word(&mut self, word: &str, param: Option<i32>) {
        match (self.state.dest, word) {
            (_, "deff") => self.default_font = param,
            (_, "uc") => self.state.uc = param.unwrap_or(1).clamp(0, 16) as u32,
            (Dest::FontTable, "f") => {
                let number = param.unwrap_or(0);
                let index = self.fonts.len();
                self.fonts.push(Font::default());
                self.numbers.insert(number, index);
                self.entry = Some((index, false));
            }
            (Dest::Text(_), "f") => self.state.font = param,
            (Dest::Text(_), "plain") => {
                self.state.font = None;
                self.state.deleted = false;
            }
            (Dest::Text(_), "deleted") => self.state.deleted = param != Some(0),
            (_, "u") => {
                // A negative N is a UTF-16 unit written as a signed 16-bit number.
                if let Some(unit) =
                    param.and_then(|n| u16::try_from(if n < 0 { n + 65536 } else { n }).ok())
                {
                    self.unit(unit);
                }
                if param.is_some() {
                    self.skip = self.state.uc;
                }
            }
            (Dest::Text(_), "par" | "sect" | "line" | "page" | "cell" | "nestcell") => {
                self.end_paragraph()
            }
            (Dest::Text(_), _) => {
                let c = match word {
                    "tab" => '\t',
                    "emdash" => '\u{2014}',
                    "endash" => '\u{2013}',
                    "bullet" => '\u{2022}',
                    "lquote" => '\u{2018}',
                    "rquote" => '\u{2019}',
                    "ldblquote" => '\u{201C}',
                    "rdblquote" => '\u{201D}',
                    "emspace" => '\u{2003}',
                    "enspace" => '\u{2002}',
                    "qmspace" => '\u{2005}',
                    "zwj" => '\u{200D}',
                    "zwnj" => '\u{200C}',
                    _ => return,
                };
                self.char(c);
            }
            _ => {}
        }
    }

    fn symbol(&mut self, s: u8) {
        match s {
            b'\\' | b'{' | b'}' => self.char(char::from(s)),
            b'~' => self.char('\u{A0}'),
            // A non-breaking hyphen.
            b'_' => self.char('-'),
            b'\r' | b'\n' => {
                if let Dest::Text(_) = self.state.dest {
                    self.end_paragraph();
                }
            }
            _ => {}
        }
    }

    /// 1 UTF-16 unit from `\uN`; a surrogate pair gives 1 character.
    fn unit(&mut self, unit: u16) {
        match (self.high.take(), unit) {
            (Some(high), 0xDC00..=0xDFFF) => {
                let code =
                    0x10000 + ((u32::from(high) - 0xD800) << 10) + (u32::from(unit) - 0xDC00);
                self.char(char::from_u32(code).unwrap_or('\u{FFFD}'));
            }
            (high, 0xD800..=0xDBFF) => {
                if high.is_some() {
                    self.char('\u{FFFD}');
                }
                self.high = Some(unit);
            }
            (high, _) => {
                if high.is_some() {
                    self.char('\u{FFFD}');
                }
                self.char(char::from_u32(unit.into()).unwrap_or('\u{FFFD}'));
            }
        }
    }

    /// Writes a high surrogate left without its low half as U+FFFD.
    fn lone_surrogate(&mut self) {
        if self.high.take().is_some() {
            self.char('\u{FFFD}');
        }
    }

    fn char(&mut self, c: char) {
        self.lone_surrogate();
        match self.state.dest {
            Dest::Text(story) => {
                if c == '\r' || c == '\n' || self.state.deleted {
                    return;
                }
                let number = self.state.font.or(self.default_font);
                let font = number.and_then(|n| self.numbers.get(&n).copied());
                self.stories[story].push(font, c);
            }
            Dest::FontTable => {
                if let Some((index, ended)) = &mut self.entry {
                    if c == ';' {
                        *ended = true;
                    } else if !*ended {
                        self.fonts[*index].name.push(c);
                    }
                }
            }
            Dest::Upr(_) | Dest::Skip => {}
        }
    }

    fn end_paragraph(&mut self) {
        if let Dest::Text(story) = self.state.dest {
            self.stories[story].end_paragraph();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::flow::Run;

    fn runs(doc: &Document, story: usize) -> Vec<Vec<(Option<&str>, &str)>> {
        doc.stories[story]
            .paragraphs
            .iter()
            .map(|p| {
                p.iter()
                    .map(|Run { font, text }| {
                        let name = font.and_then(|f| doc.fonts.get(f)).map(|f| f.name.trim());
                        (name, text.as_str())
                    })
                    .collect()
            })
            .collect()
    }

    const HEAD: &str =
        "{\\rtf1\\ansi\\ansicpg1252\\deff0{\\fonttbl{\\f0\\froman\\fcharset0 Times New Roman;}\
        {\\f1\\fnil\\fcharset0 Nudi 01 e{\\*\\panose 02020603050405020304};}\
        {\\f2\\fnil\\fcharset2 BRH Kannada;}}";

    fn rtf(body: &str) -> Document {
        read(format!("{HEAD}{body}}}").as_bytes()).unwrap()
    }

    #[test]
    fn fonts_switch_runs_and_hex_bytes_keep_their_codes() {
        let doc = rtf("\\pard\\plain\\f1 P\\'c0 {\\f0 Office}\\par\\f2\\u-4016\\'50\\'c0\\par");
        assert_eq!(doc.fonts.len(), 3);
        assert_eq!(
            runs(&doc, 0),
            [
                vec![
                    (Some("Nudi 01 e"), "PÀ "),
                    (Some("Times New Roman"), "Office")
                ],
                vec![(Some("BRH Kannada"), "\u{F050}À")],
            ]
        );
    }

    #[test]
    fn unicode_escapes_skip_their_fallback_by_uc() {
        let doc = rtf("\\uc2\\u3221\\'3f\\'3f\\u3240?? {\\uc0\\u3221}a\\par");
        assert_eq!(runs(&doc, 0), [vec![(Some("Times New Roman"), "ಕನ ಕa")]]);
    }

    #[test]
    fn plain_returns_to_the_default_font() {
        let doc = rtf("\\f1 P\\plain C\\par");
        assert_eq!(
            runs(&doc, 0),
            [vec![
                (Some("Nudi 01 e"), "P"),
                (Some("Times New Roman"), "C")
            ]]
        );
    }

    #[test]
    fn skipped_destinations_and_fields_show_only_text() {
        let doc = rtf(
            "{\\colortbl;\\red0;}{\\*\\generator Writer;}{\\stylesheet{\\s0 Normal;}}\
             {\\info{\\title T}}{\\*\\unknown x}{\\pict\\bin3 }}}abc}\
             a{\\field{\\*\\fldinst HYPERLINK x}{\\fldrslt b}}c\\~d\\_e\\-f\\\\\\{\\}\\par",
        );
        assert_eq!(
            runs(&doc, 0),
            [vec![(Some("Times New Roman"), "abc\u{A0}d-ef\\{}")]]
        );
    }

    #[test]
    fn notes_headers_and_text_boxes_are_their_own_stories() {
        let doc = rtf("{\\header \\pard H\\par}Body{\\footnote \\pard Note}text\
             {\\shp{\\*\\shpinst{\\sp{\\sn x}{\\sv 1}}{\\shptxt Box\\par}}{\\shprslt Box}}\\par");
        let names: Vec<&str> = doc.stories.iter().map(|s| s.name).collect();
        assert_eq!(
            names,
            ["body", "footnotes", "headers and footers", "text boxes"]
        );
        let text = |s: usize| -> Vec<String> {
            doc.stories[s]
                .paragraphs
                .iter()
                .map(|p| p.iter().map(|r| r.text.as_str()).collect())
                .collect()
        };
        assert_eq!(text(0), ["Bodytext"]);
        assert_eq!(text(1), ["Note"]);
        assert_eq!(text(2), ["H"]);
        assert_eq!(text(3), ["Box"]);
    }

    #[test]
    fn upr_reads_its_unicode_copy() {
        let doc = rtf("{\\upr{a}{\\*\\ud{\\u3221?}}}\\line b\\par");
        assert_eq!(
            runs(&doc, 0),
            [
                vec![(Some("Times New Roman"), "ಕ")],
                vec![(Some("Times New Roman"), "b")]
            ]
        );
    }

    #[test]
    fn font_names_may_be_escaped_and_tables_need_no_inner_groups() {
        let doc = read(b"{\\rtf1{\\fonttbl\\f0 Nudi\\'20Akshara;\\f1 Arial;}\\f0 P\\f1 A\\f9 x}")
            .unwrap();
        assert_eq!(
            runs(&doc, 0),
            [vec![
                (Some("Nudi Akshara"), "P"),
                (Some("Arial"), "A"),
                (None, "x")
            ]]
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
    fn each_footnote_is_its_own_paragraph() {
        let doc = rtf("a{\\footnote N1}b{\\footnote N2}c\\\npar d\\par");
        assert_eq!(texts(&doc, 0), ["abc", "par d"]);
        assert_eq!(texts(&doc, 1), ["N1", "N2"]);
    }

    #[test]
    fn a_group_ends_the_unicode_fallback() {
        let doc = rtf("\\u3221{a}b\\u3221 c\\par");
        assert_eq!(texts(&doc, 0), ["ಕabಕ"]);
    }

    #[test]
    fn surrogate_pairs_join_and_lone_halves_become_replacement_characters() {
        let doc = rtf("\\u-10179?\\u-8704?x\\u-10179?y\\u-8704?\\u x\\par");
        assert_eq!(texts(&doc, 0), ["\u{1F600}x\u{FFFD}y\u{FFFD}x"]);
    }

    #[test]
    fn tracked_deletions_and_list_numbers_are_left_out() {
        let doc = rtf(
            "{\\listtext 1.\\tab}a{\\deleted b}c\\deleted d\\deleted0 e\\deleted f\\plain g\\par",
        );
        assert_eq!(texts(&doc, 0), ["aceg"]);
    }

    #[test]
    fn broken_input_never_panics() {
        for input in [
            "{\\rtf",
            "{\\rtf1\\'",
            "{\\rtf1\\'4",
            "{\\rtf1\\u",
            "{\\rtf1\\u99999999999 x}",
            "{\\rtf1\\bin-5 x}",
            "{\\rtf1\\bin99999 x}",
            "{\\rtf1}}}}}{{{",
            "{\\rtf1\\uc99999\\u3221 abc}",
            "{\\rtf1{\\fonttbl;;\\f}\\f-2147483648 x}",
        ] {
            let _ = read(input.as_bytes());
        }
        let deep = format!("{{\\rtf1{}x{}y}}", "{".repeat(5000), "}".repeat(5000));
        let doc = read(deep.as_bytes()).unwrap();
        assert_eq!(runs(&doc, 0), [vec![(None, "y")]]);
        assert!(read(b"plain").is_err());
    }
}
