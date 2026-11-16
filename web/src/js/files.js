// Opening TXT, HTML and DOCX files as editor text, and saving the editor text
// as TXT, HTML or DOCX.
import { readDocx, writeDocx } from "./docx.js";
import { decodeBytes, escapeMarkup, joinSegments, legacyLayout, looksLegacy, resolveAuto } from "./text.js";

const BLOCK = new Set([
  "ADDRESS", "ARTICLE", "ASIDE", "BLOCKQUOTE", "DD", "DIV", "DL", "DT", "FIGCAPTION", "FIGURE", "FOOTER",
  "H1", "H2", "H3", "H4", "H5", "H6", "HEADER", "HR", "LI", "MAIN", "NAV", "OL", "P", "PRE", "SECTION",
  "TABLE", "TR", "UL",
]);
const SKIP = new Set(["SCRIPT", "STYLE", "NOSCRIPT", "TEMPLATE", "HEAD", "TITLE", "TEXTAREA"]);
const FONT_FAMILY = /font-family\s*:\s*([^;}]+)/i;
// The page's Content-Security-Policy also applies to documents it parses, and
// would log every style attribute and element. Renamed, they stay readable:
// attributes as data-source-style, style elements as raw-text textareas.
const STYLE = "data-source-style";

function parseHtml(html) {
  const renamed = html
    .replace(/(<[a-z][^<>]*?\s)style(\s*=)/gi, `$1${STYLE}$2`)
    .replace(/<style\b[^>]*>/gi, `<textarea ${STYLE}>`)
    .replace(/<\/style\s*>/gi, "</textarea>");
  return new DOMParser().parseFromString(renamed, "text/html");
}

/**
 * The font that style rules in the page give class names and tag names: the
 * layout of a legacy font or false, and the rule's place in the page, since a
 * later rule wins. Tag rules count only for legacy fonts, so a page-wide rule
 * such as body{font-family:Arial} leaves text with no other font to be judged
 * by how it looks.
 */
function legacySelectors(doc, fontLayout) {
  const classes = new Map();
  const tags = new Map();
  let order = 0;
  for (const style of doc.querySelectorAll(`textarea[${STYLE}]`)) {
    const css = style.textContent.replace(/\/\*[\s\S]*?\*\//g, "").replace(/<!--|-->/g, "");
    for (const [, selectors, body] of css.matchAll(/([^{}@]+)\{([^}]*)\}/g)) {
      const family = body.match(FONT_FAMILY)?.[1];
      if (!family) continue;
      const font = { layout: legacyLayout(family, fontLayout) ?? false, order: order++ };
      for (const selector of selectors.split(",")) {
        const [, tag, cls] = selector.trim().match(/^([a-z0-9]*)(?:\.([\w-]+))?$/i) ?? [];
        if (cls) classes.set(cls, font);
        else if (tag && font.layout) tags.set(tag.toUpperCase(), font);
      }
    }
  }
  return { classes, tags };
}

/** The layout of the legacy font an element sets, false for another font, or null for none. */
function elementFont(el, selectors, fontLayout) {
  if (el.tagName === "FONT" && el.hasAttribute("face")) return legacyLayout(el.getAttribute("face"), fontLayout) ?? false;
  const family = el.getAttribute(STYLE)?.match(FONT_FAMILY)?.[1];
  if (family) return legacyLayout(family, fontLayout) ?? false;
  const byClass = [...el.classList]
    .map((c) => selectors.classes.get(c))
    .filter(Boolean)
    .sort((a, b) => b.order - a.order)[0];
  return (byClass ?? selectors.tags.get(el.tagName))?.layout ?? null;
}

/** "auto" for text that looks like legacy glyph codes in an unknown font, else false. */
const byLooks = (text) => (looksLegacy(text) ? "auto" : false);

/**
 * Reads an HTML document as segments, 1 block per line, with white space
 * collapsed as a browser shows it. Text inherits the nearest font that an
 * element sets. Text with no font, or a whole page with no font information,
 * is judged by whether it looks like legacy glyph codes.
 * @param {Document} doc
 */
export function htmlSegments(doc, fontLayout) {
  const selectors = legacySelectors(doc, fontLayout);
  const segments = [];
  let fontsSeen = false;
  const last = () => segments[segments.length - 1];
  const atLineStart = () => !last() || last().text.endsWith("\n");
  const lineBreak = (always) => {
    if (!last() || (!always && atLineStart())) return;
    last().text = last().text.replace(/ +$/, "");
    segments.push({ text: "\n", font: false });
  };
  const walk = (node, inherited) => {
    if (node.nodeType === Node.TEXT_NODE) {
      let text = node.data.replace(/\s+/g, " ");
      if (atLineStart() || last().text.endsWith(" ")) text = text.replace(/^ /, "");
      if (text) segments.push({ text, font: inherited });
      return;
    }
    if (node.nodeType !== Node.ELEMENT_NODE || SKIP.has(node.tagName)) return;
    if (node.tagName === "BR") {
      lineBreak(true);
      return;
    }
    const own = elementFont(node, selectors, fontLayout);
    if (own !== null) fontsSeen = true;
    const font = own ?? inherited;
    const block = BLOCK.has(node.tagName);
    if (block) lineBreak(false);
    for (const child of node.childNodes) walk(child, font);
    if (block) lineBreak(false);
  };
  walk(doc.body ?? doc.documentElement, null);
  while (last()?.text === "\n") segments.pop();
  if (last()) last().text = last().text.replace(/ +$/, "");

  if (!fontsSeen) {
    const lines = segments.map((s) => s.text).join("").split("\n");
    return lines.flatMap((line, i) => [
      ...(i ? [{ text: "\n", legacy: false }] : []),
      { text: line, legacy: byLooks(line) },
    ]);
  }
  return segments.map(({ text, font }) => ({ text, legacy: font === null ? byLooks(text) : font }));
}

/**
 * Opens a file as editor text.
 * @param {File} file
 * @param {{convert: Function, bestLayout: Function, decodeWindows1252: Function}} kannadi
 * @returns {Promise<{text: string, unknown: number[], converted: number, layouts: string[]}>}
 */
export async function openFile(file, { convert, bestLayout, fontLayout, decodeWindows1252 }) {
  const join = (segments) => joinSegments(resolveAuto(segments, bestLayout), convert);
  const bytes = new Uint8Array(await file.arrayBuffer());
  const name = file.name.toLowerCase();
  if (name.endsWith(".docx")) return join(await readDocx(bytes, fontLayout));
  const text = decodeBytes(bytes, decodeWindows1252);
  if (name.endsWith(".html") || name.endsWith(".htm") || /^\s*<(!doctype|html)/i.test(text)) {
    return join(htmlSegments(parseHtml(text), fontLayout));
  }
  return join([{ text, legacy: byLooks(text) }]);
}

/**
 * Pasted clipboard data as editor text.
 * @param {{convert: Function, bestLayout: Function}} kannadi
 */
export function pasted(html, plain, { convert, bestLayout, fontLayout }) {
  const join = (segments) => joinSegments(resolveAuto(segments, bestLayout), convert);
  if (html) {
    const segments = htmlSegments(parseHtml(html), fontLayout);
    if (segments.some((s) => s.legacy)) return join(segments);
  }
  return join([{ text: plain, legacy: byLooks(plain) }]);
}

const HTML_TEMPLATE = (title, body) => `<!doctype html>
<html lang="kn">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${title}</title>
<style>body{max-width:42rem;margin:2rem auto;padding:0 1rem;font:1.125rem/1.9 "Noto Sans Kannada","Nirmala UI","Tunga",sans-serif}p{margin:0 0 1rem}</style>
</head>
<body>
${body}
</body>
</html>
`;

/** The editor text as a Blob in the given format. */
export async function exportBlob(text, format, title) {
  if (format === "docx") {
    return new Blob([await writeDocx(text)], {
      type: "application/vnd.openxmlformats-officedocument.wordprocessingml.document",
    });
  }
  if (format === "html") {
    const body = text
      .split(/\n{2,}/)
      .map((p) => `<p>${escapeMarkup(p).replace(/\n/g, "<br>")}</p>`)
      .join("\n");
    return new Blob([HTML_TEMPLATE(escapeMarkup(title), body)], { type: "text/html;charset=utf-8" });
  }
  // The byte order mark lets older Windows editors detect UTF-8.
  return new Blob(["﻿", text], { type: "text/plain;charset=utf-8" });
}
