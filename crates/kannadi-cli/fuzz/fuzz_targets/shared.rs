// The modules a .doc or .rtf file passes through, from the kannadi binary.

#[path = "../../src/convert.rs"]
pub mod convert;
#[path = "../../src/doc/mod.rs"]
pub mod doc;
#[path = "../../src/flow.rs"]
pub mod flow;
#[path = "../../src/flow_writer.rs"]
pub mod flow_writer;
#[path = "../../src/fonts.rs"]
pub mod fonts;
#[path = "../../src/report.rs"]
pub mod report;
#[path = "../../src/rtf.rs"]
pub mod rtf;

use convert::{Direction, Settings};
use kannadi::legacy::Options;

/// Writes `document` as text, which must never fail for a document that was read.
pub fn write(document: &flow::Document) {
    let settings = Settings::new(Direction::ToUnicode(Options::default()));
    flow_writer::write(document, &settings, "fuzz").expect("writing a read document");
}
