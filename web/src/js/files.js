// Opening TXT, HTML and DOCX files as editor text, and saving the editor text
// as TXT, HTML or DOCX.
import { readDocx, writeDocx } from "./docx.js";
import { decodeBytes, isLegacyFont, joinSegments, looksLegacy } from "./text.js";

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

/** Class names and tag names that style rules in the page set in a legacy font. */
function legacySelectors(doc) {
  const classes = new Set();
  const tags = new Set();
  for (const style of doc.querySelectorAll(`textarea[${STYLE}]`)) {
    const css = style.textContent.replace(/\/\*[\s\S]*?\*\//g, "").replace(/<!--|-->/g, "");
    for (const [, selectors, body] of css.matchAll(/([^{}@]+)\{([^}]*)\}/g)) {
      const family = body.match(FONT_FAMILY)?.[1];
      if (!isLegacyFont(family)) continue;
      for (const selector of selectors.split(",")) {
        const [, tag, cls] = selector.trim().match(/^([a-z0-9]*)(?:\.([\w-]+))?$/i) ?? [];
        if (cls) classes.add(cls);
        else if (tag) tags.add(tag.toUpperCase());
      }
    }
  }
  return { classes, tags };
}

function elementFont(el, selectors) {
  if (el.tagName === "FONT" && el.hasAttribute("face")) return isLegacyFont(el.getAttribute("face"));
  const family = el.getAttribute(STYLE)?.match(FONT_FAMILY)?.[1];
  if (family) return isLegacyFont(family);
  if ([...el.classList].some((c) => selectors.classes.has(c))) return true;
  if (selectors.tags.has(el.tagName)) return true;
  return null;
}

/**
 * Reads an HTML document as segments. Text inherits the nearest font that
 * an element sets; with no font information at all, each block is judged by
 * whether its text looks like legacy glyph codes.
 * @param {Document} doc
 */
export function htmlSegments(doc) {
  const selectors = legacySelectors(doc);
  const segments = [];
  let fontsSeen = false;
  const newline = () => {
    const last = segments[segments.length - 1];
    if (last && !last.text.endsWith("\n")) segments.push({ text: "\n", legacy: false, font: false });
  };
  const walk = (node, inherited) => {
    if (node.nodeType === Node.TEXT_NODE) {
      const text = node.data.replace(/\s+/g, " ");
      if (text.trim() || (segments.length && !segments[segments.length - 1].text.endsWith("\n"))) {
        segments.push({ text, legacy: inherited === true, font: inherited !== null });
      }
      return;
    }
    if (node.nodeType !== Node.ELEMENT_NODE || SKIP.has(node.tagName)) return;
    if (node.tagName === "BR") {
      segments.push({ text: "\n", legacy: false, font: false });
      return;
    }
    const own = elementFont(node, selectors);
    if (own !== null) fontsSeen = true;
    const font = own ?? inherited;
    const block = BLOCK.has(node.tagName);
    if (block) newline();
    for (const child of node.childNodes) walk(child, font);
    if (block) newline();
  };
  walk(doc.body ?? doc.documentElement, null);

  if (!fontsSeen) {
    // Judge each line on its own text.
    const lines = segments.map((s) => s.text).join("").split("\n");
    return lines.flatMap((line, i) => [
      ...(i ? [{ text: "\n", legacy: false }] : []),
      { text: line, legacy: looksLegacy(line) },
    ]);
  }
  return segments.map(({ text, legacy }) => ({ text, legacy }));
}

function trimLines(text) {
  return text
    .split("\n")
    .map((line) => line.replace(/^ +| +$/g, ""))
    .join("\n")
    .replace(/\n{3,}/g, "\n\n")
    .trim();
}

/**
 * Opens a file as editor text.
 * @param {File} file
 * @param {{convert: Function, decodeWindows1252: Function}} kannadi
 * @returns {Promise<{text: string, unknown: number[], converted: number}>}
 */
export async function openFile(file, { convert, decodeWindows1252 }) {
  const bytes = new Uint8Array(await file.arrayBuffer());
  const name = file.name.toLowerCase();
  if (name.endsWith(".docx")) {
    return joinSegments(await readDocx(bytes), convert);
  }
  const text = decodeBytes(bytes, decodeWindows1252);
  if (name.endsWith(".html") || name.endsWith(".htm") || /^\s*<(!doctype|html)/i.test(text)) {
    return fromSegments(htmlSegments(parseHtml(text)), convert);
  }
  return fromSegments([{ text, legacy: looksLegacy(text) }], convert);
}

/** Pasted clipboard data as editor text. */
export function pasted(html, plain, convert) {
  if (html) {
    const segments = htmlSegments(parseHtml(html));
    if (segments.some((s) => s.legacy)) return fromSegments(segments, convert);
  }
  return fromSegments([{ text: plain, legacy: looksLegacy(plain) }], convert);
}

function fromSegments(segments, convert) {
  const joined = joinSegments(segments, convert);
  const text = trimLines(joined.text);
  if (text === joined.text) return joined;
  // Trimming moved text; find each unknown glyph again by its order.
  const glyphs = joined.unknown.map((o) => String.fromCodePoint(joined.text.codePointAt(o)));
  const unknown = [];
  let from = 0;
  for (const g of glyphs) {
    const at = text.indexOf(g, from);
    if (at < 0) break;
    unknown.push(at);
    from = at + g.length;
  }
  return { ...joined, text, unknown };
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

function escapeHtml(text) {
  return text.replace(/[<>&"]/g, (c) => ({ "<": "&lt;", ">": "&gt;", "&": "&amp;", '"': "&quot;" })[c]);
}

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
      .map((p) => `<p>${escapeHtml(p).replace(/\n/g, "<br>")}</p>`)
      .join("\n");
    return new Blob([HTML_TEMPLATE(escapeHtml(title), body)], { type: "text/html;charset=utf-8" });
  }
  // The byte order mark lets older Windows editors detect UTF-8.
  return new Blob(["﻿", text], { type: "text/plain;charset=utf-8" });
}
