// Tests the packed npm package: KANNADI_TGZ=kannadi-x.y.z.tgz node --test crates/kannadi-wasm/npm/test/node.test.mjs
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtempSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import { after, test } from "node:test";

const tarball = resolve(process.env.KANNADI_TGZ ?? assert.fail("set KANNADI_TGZ to the packed .tgz"));
const dir = mkdtempSync(join(tmpdir(), "kannadi-npm-"));
after(() => rmSync(dir, { recursive: true, force: true }));
writeFileSync(join(dir, "package.json"), '{ "type": "module" }');
execFileSync("npm", ["install", "--no-audit", "--no-fund", "--offline", tarball], { cwd: dir, stdio: "ignore" });
const kannadi = await import(join(dir, "node_modules", "kannadi", "node.mjs"));
const viaName = execFileSync("node", ["-e", 'import("kannadi").then(k => console.log(k.bestLayout("PÀ£ÀßqÀ")))'], { cwd: dir }).toString().trim();

const NUDI = "¸ÀPÁðj ¥ÁæxÀ«ÄPÀ ±Á¯ÉUÀ¼À°è ²PÀëPÀjUÉ vÀgÀ¨ÉÃw ¤ÃqÀÄªÀÅzÀÄ.";
const UNICODE = "ಸರ್ಕಾರಿ ಪ್ರಾಥಮಿಕ ಶಾಲೆಗಳಲ್ಲಿ ಶಿಕ್ಷಕರಿಗೆ ತರಬೇತಿ ನೀಡುವುದು.";

test("the package name resolves to the Node entry", () => {
  assert.equal(viaName, "nudi");
});

test("converts a real Nudi line and back", () => {
  const c = kannadi.toUnicode(NUDI, "nudi", new kannadi.NudiOptions());
  assert.equal(c.text, UNICODE);
  assert.equal(c.isClean, true);
  assert.equal(kannadi.toUnicode(kannadi.unicodeToNudi(UNICODE).text, "nudi", new kannadi.NudiOptions()).text, UNICODE);
});

test("reports unknown glyphs in UTF-16 units of the output", () => {
  const c = kannadi.toUnicode("PÀ£ÀßqÀ õ ¥Àæ õ", "baraha", new kannadi.NudiOptions());
  assert.equal(c.unknownInText.length, 2);
  for (const at of c.unknownInText) assert.equal(c.text[at], "õ");
});

test("finds layouts from fonts and text, and rejects unknown layouts", () => {
  assert.equal(kannadi.fontLayout("BRH Kannada"), "nudi");
  assert.equal(kannadi.fontLayout("Noto Sans Kannada"), undefined);
  assert.equal(kannadi.bestLayout(NUDI), "nudi");
  assert.throws(() => kannadi.toUnicode("x", "devanagari", new kannadi.NudiOptions()), /unknown layout/);
});
