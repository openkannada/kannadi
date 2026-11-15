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
const counts = $("counts");
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
let fileRead = 0;

function plural(n, one, many) {
  return `${n.toLocaleString("en-IN")} ${n === 1 ? one : many}`;
}

// The live region changes only when the result changes in kind, so a screen
// reader is not interrupted after every pause in typing.
function setStatus(message, clean = false) {
  if (status.textContent !== message) status.textContent = message;
  status.classList.toggle("clean", clean);
}

function convert() {
  clearTimeout(pending);
  pending = 0;
  const text = input.value;
  const options = new NudiOptions();
  options.zeroAsAnusvara = zero.checked;
  options.kannadaDigits = digits.checked;
  const result = nudiToUnicode(text, options);
  options.free();
  const unknown = result.unknown;
  const malformed = result.malformed;
  output.value = result.text;
  result.free();

  const hasText = text.length > 0;
  copyButton.disabled = !hasText;
  downloadButton.disabled = !hasText;
  counts.textContent = hasText ? `${plural(text.length, "character", "characters")} converted.` : "";
  const problems = unknown.length + malformed.length / 2;
  if (!hasText) {
    setStatus("Paste text or open a file to convert it.");
  } else if (problems === 0) {
    setStatus("No problems found.", true);
  } else {
    setStatus(
      `${plural(unknown.length, "unknown glyph", "unknown glyphs")} and ` +
        `${plural(malformed.length / 2, "malformed syllable", "malformed syllables")}. See the report.`,
    );
  }
  showReport(text, output.value, unknown, malformed);
}

function scheduleConvert() {
  clearTimeout(pending);
  // About 1 ms per 2,500 characters: wait longer on large text so typing stays smooth.
  pending = setTimeout(convert, Math.min(1000, 120 + input.value.length / 2500));
}

function flush() {
  if (pending) convert();
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

function isLowSurrogate(text, i) {
  const unit = text.charCodeAt(i);
  return unit >= 0xdc00 && unit <= 0xdfff;
}

function select(textarea, start, end) {
  flush();
  // Blurring first makes the browser scroll the selection into view on focus.
  textarea.blur();
  textarea.setSelectionRange(start, end);
  textarea.focus();
}

function contextNode(text, start, end) {
  let from = Math.max(0, start - CONTEXT);
  if (isLowSurrogate(text, from)) from -= 1;
  let to = Math.min(text.length, end + CONTEXT);
  if (isLowSurrogate(text, to)) to += 1;
  const flat = (s) => s.replace(/\s+/g, " ");
  const span = document.createElement("span");
  span.className = "context";
  span.lang = "kn";
  const mark = document.createElement("mark");
  mark.textContent = text.slice(start, end);
  span.append(`“${flat(text.slice(from, start))}`, mark, `${flat(text.slice(end, to))}”`);
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

// Merges both reports in line order and builds at most REPORT_LIMIT items.
function showReport(text, converted, unknown, malformed) {
  const looksUnicode = KANNADA.test(text);
  const total = unknown.length + malformed.length / 2;
  report.hidden = total === 0 && !looksUnicode;
  if (report.hidden) return;

  reportSummary.textContent = looksUnicode
    ? "This text already contains Unicode Kannada. Only text typed in a Nudi or Baraha font needs converting, so Kannada letters are reported as unknown glyphs."
    : "Select an item to highlight it. Unknown glyphs are kept in the result as they were.";

  const inputLines = lineStarts(text);
  const outputLines = lineStarts(converted);
  const items = [];
  let u = 0;
  let m = 0;
  while (items.length < REPORT_LIMIT && (u < unknown.length || m < malformed.length)) {
    const uLine = u < unknown.length ? lineOf(inputLines, unknown[u]) : Infinity;
    const mLine = m < malformed.length ? lineOf(outputLines, malformed[m]) : Infinity;
    if (uLine <= mLine) {
      const offset = unknown[u++];
      const glyph = String.fromCodePoint(text.codePointAt(offset));
      items.push(
        reportItem(uLine, `unknown glyph ${glyph} (${codePoints(glyph)})`, () =>
          select(input, offset, offset + glyph.length),
        ),
      );
    } else {
      const start = malformed[m++];
      const end = malformed[m++];
      items.push(
        reportItem(
          mLine,
          `malformed syllable (${codePoints(converted.slice(start, end))})`,
          () => select(output, start, end),
          contextNode(converted, start, end),
        ),
      );
    }
  }
  reportList.replaceChildren(...items);
  reportMore.hidden = total <= REPORT_LIMIT;
  reportMore.textContent = `Showing the first ${REPORT_LIMIT} of ${total.toLocaleString("en-IN")} items.`;
}

// Legacy files are Windows-1252. A short one whose high bytes happen to form
// valid UTF-8 would be misread; real Nudi text almost never does.
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
  const read = ++fileRead;
  try {
    const bytes = new Uint8Array(await file.arrayBuffer());
    if (read !== fileRead) return;
    input.value = decodeFile(bytes);
    fileName = file.name;
    convert();
  } catch {
    if (read === fileRead) setStatus(`Could not read ${file.name}.`);
  }
}

function replaceInput(text) {
  fileRead++;
  input.value = text;
  fileName = null;
  convert();
}

async function copyResult() {
  flush();
  let copied = false;
  try {
    await navigator.clipboard.writeText(output.value);
    copied = true;
  } catch {
    output.select();
    copied = document.execCommand("copy");
  }
  counts.textContent = copied ? "Copied the Unicode text." : "Could not copy. Select the result and copy it by hand.";
}

function downloadResult() {
  flush();
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
    fileRead++;
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
  $("sample").addEventListener("click", () => replaceInput(SAMPLE));

  // A file dropped anywhere on the page opens in the converter, so the browser
  // never navigates away to show it.
  document.addEventListener("dragover", (event) => {
    if (!event.dataTransfer.types.includes("Files")) return;
    event.preventDefault();
    input.classList.add("dragging");
  });
  document.addEventListener("dragleave", (event) => {
    if (event.relatedTarget === null) input.classList.remove("dragging");
  });
  document.addEventListener("drop", (event) => {
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
