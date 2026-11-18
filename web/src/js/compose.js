// Phonetic typing: Latin letters typed in Baraha or ISO 15919 turn into Kannada
// as you type. The word being typed is kept as Latin in `latin`, and each new
// letter re-renders the whole word, since a later letter can change an earlier
// one (k is ಕ್, ka is ಕ).

const LETTERS = {
  baraha: /^[A-Za-z~^_]+$/,
  iso15919: /^[\p{Script=Latin}\p{M}:]+$/u,
};
// Phone keyboards that compose whole words often commit the word and the
// space after it as 1 insertion.
const WORD_AND_BREAK = /^(.+?)([\s.,;:!?]+)$/u;

export class Composer {
  /**
   * @param {import("./editor.js").DocEditor} editor
   * @param {(text: string, from: string, to: string) => string} transliterate
   * @param {(range: number[]) => void} onComposing
   */
  constructor(editor, transliterate, onComposing) {
    this.editor = editor;
    this.transliterate = transliterate;
    this.onComposing = onComposing;
    this.mode = "kannada";
    this.word = null;
    this.busy = false;
  }

  setMode(mode) {
    this.commit();
    this.mode = mode;
  }

  get active() {
    return this.mode !== "kannada";
  }

  end() {
    return this.word ? this.word.start + this.word.rendered.length : -1;
  }

  commit() {
    if (!this.word) return;
    this.word = null;
    this.onComposing([]);
  }

  render(replaceEnd) {
    const w = this.word;
    const out = w.latin ? this.transliterate(w.latin, this.mode, "kannada") : "";
    this.busy = true;
    try {
      this.editor.replace(w.start, replaceEnd, out, "compose");
    } finally {
      this.busy = false;
    }
    w.rendered = out;
    if (out) this.onComposing([w.start, w.start + out.length]);
    else this.commit();
  }

  /** Called after every edit; turns newly typed Latin letters into Kannada. */
  handle(edit, after, inputType) {
    if (!this.active || this.busy) return;
    if (inputType !== "insertText" && inputType !== "insertCompositionText") {
      this.commit();
      return;
    }
    const inserted = after.slice(edit.start, edit.newEnd);
    const insertOnly = edit.oldEnd === edit.start && inserted;
    if (insertOnly && LETTERS[this.mode].test(inserted)) {
      this.extend(edit.start, inserted);
      this.render(edit.newEnd);
      return;
    }
    const m = insertOnly && inserted.match(WORD_AND_BREAK);
    if (m && LETTERS[this.mode].test(m[1])) {
      this.extend(edit.start, m[1]);
      this.render(edit.start + m[1].length);
    }
    this.commit();
  }

  extend(at, letters) {
    if (this.word && at === this.end()) this.word.latin += letters;
    else this.word = { start: at, latin: letters, rendered: "" };
  }

  /** Backspace inside the word removes its last Latin letter and renders the word again. */
  beforeInput(e) {
    if (!this.word || e.inputType !== "deleteContentBackward" || !e.cancelable) return;
    const sel = this.editor.selection();
    if (!sel || sel.start !== sel.end || sel.end !== this.end()) return;
    e.preventDefault();
    this.word.latin = Array.from(this.word.latin).slice(0, -1).join("");
    this.render(this.end());
  }

  /** Escape keeps the word as typed, in Latin letters. */
  revert() {
    if (!this.word) return false;
    const { start, latin } = this.word;
    const end = this.end();
    this.word = null;
    this.busy = true;
    try {
      this.editor.replace(start, end, latin, "compose");
    } finally {
      this.busy = false;
    }
    this.onComposing([]);
    return true;
  }

  /** The caret left the word: the word is done. */
  selectionChanged() {
    if (!this.word || this.busy) return;
    const sel = this.editor.selection();
    if (!sel || sel.start !== sel.end || sel.end !== this.end()) this.commit();
  }
}
