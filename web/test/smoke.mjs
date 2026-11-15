// Checks the built page package: node web/test/smoke.mjs [dist directory]
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const dist = resolve(process.argv[2] ?? fileURLToPath(new URL("../dist", import.meta.url)));
const url = (path) => pathToFileURL(join(dist, path)).href;
const wasm = await import(url("pkg/kannadi_wasm.js"));
wasm.initSync({ module: readFileSync(join(dist, "pkg", "kannadi_wasm_bg.wasm")) });
const kannadi = await import(url("js/kannadi.js"));
const { openFile } = await import(url("js/files.js"));
const { writeZip } = await import(url("js/zip.js"));

// Karnataka RDPR page, Dec 2002, BRH Kannada:
// https://web.archive.org/web/20021227171554/http://www.kar.nic.in:80/rdpr/kan-www/Schemes/Chapter-1/Matter/1_2(M).htm
const NUDI = "¸ÀPÁðj ¥ÁæxÀ«ÄPÀ ±Á¯ÉUÀ¼À°è ²PÀëPÀjUÉ vÀgÀ¨ÉÃw ¤ÃqÀÄªÀÅzÀÄ.";
const UNICODE = "ಸರ್ಕಾರಿ ಪ್ರಾಥಮಿಕ ಶಾಲೆಗಳಲ್ಲಿ ಶಿಕ್ಷಕರಿಗೆ ತರಬೇತಿ ನೀಡುವುದು.";

const clean = kannadi.convertLegacy(NUDI);
assert.equal(clean.text, UNICODE);
assert.deepEqual(clean.unknown, []);

// Unknown glyphs are reported where they sit in the output, in UTF-16 units.
const withUnknown = kannadi.convertLegacy("PÀ£ÀßqÀ õ ¥Àæ õ");
assert.equal(withUnknown.unknown.length, 2);
for (const at of withUnknown.unknown) assert.equal(withUnknown.text[at], "õ");

assert.equal(kannadi.convertLegacy("1956", { kannadaDigits: true }).text, "೧೯೫೬");
assert.equal(kannadi.convertLegacy("¥ÁægÀ0¨sÀ").text, "ಪ್ರಾರಂಭ");
assert.equal(kannadi.convertLegacy("¥ÁægÀ0¨sÀ", { zeroAsAnusvara: false }).text, "ಪ್ರಾರ0ಭ");

// From a 2002 RDPR page: an unknown glyph splits ಙ from its virama.
const broken = "ರಾಜಕೀಯ ಪ್ರಙõ್ ಬೆಳೆಸುವ";
const ranges = Array.from(kannadi.malformed(broken));
assert.deepEqual(ranges, [broken.indexOf("õ"), broken.indexOf("õ") + 2]);
const fixes = kannadi.fixes(broken, ranges[0], ranges[1]);
assert.equal(fixes[0].kind, "join");
const fixed = broken.slice(0, fixes[0].start) + fixes[0].replacement + broken.slice(fixes[0].end);
assert.equal(fixed, "ರಾಜಕೀಯ ಪ್ರಙ್ ಬೆಳೆಸುವ");
assert.deepEqual(Array.from(kannadi.malformed(fixed)), []);

assert.equal(kannadi.countAksharas("ಕನ್ನಡ ನಾಡು, 2026"), 5);
assert.equal(kannadi.modernize("ಬಾೞ್"), "ಬಾಳ್");
assert.equal(kannadi.normalize("ಕೀ"), "ಕೀ");
assert.equal(kannadi.transliterate("kannaDa", "baraha", "kannada"), "ಕನ್ನಡ");
assert.throws(() => kannadi.transliterate("x", "klingon", "kannada"), /unknown scheme/);
assert.equal(kannadi.decodeWindows1252(new Uint8Array([0x50, 0xc0, 0x86])), "PÀ†");

// Opening files end to end, with the real converter.
const io = { convert: (t) => kannadi.convertLegacy(t), decodeWindows1252: kannadi.decodeWindows1252 };
const bytes = new Uint8Array([...NUDI].map((c) => c.charCodeAt(0) < 256 ? c.charCodeAt(0) : 0x3f));
const txt = await openFile(new File([bytes], "page.txt"), io);
assert.equal(txt.text, UNICODE);
assert.equal(txt.converted, NUDI.length);

const W = 'xmlns:w="http://schemas.openxmlformats.org/wordprocessingml/2006/main"';
const docx = await writeZip([
  [
    "word/document.xml",
    `<w:document ${W}><w:body><w:p><w:r><w:t xml:space="preserve">Report: </w:t></w:r>` +
      `<w:r><w:rPr><w:rFonts w:ascii="BRH Kannada"/></w:rPr><w:t>${NUDI.slice(0, 9)}</w:t></w:r>` +
      `<w:r><w:rPr><w:rFonts w:ascii="BRH Kannada"/></w:rPr><w:t>${NUDI.slice(9)} õ</w:t></w:r></w:p></w:body></w:document>`,
  ],
]);
const opened = await openFile(new File([docx], "report.docx"), io);
assert.equal(opened.text, `Report: ${UNICODE} õ`);
assert.deepEqual(opened.unknown, [opened.text.indexOf("õ")]);

// The page references only files the build produced.
const html = readFileSync(join(dist, "index.html"), "utf8");
for (const [, ref] of html.matchAll(/(?:href|src)="([^":#]+)"/g)) readFileSync(join(dist, ref.endsWith("/") ? `${ref}index.html` : ref));
const css = readFileSync(join(dist, "style.css"), "utf8");
for (const [, ref] of css.matchAll(/url\(([^)]+)\)/g)) readFileSync(join(dist, ref));
assert.match(html, /<h1>Kannada editor and Nudi to Unicode converter<\/h1>/);

console.log("web smoke test passed");
