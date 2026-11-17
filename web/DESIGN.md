# Editor page design

The page is a tool: people convert old Nudi documents and type Kannada. The design must keep that task clear.

It follows the Open Kannada design language, shared by every Open Kannada surface, so this page and the Open Kannada site look like 1 product. This note covers what is particular to this page. The source of truth for the page's own rules is `src/style.css`.

## Tokens

`src/tokens.css` is a copy of the Open Kannada design tokens. Do not edit it here; when the tokens change, copy the new file across. It loads before `style.css`.

Every color is 1 `light-dark()` pair, so dark mode needs no second block. `data-theme="light"` or `"dark"` on `<html>` pins a theme, and with no attribute the page follows the system. `style.css` maps its own names onto the tokens in its `:root` block, with no color values there. Browsers without `light-dark()` (Safari before 17.5, Chrome before 123) get the light colors from an `@supports` fallback. The smoke test checks that `tokens.css` loads first, that the `:root` block holds no colors, and that every custom property `style.css` uses is defined.

| Page name | Token | Use |
|---|---|---|
| `--ground`, `--sheet` | `--background`, `--card` | page and document |
| `--ink`, `--ink-2` | `--foreground`, `--muted-foreground` | text and secondary text |
| `--line`, `--disc` | `--primary` | route lines, the chosen stop, primary buttons, links |
| `--unknown`, `--unknown-bg` | `--mark-unknown-line`, `--mark-unknown` | unknown glyphs |
| `--malformed-line`, `--malformed-bg` | `--mark-malformed-line`, `--mark-malformed` | malformed syllables |
| `--focus` | `--ring` | focus rings |

Rose and amber mean only "look at this in your text": inline marks, problem stops and popover samples.

## Layout

- **The board** is a bar of Liquid Glass floating at the top, with the kannadi mirror mark, the message panel and the document actions. On phones the message moves to a second row.
- **The route strip** shows the typing modes as stops on an indigo line; the chosen stop fills. It scrolls with the page.
- **The sheet** is solid, with the document on it and nothing else.
- **The problems rail** draws each unknown glyph and malformed syllable as a stop on a vertical line.
- **The status bar** is a glass bar at the bottom with the counts and the save state.
- Turmeric, kumkuma and indigo fields sit fixed behind the page, so the glass has something to bend.

## Glass

The board and the status bar are glass: `--glass` over a backdrop blurred by `--glass-blur` and saturated by `--glass-saturate`, with a lit top rim, a faint bottom rim, a soft specular glow and `--shadow-float`. With reduced transparency, or where `backdrop-filter` is missing, they are the solid `--glass-solid`. The Save menu opens from the board and is solid `--popover`: a pane inside a glass pane cannot blur the page behind it. The light that follows the pointer and the edge refraction are left out here, to keep the page free of extra script.

## Contrast

Measured with the tokens in this pull request, in light and dark mode. Every pair passes WCAG AA.

| Pair | Light | Dark |
|---|---|---|
| Secondary text on the glass board | 6.8:1 | 7.5:1 |
| Primary button text | 7.5:1 | 8.1:1 |
| Badge text on `--unknown` | 5.7:1 | 8.1:1 |
| `--badge-ink` on `--malformed-line` | 4.7:1 | 10.8:1 |
| Focus ring on the page | 4.75:1 | 9.4:1 |
| Focus ring on the sheet | 5.2:1 | 8.7:1 |
| Malformed stop ring on the sheet | 4.8:1 | 10.1:1 |

Text on `--unknown` and `--line` uses `--badge-ink` and `--line-ink`, never `--ink`.

## Components

- Board plates: transparent pills that become a bead of glass on hover, 2.5rem tall.
- Tool buttons: outlined pills on the page, 2.25rem tall; the selected segment is filled `--line`.
- Stops: a 1rem dot with a 3px ring on a 3px line; the chosen stop fills with `--disc`.
- Fix popover: next to its mark on wide screens, a bottom sheet on phones. Each fix shows the resulting word.
- Typing chart (landing pages): each letter is a small plate with the Kannada letter large, its Baraha key in a `kbd`, and its ISO 15919 spelling small. Spellings come from the crate at build time.
- Page links: pill-shaped plates at the end of every page.
- Marks: CSS Custom Highlight API with a wavy underline and a tinted background. The DOM is never changed for them.

## Type

1 family, Noto Sans Kannada (`--type-sans`), with Noto Serif Kannada (`--type-serif`) as the document's serif option. The document is `--type-doc`, and `--type-doc-wide` from 768px, with line height `--type-doc-leading` for ottaksharas. UI text is 0.9375rem.

## Motion

Buttons give a little when pressed and spring back on `--ease-spring`. The board message flips in when it changes. Hovers fade in over `--duration-quick`. All motion stops under `prefers-reduced-motion`.

## Do not

- Do not add a third mark color, or use rose or amber for anything else.
- Do not make the sheet or the content sections glass.
- Do not load fonts or scripts from another host; the Content-Security-Policy forbids it.
