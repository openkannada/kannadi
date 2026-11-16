# Editor page design

The page is a tool: people convert old Nudi documents and type Kannada. The design must keep that task clear. The source of truth is `src/style.css`; this note says why it looks the way it does.

## World

KSRTC bus route boards: bottle-green enamel boards with white lettering, Kannada set large and English small beneath it, a Karnataka-yellow route disc, and stops drawn as dots on a line.

- The **board** (top bar and status bar) carries the name, the message panel and the document actions.
- The **route strip** shows the typing modes as stops on a line.
- The **sheet** is plain white (dark: near-black green). The document sits on it, nothing else.
- The **problems rail** draws each unknown glyph and malformed syllable as a stop on a vertical line.

Red marks a glyph the converter could not place. Amber marks a syllable that cannot be valid. These 2 colors mean only that, everywhere: inline marks, rail stops, popover samples.

## Tokens

All colors are custom properties on `:root`, redefined for dark mode both under `prefers-color-scheme` and `[data-theme="dark"]`. The smoke test checks that the 2 dark blocks match.

| Token | Light | Dark | Use |
|---|---|---|---|
| `--ground` / `--sheet` | #edf1ef / #fff | #0c1311 / #131b18 | page and document |
| `--ink` / `--ink-2` | #14201b / #47564f | #e5eee9 / #a3b5ad | text; secondary text is 6.8:1 or more |
| `--board` / `--board-well` | #0c4a36 / #08321f | #0f4a37 / #05241a | board and message panel |
| `--disc` | #ffcc00 | #ffd23f | route disc, active stop, focus on the board |
| `--line` / `--line-ink` | #0c4a36 / #fff | #4fae86 / #0a2018 | route lines, selected states, primary buttons |
| `--unknown` / `--unknown-bg` | #c8102e / #fde3e7 | #ff8395 / #4d1621 | unknown glyphs |
| `--malformed-line` / `--malformed-bg` | #9a5c00 / #ffecb8 | #ffb13b / #42300b | malformed syllables |
| `--focus` | #0c4a36 | #ffd23f | focus rings on the sheet |

Rules that keep contrast at WCAG AA:
- Never put `--board` on `--sheet` in dark mode; it is 1.4:1. Use `--line`.
- Inside a white menu on the board, reset `--focus` to `--line`; yellow on white is 1.5:1.
- Text on `--unknown` uses `--badge-ink`; text on `--line` uses `--line-ink`.

## Type

1 family: Noto Sans Kannada (variable, self-hosted subsets, OFL), with Noto Serif Kannada as the document's serif option. The Kannada subset is preloaded; the serif files load only when chosen.

- Document: 1.1875rem, line height 1.95 for ottaksharas; 1.125rem on phones.
- UI: 0.9375rem; labels 0.8125rem. Headings in a fixed rem scale near 1.2.
- Board name: Kannada 1.45rem bold over English 0.8125rem.

## Components

- Board plates: outlined white buttons, 2.5rem tall.
- Tool buttons: outlined on the sheet, 2.25rem tall; selected segment filled `--line`.
- Stops: 1rem dot with a 3px ring on a 3px line; the active stop fills with `--disc`.
- Fix popover: next to its mark on wide screens, a bottom sheet on phones. Each fix shows the resulting word.
- Typing chart (landing pages): each letter is a small plate with the Kannada letter large, its Baraha key in a `kbd`, and its ISO 15919 spelling small. Spellings come from the crate at build time.
- Page links: pill-shaped plates at the end of every page.
- Marks: CSS Custom Highlight API with a wavy underline and a tinted background. The DOM is never changed for them.

## Motion

1 authored moment: the board message flips in (260 ms, expo out) when it changes. The popover rises 6px. Hovers fade in 150 ms. All motion stops under `prefers-reduced-motion`.

## Do not

- Do not add a third mark color, or use red or amber for anything else.
- Do not add eyebrows, gradient text, glass, or card grids.
- Do not load fonts or scripts from another host; the Content-Security-Policy forbids it.
