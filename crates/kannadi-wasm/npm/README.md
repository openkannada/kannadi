# kannadi

Convert Kannada text typed in the legacy Nudi, Baraha, Akruti, Shree-Lipi, Nudi B-Akshara and Kan-Eng fonts to Unicode and back, in Node and in the browser. This is the [kannadi](https://github.com/openkannada/kannadi) Rust library compiled to WebAssembly, the same code that runs the [online editor](https://openkannada.org/editor/).

```sh
npm install kannadi
```

In Node 18 or later, the module is ready on import:

```js
import { toUnicode, unicodeToNudi, bestLayout, fontLayout, NudiOptions } from "kannadi";

const c = toUnicode("PÀ£ÀßqÀ", "nudi", new NudiOptions()); // text copied from a Nudi or Baraha document
console.log(c.text, c.isClean);       // ಕನ್ನಡ true
console.log(bestLayout("¸ÀPÁðj"));    // nudi
console.log(fontLayout("BRH Kannada")); // nudi
console.log(unicodeToNudi("ಕನ್ನಡ").text); // PÀ£ÀßqÀ, to set in a Nudi font
```

In the browser, or with a bundler such as Vite or webpack, call the default export once to load the WebAssembly file:

```js
import init, { toUnicode, NudiOptions } from "kannadi";

await init();
toUnicode("PÀ£ÀßqÀ", "nudi", new NudiOptions()).text; // ಕನ್ನಡ
```

Layouts are `"nudi"` (also `"baraha"`), `"akruti"`, `"shree-lipi"`, `"nudi-b"` and `"kan-eng"`. A conversion keeps glyphs it cannot place and lists their positions in `unknown` (in the input) and `unknownInText` (in the output), in UTF-16 units, so they index JavaScript strings directly. `NudiOptions` sets `kannadaDigits` and `zeroAsAnusvara`. TypeScript types are included.

Licensed under MIT or Apache-2.0, at your option.

Part of [Open Kannada](https://github.com/openkannada), free and open-source tools that make Kannada work properly on computers.
