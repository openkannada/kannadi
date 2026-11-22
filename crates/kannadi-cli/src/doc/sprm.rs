//! Property modifiers (Sprm, [MS-DOC] 2.2.5.1): only the fonts and the character
//! style of a run are kept.

use super::bytes::{u16_at, u8_at};

const CHAR_STYLE: u16 = 0x4A30;
const FONT_ASCII: u16 = 0x4A4F;
const FONT_OTHER: u16 = 0x4A51;
const TABLE_DEFINITION: u16 = 0xD608;
const PARAGRAPH_TABS: u16 = 0xC615;

/// Font indexes and character style a list of property modifiers sets.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct CharProps {
    /// For characters below U+0080.
    pub ascii: Option<u16>,
    /// For other characters that are not East Asian.
    pub other: Option<u16>,
    pub style: Option<u16>,
}

impl CharProps {
    /// Reads `grpprl`, stopping at the first modifier that runs past its end.
    pub fn parse(grpprl: &[u8]) -> CharProps {
        let mut props = CharProps::default();
        for (sprm, operand) in sprms(grpprl) {
            let value = u16_at(operand, 0);
            match sprm {
                FONT_ASCII => props.ascii = value.or(props.ascii),
                FONT_OTHER => props.other = value.or(props.other),
                CHAR_STYLE => props.style = value.or(props.style),
                _ => {}
            }
        }
        props
    }

    /// These properties with each one `over` sets replacing them.
    pub fn and(self, over: CharProps) -> CharProps {
        CharProps {
            ascii: over.ascii.or(self.ascii),
            other: over.other.or(self.other),
            style: over.style.or(self.style),
        }
    }

    /// The font a character is drawn in: the ASCII font below U+0080, else the other.
    pub fn font_for(&self, c: char) -> Option<u16> {
        if c.is_ascii() {
            self.ascii
        } else {
            self.other
        }
    }
}

/// Each modifier in `grpprl` with its operand, up to the first that runs past the end.
pub fn sprms(grpprl: &[u8]) -> impl Iterator<Item = (u16, &[u8])> {
    let mut at = 0;
    std::iter::from_fn(move || {
        let sprm = u16_at(grpprl, at)?;
        let rest = grpprl.get(at + 2..)?;
        let size = operand_size(sprm, rest)?;
        let operand = rest.get(..size)?;
        at += 2 + size;
        Some((sprm, operand))
    })
}

/// True if a paragraph's modifiers mark it as the end of a table row (sprmPFTtp or
/// sprmPFInnerTtp), whose mark holds no text.
pub fn ends_row(grpprl: &[u8]) -> bool {
    sprms(grpprl)
        .any(|(sprm, operand)| matches!(sprm, 0x2417 | 0x244C) && operand.first() == Some(&1))
}

/// Bytes in the operand of `sprm`, whose operand starts `operand` ([MS-DOC]
/// 2.2.5.1, the spra field).
fn operand_size(sprm: u16, operand: &[u8]) -> Option<usize> {
    Some(match sprm >> 13 {
        0 | 1 => 1,
        2 | 4 | 5 => 2,
        3 => 4,
        7 => 3,
        _ => match sprm {
            // cb counts the rest of the operand, plus 1.
            TABLE_DEFINITION => 2 + usize::from(u16_at(operand, 0)?).saturating_sub(1),
            PARAGRAPH_TABS => match u8_at(operand, 0)? {
                255 => {
                    let deleted = usize::from(u8_at(operand, 1)?);
                    let added = usize::from(u8_at(operand, 2 + deleted * 4)?);
                    3 + deleted * 4 + added * 3
                }
                cb => 1 + usize::from(cb),
            },
            _ => 1 + usize::from(u8_at(operand, 0)?),
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fonts_and_style_are_read_past_other_modifiers() {
        let grpprl = [
            0x35, 0x08, 0x01, // sprmCFBold, 1 byte
            0x4F, 0x4A, 0x03, 0x00, // sprmCRgFtc0 = 3
            0x08, 0xD6, 0x03, 0x00, 0xAA, 0xBB, // sprmTDefTable, cb 3
            0x15, 0xC6, 0x02, 0x00, 0x00, // sprmPChgTabs, cb 2
            0x51, 0x4A, 0x04, 0x00, // sprmCRgFtc2 = 4
            0x30, 0x4A, 0x0A, 0x00, // sprmCIstd = 10
        ];
        assert_eq!(
            CharProps::parse(&grpprl),
            CharProps {
                ascii: Some(3),
                other: Some(4),
                style: Some(10)
            }
        );
    }

    #[test]
    fn a_table_definition_longer_than_255_bytes_is_stepped_over() {
        let mut grpprl = vec![0x08, 0xD6, 0x01, 0x01];
        grpprl.extend([0u8; 256]);
        grpprl.extend([0x4F, 0x4A, 0x07, 0x00]);
        assert_eq!(CharProps::parse(&grpprl).ascii, Some(7));
    }

    #[test]
    fn a_modifier_cut_short_ends_the_list() {
        assert_eq!(CharProps::parse(&[0x4F, 0x4A, 0x03]), CharProps::default());
        let long_tabs = [
            0x15, 0xC6, 0xFF, 0x01, 0, 0, 0, 0, 0x01, 0, 0, 0, 0x4F, 0x4A, 0x02, 0x00,
        ];
        assert_eq!(CharProps::parse(&long_tabs).ascii, Some(2));
    }

    #[test]
    fn row_ends_are_found_after_other_modifiers() {
        assert!(ends_row(&[0x16, 0x24, 0x01, 0x17, 0x24, 0x01]));
        assert!(ends_row(&[0x4C, 0x24, 0x01]));
        assert!(!ends_row(&[0x17, 0x24, 0x00, 0x16, 0x24, 0x01]));
        assert!(!ends_row(&[0x17, 0x24]));
    }

    #[test]
    fn the_slot_follows_the_character() {
        let p = CharProps {
            ascii: Some(1),
            other: Some(2),
            style: None,
        };
        assert_eq!(p.font_for('P'), Some(1));
        assert_eq!(p.font_for('À'), Some(2));
        let q = p.and(CharProps {
            ascii: Some(5),
            ..CharProps::default()
        });
        assert_eq!((q.ascii, q.other), (Some(5), Some(2)));
    }
}
