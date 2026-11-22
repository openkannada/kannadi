# kannadi for Python

Convert Kannada text typed in the legacy Nudi, Baraha, Akruti, Shree-Lipi, Nudi B-Akshara and Kan-Eng fonts to Unicode and back, and split, normalize and transliterate Kannada. These are bindings to the [kannadi](https://github.com/openkannada/kannadi) Rust library, so they convert exactly as the `kannadi` command and the [online editor](https://openkannada.org/editor/) do.

```sh
pip install kannadi
```

```python
import kannadi

c = kannadi.to_unicode("PÀ£ÀßqÀ")          # text copied from a Nudi or Baraha document
print(c.text, c.is_clean)                  # ಕನ್ನಡ True

kannadi.best_layout("¸ÀPÁðj")              # 'nudi'
kannadi.font_layout("BRH Kannada")         # 'nudi'
kannadi.from_unicode("ಕನ್ನಡ").text         # 'PÀ£ÀßqÀ', to set in a Nudi font
kannadi.aksharas("ಕನ್ನಡ")                  # ['ಕ', 'ನ್ನ', 'ಡ']
kannadi.transliterate("ಕನ್ನಡ", "kannada", "iso15919")  # 'kannaḍa'
kannadi.normalize("ಕೊ")                    # the canonical form, so text that renders the same compares equal
```

`to_unicode` takes the layout as its second argument (`"nudi"`, `"akruti"`, `"shree-lipi"`, `"nudi-b"` or `"kan-eng"`; `"baraha"` is the Nudi layout). Glyphs it cannot place stay in the text and are listed in `unknown` as `(index, glyph)`, with the index into the input. `malformed` lists `(start, end)` ranges of the output that are not valid syllables. Wheels are built for CPython 3.10 and later (abi3).

The library, its tables and the evidence for every glyph are in the [kannadi repository](https://github.com/openkannada/kannadi). Licensed under MIT or Apache-2.0, at your option.

Part of [Open Kannada](https://github.com/openkannada), free and open-source tools that make Kannada work properly on computers.
