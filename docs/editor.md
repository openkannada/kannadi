# The editor page in this repository

The editor at [openkannada.org/editor](https://openkannada.org/editor/) runs this library as WebAssembly. `web/` is a standalone build of the editor page that any static host can serve.

[web/](../web) is a static Kannada editor that runs in the browser. The crate runs as WebAssembly through the bindings in [crates/kannadi-wasm](../crates/kannadi-wasm), so text never leaves the device.

- Paste text, or open a TXT, DOCX or HTML file. Nudi, Baraha, Nudi B-Akshara, Kan-Eng, Akruti and Shree-Lipi text is converted to Unicode. In DOCX files, only text set in one of their fonts is converted; in HTML files, also text with no font that looks like legacy codes.
- Unknown glyphs and malformed syllables are marked in the text and listed beside it. Selecting one offers checked fixes, or a replacement typed in Baraha letters.
- Type Kannada with Baraha or ISO 15919 letters.
- Normalize, modernize Old Kannada letters, and count aksharas and words.
- Save as DOCX, TXT or HTML, or as Nudi text for documents that must still be set in a legacy font. The last document is kept in the browser's local storage.

The build writes 6 pages from 1 template: the editor home, Nudi to Unicode, Unicode to Nudi, Baraha to Unicode, Kannada typing, and English to Kannada typing. Each page has its own title, definition, how-to, FAQ and JSON-LD; the content is in [web/site/pages.mjs](../web/site/pages.mjs). The typing pages' letter charts are computed by the crate's transliterator at build time. Set `KANNADI_SITE_URL` to the site's address to add canonical links, `og:url` and a sitemap; without it they are left out. `node web/site/og.mjs` renders the Open Graph cards with Chrome.

Build and run it locally. It needs the `wasm32-unknown-unknown` target and the `wasm-bindgen` command at the version in `Cargo.lock`:

```sh
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli --version 0.2.130 --locked
web/build.sh
python3 web/serve.py 8000
```

`web/dist` is a plain folder of static files that any static host can serve. `web/serve.py` is only for local testing: the standard `python3 -m http.server` drops some of the page's parallel requests. `node --test web/test/units.mjs` tests the page modules, and `node web/test/smoke.mjs` checks the built package.
