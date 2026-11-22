//! Text read from a Word or RTF file: stories of paragraphs, each a list of runs
//! with the font they are set in. Legacy runs hold their glyph codes as the
//! Windows-1252 characters of those codes.

/// A font from the document's font table.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Font {
    pub name: String,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Hash)]
pub struct Run {
    /// Index into [`Document::fonts`], or `None` when the file names no font.
    pub font: Option<usize>,
    pub text: String,
}

/// A part of the document with its own text, such as the body or the footnotes.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Story {
    pub name: &'static str,
    pub paragraphs: Vec<Vec<Run>>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Document {
    pub fonts: Vec<Font>,
    pub stories: Vec<Story>,
}

impl Story {
    pub fn new(name: &'static str) -> Self {
        Story {
            name,
            paragraphs: vec![Vec::new()],
        }
    }

    /// Adds `c` to the open paragraph, joining the last run when the font is the same.
    pub fn push(&mut self, font: Option<usize>, c: char) {
        let paragraph = self.paragraphs.last_mut().expect("a story has a paragraph");
        match paragraph.last_mut() {
            Some(run) if run.font == font => run.text.push(c),
            _ => paragraph.push(Run {
                font,
                text: c.to_string(),
            }),
        }
    }

    pub fn end_paragraph(&mut self) {
        self.paragraphs.push(Vec::new());
    }

    /// True when no paragraph holds text.
    pub fn is_empty(&self) -> bool {
        self.paragraphs
            .iter()
            .all(|p| p.iter().all(|r| r.text.is_empty()))
    }

    /// Drops the empty paragraphs at the end, such as the one left open after the
    /// last paragraph mark.
    pub fn finish(&mut self) {
        while self.paragraphs.len() > 1 && self.paragraphs.last().is_some_and(Vec::is_empty) {
            self.paragraphs.pop();
        }
    }
}

/// A symbol font's character at U+F020 to U+F0FF, as Word stores it, back to the
/// Windows-1252 character of its code; any other character unchanged.
pub fn glyph_code(c: char) -> char {
    match u32::from(c) {
        code @ 0xF020..=0xF0FF => {
            let byte = (code - 0xF000) as u8;
            kannadi::legacy::windows_1252_char(byte)
        }
        _ => c,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn runs_join_by_font_and_paragraphs_close() {
        let mut story = Story::new("body");
        for (font, c) in [(Some(0), 'a'), (Some(0), 'b'), (None, 'c')] {
            story.push(font, c);
        }
        story.end_paragraph();
        story.end_paragraph();
        story.finish();
        assert_eq!(story.paragraphs.len(), 1);
        assert_eq!(story.paragraphs[0].len(), 2);
        assert_eq!(story.paragraphs[0][0].text, "ab");
        assert!(!story.is_empty());
    }

    #[test]
    fn symbol_font_codes_read_as_their_byte() {
        assert_eq!(glyph_code('\u{F050}'), 'P');
        assert_eq!(glyph_code('\u{F0C0}'), 'À');
        assert_eq!(glyph_code('\u{F092}'), '\u{2019}');
        assert_eq!(glyph_code('\u{0C95}'), '\u{0C95}');
        assert_eq!(glyph_code('\u{F01F}'), '\u{F01F}');
    }
}
