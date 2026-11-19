//! Normalization and modernization, following the Kannada text normalization profile
//! (`NORMALIZATION.md` in the repository).
//!
//! The profile has 3 forms, each a superset of the one before:
//!
//! - [`Form::Canonical`]: 1 spelling for text that renders the same, or that Unicode
//!   allows only 1 way. Safe for storage. [`normalize`] gives this form.
//! - [`Form::Legacy`]: also repairs mistakes that legacy font converters and OCR make,
//!   such as a zero typed for ಂ. For converted or scanned text.
//! - [`Form::Search`]: also folds differences a reader ignores when searching: Old
//!   Kannada letters, joiners and the nukta. For search keys and comparisons only.

use unicode_normalization::char::canonical_combining_class;
use unicode_normalization::UnicodeNormalization;

use crate::chars::{
    class, is_consonant, is_kannada, is_vowel_sign, Class, ANUSVARA, LLLA, NAKAARA_POLLU, NUKTA,
    RRA, VIRAMA, ZWJ, ZWNJ,
};

/// RA (ರ), U+0CB0.
const RA: char = '\u{0CB0}';
/// KANNADA ARCHAIC SHRII, U+0CDC (Unicode 17.0).
const ARCHAIC_SHRII: char = '\u{0CDC}';
/// KANNADA SIGN COMBINING ANUSVARA ABOVE RIGHT, U+0CF3 (Unicode 15.0).
const ANUSVARA_ABOVE_RIGHT: char = '\u{0CF3}';
/// Format characters that [`Form::Search`] drops next to Kannada: ZWSP, soft hyphen,
/// word joiner and BOM.
const FORMAT: [char; 4] = ['\u{200B}', '\u{00AD}', '\u{2060}', '\u{FEFF}'];

/// How far normalization goes. Each form includes the ones before it.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Form {
    /// 1 spelling for text that renders the same, or that Unicode allows only 1 way.
    #[default]
    Canonical,
    /// Canonical, plus repairs for mistakes of legacy font converters and OCR.
    Legacy,
    /// Legacy, plus folding for search: Old Kannada letters, joiners and the nukta.
    Search,
}

impl Form {
    /// All forms, from the least to the most change.
    pub const ALL: [Form; 3] = [Form::Canonical, Form::Legacy, Form::Search];

    /// The form's name: `canonical`, `legacy` or `search`.
    pub const fn name(self) -> &'static str {
        match self {
            Form::Canonical => "canonical",
            Form::Legacy => "legacy",
            Form::Search => "search",
        }
    }

    /// The form with this name, ignoring case.
    pub fn from_name(name: &str) -> Option<Form> {
        Form::ALL
            .into_iter()
            .find(|f| f.name().eq_ignore_ascii_case(name))
    }
}

/// A rule of the profile. [`Rule::id`] is its identifier in `NORMALIZATION.md`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Rule {
    /// C1: Unicode NFC.
    Nfc,
    /// C2: ಉ + ಾ, ಒ + ೌ and ಋ + ಾ become ಊ, ಔ and ೠ (Unicode Table 12-32).
    VowelLetter,
    /// C3: the nukta moves to directly after its consonant.
    NuktaOrder,
    /// C4: a mark repeated twice in a row is kept once.
    RepeatedMark,
    /// C5: a joiner between a consonant and its virama is removed; after ರ it is ZWJ.
    JoinerBeforeVirama,
    /// C6: ರ + virama + ZWJ before a consonant becomes ರ + ZWJ + virama.
    NonRephRa,
    /// C7: joiners between a virama and a consonant become 1 ZWNJ.
    ExplicitVirama,
    /// C8: any other joiner next to Kannada is removed, unless a sign follows a consonant.
    StrayJoiner,
    /// L1: ೦ after a letter or sign, and not before a digit, becomes ಂ.
    ZeroAsAnusvara,
    /// L2: Latin o, O or 0 between a vowel sign and a Kannada letter becomes ಂ.
    LatinAsAnusvara,
    /// L3: consonant + ರ್ + vowel sign becomes ರ್ + consonant + vowel sign.
    MisplacedReph,
    /// S1: ೞ, ಱ, ೝ, ೳ and U+0CDC become ಳ, ರ, ನ್, ಂ and ಶ್ರೀ.
    OldLetter,
    /// S2: ZWNJ, ZWJ, ZWSP, soft hyphen, word joiner and BOM next to Kannada are removed.
    Invisible,
    /// S3: the nukta is removed.
    Nukta,
}

impl Rule {
    /// Every rule, in the order the profile lists them.
    pub const ALL: [Rule; 14] = [
        Rule::Nfc,
        Rule::VowelLetter,
        Rule::NuktaOrder,
        Rule::RepeatedMark,
        Rule::JoinerBeforeVirama,
        Rule::NonRephRa,
        Rule::ExplicitVirama,
        Rule::StrayJoiner,
        Rule::ZeroAsAnusvara,
        Rule::LatinAsAnusvara,
        Rule::MisplacedReph,
        Rule::OldLetter,
        Rule::Invisible,
        Rule::Nukta,
    ];

    /// The rule's identifier in `NORMALIZATION.md`, such as `C5`.
    pub const fn id(self) -> &'static str {
        match self {
            Rule::Nfc => "C1",
            Rule::VowelLetter => "C2",
            Rule::NuktaOrder => "C3",
            Rule::RepeatedMark => "C4",
            Rule::JoinerBeforeVirama => "C5",
            Rule::NonRephRa => "C6",
            Rule::ExplicitVirama => "C7",
            Rule::StrayJoiner => "C8",
            Rule::ZeroAsAnusvara => "L1",
            Rule::LatinAsAnusvara => "L2",
            Rule::MisplacedReph => "L3",
            Rule::OldLetter => "S1",
            Rule::Invisible => "S2",
            Rule::Nukta => "S3",
        }
    }

    /// A short name, such as `explicit-virama`.
    pub const fn name(self) -> &'static str {
        match self {
            Rule::Nfc => "nfc",
            Rule::VowelLetter => "vowel-letter",
            Rule::NuktaOrder => "nukta-order",
            Rule::RepeatedMark => "repeated-mark",
            Rule::JoinerBeforeVirama => "joiner-before-virama",
            Rule::NonRephRa => "non-reph-ra",
            Rule::ExplicitVirama => "explicit-virama",
            Rule::StrayJoiner => "stray-joiner",
            Rule::ZeroAsAnusvara => "zero-as-anusvara",
            Rule::LatinAsAnusvara => "latin-as-anusvara",
            Rule::MisplacedReph => "misplaced-reph",
            Rule::OldLetter => "old-letter",
            Rule::Invisible => "invisible",
            Rule::Nukta => "nukta",
        }
    }

    /// The first form that applies the rule.
    pub const fn form(self) -> Form {
        match self.id().as_bytes()[0] {
            b'C' => Form::Canonical,
            b'L' => Form::Legacy,
            _ => Form::Search,
        }
    }

    const fn index(self) -> usize {
        self as usize
    }
}

/// How many times each rule changed the text.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Counts([usize; Rule::ALL.len()]);

impl Counts {
    /// How many times `rule` changed the text.
    pub fn get(&self, rule: Rule) -> usize {
        self.0[rule.index()]
    }

    /// Every rule with its count, in profile order.
    pub fn iter(&self) -> impl Iterator<Item = (Rule, usize)> + '_ {
        Rule::ALL.into_iter().map(|r| (r, self.get(r)))
    }

    /// The number of changes, all rules together.
    pub fn total(&self) -> usize {
        self.0.iter().sum()
    }

    /// Adds `other`'s counts to these.
    pub fn add(&mut self, other: &Counts) {
        for (a, b) in self.0.iter_mut().zip(other.0) {
            *a += b;
        }
    }

    fn bump(&mut self, rule: Rule) {
        self.0[rule.index()] += 1;
    }
}

/// Normalizes Kannada text to the canonical form of the profile, so that text that
/// renders the same compares equal.
///
/// - Unicode NFC, which joins two-part vowel signs (ಿ + ೕ is ೀ, ೆ + ೂ is ೊ).
/// - The vowel letters Unicode says not to build from parts: ಉ + ಾ is ಊ, ಒ + ೌ is ಔ,
///   ಋ + ಾ is ೠ.
/// - The nukta directly after its consonant; a mark repeated twice in a row once.
/// - Joiners: ZWNJ after a virama keeps a visible virama (ನ್‌ನ), ZWJ between ರ and its
///   virama prevents the arkavattu (ರ‍್ಯ). Other spellings of the same rendering are
///   rewritten to these, and joiners that change nothing are removed.
///
/// Text outside the Kannada block is only touched by NFC. Old Kannada letters are kept;
/// see [`normalize_as`] with [`Form::Search`] or [`modernize`].
///
/// ```
/// assert_eq!(kannadi::normalize("ಕಿ\u{0CD5}ರ್ತಿ"), "ಕೀರ್ತಿ");
/// // A ZWNJ after a word-final virama changes nothing, so it goes.
/// assert_eq!(kannadi::normalize("ಆಫ್\u{200C} ಇಂಡಿಯಾ"), "ಆಫ್ ಇಂಡಿಯಾ");
/// // ZWJ after a virama shows the virama, as ZWNJ does.
/// assert_eq!(kannadi::normalize("ರಾಜ್\u{200D}ಕುಮಾರ್"), "ರಾಜ್\u{200C}ಕುಮಾರ್");
/// ```
pub fn normalize(text: &str) -> String {
    normalize_as(text, Form::Canonical)
}

/// Normalizes text to `form`.
///
/// ```
/// use kannadi::normalize::{normalize_as, Form};
///
/// assert_eq!(normalize_as("ಬೆ೦ಗಳೂರು", Form::Legacy), "ಬೆಂಗಳೂರು");
/// assert_eq!(normalize_as("ಸಕರ್ಾರ", Form::Legacy), "ಸರ್ಕಾರ");
/// assert_eq!(normalize_as("ಬಾೞ್ ಜ಼ೀರೋ", Form::Search), "ಬಾಳ್ ಜೀರೋ");
/// ```
pub fn normalize_as(text: &str, form: Form) -> String {
    normalize_counted(text, form).0
}

/// Normalizes text to `form` and counts the changes each rule made.
///
/// ```
/// use kannadi::normalize::{normalize_counted, Form, Rule};
///
/// let (text, counts) = normalize_counted("ಡಿಸೆಂಬರ್\u{200D}ನ", Form::Canonical);
/// assert_eq!(text, "ಡಿಸೆಂಬರ\u{200D}\u{0CCD}ನ");
/// assert_eq!(counts.get(Rule::NonRephRa), 1);
/// ```
pub fn normalize_counted(text: &str, form: Form) -> (String, Counts) {
    let mut counts = Counts::default();
    let mut chars = canonical(text, &mut counts);
    if form >= Form::Legacy {
        let before = chars.clone();
        chars = legacy(&chars, &mut counts);
        if chars != before {
            chars = canonical_chars(&chars, &mut counts);
        }
    }
    if form >= Form::Search {
        // Folding turns ೝ and U+0CDC into letters that the legacy rules apply to.
        chars = search(&chars, &mut counts);
        chars = legacy(&chars, &mut counts);
        chars = canonical_chars(&chars, &mut counts);
    }
    (chars.into_iter().collect(), counts)
}

/// True if `text` is already in `form`.
pub fn is_normalized(text: &str, form: Form) -> bool {
    normalize_counted(text, form).1.total() == 0
}

fn canonical(text: &str, counts: &mut Counts) -> Vec<char> {
    let chars: Vec<char> = text.chars().collect();
    canonical_chars(&chars, counts)
}

/// Repeats NFC and the mark and joiner rules until nothing changes: removing a joiner
/// can bring 2 parts of a vowel sign together, and moving a nukta can put it after a
/// virama. 2 passes settle every case the tests know; the cap is a guard.
fn canonical_chars(chars: &[char], counts: &mut Counts) -> Vec<char> {
    let mut text = chars.to_vec();
    for _ in 0..8 {
        let composed = nfc(&text, counts);
        let out = marks_and_joiners(&composed, counts);
        if out == text {
            break;
        }
        text = out;
    }
    text
}

/// NFC, counting each composed or reordered character once.
fn nfc(chars: &[char], counts: &mut Counts) -> Vec<char> {
    let composed: Vec<char> = chars.iter().copied().nfc().collect();
    if composed.as_slice() != chars {
        let reordered = chars
            .windows(2)
            .filter(|w| {
                let (a, b) = (
                    canonical_combining_class(w[0]),
                    canonical_combining_class(w[1]),
                );
                a > b && b != 0
            })
            .count();
        let changes = (chars.len() - composed.len()) + reordered;
        counts.0[Rule::Nfc.index()] += changes.max(1);
    }
    composed
}

fn is_joiner(c: char) -> bool {
    c == ZWNJ || c == ZWJ
}

/// True if `out` ends with a consonant, with or without a nukta.
fn ends_with_consonant(out: &[char]) -> bool {
    match out {
        [.., c, NUKTA] | [.., c] => is_consonant(*c),
        [] => false,
    }
}

/// True if the ರ at `out[i]` starts its syllable, so a virama after it can form the
/// arkavattu.
fn starts_syllable(out: &[char], i: usize) -> bool {
    i == 0 || out[i - 1] != VIRAMA
}

fn marks_and_joiners(chars: &[char], counts: &mut Counts) -> Vec<char> {
    let mut out: Vec<char> = Vec::with_capacity(chars.len());
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if is_joiner(c) {
            let start = i;
            while i < chars.len() && is_joiner(chars[i]) {
                i += 1;
            }
            joiners(&mut out, &chars[start..i], chars.get(i).copied(), counts);
            continue;
        }
        i += 1;
        let prev = out.last().copied();
        match (prev, c) {
            (Some('ಉ'), 'ಾ') => replace_last(&mut out, 'ಊ', Rule::VowelLetter, counts),
            (Some('ಒ'), 'ೌ') => replace_last(&mut out, 'ಔ', Rule::VowelLetter, counts),
            (Some('ಋ'), 'ಾ') => replace_last(&mut out, 'ೠ', Rule::VowelLetter, counts),
            (Some(p), c) if p == c && class(c).is_mark() => counts.bump(Rule::RepeatedMark),
            (_, NUKTA) => place_nukta(&mut out, counts),
            _ => out.push(c),
        }
    }
    out
}

fn replace_last(out: &mut [char], c: char, rule: Rule, counts: &mut Counts) {
    if let Some(last) = out.last_mut() {
        *last = c;
    }
    counts.bump(rule);
}

/// Puts a nukta directly after the consonant whose vowel signs come before it.
fn place_nukta(out: &mut Vec<char>, counts: &mut Counts) {
    let mut at = out.len();
    while at > 0 && is_vowel_sign(out[at - 1]) {
        at -= 1;
    }
    if at < out.len() && ends_with_consonant(&out[..at]) {
        if out[at - 1] == NUKTA {
            counts.bump(Rule::RepeatedMark);
        } else {
            counts.bump(Rule::NuktaOrder);
            out.insert(at, NUKTA);
        }
        return;
    }
    out.push(NUKTA);
}

/// Handles a run of ZWNJ and ZWJ, given the output so far and the character after the run.
fn joiners(out: &mut Vec<char>, run: &[char], next: Option<char>, counts: &mut Counts) {
    let prev = out.last().copied();
    if !prev.is_some_and(is_kannada) && !next.is_some_and(is_kannada) {
        out.extend_from_slice(run);
        return;
    }
    if next == Some(VIRAMA) && ends_with_consonant(out) {
        let n = out.len();
        if prev == Some(RA) && starts_syllable(out, n - 1) {
            out.push(ZWJ);
            if run != [ZWJ] {
                counts.bump(Rule::JoinerBeforeVirama);
            }
        } else {
            counts.bump(Rule::JoinerBeforeVirama);
        }
        return;
    }
    if prev == Some(VIRAMA) {
        if !next.is_some_and(is_consonant) {
            counts.bump(Rule::StrayJoiner);
            return;
        }
        let n = out.len();
        let ra = n >= 2 && out[n - 2] == RA && starts_syllable(out, n - 2);
        if ra && run.iter().all(|&j| j == ZWJ) {
            out.insert(n - 1, ZWJ);
            counts.bump(Rule::NonRephRa);
            return;
        }
        out.push(ZWNJ);
        if run != [ZWNJ] {
            counts.bump(Rule::ExplicitVirama);
        }
        return;
    }
    // A joiner between a consonant and its sign changes the shape in some fonts.
    if ends_with_consonant(out) && next.is_some_and(|n| class(n).is_mark()) {
        out.extend_from_slice(run);
        return;
    }
    counts.bump(Rule::StrayJoiner);
}

fn is_letter_or_sign(c: char) -> bool {
    matches!(class(c), Class::Vowel | Class::Consonant | Class::VowelSign)
        || matches!(c, ANUSVARA | NUKTA)
}

fn is_digit(c: char) -> bool {
    c.is_ascii_digit() || class(c) == Class::Digit
}

fn is_letter(c: char) -> bool {
    matches!(class(c), Class::Vowel | Class::Consonant)
}

fn legacy(chars: &[char], counts: &mut Counts) -> Vec<char> {
    let mut out: Vec<char> = Vec::with_capacity(chars.len());
    for (i, &c) in chars.iter().enumerate() {
        let prev = out.last().copied();
        let next = chars.get(i + 1).copied();
        if c == '೦' && prev.is_some_and(is_letter_or_sign) && !next.is_some_and(is_digit) {
            out.push(ANUSVARA);
            counts.bump(Rule::ZeroAsAnusvara);
        } else if matches!(c, 'o' | 'O' | '0')
            && prev.is_some_and(is_vowel_sign)
            && next.is_some_and(is_letter)
        {
            out.push(ANUSVARA);
            counts.bump(Rule::LatinAsAnusvara);
        } else if is_vowel_sign(c) && out.ends_with(&[RA, VIRAMA]) && move_reph(&mut out) {
            out.push(c);
            counts.bump(Rule::MisplacedReph);
        } else {
            out.push(c);
        }
    }
    out
}

/// Moves a ರ್ at the end of `out` to the start of the consonant cluster before it.
/// Returns false, changing nothing, when no cluster comes before it.
fn move_reph(out: &mut Vec<char>) -> bool {
    let ra = out.len() - 2;
    let mut start = ra;
    if start > 0 && out[start - 1] == NUKTA {
        start -= 1;
    }
    if start == 0 || !is_consonant(out[start - 1]) {
        return false;
    }
    start -= 1;
    while start >= 2 && out[start - 1] == VIRAMA && is_consonant(out[start - 2]) {
        start -= 2;
    }
    if start > 0 && out[start - 1] == VIRAMA {
        return false;
    }
    out.truncate(ra);
    out.splice(start..start, [RA, VIRAMA]);
    true
}

fn search(chars: &[char], counts: &mut Counts) -> Vec<char> {
    let mut out: Vec<char> = Vec::with_capacity(chars.len());
    for (i, &c) in chars.iter().enumerate() {
        let next = chars.get(i + 1).copied();
        match c {
            LLLA => fold(&mut out, &['ಳ'], counts),
            RRA => fold(&mut out, &['ರ'], counts),
            NAKAARA_POLLU => fold(&mut out, &['ನ', VIRAMA], counts),
            ANUSVARA_ABOVE_RIGHT => fold(&mut out, &[ANUSVARA], counts),
            ARCHAIC_SHRII => fold(&mut out, &['ಶ', VIRAMA, 'ರ', 'ೀ'], counts),
            NUKTA => counts.bump(Rule::Nukta),
            c if (is_joiner(c) || FORMAT.contains(&c))
                && (out.last().is_some_and(|&p| is_kannada(p)) || next.is_some_and(is_kannada)) =>
            {
                counts.bump(Rule::Invisible)
            }
            _ => out.push(c),
        }
    }
    out
}

fn fold(out: &mut Vec<char>, with: &[char], counts: &mut Counts) {
    out.extend_from_slice(with);
    counts.bump(Rule::OldLetter);
}

fn is_letter_or_mark(c: char) -> bool {
    let c = class(c);
    matches!(c, Class::Vowel | Class::Consonant) || c.is_mark()
}

/// Replaces a Kannada digit zero (೦) written right after a letter or sign with an
/// anusvara (ಂ). Legacy fonts and OCR often confuse the two.
///
/// Use this on converted or scanned text only: a number written directly after a
/// word, such as ಕೊಠಡಿ೦೧, is changed too. [`Form::Legacy`] leaves a zero before a
/// digit alone.
///
/// ```
/// assert_eq!(kannadi::fix_zero_as_anusvara("ಬೆ೦ಗಳೂರು"), "ಬೆಂಗಳೂರು");
/// assert_eq!(kannadi::fix_zero_as_anusvara("೧೦೦"), "೧೦೦");
/// ```
pub fn fix_zero_as_anusvara(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut prev: Option<char> = None;
    for c in text.chars() {
        match prev {
            Some(p) if c == '೦' && is_letter_or_mark(p) => out.push(ANUSVARA),
            _ => out.push(c),
        }
        prev = Some(c);
    }
    out
}

/// Maps Old Kannada letters to their modern equivalents, for search and indexing.
///
/// - ೞ (LLLA) becomes ಳ
/// - ಱ (RRA) becomes ರ
/// - ೝ (nakaara pollu) becomes ನ್, followed by ZWNJ when a consonant comes next,
///   so it does not join that consonant into a conjunct
///
/// ```
/// assert_eq!(kannadi::modernize("ಅಱಿವು ಬಾೞ್"), "ಅರಿವು ಬಾಳ್");
/// assert_eq!(kannadi::modernize("ಅವೝಕಂಡ"), "ಅವನ್\u{200C}ಕಂಡ");
/// ```
pub fn modernize(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            LLLA => out.push('ಳ'),
            RRA => out.push('ರ'),
            NAKAARA_POLLU => {
                out.push('ನ');
                out.push(VIRAMA);
                if chars
                    .peek()
                    .is_some_and(|&n| is_consonant(n) || n == NAKAARA_POLLU)
                {
                    out.push(ZWNJ);
                }
            }
            _ => out.push(c),
        }
    }
    out
}

/// True if `text` contains an Old Kannada letter that [`modernize`] would change.
pub fn has_old_kannada(text: &str) -> bool {
    text.chars()
        .any(|c| matches!(c, LLLA | RRA | NAKAARA_POLLU))
}

#[cfg(test)]
mod tests {
    use super::*;

    const N: char = ZWNJ;
    const J: char = ZWJ;

    fn canon(s: &str) -> String {
        normalize(s)
    }

    fn count(s: &str, form: Form, rule: Rule) -> usize {
        normalize_counted(s, form).1.get(rule)
    }

    #[test]
    fn nfc_composes_two_part_vowel_signs() {
        assert_eq!(canon("ಕೆ\u{0CD5}"), "ಕೇ");
        assert_eq!(canon("ಕೆ\u{0CC2}"), "ಕೊ");
        assert_eq!(canon("ಕೆ\u{0CC2}\u{0CD5}"), "ಕೋ");
        assert_eq!(canon("ಕೆ\u{0CD6}"), "ಕೈ");
        assert_eq!(count("ಕೆ\u{0CD5}ಕೆ\u{0CD5}", Form::Canonical, Rule::Nfc), 2);
    }

    #[test]
    fn look_alike_vowel_letters() {
        assert_eq!(canon("ಉಾರು"), "ಊರು");
        assert_eq!(canon("ಋಾ"), "ೠ");
        assert_eq!(canon("ಒೌ"), "ಔ");
        assert_eq!(canon("ಕಾ"), "ಕಾ");
    }

    #[test]
    fn nukta_moves_to_its_consonant() {
        assert_eq!(canon("ಜೀ\u{0CBC}ರೋ"), "ಜ\u{0CBC}ೀರೋ");
        assert_eq!(canon("ಜ್\u{0CBC}"), "ಜ\u{0CBC}್");
        assert_eq!(canon("ಜ\u{0CBC}ೀ\u{0CBC}"), "ಜ\u{0CBC}ೀ");
        assert_eq!(canon("ಅ\u{0CBC}"), "ಅ\u{0CBC}");
        assert_eq!(canon("ಜ\u{0CBC}"), "ಜ\u{0CBC}");
    }

    #[test]
    fn repeated_marks_collapse() {
        assert_eq!(canon("ಕ್್ಕ"), "ಕ್ಕ");
        assert_eq!(canon("ಕಂಂ"), "ಕಂ");
        assert_eq!(canon("ಅಥವಾಾ"), "ಅಥವಾ");
        assert_eq!(canon("ಕಕ"), "ಕಕ");
    }

    #[test]
    fn normalize_leaves_numbers_after_words_alone() {
        assert_eq!(canon("ಕೊಠಡಿ೦೧"), "ಕೊಠಡಿ೦೧");
        assert_eq!(canon("ಬೆ೦ಗಳೂರು"), "ಬೆ೦ಗಳೂರು");
    }

    #[test]
    fn joiner_before_virama() {
        assert_eq!(canon(&format!("ಮ{N}್ಯೂಸಿಯಂ")), "ಮ್ಯೂಸಿಯಂ");
        assert_eq!(canon(&format!("ಕೆಪ{J}್ಲರ್")), "ಕೆಪ್ಲರ್");
        assert_eq!(canon(&format!("ರ{N}್ಯಾಲಿ")), format!("ರ{J}್ಯಾಲಿ"));
        assert_eq!(canon(&format!("ರ{J}್ಯಾಲಿ")), format!("ರ{J}್ಯಾಲಿ"));
        // ರ inside a cluster cannot form the arkavattu.
        assert_eq!(canon(&format!("ಕ್ರ{N}್ಯ")), "ಕ್ರ್ಯ");
        assert_eq!(canon(&format!("ಜ\u{0CBC}{N}್ಞ")), "ಜ\u{0CBC}್ಞ");
    }

    #[test]
    fn non_reph_ra() {
        assert_eq!(canon(&format!("ನೆದರ್{J}ಲ್ಯಾಂಡ್ಸ್")), format!("ನೆದರ{J}್ಲ್ಯಾಂಡ್ಸ್"));
        assert_eq!(
            count(&format!("ರ್{J}ಲ"), Form::Canonical, Rule::NonRephRa),
            1
        );
        // With ZWNJ the virama stays visible: a different rendering, left alone.
        assert_eq!(canon(&format!("ರ್{N}ಲ")), format!("ರ್{N}ಲ"));
        assert_eq!(canon(&format!("ಕ್ರ್{J}ಯ")), format!("ಕ್ರ್{N}ಯ"));
    }

    #[test]
    fn explicit_virama() {
        assert_eq!(canon(&format!("ರಾಜ್{J}ಕುಮಾರ್")), format!("ರಾಜ್{N}ಕುಮಾರ್"));
        assert_eq!(canon(&format!("ಡೇವಿಡ್{N}{N}ಸನ್")), format!("ಡೇವಿಡ್{N}ಸನ್"));
        assert_eq!(canon(&format!("ನ್{J}{N}ನ")), format!("ನ್{N}ನ"));
        assert_eq!(canon(&format!("ನ್{N}{J}ನ")), format!("ನ್{N}ನ"));
        assert_eq!(canon(&format!("ನ್{N}ನ")), format!("ನ್{N}ನ"));
        assert_eq!(
            count(&format!("ನ್{N}ನ"), Form::Canonical, Rule::ExplicitVirama),
            0
        );
    }

    #[test]
    fn stray_joiners_go() {
        assert_eq!(canon(&format!("ಆಫ್{N} ಇಂಡಿಯಾ")), "ಆಫ್ ಇಂಡಿಯಾ");
        assert_eq!(canon(&format!("ಆಫ್{J}")), "ಆಫ್");
        assert_eq!(canon(&format!("ಪ್ರಶಸ್ತಿ{N}")), "ಪ್ರಶಸ್ತಿ");
        assert_eq!(canon(&format!("ಸಿ{N}ಇಒ")), "ಸಿಇಒ");
        assert_eq!(canon(&format!("ಜವಾಹರ{N}ಲಾಲ್")), "ಜವಾಹರಲಾಲ್");
        assert_eq!(canon(&format!(" {N}ಕನ್ನಡ")), " ಕನ್ನಡ");
        assert_eq!(canon(&format!("ಕ್{N}ಅ")), "ಕ್ಅ");
    }

    #[test]
    fn joiners_that_may_change_a_shape_stay() {
        assert_eq!(canon(&format!("ಮ{N}ೂಲಕ")), format!("ಮ{N}ೂಲಕ"));
        assert_eq!(canon(&format!("ಕ{J}ಂ")), format!("ಕ{J}ಂ"));
    }

    #[test]
    fn joiners_outside_kannada_stay() {
        let persian = "\u{0645}\u{06CC}{N}\u{062E}\u{0648}".replace("{N}", "\u{200C}");
        assert_eq!(canon(&persian), persian);
        assert_eq!(canon(&format!("a{N}b {J}")), format!("a{N}b {J}"));
    }

    #[test]
    fn canonical_is_idempotent_on_the_tricky_cases() {
        for s in [
            format!("ರ{N}್ಯಾ"),
            format!("ರ್{J}ಲ"),
            format!("ನ್{J}{N}{J}ನ"),
            "ಜೀ\u{0CBC}್".to_string(),
            format!("ಕ್{N}{N}"),
        ] {
            let once = canon(&s);
            assert_eq!(canon(&once), once, "{s:?}");
            assert!(is_normalized(&once, Form::Canonical));
        }
    }

    #[test]
    fn legacy_zero_and_latin_as_anusvara() {
        let leg = |s: &str| normalize_as(s, Form::Legacy);
        assert_eq!(leg("ಬೆ೦ಗಳೂರು"), "ಬೆಂಗಳೂರು");
        assert_eq!(leg("ಸ್ವಯ೦ ಸೇವಕ"), "ಸ್ವಯಂ ಸೇವಕ");
        assert_eq!(leg("ಕೊಠಡಿ೦೧"), "ಕೊಠಡಿ೦೧");
        assert_eq!(leg("೧೦೦"), "೧೦೦");
        assert_eq!(leg("ಕ ೦"), "ಕ ೦");
        assert_eq!(leg("ಹೊoದಿದೆ"), "ಹೊಂದಿದೆ");
        assert_eq!(leg("ಡಿ0ಟಿ"), "ಡಿಂಟಿ");
        assert_eq!(leg("ಮಾo"), "ಮಾo");
        assert_eq!(leg("ಕo"), "ಕo");
        assert_eq!(canon("ಹೊoದಿದೆ"), "ಹೊoದಿದೆ");
    }

    #[test]
    fn legacy_misplaced_reph() {
        let leg = |s: &str| normalize_as(s, Form::Legacy);
        assert_eq!(leg("ಸಕರ್ಾರ"), "ಸರ್ಕಾರ");
        assert_eq!(leg("ಆಥರ್ಿಕ"), "ಆರ್ಥಿಕ");
        assert_eq!(leg("ವಿದ್ಯಾಥರ್ಿಗಳು"), "ವಿದ್ಯಾರ್ಥಿಗಳು");
        assert_eq!(leg("ಕ್ಷರ್ಿ"), "ರ್ಕ್ಷಿ");
        assert_eq!(leg("ರ್ಿ"), "ರ್ಿ");
        assert_eq!(leg("ಸರ್ಕಾರ"), "ಸರ್ಕಾರ");
        assert_eq!(count("ಸಕರ್ಾರ", Form::Legacy, Rule::MisplacedReph), 1);
        assert_eq!(canon("ಸಕರ್ಾರ"), "ಸಕರ್ಾರ");
    }

    #[test]
    fn search_folds_old_letters_joiners_and_nukta() {
        let s = |t: &str| normalize_as(t, Form::Search);
        assert_eq!(s("ಅಱಿವು ಬಾೞ್"), "ಅರಿವು ಬಾಳ್");
        assert_eq!(s("ಅವೝ"), "ಅವನ್");
        assert_eq!(s("ಅವೝಕಂಡ"), "ಅವನ್ಕಂಡ");
        assert_eq!(s("\u{0CDC}"), "ಶ್ರೀ");
        assert_eq!(s("ಕ\u{0CF3}"), "ಕಂ");
        assert_eq!(s(&format!("ಕನ್{N}ನಡ")), "ಕನ್ನಡ");
        assert_eq!(s("ಕಾಂಗ್ರೆಸ್\u{200B}ನಿಂದ"), "ಕಾಂಗ್ರೆಸ್ನಿಂದ");
        assert_eq!(s("ಜ಼ೀರೋ"), "ಜೀರೋ");
        assert_eq!(s("ಬೆ೦ಗಳೂರು"), "ಬೆಂಗಳೂರು");
        assert_eq!(s("a\u{00AD}b"), "a\u{00AD}b");
        assert_eq!(normalize_as("ಅಱಿ", Form::Legacy), "ಅಱಿ");
    }

    #[test]
    fn forms_have_names_and_rules_have_ids() {
        for f in Form::ALL {
            assert_eq!(Form::from_name(f.name()), Some(f));
        }
        assert_eq!(Form::from_name("SEARCH"), Some(Form::Search));
        assert_eq!(Form::from_name("x"), None);
        let ids: Vec<_> = Rule::ALL.iter().map(|r| r.id()).collect();
        assert_eq!(ids.len(), 14);
        assert!(Rule::ALL.iter().enumerate().all(|(i, r)| r.index() == i));
        assert_eq!(Rule::MisplacedReph.form(), Form::Legacy);
        assert_eq!(Rule::Nukta.form(), Form::Search);
    }

    #[test]
    fn zero_after_letter_becomes_anusvara() {
        assert_eq!(fix_zero_as_anusvara("ಬೆ೦ಗಳೂರು"), "ಬೆಂಗಳೂರು");
        assert_eq!(fix_zero_as_anusvara("ಅ೦ಗಡಿ"), "ಅಂಗಡಿ");
        assert_eq!(fix_zero_as_anusvara("೧೦೦"), "೧೦೦");
        assert_eq!(fix_zero_as_anusvara("೦"), "೦");
        assert_eq!(fix_zero_as_anusvara("ಕ ೦"), "ಕ ೦");
        assert_eq!(fix_zero_as_anusvara("ಕ೦೦"), "ಕಂ೦");
    }

    #[test]
    fn modernize_maps_old_letters() {
        assert_eq!(modernize("ಕೞ್ತೆ"), "ಕಳ್ತೆ");
        assert_eq!(modernize("ಅಱಿ"), "ಅರಿ");
        assert_eq!(modernize("ಅವೝ"), "ಅವನ್");
        assert_eq!(modernize("ಅವೝ ಕಂಡ"), "ಅವನ್ ಕಂಡ");
        assert_eq!(modernize("ಅವೝಕಂಡ"), "ಅವನ್\u{200C}ಕಂಡ");
        assert_eq!(modernize("ೝೝ"), "ನ್\u{200C}ನ್");
        assert_eq!(modernize("plain ಕನ್ನಡ"), "plain ಕನ್ನಡ");
        assert!(has_old_kannada("ಅಱಿ"));
        assert!(!has_old_kannada("ಅರಿ"));
    }
}
