//! Kannada text tools.
//!
//! - [`aksharas()`]: split text into aksharas (orthographic syllables).
//! - [`normalize()`]: make equal-looking Kannada text compare equal.
//! - [`modernize()`]: map Old Kannada letters (ೞ, ಱ, ೝ) to modern ones.
//! - [`transliterate()`]: Kannada to and from ISO 15919, IAST and Baraha phonetic.
//! - [`numerals`]: Kannada digits.
//! - [`legacy`]: legacy font text (Nudi, Baraha, Akruti) to and from Unicode.
//! - [`phonetic`]: sound-alike keys for fuzzy search and loose Latin typing.
//!
//! ```
//! use kannadi::{transliterate, Scheme};
//!
//! let latin = transliterate("ಕನ್ನಡ ನಾಡು", Scheme::Kannada, Scheme::Iso15919);
//! assert_eq!(latin, "kannaḍa nāḍu");
//! assert_eq!(transliterate(&latin, Scheme::Iso15919, Scheme::Kannada), "ಕನ್ನಡ ನಾಡು");
//! ```

#![forbid(unsafe_code)]
#![warn(missing_docs)]

pub mod akshara;
pub mod chars;
pub mod legacy;
mod lookup;
pub mod normalize;
pub mod numerals;
pub mod phonetic;
pub mod translit;

pub use akshara::{aksharas, count_aksharas, split, Akshara, AksharaKind, Aksharas};
pub use normalize::{fix_zero_as_anusvara, has_old_kannada, modernize, normalize};
pub use translit::{from_latin, to_latin, transliterate, Scheme, UnknownScheme};

/// Runs the README examples as doctests.
#[cfg(doctest)]
#[doc = include_str!("../../../README.md")]
struct ReadmeDoctests;
