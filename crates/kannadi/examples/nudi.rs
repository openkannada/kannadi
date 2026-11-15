//! Converts a Nudi or Baraha file to Unicode: `cargo run --example nudi -- file.txt`.

use kannadi::legacy::{decode_windows_1252, nudi_to_unicode, Options};

fn main() -> std::io::Result<()> {
    let path = std::env::args().nth(1).expect("usage: nudi <file>");
    let text = decode_windows_1252(&std::fs::read(path)?);
    let converted = nudi_to_unicode(&text, &Options::default());
    println!("{}", converted.text);
    eprintln!(
        "unknown glyphs: {}, malformed syllables: {}",
        converted.unknown.len(),
        converted.malformed.len()
    );
    for u in converted.unknown.iter().take(20) {
        eprintln!("  U+{:04X} at byte {}", u.glyph as u32, u.offset);
    }
    for range in converted.malformed.iter().take(20) {
        eprintln!(
            "  malformed \"{}\" in \"{}\"",
            &converted.text[range.clone()],
            context(&converted.text, range.clone())
        );
    }
    Ok(())
}

/// Up to 8 characters on each side of `range`.
fn context(text: &str, range: std::ops::Range<usize>) -> &str {
    let start = text[..range.start]
        .char_indices()
        .rev()
        .nth(7)
        .map_or(0, |(i, _)| i);
    let end = text[range.end..]
        .char_indices()
        .nth(8)
        .map_or(text.len(), |(i, _)| range.end + i);
    &text[start..end]
}
