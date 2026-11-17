// Checks the built page package: node web/test/smoke.mjs [dist directory]
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { cpSync, existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
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
assert.equal(kannadi.convertLegacy("dâ}â", { layout: "akruti" }).text, "ಕನ");
assert.equal(kannadi.convertLegacy("dâ}â").text, "ಕನ");
assert.equal(kannadi.convertLegacy(NUDI).text, UNICODE);
assert.throws(() => kannadi.convertLegacy("PÀ", { layout: "klingon" }), /unknown layout/);
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

// Unicode to Nudi, and the bytes of a Nudi text file.
const nudi = kannadi.toNudi(UNICODE);
assert.equal(nudi.text, NUDI);
assert.deepEqual([nudi.unencodable, nudi.malformed], [[], []]);
const mixed = kannadi.toNudi("ಕ A");
assert.deepEqual(mixed.unencodable.map((o) => "ಕ A"[o]), ["A"]);
assert.equal(kannadi.decodeWindows1252(kannadi.encodeWindows1252(NUDI)), NUDI);

// Opening files end to end, with the real converter.
const io = {
  convert: (t, layout) => kannadi.convertLegacy(t, { layout }),
  bestLayout: kannadi.bestLayout,
  fontLayout: kannadi.fontLayout,
  decodeWindows1252: kannadi.decodeWindows1252,
};

// Font names follow the crate's rules.
assert.equal(kannadi.fontLayout("BRH Kannada"), "nudi");
assert.equal(kannadi.fontLayout("AkrutiKndPadma,Bold"), "akruti");
for (const name of ["Nudi Parijatha", "BRH Devanagari", "BRH Telugu", "Arial"]) {
  assert.equal(kannadi.fontLayout(name), null, name);
  assert.ok(!kannadi.isLegacyFont(name), name);
}
assert.equal(kannadi.fontLayout("Shree-Kan-0850"), "shree-lipi");
assert.ok(kannadi.isLegacyFont("Shree-Kan-0851"));
assert.equal(kannadi.fontLayout("Shree-Kan-0851"), null);
assert.equal(kannadi.convertLegacy("PÜ®Ü°vÜ", { layout: "shree-lipi" }).text, "ಕನ್ನಡ");
assert.equal(kannadi.fontLayout("Kan-EngPraja"), "kan-eng");
assert.equal(kannadi.convertLegacy("ÉÜá°Üáè", { layout: "kan-eng" }).text, "ಮತ್ತು");
assert.ok(kannadi.drawsKannadaDigits("Nudi 04 k"));
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
assert.deepEqual(opened.layouts, ["nudi"]);

// A short Akruti run reads cleanly as Nudi too; its font decides.
const akrutiDocx = await writeZip([
  [
    "word/document.xml",
    `<w:document ${W}><w:body><w:p><w:r><w:rPr><w:rFonts w:ascii="AkrutiKndPadmaDemi"/></w:rPr>` +
      "<w:t>Eyâpâ</w:t></w:r></w:p></w:body></w:document>",
  ],
]);
const akruti = await openFile(new File([akrutiDocx], "act.docx"), io);
assert.equal(akruti.text, "ಇತರ");
assert.deepEqual(akruti.layouts, ["akruti"]);
assert.equal(kannadi.convertLegacy("Eyâpâ").text, "ಇತರ");
assert.equal(kannadi.convertLegacy("%zâÀÖ").text, "ಅಥವಾ");

// Every page references only files the build produced, from its own folder.
const { PAGES } = await import(pathToFileURL(resolve(dirname(fileURLToPath(import.meta.url)), "../site/pages.mjs")).href);
const css = readFileSync(join(dist, "style.css"), "utf8");
for (const [, ref] of css.matchAll(/url\(([^)]+)\)/g)) readFileSync(join(dist, ref));
const pageHtml = (path) => readFileSync(join(dist, path, "index.html"), "utf8");
for (const page of PAGES) {
  const html = pageHtml(page.path);
  for (const [, ref] of html.matchAll(/(?:href|src)="([^":#]+)"/g)) {
    readFileSync(join(dist, page.path, ref.endsWith("/") ? `${ref}index.html` : ref));
  }
  assert.ok(html.includes(`<h1>${page.h1}</h1>`), page.path);
  const blocks = [...html.matchAll(/<script type="application\/ld\+json">([\s\S]*?)<\/script>/g)].map((m) => JSON.parse(m[1]));
  const faq = blocks.find((b) => b["@type"] === "FAQPage");
  const marked = page.faq.filter((f) => !f.shared || page.path === "");
  assert.deepEqual(faq.mainEntity.map((e) => e.name), marked.map((f) => f.q), "shared answers are marked up on the home page only");
  for (const e of faq.mainEntity) assert.ok(!/[<>]/.test(e.acceptedAnswer.text), `${page.path}: ${e.acceptedAnswer.text}`);
  assert.ok(page.description.length <= 160, `${page.path} description is ${page.description.length} characters`);
  assert.ok(!html.includes('property="og:image"'), "no relative og:image without a site URL");
  const visible = html.slice(html.indexOf('id="faq-heading"')).match(/<h3>/g).length;
  assert.equal(visible, page.faq.length, page.path);
  assert.ok(blocks.some((b) => b["@type"] === "SoftwareApplication"), page.path);
  assert.equal(html.includes('data-default-mode="baraha"'), page.mode === "baraha", page.path);
  assert.ok(!html.includes('rel="canonical"'), "no canonical link without a site URL");
  assert.ok(existsSync(join(dist, "og", `${page.path ? page.path.slice(0, -1) : "home"}.png`)), page.path);
}
assert.match(pageHtml("kannada-typing/"), /<li><span class="chart-kn" lang="kn">ಟ<\/span><kbd>Ta<\/kbd>/, "the letter chart is computed by the transliterator");
assert.match(pageHtml("english-to-kannada-typing/"), /<td lang="kn">ನಮಸ್ಕಾರ<\/td><td><kbd>namaskAra<\/kbd>/);
assert.ok(!pageHtml("english-to-kannada-typing/").includes('class="chart"'), "each typing page has its own table");
const llms = readFileSync(join(dist, "llms.txt"), "utf8");
assert.ok(llms.includes("[Unicode to Nudi converter](unicode-to-nudi/)") && llms.includes("Nudi text"));
assert.ok(!/not available/i.test(llms) && !llms.includes("]()"));
assert.ok(!existsSync(join(dist, "sitemap.xml")));

// With a site URL, pages get canonical links and the site gets a sitemap.
const copy = mkdtempSync(join(tmpdir(), "kannadi-site-"));
try {
  cpSync(dist, copy, { recursive: true });
  const site = fileURLToPath(new URL("../site/build.mjs", import.meta.url));
  // Rebuilding over built pages gives the same bytes.
  execFileSync(process.execPath, [site, copy], { stdio: "ignore" });
  for (const page of PAGES) assert.equal(readFileSync(join(copy, page.path, "index.html"), "utf8"), pageHtml(page.path), page.path);
  for (const bad of ["http://kannadi.example/", "https://kannadi.example/?x=1", "https://user@kannadi.example/"]) {
    assert.throws(() => execFileSync(process.execPath, [site, copy, bad], { stdio: "ignore" }), bad);
  }
  execFileSync(process.execPath, [site, copy, "https://kannadi.example/tools/"], { stdio: "ignore" });
  const sitemap = readFileSync(join(copy, "sitemap.xml"), "utf8");
  for (const page of PAGES) assert.ok(sitemap.includes(`<loc>https://kannadi.example/tools/${page.path}</loc>`), page.path);
  assert.match(readFileSync(join(copy, "robots.txt"), "utf8"), /Sitemap: https:\/\/kannadi\.example\/tools\/sitemap\.xml/);
  const typing = readFileSync(join(copy, "kannada-typing", "index.html"), "utf8");
  assert.ok(typing.includes('<link rel="canonical" href="https://kannadi.example/tools/kannada-typing/">'));
  assert.ok(typing.includes('content="https://kannadi.example/tools/og/kannada-typing.png"'));
} finally {
  rmSync(copy, { recursive: true, force: true });
}

// The page takes its colors from the Open Kannada tokens: tokens.css loads first, every color in it is
// 1 light-dark() pair, and style.css only maps its own names onto those tokens.
const html = readFileSync(join(dist, "index.html"), "utf8");
assert.ok(html.indexOf('href="tokens.css"') > 0 && html.indexOf('href="tokens.css"') < html.indexOf('href="style.css"'), "tokens.css loads before style.css");
const designTokens = readFileSync(join(dist, "tokens.css"), "utf8");
assert.match(designTokens, /--primary: light-dark\(oklch\(/);
assert.match(designTokens, /:root\[data-theme="dark"\] \{\s*color-scheme: dark;/);
const pageRoot = css.match(/:root \{([^}]*)\}/)[1];
assert.doesNotMatch(pageRoot, /#[0-9a-f]{3,8}\b|rgb\(|oklch\(\s*\d/i, "style.css defines no colors of its own");
assert.doesNotMatch(css, /prefers-color-scheme: dark/, "dark mode comes from the tokens' light-dark() pairs");
const defined = new Set([...`${designTokens}\n${css}`.matchAll(/(--[\w-]+)\s*:/g)].map((m) => m[1]));
const undefinedVars = [...new Set([...css.matchAll(/var\((--[\w-]+)/g)].map((m) => m[1]))].filter((v) => !defined.has(v));
assert.deepEqual(undefinedVars, [], "every custom property style.css uses is defined");
const fallback = css.match(/@supports not \(color: light-dark\(#000, #fff\)\) \{\s*:root \{([^}]*)\}/)?.[1] ?? "";
for (const name of ["--primary", "--card", "--foreground", "--mark-unknown-line", "--mark-malformed"]) {
  assert.match(fallback, new RegExp(`${name}: #[0-9a-f]{6,8};`), `browsers without light-dark() get a plain ${name}`);
}

console.log("web smoke test passed");
