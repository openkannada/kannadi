# kannadi-typing

Type Kannada in English letters. `kannadi-typing` turns loosely typed Latin letters into ranked Kannada words: "kannada", "kannda" and "Kannada" all give ಕನ್ನಡ first, and "til" offers ತಿಳಿ. It learns from the user's picks in memory, and runs in the browser as WebAssembly with no network access.

```sh
$ kannadi type kannada kannda til
kannada	ಕನ್ನಡ	ಕನ್ನಡದ	ಕನ್ನಡಿ	ಕಣ್ಣದ	ಕನ್ನಡ್
kannda	ಕನ್ನಡ	ಕನ್ಯಾ	ಕನ್ನಡಿ	ಕ್ಯಾನಡ	ಕಣ್ಣದಾಸನ್
til	ಟಿಲ್	ತಿಳಿ	ತಿಳಿದು	ಟೈಲ್	ತಿಲೈ
```

```rust
let mut engine = kannadi_typing::Engine::bundled();
assert_eq!(engine.suggest("kannada", None, 3)[0], "ಕನ್ನಡ");
engine.learn("til", "ತಿಳಿ"); // ತಿಳಿ now comes first for "til"
let saved = engine.export_user(); // keep it wherever the app keeps settings
```

## Results

Top-1 and top-5 accuracy for English letters to Kannada on the [Dakshina](https://github.com/google-research-datasets/dakshina) Kannada test set, scored the way [IndicXlit](https://github.com/AI4Bharat/IndicXlit) scores it: each distinct romanization is 1 item, and a prediction is right when it matches any native word the test set gives for that romanization.

| System | Top-1 | Top-5 |
|---|---:|---:|
| IndicXlit, as published | 77.18% | not published |
| kannadi-typing | **88.09%** | **94.65%** |
| kannadi-typing, letters model only (no lexicon) | 70.84% | 73.75% |
| kannadi-typing, with the 2,500 test words removed from the lexicon | 60.48% | 72.07% |

5,013 items. Times are for 1 call on an Apple M1 Pro.

| Test | Items | Top-1 | Top-5 | Median | 99th percentile |
|---|---:|---:|---:|---:|---:|
| Dakshina test words | 5,013 | 88.09% | 94.65% | 0.87 ms | 1.73 ms |
| Dakshina test sentences, each word after the word before it | 52,328 | 86.94% | 91.74% | 0.80 ms | 2.09 ms |
| The same sentences, without the previous word | 52,328 | 86.90% | 91.72% | 0.81 ms | 2.70 ms |

In the browser, as WebAssembly in Node 24 on the same machine, every keystroke of every test word (49,676 calls) took 1.28 ms at the median, 2.84 ms at the 95th percentile and 3.51 ms at the 99th. Loading the model takes 75 ms.

| File | Size | Gzipped |
|---|---:|---:|
| `kannadi_typing_wasm_bg.wasm` | 451,564 bytes | 165,730 bytes |
| `model/kn.typing` | 2,673,232 bytes | 1,612,760 bytes |

What the numbers mean:

- The Dakshina test words were sampled from Kannada Wikipedia, and the lexicon is the list of Kannada Wikipedia words seen at least 5 times, so it holds 98.8% of them. IndicXlit's own evaluation script also has an option to rerank its candidates by corpus word frequency. The row with the test words removed from the lexicon shows what happens to words the lexicon lacks: the letters model still finds 60% of them, but lexicon words that look alike take the first place more often.
- No Dakshina dev or test word is in the training pairs. The ranking weights were tuned on the dev set only.
- The previous word adds 0.04 points. Most remaining errors are spellings the test set itself gives inconsistently (ಆದರ್ಶ and ಆದರ್ಶ್, ಐದನೆ and ಐದನೇ) and names and English words missing from the lexicon.
- A completion never takes the first place, so the first word is always what the letters spell. With 2 letters still to type, the intended word is in the first 5 for 64% of the dev words.

Reproduce the tables with `DATA=... crates/kannadi-typing/train.sh` after fetching the training data, below. CI checks the shipped model against the test set on every change.

## How it works

1. **Units.** A Kannada word is split into sound units: a vowel letter, a consonant that carries a vowel, a consonant with a virama, a vowel sign, the inherent a, or a sign such as the anusvara. The inherent a is a unit because typists spell it ("kannada") or drop it ("kannda").
2. **Alignment.** EM (forward-backward) learns which runs of 0 to 3 Latin letters pair with each unit, from 2.9 million transliteration pairs. Only the inherent a and the anusvara, visarga and candrabindu may take no letters.
3. **Letters model.** A 4-gram Kneser-Ney model over (letters, unit) pairs, pruned to 184,000 n-grams and stored as a trie with 8-bit log probabilities.
4. **Search.** A beam search reads the letters left to right. 1 pass follows a lexicon automaton (an [`fst`](https://crates.io/crates/fst) of 232,003 words with their frequency class), so only known words come out; another pass has no lexicon, for words it lacks. The best partial readings in the lexicon are also completed to the commonest word they start, and to every word 1 unit longer.
5. **Rules.** The strict Baraha and ITRANS readings of the input are always candidates. Capitals inside a word, as in `kannaDa` or `kRRiShNa`, say exactly which letter is meant, so these readings come first then.
6. **Ranking.** A candidate's score adds the letters model's log probability, its frequency in Kannada Wikipedia, whether the lexicon has it, how much likelier the previous word makes it, whether a rule reading gives it, and how often the user picked it. The weights are tuned by coordinate ascent on the Dakshina dev words and the first 10,000 words of the dev sentences.
7. **Learning.** `learn(input, word)` puts the word first for the same letters from then on, newest pick first, and ranks the word higher for other letters. A picked word the lexicon lacks joins a small lexicon of the user's own. Up to 2,000 picks and 5,000 words are kept in memory; `export_user` and `import_user` move them to and from the app's own storage as text. Nothing is sent anywhere.

## In the browser

`crates/kannadi-typing-wasm/build.sh` builds the WebAssembly package and copies the model next to it. The page fetches the model on its own, so the browser can cache it apart from the code.

```js
import init, { TypingEngine } from "./kannadi_typing_wasm.js";
await init();
const model = new Uint8Array(await (await fetch("kn.typing")).arrayBuffer());
const engine = new TypingEngine(model);
engine.suggest("kannada", undefined, 5); // ["ಕನ್ನಡ", "ಕನ್ನಡದ", ...]
engine.suggest("hesaru", "ನನ್ನ", 5);     // with the word before the cursor
engine.learn("til", "ತಿಳಿ");
localStorage.setItem("kannadi-typing", engine.exportUser());
engine.importUser(localStorage.getItem("kannadi-typing") ?? "");
```

`node crates/kannadi-typing-wasm/test/smoke.mjs` checks the package and times every keystroke of the test words.

## Training data

| Data | Used for | License |
|---|---|---|
| [Aksharantar](https://huggingface.co/datasets/ai4bharat/Aksharantar) Kannada training split, 2,906,728 pairs (AI4Bharat) | Alignment and the letters model. 97% are pairs mined from IndicCorp, which Aksharantar releases under CC0; the rest come from Dakshina, Samanantar and Wikidata. | CC0 1.0 and CC BY 4.0 |
| [Dakshina](https://github.com/google-research-datasets/dakshina) Kannada lexicons (Google), train split | More training pairs, typed by people | CC BY-SA 4.0 |
| Dakshina Kannada dev and test lexicons | Tuning and testing; copied in `eval/` | CC BY-SA 4.0 |
| Dakshina Kannada Wikipedia sentences, train split | The 30,000 word pairs for the previous word | CC BY-SA 4.0 |
| Kannada Wikipedia word counts: the article text of `knwiki-latest-pages-articles.xml.bz2` of 16 November 2026, 9 of every 10 articles | The lexicon and its frequencies | Counts of Wikipedia text, CC BY-SA 4.0 |

Pairs whose Kannada word is in the Dakshina dev or test lexicon are removed before training. `model/kn.typing` is derived from all of the above, so it is licensed under [CC BY-SA 4.0](../../LICENSES/CC-BY-SA-4.0.txt), with credit to AI4Bharat (Aksharantar), Google (Dakshina) and the contributors to Kannada Wikipedia. The code is under MIT or Apache 2.0, like the rest of kannadi.

To rebuild the model, put these files in a folder and run `DATA=folder crates/kannadi-typing/train.sh`:

- `aksharantar/kan_train.json` from `kan.zip` on the Aksharantar page;
- `dakshina/dakshina_dataset_v1.0/kn/` from the Dakshina archive (lexicons, romanized, and native_script_wikipedia);
- `wiki-sentences.tsv`, the decompressed `kn.wiki-filt.train.text.sorted.tsv.gz` from that folder;
- `wiki-train.tsv`, the Wikipedia word counts: 1 word, a tab and its count per line; lines starting with # are comments.

Building takes 45 seconds on 10 cores and about 3 GB of memory; tuning and testing take 2 minutes more.

Prior art consulted for method only: [Varnam](https://github.com/varnamproject/govarnam) (its Kannada scheme and its idea of a local table of the user's words) and IndicXlit. No code or data was copied from either.
