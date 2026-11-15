// Pure text helpers shared by the editor and the importers. No DOM access.

// Windows-1252 characters above ASCII, where Nudi and Baraha fonts draw glyphs.
const LEGACY_CHAR = /[¡-ÿŒœŠšŸŽžƒˆ˜–—‘-„†-•…‰‹›€™]/gu;
const KANNADA_CHAR = /[ಀ-೿]/gu;
const NON_SPACE = /\S/gu;
// Legacy font families. Nudi's Unicode fonts carry "Uni" in their names.
const LEGACY_FONT = /^(?!.*uni)\s*['"]?(nudi|brh\b|baraha)/i;

const count = (text, pattern) => (text.match(pattern) || []).length;

/**
 * True when text reads like Nudi or Baraha glyph codes. In real Nudi lines at
 * least 59% of the non-space characters are Latin-1 symbols such as À and Ä
 * (10th percentile, 2002 RDPR pages); accented European text stays near 33%.
 */
export function looksLegacy(text) {
  const legacy = count(text, LEGACY_CHAR);
  if (legacy < 2) return false;
  if (count(text, KANNADA_CHAR) >= legacy) return false;
  return legacy / count(text, NON_SPACE) >= 0.4;
}

/** True for a Nudi or Baraha ANSI font family name, such as "BRH Kannada". */
export function isLegacyFont(family) {
  return typeof family === "string" && family.split(",").some((f) => LEGACY_FONT.test(f.trim()));
}

/**
 * Decodes a text file: UTF-16 with a byte order mark, UTF-8 when valid, and
 * Windows-1252 otherwise, the encoding of legacy Kannada files.
 */
export function decodeBytes(bytes, decodeWindows1252) {
  if (bytes[0] === 0xff && bytes[1] === 0xfe) return new TextDecoder("utf-16le").decode(bytes);
  if (bytes[0] === 0xfe && bytes[1] === 0xff) return new TextDecoder("utf-16be").decode(bytes);
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    return decodeWindows1252(bytes);
  }
}

/**
 * Joins text segments, converting the legacy ones. Adjacent segments of the
 * same kind are merged first, so a syllable split across Word runs converts whole.
 * @param {{text: string, legacy: boolean}[]} segments
 * @param {(text: string) => {text: string, unknown: number[]}} convert
 * @returns {{text: string, unknown: number[], converted: number}}
 */
export function joinSegments(segments, convert) {
  const merged = [];
  for (const s of segments) {
    if (!s.text) continue;
    const last = merged[merged.length - 1];
    if (last && last.legacy === s.legacy) last.text += s.text;
    else merged.push({ ...s });
  }
  let text = "";
  const unknown = [];
  let converted = 0;
  for (const s of merged) {
    if (s.legacy) {
      const result = convert(s.text);
      for (const offset of result.unknown) unknown.push(text.length + offset);
      text += result.text;
      converted += s.text.length;
    } else {
      text += s.text;
    }
  }
  return { text, unknown, converted };
}

/**
 * The single edit that turns `before` into `after`: the range start..oldEnd
 * of `before` was replaced by start..newEnd of `after`.
 */
export function diff(before, after) {
  let start = 0;
  const limit = Math.min(before.length, after.length);
  while (start < limit && before.charCodeAt(start) === after.charCodeAt(start)) start++;
  let oldEnd = before.length;
  let newEnd = after.length;
  while (oldEnd > start && newEnd > start && before.charCodeAt(oldEnd - 1) === after.charCodeAt(newEnd - 1)) {
    oldEnd--;
    newEnd--;
  }
  return { start, oldEnd, newEnd };
}

/**
 * Moves anchored glyphs through an edit. Anchors inside the edited range, or
 * whose glyph is no longer at the new position, are dropped.
 * @param {{offset: number, glyph: string}[]} anchors
 */
export function shiftAnchors(anchors, edit, after) {
  const delta = edit.newEnd - edit.oldEnd;
  const out = [];
  for (const a of anchors) {
    let offset = a.offset;
    if (offset + a.glyph.length <= edit.start) {
      // before the edit
    } else if (offset >= edit.oldEnd) {
      offset += delta;
    } else {
      continue;
    }
    if (after.startsWith(a.glyph, offset)) out.push({ offset, glyph: a.glyph });
  }
  return out;
}

/** Words separated by white space. */
export function countWords(text) {
  return count(text, /\S+/gu);
}

/** "U+0CCD U+0C95" for a short string. */
export function codePoints(text) {
  return Array.from(text, (c) => "U+" + c.codePointAt(0).toString(16).toUpperCase().padStart(4, "0")).join(" ");
}
