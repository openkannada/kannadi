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
    Ok(())
}
