// Checks the built page package: node web/test/smoke.mjs [dist directory]
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";

const dist = resolve(process.argv[2] ?? new URL("../dist", import.meta.url).pathname);
const wasm = await import(join(dist, "pkg", "kannadi_wasm.js"));
wasm.initSync({ module: readFileSync(join(dist, "pkg", "kannadi_wasm_bg.wasm")) });
const { NudiOptions, nudiToUnicode, decodeWindows1252, transliterate, normalize, aksharas } = wasm;

function convert(text, set = () => {}) {
  const options = new NudiOptions();
  set(options);
  const result = nudiToUnicode(text, options);
  const out = {
    text: result.text,
    clean: result.isClean,
    unknown: result.unknown.map((u) => ({ offset: u.offset, glyph: u.glyph })),
    malformed: result.malformed.map((s) => [s.start, s.end]),
  };
  options.free();
  result.free();
  return out;
}

// Karnataka RDPR page, Dec 2002, BRH Kannada:
// https://web.archive.org/web/20021227171554/http://www.kar.nic.in:80/rdpr/kan-www/Schemes/Chapter-1/Matter/1_2(M).htm
const clean = convert("¸ÀPÁðj ¥ÁæxÀ«ÄPÀ ±Á¯ÉUÀ¼À°è ²PÀëPÀjUÉ vÀgÀ¨ÉÃw ¤ÃqÀÄªÀÅzÀÄ.");
assert.equal(clean.text, "ಸರ್ಕಾರಿ ಪ್ರಾಥಮಿಕ ಶಾಲೆಗಳಲ್ಲಿ ಶಿಕ್ಷಕರಿಗೆ ತರಬೇತಿ ನೀಡುವುದು.");
assert.ok(clean.clean);

// Offsets are UTF-16 code units: they index JavaScript strings directly.
const input = "PÀ£ÀßqÀ õ ¥Àæ";
const withUnknown = convert(input);
assert.equal(withUnknown.unknown.length, 1);
assert.equal(input[withUnknown.unknown[0].offset], "õ");
assert.equal(withUnknown.unknown[0].glyph, "õ");

const bad = convert("PÀ ÁA");
assert.ok(bad.malformed.length > 0);
const [start, end] = bad.malformed[0];
assert.ok(start > 0 && end > start && end <= bad.text.length);

assert.equal(convert("1956", (o) => (o.kannadaDigits = true)).text, "೧೯೫೬");
assert.equal(convert("¥ÁægÀ0¨sÀ").text, "ಪ್ರಾರಂಭ");
assert.equal(convert("¥ÁægÀ0¨sÀ", (o) => (o.zeroAsAnusvara = false)).text, "ಪ್ರಾರ0ಭ");

assert.equal(decodeWindows1252(new Uint8Array([0x50, 0xc0, 0x86])), "PÀ†");
assert.equal(transliterate("ಕನ್ನಡ", "kannada", "iso15919"), "kannaḍa");
assert.throws(() => transliterate("x", "klingon", "kannada"), /unknown scheme/);
assert.equal(normalize("ಕೀ"), "ಕೀ");
assert.deepEqual(aksharas("ಕನ್ನಡ"), ["ಕ", "ನ್ನ", "ಡ"]);

// The page references only files the build produced.
const html = readFileSync(join(dist, "index.html"), "utf8");
for (const [, ref] of html.matchAll(/(?:href|src)="([^":#]+)"/g)) {
  readFileSync(join(dist, ref));
}
assert.match(html, /<h1>Nudi to Unicode converter<\/h1>/);

console.log("web smoke test passed");
