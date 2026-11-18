# Contributing

Bug reports with a sample file are the most useful contribution. If a document converts wrongly, open an issue with a short excerpt, the font name it was typed in, and what the text should say. Every confirmed error becomes a test.

For code changes, read [AGENTS.md](AGENTS.md). It has the commands, the architecture and the rules this repository follows. In short:

- Branch from `main` and open a pull request.
- Every behavior change comes with a test.
- Glyph tables are derived from font files and real documents. Do not copy tables or code from other converters.
- Run the commands listed in AGENTS.md before asking for review.

## Sign off your commits

Every commit needs a `Signed-off-by` line with your name and email. It says that you wrote the change, or have the right to submit it, under the [Developer Certificate of Origin](https://developercertificate.org/). Git adds it for you:

```sh
git commit -s
```

There is no contributor license agreement. You keep the copyright in your contribution, and it is licensed like the files it changes: MIT OR Apache-2.0 for code and tables, CC BY 4.0 for docs. A check on each pull request fails if a commit has no `Signed-off-by` line. To add the line to commits already on your branch, run `git rebase --signoff origin/main`.

## Releasing

Releases are built by `.github/workflows/release.yml`, which [dist](https://opensource.axo.dev/cargo-dist/) generates from `dist-workspace.toml`. Run `dist generate` after changing that file.

1. Set the new version in the root `Cargo.toml`, and in the `kannadi` dependency of `crates/kannadi-cli/Cargo.toml`.
2. Run `KANNADI_BLESS=1 cargo test -p kannadi-cli` to rewrite the man pages and completion scripts in `crates/kannadi-cli/assets`, which carry the version.
3. Give the changes under `Unreleased` in `CHANGELOG.md` a heading with the version and date.
4. Push a tag such as `v0.1.0`. The workflow builds the archives for macOS, Linux and Windows with their checksums and the install scripts, creates the GitHub Release, and writes the Homebrew formula to `openkannada/homebrew-tap`. That last step needs a `HOMEBREW_TAP_TOKEN` secret that can push to the tap, and the tap needs at least 1 commit. A tag with a suffix, such as `v0.2.0-rc.1`, makes a prerelease and leaves the tap alone.
5. Once the release is out, publish the crates: `cargo publish -p kannadi`, then `cargo publish -p kannadi-cli`. A version on crates.io cannot be replaced, so it goes last.

dist 0.34.0 writes `uses: $/.github/workflows/publish-homebrew.yml` in `release.yml`, which GitHub rejects. After running `dist generate`, change that line to `uses: ./.github/workflows/publish-homebrew.yml`. `allow-dirty = ["ci"]` in `dist-workspace.toml` lets dist accept the edit, and also stops `dist generate` from rewriting `release.yml`: remove it for the run, then put it back.

`pr-run-mode` in `dist-workspace.toml` is `plan`, so pull requests only check the release plan. Set it to `upload` in a pull request to build every archive as a dry run.
