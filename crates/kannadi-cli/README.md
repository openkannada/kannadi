# kannadi-cli

The `kannadi` command converts Kannada documents typed in the legacy Nudi, Baraha ANSI, Nudi B-Akshara, Kan-Eng, Akruti and Shree-Lipi fonts to Unicode, and Unicode back to Nudi. It converts single files or whole folders of TXT, HTML, DOCX and PDF files, and writes a report of anything it could not convert cleanly next to each file.

```sh
cargo install --locked kannadi-cli
kannadi to-unicode old-orders/ --out converted/
kannadi to-nudi letter.txt --out for-printing/ --strict
```

- `kannadi --help` and `kannadi to-unicode --help` list every option, with examples.
- Exit status: 0 on success, even when reports list problems or likely typos; 1 with `--strict` when a file has a problem; 2 for bad arguments or a file or folder that could not be read or written.
- `kannadi completions bash` (or `zsh`, `fish`, `powershell`, `elvish`) prints a completion script, and `kannadi manpages DIR` writes the man pages.

The conversion rules live in the [kannadi](https://crates.io/crates/kannadi) library. The [project README](https://github.com/openkannada/kannadi#readme) has the measured results, the other ways to install, and how each file format is handled.

Licensed under MIT or Apache-2.0, at your option.
