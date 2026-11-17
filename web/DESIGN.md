# Editor page design

The page is a tool: people convert old Nudi documents and type Kannada. The design must keep that task clear.

It follows the Open Kannada design language, shared by every Open Kannada surface, so this page and the Open Kannada site look like 1 product. This note covers what is particular to this page. The source of truth for the page's own rules is `src/style.css`.

## Tokens

`src/tokens.css` is a copy of the Open Kannada design tokens. Do not edit it here; when the tokens change, copy the new file across. It loads before `style.css`.

Every color is 1 `light-dark()` pair, so dark mode needs no second block. `data-theme="light"` or `"dark"` on `<html>` pins a theme, and with no attribute the page follows the system. `style.css` maps its own names onto the tokens in 1 `:root` block and defines no colors of its own; the smoke test checks both.

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
- **The route strip** shows the typing modes as stops on an indigo line; the chosen stop fills.
- **The sheet** is solid, with the document on it and nothing else.
- **The problems rail** draws each unknown glyph and malformed syllable as a stop on a vertical line.
- **The status bar** is a glass bar at the bottom with the counts and the save state.
- Turmeric, kumkuma and indigo fields sit fixed behind the page, so the glass has something to bend.

## Glass

The board, its Save menu and the status bar are glass: `--glass` over a backdrop blurred by `--glass-blur` and saturated by `--glass-saturate`, with a lit top rim, a faint bottom rim, a soft specular glow and `--shadow-float`. With reduced transparency, or where `backdrop-filter` is missing, they are the solid `--glass-solid`. The light that follows the pointer and the edge refraction are left out here, to keep the page free of extra script.

## Type

1 family, Noto Sans Kannada (`--type-sans`), with Noto Serif Kannada (`--type-serif`) as the document's serif option. The document is `--type-doc`, and `--type-doc-wide` from 768px, with line height `--type-doc-leading` for ottaksharas. UI text is 0.9375rem.

## Motion

Buttons give a little when pressed and spring back on `--ease-spring`. The board message flips in when it changes. Hovers fade in over `--duration-quick`. All motion stops under `prefers-reduced-motion`.

## Do not

- Do not add a third mark color, or use rose or amber for anything else.
- Do not make the sheet or the content sections glass.
- Do not load fonts or scripts from another host; the Content-Security-Policy forbids it.
