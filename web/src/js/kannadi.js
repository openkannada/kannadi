// Thin wrapper over the WebAssembly bindings: plain values in and out, and
// every wasm object freed before returning.
import init, * as wasm from "../pkg/kannadi_wasm.js";

export { init };

/**
 * Converts Nudi or Baraha text. Offsets are UTF-16 code units in the output.
 * @returns {{text: string, unknown: number[], malformed: number[]}}
 */
export function convertLegacy(text, { kannadaDigits = false, zeroAsAnusvara = true } = {}) {
  const options = new wasm.NudiOptions();
  options.kannadaDigits = kannadaDigits;
  options.zeroAsAnusvara = zeroAsAnusvara;
  const result = wasm.nudiToUnicode(text, options);
  options.free();
  const out = {
    text: result.text,
    unknown: Array.from(result.unknownInText),
    malformed: Array.from(result.malformed),
  };
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

export const transliterate = wasm.transliterate;
export const normalize = wasm.normalize;
export const modernize = wasm.modernize;
export const countAksharas = wasm.countAksharas;
export const decodeWindows1252 = wasm.decodeWindows1252;
