import init, { NudiOptions, decodeWindows1252, nudiToUnicode } from "./pkg/kannadi_wasm.js";

// From a Karnataka RDPR scheme page, Dec 2002, set in BRH Kannada:
// https://web.archive.org/web/20021227171554/http://www.kar.nic.in:80/rdpr/kan-www/Schemes/Chapter-1/Matter/1_2(M).htm
const SAMPLE = [
  "2. ¸ÉÃªÉAiÀÄ°ègÀÄªÀ ²PÀëPÀjUÉ vÀgÀ¨ÉÃw PÁAiÀÄðPÀæªÀÄ",
  "",
  "2.1 PÁAiÀÄðPÀæªÀÄzÀ »£Àß¯É ºÁUÀÆ GzÉÝÃ±À",
  "¸ÀPÁðj ¥ÁæxÀ«ÄPÀ ±Á¯ÉUÀ¼À°è PÁAiÀÄð¤ªÀð»¸ÀÄwÛgÀÄªÀ ²PÀëPÀjUÉ ¥ÀÅ£Àgï ZÉÃvÀ£À",
  "vÀgÀ¨ÉÃw ¤ÃqÀÄªÀÅzÀÄ.",
].join("\n");

const REPORT_LIMIT = 200;
const CONTEXT = 10;
const KANNADA = /[ಀ-೿]/;

const $ = (id) => document.getElementById(id);
const input = $("input");
const output = $("output");
const status = $("status");
const zero = $("zero");
const digits = $("digits");
const copyButton = $("copy");
const downloadButton = $("download");
const report = $("report");
const reportSummary = $("report-summary");
const reportList = $("report-list");
const reportMore = $("report-more");

let fileName = null;
let pending = 0;

function setStatus(message, clean = false) {
  status.textContent = message;
  status.classList.toggle("clean", clean);
}

function plural(n, one, many) {
  return `${n.toLocaleString("en-IN")} ${n === 1 ? one : many}`;
}

function convert() {
  clearTimeout(pending);
  const text = input.value;
  const options = new NudiOptions();
  options.zeroAsAnusvara = zero.checked;
  options.kannadaDigits = digits.checked;
  const result = nudiToUnicode(text, options);
  options.free();
  const unknown = result.unknown.map((u) => {
    const item = { offset: u.offset, glyph: u.glyph };
    u.free();
    return item;
  });
  const malformed = result.malformed.map((s) => {
    const item = { start: s.start, end: s.end };
    s.free();
    return item;
  });
  output.value = result.text;
  result.free();

  const hasText = text.length > 0;
  copyButton.disabled = !hasText;
  downloadButton.disabled = !hasText;
  if (!hasText) {
    setStatus("Paste text or open a file to convert it.");
  } else if (unknown.length === 0 && malformed.length === 0) {
    setStatus(`Converted ${plural(text.length, "character", "characters")}. No problems found.`, true);
  } else {
    setStatus(
      `Converted ${plural(text.length, "character", "characters")}. ` +
        `${plural(unknown.length, "unknown glyph", "unknown glyphs")} and ` +
        `${plural(malformed.length, "malformed syllable", "malformed syllables")}. See the report.`,
    );
  }
  showReport(text, output.value, unknown, malformed);
}

function scheduleConvert() {
  clearTimeout(pending);
  pending = setTimeout(convert, 120);
}

function lineStarts(text) {
  const starts = [0];
  for (let i = text.indexOf("\n"); i !== -1; i = text.indexOf("\n", i + 1)) starts.push(i + 1);
  return starts;
}

function lineOf(starts, offset) {
  let lo = 0;
  let hi = starts.length - 1;
  while (lo < hi) {
    const mid = (lo + hi + 1) >> 1;
    if (starts[mid] <= offset) lo = mid;
    else hi = mid - 1;
  }
  return lo + 1;
}

function codePoints(text) {
  return Array.from(text, (c) => "U+" + c.codePointAt(0).toString(16).toUpperCase().padStart(4, "0")).join(" ");
}

function select(textarea, start, end) {
  // Blurring first makes the browser scroll the selection into view on focus.
  textarea.blur();
  textarea.setSelectionRange(start, end);
  textarea.focus();
}

function contextNode(text, start, end) {
  const span = document.createElement("span");
  span.className = "context";
  span.lang = "kn";
  const flat = (s) => s.replace(/\s+/g, " ");
  const before = flat(text.slice(Math.max(0, start - CONTEXT), start));
  const after = flat(text.slice(end, end + CONTEXT));
  const mark = document.createElement("mark");
  mark.textContent = text.slice(start, end);
  span.append(`“${before}`, mark, `${after}”`);
  return span;
}

function reportItem(line, label, onSelect, context) {
  const li = document.createElement("li");
  const button = document.createElement("button");
  button.type = "button";
  button.textContent = `Line ${line}: ${label}`;
  button.addEventListener("click", onSelect);
  li.append(button);
  if (context) li.append(" in ", context);
  return li;
}

function showReport(text, converted, unknown, malformed) {
  const looksUnicode = KANNADA.test(text);
  const total = unknown.length + malformed.length;
  report.hidden = total === 0 && !looksUnicode;
  if (report.hidden) return;

  reportSummary.textContent = looksUnicode
    ? "This text already contains Unicode Kannada. Only text typed in a Nudi or Baraha font needs converting, so Kannada letters are reported as unknown glyphs."
    : "Select an item to highlight it. Unknown glyphs are kept in the result as they were.";

  const inputLines = lineStarts(text);
  const outputLines = lineStarts(converted);
  const items = [];
  for (const u of unknown) {
    if (items.length >= REPORT_LIMIT) break;
    const end = u.offset + u.glyph.length;
    items.push(
      reportItem(
        lineOf(inputLines, u.offset),
        `unknown glyph ${u.glyph} (${codePoints(u.glyph)})`,
        () => select(input, u.offset, end),
      ),
    );
  }
  for (const m of malformed) {
    if (items.length >= REPORT_LIMIT) break;
    items.push(
      reportItem(
        lineOf(outputLines, m.start),
        `malformed syllable (${codePoints(converted.slice(m.start, m.end))})`,
        () => select(output, m.start, m.end),
        contextNode(converted, m.start, m.end),
      ),
    );
  }
  reportList.replaceChildren(...items);
  reportMore.hidden = total <= REPORT_LIMIT;
  reportMore.textContent = `Showing the first ${REPORT_LIMIT} of ${total.toLocaleString("en-IN")} items.`;
}

function decodeFile(bytes) {
  if (bytes[0] === 0xff && bytes[1] === 0xfe) return new TextDecoder("utf-16le").decode(bytes);
  if (bytes[0] === 0xfe && bytes[1] === 0xff) return new TextDecoder("utf-16be").decode(bytes);
  try {
    return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
  } catch {
    return decodeWindows1252(bytes);
  }
}

async function openFile(file) {
  if (!file) return;
  try {
    const bytes = new Uint8Array(await file.arrayBuffer());
    input.value = decodeFile(bytes);
    fileName = file.name;
    convert();
  } catch {
    setStatus(`Could not read ${file.name}.`);
  }
}

async function copyResult() {
  try {
    await navigator.clipboard.writeText(output.value);
  } catch {
    output.select();
    document.execCommand("copy");
  }
  setStatus("Copied the Unicode text.", true);
}

function downloadResult() {
  const base = fileName ? fileName.replace(/\.[^.]*$/, "") : "kannada";
  // The byte order mark lets older Windows editors detect UTF-8.
  const blob = new Blob(["﻿", output.value], { type: "text/plain;charset=utf-8" });
  const link = document.createElement("a");
  link.href = URL.createObjectURL(blob);
  link.download = `${base}-unicode.txt`;
  link.click();
  setTimeout(() => URL.revokeObjectURL(link.href), 1000);
}

function wire() {
  input.addEventListener("input", () => {
    fileName = null;
    scheduleConvert();
  });
  zero.addEventListener("change", convert);
  digits.addEventListener("change", convert);
  copyButton.addEventListener("click", copyResult);
  downloadButton.addEventListener("click", downloadResult);
  $("file").addEventListener("change", (event) => {
    openFile(event.target.files[0]);
    event.target.value = "";
  });
  $("sample").addEventListener("click", () => {
    input.value = SAMPLE;
    fileName = null;
    convert();
  });
  input.addEventListener("dragover", (event) => {
    if (!event.dataTransfer.types.includes("Files")) return;
    event.preventDefault();
    input.classList.add("dragging");
  });
  input.addEventListener("dragleave", () => input.classList.remove("dragging"));
  input.addEventListener("drop", (event) => {
    input.classList.remove("dragging");
    if (event.dataTransfer.files.length === 0) return;
    event.preventDefault();
    openFile(event.dataTransfer.files[0]);
  });
}

try {
  await init();
  wire();
  convert();
} catch {
  setStatus("This browser could not load the converter. Use a recent version of Chrome, Edge, Firefox or Safari.");
}
