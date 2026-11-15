# Claude Code

@AGENTS.md

AGENTS.md, imported above, is the canonical guide for this repository.

Claude-specific notes:

- Font files and downloaded documents go in a scratch directory outside the repository. Commit only derived data, with its source recorded.
- Before claiming a converter or transliteration change works, run the full test suite and the long property tests listed in AGENTS.md.
- Use a fresh-context subagent for the independent review of each pull request. Give it the diff, not your conclusions.
