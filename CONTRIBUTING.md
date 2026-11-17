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
