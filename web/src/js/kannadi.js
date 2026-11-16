// Thin wrapper over the WebAssembly bindings: plain values in and out, and
// every wasm object freed before returning.
import init, * as wasm from "../pkg/kannadi_wasm.js";

export { init };

/**
 * Converts legacy font text in `layout` ("nudi", "akruti", "shree-lipi", or "auto" for the
 * layout the text reads best in). Unknown glyph offsets are UTF-16 code units in
 * the output. The result says which layout was used.
 * @returns {{text: string, unknown: number[], layout: string}}
 */
export function convertLegacy(text, { kannadaDigits = false, zeroAsAnusvara = true, layout = "auto" } = {}) {
  const used = layout === "auto" ? wasm.bestLayout(text) : layout;
  const options = new wasm.NudiOptions();
  options.kannadaDigits = kannadaDigits;
  options.zeroAsAnusvara = zeroAsAnusvara;
  let result;
  try {
    result = wasm.toUnicode(text, used, options);
  } finally {
    options.free();
  }
  const out = {
    text: result.text,
    unknown: Array.from(result.unknownInText),
    layout: used,
  };
  result.free();
  return out;
}

/**
 * Converts Unicode Kannada to Nudi and Baraha glyph codes. Report offsets are
 * UTF-16 code units in the input.
 * @returns {{text: string, unencodable: number[], malformed: number[]}}
 */
export function toNudi(text) {
  const result = wasm.unicodeToNudi(text);
  const out = { text: result.text, unencodable: Array.from(result.unencodable), malformed: Array.from(result.malformed) };
  result.free();
  return out;
}

/** Flat [start, end, ...] ranges of aksharas that are not well formed. */
export function malformed(text) {
  return wasm.malformed(text);
}

/** Checked repairs for the malformed akshara at start..end. */
export function fixes(text, start, end) {
  return wasm.fixes(text, start, end).map((f) => {
    const fix = { start: f.start, end: f.end, replacement: f.replacement, kind: f.kind };
    f.free();
    return fix;
  });
}

export const bestLayout = wasm.bestLayout;
/** The layout of 1 font name, such as "nudi" for "BRH Kannada", or null. */
export const fontLayout = (name) => wasm.fontLayout(name) ?? null;
export const isLegacyFont = wasm.isLegacyFont;
export const drawsKannadaDigits = wasm.drawsKannadaDigits;
export const transliterate = wasm.transliterate;
export const normalize = wasm.normalize;
export const modernize = wasm.modernize;
export const countAksharas = wasm.countAksharas;
export const decodeWindows1252 = wasm.decodeWindows1252;
export const encodeWindows1252 = wasm.encodeWindows1252;
