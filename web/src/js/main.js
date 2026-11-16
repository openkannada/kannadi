import * as kannadi from "./kannadi.js";
import { Composer } from "./compose.js";
import { DocEditor } from "./editor.js";
import { exportBlob, openFile, pasted } from "./files.js";
import { Issues } from "./issues.js";
import { countWords, looksLegacy, nudiNotes } from "./text.js";

// From a Karnataka RDPR scheme page, Dec 2002, set in BRH Kannada:
// https://web.archive.org/web/20021227171554/http://www.kar.nic.in:80/rdpr/kan-www/Schemes/Chapter-1/Matter/1_2(M).htm
const SAMPLE = [
  "2. ¸ÉÃªÉAiÀÄ°ègÀÄªÀ ²PÀëPÀjUÉ vÀgÀ¨ÉÃw PÁAiÀÄðPÀæªÀÄ",
  "",
  "2.1 PÁAiÀÄðPÀæªÀÄzÀ »£Àß¯É ºÁUÀÆ GzÉÝÃ±À",
  "¸ÀPÁðj ¥ÁæxÀ«ÄPÀ ±Á¯ÉUÀ¼À°è PÁAiÀÄð¤ªÀð»¸ÀÄwÛgÀÄªÀ ²PÀëPÀjUÉ ¥ÀÅ£Àgï ZÉÃvÀ£À",
  "vÀgÀ¨ÉÃw ¤ÃqÀÄªÀÅzÀÄ.",
].join("\n");

const STORE = "kannadi.editor.v1";
const $ = (id) => document.getElementById(id);

const store = {
  read() {
    try {
      return JSON.parse(localStorage.getItem(STORE) ?? "null");
    } catch {
      return null;
    }
  },
  write(value) {
    try {
      localStorage.setItem(STORE, JSON.stringify(value));
      return true;
    } catch {
      return false;
    }
  },
};

const ui = {
  doc: $("doc"),
  empty: $("empty"),
  message: $("message"),
  messageText: $("message-text"),
  messageAction: $("message-action"),
  aksharas: $("count-aksharas"),
  words: $("count-words"),
  chars: $("count-chars"),
  saved: $("saved"),
  problems: $("problem-count"),
  problemsEmpty: $("problems-empty"),
  file: $("file"),
  autoConvert: $("auto-convert"),
  zero: $("zero"),
  digits: $("digits"),
};

let settings = { mode: "kannada", font: "sans", theme: "auto", autoConvert: true, zero: true, digits: false };
let fileName = "kannada";
let countTimer = 0;
let saveTimer = 0;

function say(text, action) {
  ui.messageText.textContent = text;
  ui.message.classList.remove("flip");
  void ui.message.offsetWidth;
  ui.message.classList.add("flip");
  ui.messageAction.hidden = !action;
  ui.messageAction.onclick = null;
  if (action) {
    ui.messageAction.textContent = action.label;
    ui.messageAction.onclick = () => {
      ui.messageAction.hidden = true;
      action.run();
    };
  }
}

function plural(n, one, many) {
  return `${n.toLocaleString("en-IN")} ${n === 1 ? one : many}`;
}

const convert = (text) => kannadi.convertLegacy(text, { zeroAsAnusvara: settings.zero, kannadaDigits: settings.digits });

function report(result, what) {
  const issues = result.unknown.length;
  say(
    `${what}: ${plural(result.converted ?? 0, "character", "characters")} converted from Nudi or Baraha.` +
      (issues ? ` ${plural(issues, "unknown glyph", "unknown glyphs")} to check.` : ""),
  );
}

const editor = new DocEditor(ui.doc, (edit, after, inputType) => {
  issues.edited(edit, after);
  composer.handle(edit, after, inputType);
  update(after);
});

const issues = new Issues(editor, kannadi, {
  list: $("problem-list"),
  count: ui.problems,
  more: $("problem-more"),
  popover: $("popover"),
  scheme: () => (settings.mode === "iso15919" ? "iso15919" : "baraha"),
  onChange(i) {
    ui.problems.textContent = i.total.toLocaleString("en-IN");
    ui.problems.dataset.zero = String(i.total === 0);
    ui.problemsEmpty.hidden = i.total > 0;
  },
});

const composer = new Composer(editor, kannadi.transliterate, (range) => editor.highlight("composing", range));

function update(text) {
  ui.empty.hidden = text.length > 0;
  ui.chars.textContent = text.length.toLocaleString("en-IN");
  clearTimeout(countTimer);
  countTimer = setTimeout(() => {
    const current = editor.text;
    ui.words.textContent = countWords(current).toLocaleString("en-IN");
    ui.aksharas.textContent = kannadi.countAksharas(current).toLocaleString("en-IN");
  }, Math.min(600, 80 + text.length / 10000));
  clearTimeout(saveTimer);
  ui.saved.textContent = "Saving";
  saveTimer = setTimeout(save, 500);
}

function save() {
  const unknown = issues.unknown.map((u) => u.offset);
  const ok = store.write({ text: editor.text, unknown, settings, fileName, savedAt: Date.now() });
  if (!editor.text) ui.saved.textContent = "Not saved yet";
  else ui.saved.textContent = ok ? "Saved in this browser" : "Too large to save in this browser";
}

function saveSettings() {
  clearTimeout(saveTimer);
  save();
}

/** Inserts text at the selection, converting it first when it looks like Nudi or Baraha. */
function insert(result, what) {
  const sel = editor.selection() ?? { start: editor.text.length, end: editor.text.length };
  editor.replace(sel.start, sel.end, result.text);
  issues.addUnknown(result.unknown.map((o) => sel.start + o));
  if (result.converted) report(result, what);
  return sel.start;
}

async function open(file) {
  if (!file) return;
  try {
    const result = await openFile(file, { convert, decodeWindows1252: kannadi.decodeWindows1252 });
    issues.clear();
    editor.setText(result.text);
    issues.addUnknown(result.unknown);
    editor.el.focus();
    editor.select(0);
    fileName = file.name.replace(/\.[^.]*$/, "") || "kannada";
    if (result.converted) report(result, `Opened ${file.name}`);
    else say(`Opened ${file.name}. Nothing in it needed converting.`);
  } catch (e) {
    say(`Could not open ${file.name}. ${e.message ?? ""}`.trim());
  }
}

/** The document in Nudi glyph codes. */
function nudi() {
  return kannadi.toNudi(editor.text);
}

async function exportAs(format) {
  issues.flush();
  let blob;
  let name = `${fileName}.${format}`;
  let notes = "";
  if (format === "nudi") {
    const encoded = nudi();
    const bytes = kannadi.encodeWindows1252(encoded.text);
    const chars = Array.from(encoded.text);
    const questionMarks = chars.filter((c, i) => bytes[i] === 0x3f && c !== "?").length;
    blob = new Blob([bytes], { type: "text/plain;charset=windows-1252" });
    name = `${fileName}-nudi.txt`;
    notes = ` Set it in a Nudi or BRH font to read it. ${nudiNotes(encoded.unencodable.length, encoded.malformed.length / 2, questionMarks)}`;
  } else {
    blob = await exportBlob(editor.text, format, fileName);
  }
  const link = document.createElement("a");
  link.href = URL.createObjectURL(blob);
  link.download = name;
  link.click();
  setTimeout(() => URL.revokeObjectURL(link.href), 1000);
  if (format !== "nudi") settings.lastFormat = format;
  saveSettings();
  say(`Saved ${link.download} to your downloads.${notes}`.trim());
  $("export-menu").open = false;
}

/** Applies a text transform to the selection, or to the whole document. */
function transform(fn, done) {
  const sel = editor.selection();
  const whole = !sel || sel.start === sel.end;
  const start = whole ? 0 : sel.start;
  const end = whole ? editor.text.length : sel.end;
  const before = editor.text.slice(start, end);
  const after = fn(before);
  if (after === before) {
    say(`${done}: nothing to change.`);
    return;
  }
  editor.replace(start, end, after);
  say(`${done}${whole ? "" : " in the selection"}. Undo with Ctrl+Z.`);
}

function convertSelection() {
  const sel = editor.selection();
  const whole = !sel || sel.start === sel.end;
  const start = whole ? 0 : sel.start;
  const end = whole ? editor.text.length : sel.end;
  const result = convert(editor.text.slice(start, end));
  editor.replace(start, end, result.text);
  issues.addUnknown(result.unknown.map((o) => start + o));
  report({ ...result, converted: end - start }, whole ? "Converted the document" : "Converted the selection");
}

function setMode(mode, remember = true) {
  composer.setMode(mode);
  for (const input of document.querySelectorAll('input[name="mode"]')) input.checked = input.value === mode;
  document.documentElement.dataset.mode = mode;
  if (!remember) return;
  settings.mode = mode;
  saveSettings();
}

function setFont(font) {
  settings.font = font;
  document.documentElement.dataset.font = font;
  for (const input of document.querySelectorAll('input[name="font"]')) input.checked = input.value === font;
  saveSettings();
}

function setTheme(theme) {
  settings.theme = theme;
  if (theme === "auto") delete document.documentElement.dataset.theme;
  else document.documentElement.dataset.theme = theme;
  $("theme").setAttribute("aria-label", `Theme: ${theme}. Change theme`);
  $("theme").dataset.theme = theme;
  saveSettings();
}

function nextProblem(dir) {
  const first = $("popover").querySelector(".pop-actions button");
  if (issues.open && first && !$("popover").contains(document.activeElement)) {
    first.focus();
    return;
  }
  const sel = editor.selection();
  const at = sel ? (dir > 0 ? sel.end : sel.start) : -1;
  const item = issues.next(at, dir);
  if (item) issues.show(item, true);
  else say("No problems to fix.");
}

function wire() {
  ui.doc.addEventListener("beforeinput", (e) => composer.beforeInput(e));
  ui.doc.addEventListener("paste", (e) => {
    e.preventDefault();
    const html = e.clipboardData.getData("text/html");
    const plain = e.clipboardData.getData("text/plain");
    if (!settings.autoConvert) {
      insert({ text: plain, unknown: [] });
      return;
    }
    const result = pasted(html, plain, convert);
    const start = insert(result, "Pasted");
    if (result.converted) {
      say(ui.messageText.textContent, {
        label: "Paste as is",
        run: () => {
          if (editor.text.slice(start, start + result.text.length) !== result.text) {
            say("The pasted text has changed since, so it was left as it is.");
            return;
          }
          editor.replace(start, start + result.text.length, plain);
          say("Pasted as is, without converting.");
        },
      });
    }
  });
  ui.doc.addEventListener("click", () => {
    const sel = editor.selection();
    if (!sel || sel.start !== sel.end) return;
    issues.flush();
    const item = issues.at(sel.start);
    if (item) {
      issues.popover(item, false);
      say(`${item.kind === "unknown" ? "Unknown glyph" : "Malformed syllable"} here. Press F8 to reach its fixes.`);
    }
  });
  ui.doc.addEventListener("keydown", (e) => {
    if (e.key === "Escape" && composer.revert()) e.preventDefault();
    if (e.key === "Escape" && issues.open) issues.close(false);
  });
  document.addEventListener("selectionchange", () => composer.selectionChanged());
  document.addEventListener("pointerdown", (e) => {
    if (issues.open && !$("popover").contains(e.target) && e.target !== ui.doc) issues.close(false);
  });

  ui.file.addEventListener("change", () => {
    open(ui.file.files[0]);
    ui.file.value = "";
  });
  document.addEventListener("dragover", (e) => {
    if (e.dataTransfer.types.includes("Files")) {
      e.preventDefault();
      document.body.classList.add("dragging");
    }
  });
  document.addEventListener("dragleave", (e) => {
    if (e.relatedTarget === null) document.body.classList.remove("dragging");
  });
  document.addEventListener("drop", (e) => {
    document.body.classList.remove("dragging");
    if (!e.dataTransfer.files.length) return;
    e.preventDefault();
    open(e.dataTransfer.files[0]);
  });

  $("sample").addEventListener("click", () => {
    const result = { ...convert(SAMPLE), converted: SAMPLE.length };
    issues.clear();
    editor.setText(result.text);
    issues.addUnknown(result.unknown);
    fileName = "sample";
    say("Converted a sample from a 2002 Karnataka government page, typed in BRH Kannada.");
  });
  $("new").addEventListener("click", () => {
    issues.clear();
    editor.setText("");
    fileName = "kannada";
    editor.el.focus();
    say("New document. Undo with Ctrl+Z to get the last one back.");
  });
  $("convert").addEventListener("click", convertSelection);
  $("normalize").addEventListener("click", () => transform(kannadi.normalize, "Normalized"));
  $("modernize").addEventListener("click", () => transform(kannadi.modernize, "Modernized Old Kannada letters"));
  $("copy").addEventListener("click", async () => {
    issues.flush();
    try {
      await navigator.clipboard.writeText(editor.text);
      say("Copied the whole document.");
    } catch {
      say("Could not copy. Select the text and copy it by hand.");
    }
  });
  $("copy-nudi").addEventListener("click", async () => {
    issues.flush();
    const encoded = nudi();
    try {
      await navigator.clipboard.writeText(encoded.text);
      const notes = nudiNotes(encoded.unencodable.length, encoded.malformed.length / 2);
      say(`Copied as Nudi. Paste it into Word and set a Nudi or BRH font. ${notes}`.trim());
    } catch {
      say("Could not copy. Save as Nudi text instead.");
    }
    $("export-menu").open = false;
  });
  for (const b of document.querySelectorAll("[data-export]")) {
    b.addEventListener("click", () => exportAs(b.dataset.export));
  }
  for (const input of document.querySelectorAll('input[name="mode"]')) {
    input.addEventListener("change", () => {
      setMode(input.value);
      editor.el.focus();
    });
  }
  for (const input of document.querySelectorAll('input[name="font"]')) {
    input.addEventListener("change", () => setFont(input.value));
  }
  $("theme").addEventListener("click", () => {
    const order = ["auto", "light", "dark"];
    setTheme(order[(order.indexOf(settings.theme) + 1) % order.length]);
  });
  ui.autoConvert.addEventListener("change", () => {
    settings.autoConvert = ui.autoConvert.checked;
    saveSettings();
  });
  ui.zero.addEventListener("change", () => {
    settings.zero = ui.zero.checked;
    saveSettings();
  });
  ui.digits.addEventListener("change", () => {
    settings.digits = ui.digits.checked;
    saveSettings();
  });
  $("shortcuts-open").addEventListener("click", () => $("shortcuts").showModal());

  document.addEventListener("keydown", (e) => {
    const mod = e.ctrlKey || e.metaKey;
    if (mod && !e.altKey && e.key.toLowerCase() === "o") {
      e.preventDefault();
      ui.file.click();
    } else if (mod && !e.altKey && e.key.toLowerCase() === "s") {
      e.preventDefault();
      exportAs(settings.lastFormat ?? "docx");
    } else if (mod && e.key === "/") {
      e.preventDefault();
      $("shortcuts").showModal();
    } else if (e.altKey && !mod && /^Digit[123]$/.test(e.code)) {
      e.preventDefault();
      setMode(["kannada", "baraha", "iso15919"][Number(e.code.slice(5)) - 1]);
      say(`Typing with ${document.querySelector(`label[for="mode-${settings.mode}"]`).dataset.name}.`);
    } else if (e.key === "F8") {
      e.preventDefault();
      nextProblem(e.shiftKey ? -1 : 1);
    }
  });
  addEventListener("pagehide", save);
}

function restore() {
  const saved = store.read();
  if (saved?.settings) settings = { ...settings, ...saved.settings };
  ui.autoConvert.checked = settings.autoConvert;
  ui.zero.checked = settings.zero;
  ui.digits.checked = settings.digits;
  // Typing pages open ready to type in English letters.
  const pageMode = document.documentElement.dataset.defaultMode;
  setMode(pageMode ?? settings.mode, !pageMode);
  setFont(settings.font);
  setTheme(settings.theme);
  if (saved?.text) {
    ui.doc.textContent = saved.text;
    editor.sync("restore", false);
    issues.addUnknown((saved.unknown ?? []).filter((o) => Number.isInteger(o) && o < saved.text.length));
    fileName = saved.fileName ?? "kannada";
    say(
      looksLegacy(saved.text)
        ? "Restored your last document. Some of it still looks like Nudi or Baraha: select it and choose Convert."
        : "Restored your last document from this browser.",
    );
  } else {
    update("");
    say("Ready. Paste text, open a file or start typing.");
  }
  issues.check();
}

try {
  await kannadi.init();
  wire();
  restore();
  document.documentElement.classList.add("ready");
} catch (e) {
  say("This browser could not load the editor. Use a recent version of Chrome, Edge, Firefox or Safari.");
  throw e;
}
