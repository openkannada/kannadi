// Checks the built package and times every keystroke of the Dakshina test words:
// node crates/kannadi-typing-wasm/test/smoke.mjs [build directory]
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath, pathToFileURL } from "node:url";

const here = fileURLToPath(new URL(".", import.meta.url));
const dir = resolve(process.argv[2] ?? join(here, "../../../target/typing-web"));
const wasm = await import(pathToFileURL(join(dir, "kannadi_typing_wasm.js")).href);
wasm.initSync({ module: readFileSync(join(dir, "kannadi_typing_wasm_bg.wasm")) });

let t = performance.now();
const engine = new wasm.TypingEngine(readFileSync(join(dir, "kn.typing")));
const loadMs = performance.now() - t;

assert.throws(() => new wasm.TypingEngine(new Uint8Array([1, 2, 3])));
assert.equal(engine.suggest("kannada", undefined, 5)[0], "ಕನ್ನಡ");
assert.equal(engine.suggest("kannda", undefined, 5)[0], "ಕನ್ನಡ");
assert.equal(engine.suggest("kannaDa", undefined, 5)[0], "ಕನ್ನಡ");
assert.ok(engine.suggest("til", undefined, 5).includes("ತಿಳಿ"));
assert.deepEqual(engine.suggest("a1", undefined, 5), []);

// Learning moves a pick to the front and survives export and import.
const before = engine.suggest("til", undefined, 5);
assert.notEqual(before[0], "ತಿಳಿ");
assert.ok(engine.learn("til", "ತಿಳಿ"));
assert.equal(engine.suggest("til", undefined, 5)[0], "ತಿಳಿ");
const saved = engine.exportUser();
const fresh = new wasm.TypingEngine(readFileSync(join(dir, "kn.typing")));
assert.ok(fresh.importUser(saved));
assert.equal(fresh.suggest("til", undefined, 5)[0], "ತಿಳಿ");
assert.ok(!fresh.importUser("not an export"));
assert.equal(fresh.suggest("til", undefined, 5)[0], "ತಿಳಿ");
fresh.forget("ತಿಳಿ");
assert.deepEqual(fresh.suggest("til", undefined, 5), before);
assert.ok(!engine.learn("til", "tili"));

// Every keystroke of every test word, as typed.
const words = readFileSync(join(here, "../../kannadi-typing/eval/dakshina-kn-test.tsv"), "utf8")
  .split("\n")
  .map((l) => l.split("\t")[1])
  .filter((w) => w && /^[a-zA-Z]+$/.test(w));
for (let i = 0; i < 200; i++) engine.suggest(words[i % words.length], undefined, 5);
const times = [];
for (const w of words) {
  for (let k = 1; k <= w.length; k++) {
    t = performance.now();
    engine.suggest(w.slice(0, k), undefined, 5);
    times.push(performance.now() - t);
  }
}
times.sort((a, b) => a - b);
const q = (p) => times[Math.min(times.length - 1, Math.floor(times.length * p))].toFixed(2);
console.log(`model load ${loadMs.toFixed(0)} ms; ${times.length} keystrokes: median ${q(0.5)} ms, 95th ${q(0.95)} ms, 99th ${q(0.99)} ms, max ${q(1)} ms`);
const limit = Number(process.env.KEYSTROKE_MS_LIMIT ?? 5);
assert.ok(Number(q(0.5)) < limit, `median keystroke ${q(0.5)} ms is over ${limit} ms`);
console.log("typing smoke test passed");
