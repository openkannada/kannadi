// A plain-text editor on a contenteditable element. The element holds 1 text
// node, so a text offset is a DOM offset. Typing is left to the browser; every
// other edit changes the text node directly, because Chrome's insertText drops
// line breaks in plaintext-only elements. The editor keeps its own undo
// history for both kinds, since the browser's history cannot see direct edits.
// Highlights use the CSS Custom Highlight API, which never changes the DOM.
import { diff } from "./text.js";

const HISTORY = 200;
const TYPING = new Set([
  "insertText",
  "insertCompositionText",
  "deleteContentBackward",
  "deleteContentForward",
  "compose",
]);

export class DocEditor {
  /**
   * @param {HTMLElement} el
   * @param {(edit: {start: number, oldEnd: number, newEnd: number}, after: string, inputType: string) => void} onEdit
   */
  constructor(el, onEdit) {
    this.el = el;
    this.onEdit = onEdit;
    el.contentEditable = "plaintext-only";
    if (el.contentEditable !== "plaintext-only") el.contentEditable = "true";
    this.before = this.text;
    this.undoStack = [];
    this.redoStack = [];

    el.addEventListener("beforeinput", (e) => this.beforeInput(e));
    el.addEventListener("input", (e) => {
      if (!e.isComposing) this.sync(e.inputType);
    });
    el.addEventListener("compositionend", () => this.sync("insertCompositionText"));
    el.addEventListener("keydown", (e) => {
      const mod = e.ctrlKey || e.metaKey;
      if (!mod || e.altKey || e.isComposing) return;
      const key = e.key.toLowerCase();
      if (key === "z" || key === "y") {
        e.preventDefault();
        if (key === "y" || e.shiftKey) this.redo();
        else this.undo();
      }
    });
  }

  beforeInput(e) {
    if (e.isComposing) return;
    switch (e.inputType) {
      case "historyUndo":
        e.preventDefault();
        this.undo();
        break;
      case "historyRedo":
        e.preventDefault();
        this.redo();
        break;
      case "insertParagraph":
      case "insertLineBreak": {
        e.preventDefault();
        const sel = this.selection();
        if (sel) this.replace(sel.start, sel.end, "\n", "insertLineBreak");
        break;
      }
      case "insertFromDrop": {
        const text = e.dataTransfer?.getData("text/plain");
        const target = e.getTargetRanges()[0];
        if (!text || !target) break;
        e.preventDefault();
        const at = this.offsetOf(target.startContainer, target.startOffset);
        this.replace(at, at, text, "insertFromDrop");
        break;
      }
    }
  }

  get text() {
    return this.el.textContent;
  }

  node() {
    let node = this.el.firstChild;
    if (!node || node.nodeType !== Node.TEXT_NODE || this.el.childNodes.length > 1) {
      this.flatten();
      node = this.el.firstChild;
    }
    if (!node) node = this.el.appendChild(document.createTextNode(""));
    return node;
  }

  /** Reads the DOM after a change, records it for undo and reports it. */
  sync(inputType = "", record = true) {
    this.flatten();
    const after = this.text;
    if (after === this.before) return;
    const sel = this.selection();
    const edit = diff(this.before, after, sel && sel.start === sel.end ? sel.start : -1);
    if (record) this.record(edit, this.before, after, inputType);
    this.before = after;
    this.onEdit(edit, after, inputType);
  }

  record(edit, before, after, inputType) {
    const entry = {
      start: edit.start,
      removed: before.slice(edit.start, edit.oldEnd),
      inserted: after.slice(edit.start, edit.newEnd),
      typing: TYPING.has(inputType),
      time: Date.now(),
    };
    this.redoStack = [];
    const last = this.undoStack[this.undoStack.length - 1];
    const touches =
      last && entry.start <= last.start + last.inserted.length && entry.start + entry.removed.length >= last.start;
    const breaksWord = /\s/.test(entry.inserted) && !/\s/.test(last?.inserted ?? " ");
    if (last && entry.typing && last.typing && touches && !breaksWord && entry.time - last.time < 1500) {
      // Merge into 1 step covering both edits, in the coordinates of the text before `last`.
      const lo = Math.min(last.start, entry.start);
      const hiMid = Math.max(last.start + last.inserted.length, entry.start + entry.removed.length);
      last.removed =
        before.slice(lo, last.start) + last.removed + before.slice(last.start + last.inserted.length, hiMid);
      last.inserted = after.slice(lo, hiMid - entry.removed.length + entry.inserted.length);
      last.start = lo;
      last.time = entry.time;
      return;
    }
    this.undoStack.push(entry);
    if (this.undoStack.length > HISTORY) this.undoStack.shift();
  }

  undo() {
    const entry = this.undoStack.pop();
    if (!entry) return;
    this.splice(entry.start, entry.start + entry.inserted.length, entry.removed);
    this.redoStack.push(entry);
    this.sync("historyUndo", false);
  }

  redo() {
    const entry = this.redoStack.pop();
    if (!entry) return;
    this.splice(entry.start, entry.start + entry.removed.length, entry.inserted);
    this.undoStack.push(entry);
    this.sync("historyRedo", false);
  }

  /** Changes the text node and puts the caret after the new text. */
  splice(start, end, text) {
    const node = this.node();
    node.replaceData(start, end - start, text);
    if (document.activeElement === this.el) this.select(start + text.length);
  }

  /** Replaces any element the browser inserted with plain text. */
  flatten() {
    const nodes = this.el.childNodes;
    if (nodes.length === 1 && nodes[0].nodeType === Node.TEXT_NODE) return;
    if (nodes.length === 0) return;
    const sel = this.selection();
    let text = "";
    const walk = (node, first) => {
      for (const child of node.childNodes) {
        if (child.nodeType === Node.TEXT_NODE) text += child.data;
        else if (child.nodeName === "BR") text += "\n";
        else {
          if (text && !text.endsWith("\n") && !first) text += "\n";
          walk(child, false);
        }
      }
    };
    walk(this.el, true);
    this.el.textContent = text;
    if (sel) this.select(Math.min(sel.start, text.length), Math.min(sel.end, text.length));
  }

  offsetOf(container, offset) {
    if (container === this.el) {
      let at = 0;
      for (let i = 0; i < offset && i < container.childNodes.length; i++) at += container.childNodes[i].textContent.length;
      return at;
    }
    let at = 0;
    for (const node of this.el.childNodes) {
      if (node === container) return at + offset;
      if (node.contains(container)) return at;
      at += node.textContent.length;
    }
    return at;
  }

  /** The selection as text offsets, or null when it is outside the editor. */
  selection() {
    const sel = document.getSelection();
    if (!sel || sel.rangeCount === 0) return null;
    const range = sel.getRangeAt(0);
    if (!this.el.contains(range.startContainer) || !this.el.contains(range.endContainer)) return null;
    const a = this.offsetOf(range.startContainer, range.startOffset);
    const b = this.offsetOf(range.endContainer, range.endOffset);
    return { start: Math.min(a, b), end: Math.max(a, b) };
  }

  range(start, end) {
    const node = this.node();
    const range = document.createRange();
    range.setStart(node, Math.min(start, node.length));
    range.setEnd(node, Math.min(end, node.length));
    return range;
  }

  select(start, end = start) {
    const sel = document.getSelection();
    sel.removeAllRanges();
    sel.addRange(this.range(start, end));
  }

  /** Replaces start..end with `text` as 1 undoable step, and reports the edit. */
  replace(start, end, text, inputType = "insertReplacementText") {
    if (document.activeElement !== this.el) this.el.focus({ preventScroll: true });
    this.splice(start, end, text);
    this.sync(inputType);
  }

  /** Replaces the whole text as 1 undoable step. */
  setText(text) {
    this.replace(0, this.text.length, text);
    this.select(0);
  }

  /** Draws the named highlight over [start, end, start, end, ...]. */
  highlight(name, ranges) {
    if (!globalThis.CSS?.highlights) return;
    const h = new Highlight();
    for (let i = 0; i + 1 < ranges.length; i += 2) h.add(this.range(ranges[i], ranges[i + 1]));
    CSS.highlights.set(name, h);
  }

  /** Screen rectangle of a text range, for placing a popover. */
  rect(start, end) {
    const rects = this.range(start, end).getClientRects();
    return rects[0] ?? this.el.getBoundingClientRect();
  }

  /** Scrolls a range into the middle third at once, so a popover can be placed next to it. */
  scrollTo(start, end) {
    const r = this.rect(start, end);
    if (r.top < 160 || r.bottom > innerHeight - 80) scrollBy({ top: r.top - innerHeight / 3, behavior: "instant" });
  }
}
