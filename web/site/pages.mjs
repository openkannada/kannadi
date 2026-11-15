// Content of each page the site build writes. Strings are trusted HTML from
// this file; FAQ answers are also sent as plain text in the FAQPage JSON-LD.
// Search volumes (India, monthly, DataForSEO, 2026-11-15) are noted per page.

const UNICODE_TO_NUDI = {
  q: "Can it convert Unicode to Nudi?",
  a: 'Yes. Choose <b>Save</b>, then <b>Nudi text</b> or <b>Copy as Nudi</b>, and set the result in a Nudi or BRH font. See <a href="../unicode-to-nudi/">Unicode to Nudi converter</a>.',
};
const UPLOAD = {
  q: "Is my text uploaded?",
  a: "No. The converter is compiled to WebAssembly and runs on your device. The page loads its own files and makes no other network requests. It sets no cookies and has no analytics. Your last document is kept in this browser's storage so it is still here when you come back.",
};
const ACCURACY = {
  q: "How accurate is it?",
  a: "Every glyph it cannot place is marked, and every syllable that is not valid Kannada is marked. A measured error rate on a checked set of real documents will be published when that test set is ready.",
};
const WHO = {
  q: "Who made this?",
  a: 'It is built on <a href="https://github.com/prasadus92/kannadi">kannadi</a>, an open-source Rust library for Kannada text by Prasad Subrahmanya, released under the MIT license. The fonts are Noto Sans Kannada and Noto Serif Kannada, under the SIL Open Font License.',
};
const CONVERT_STEPS = [
  "Open your file with <b>Open</b>, or copy the text from Word and paste it into the sheet. Word files keep their fonts, so only the text set in Nudi or Baraha fonts is converted.",
  "Check the marks. Red is a glyph with no Kannada letter. Amber is a sign that cannot form a syllable. Select a mark, or press <kbd>F8</kbd>, and pick a fix.",
  "Choose <b>Save</b> and pick Word, plain text or web page.",
];
const TYPING_STEPS = [
  "Choose <b>Baraha letters</b> above the sheet. It is already chosen on this page.",
  'Type a word the way it sounds: <kbd>kannaDa</kbd> gives <span lang="kn">ಕನ್ನಡ</span>. The word changes into Kannada letter by letter.',
  "Press space or a punctuation key to finish the word. Press <kbd>Esc</kbd> to keep a word in English letters.",
  "Choose <b>Save</b> for a Word file, or <b>Copy all</b> to paste the text anywhere.",
];

/** Kannada words shown with their Baraha and ISO 15919 spellings on typing pages. */
export const EXAMPLE_WORDS = ["ಕನ್ನಡ", "ನಮಸ್ಕಾರ", "ಕರ್ನಾಟಕ", "ಬೆಂಗಳೂರು", "ಮೈಸೂರು", "ಜ್ಞಾನ", "ಶ್ರೀ", "ಹೃದಯ", "ಅಮ್ಮ", "ಊರು"];
/** Letters shown in the typing chart, in the order of the Kannada alphabet. */
export const CHART = {
  Vowels: ["ಅ", "ಆ", "ಇ", "ಈ", "ಉ", "ಊ", "ಋ", "ಎ", "ಏ", "ಐ", "ಒ", "ಓ", "ಔ", "ಅಂ", "ಅಃ"],
  Consonants: [
    "ಕ", "ಖ", "ಗ", "ಘ", "ಙ", "ಚ", "ಛ", "ಜ", "ಝ", "ಞ", "ಟ", "ಠ", "ಡ", "ಢ", "ಣ", "ತ", "ಥ", "ದ", "ಧ", "ನ",
    "ಪ", "ಫ", "ಬ", "ಭ", "ಮ", "ಯ", "ರ", "ಲ", "ವ", "ಶ", "ಷ", "ಸ", "ಹ", "ಳ",
  ],
  "Vowel signs on ಕ": ["ಕ", "ಕಾ", "ಕಿ", "ಕೀ", "ಕು", "ಕೂ", "ಕೃ", "ಕೆ", "ಕೇ", "ಕೈ", "ಕೊ", "ಕೋ", "ಕೌ", "ಕಂ", "ಕಃ", "ಕ್"],
};

export const PAGES = [
  {
    path: "",
    og: { from: "PÀ£ÀßqÀ", to: "ಕನ್ನಡ" },
    // "kannada converter" 40,500; "kannada unicode" 5,400; "kannada unicode converter" 720
    title: "Kannada editor and Nudi to Unicode converter",
    description:
      "Free Kannada editor in your browser. Convert Nudi and Baraha to Unicode, fix every unknown glyph inline, type Kannada with English letters, and save as Word, HTML or text. Nothing is uploaded.",
    h1: "Kannada editor and Nudi to Unicode converter",
    intro:
      "Nudi to Unicode conversion turns Kannada text typed in the old Nudi or Baraha fonts into standard Unicode Kannada, which phones, websites and search engines can read. This editor does it in your browser, marks every glyph it could not convert, and nothing is uploaded.",
    mode: "kannada",
    howTitle: "How to convert Nudi to Unicode",
    steps: CONVERT_STEPS,
    faq: [
      UPLOAD,
      {
        q: "Which fonts does it convert?",
        a: "Nudi 1 to 5 fonts and Baraha ANSI fonts such as BRH Kannada. Text that already shows as Kannada letters is Unicode and needs no conversion.",
      },
      UNICODE_TO_NUDI,
      {
        q: "How do I type Kannada with an English keyboard?",
        a: 'Choose <b>Baraha letters</b> and type words the way they sound: <kbd>kannaDa</kbd> gives <span lang="kn">ಕನ್ನಡ</span>, <kbd>nADu</kbd> gives <span lang="kn">ನಾಡು</span>. Capital letters give long vowels and retroflex sounds. Press <kbd>Esc</kbd> to keep a word in English letters.',
      },
      {
        q: "What do Normalize and Modernize do?",
        a: 'Normalize joins vowel signs typed in 2 parts and removes doubled marks, so text that looks the same also searches the same. Modernize changes the Old Kannada letters <span lang="kn">ೞ ಱ ೝ</span> to <span lang="kn">ಳ ರ ನ್</span>.',
      },
      ACCURACY,
      WHO,
    ],
  },
  {
    path: "nudi-to-unicode/",
    og: { from: "PÀ£ÀßqÀ", to: "ಕನ್ನಡ" },
    // "nudi to unicode" 27,100; "nudi to unicode converter" 12,100; "nudi converter" 3,600
    title: "Nudi to Unicode converter: convert Nudi Kannada files online",
    description:
      "Convert Nudi Kannada text and Word files to Unicode in your browser. Every glyph that could not be converted is marked with a fix. Nothing is uploaded. Unicode to Nudi works too.",
    h1: "Nudi to Unicode converter",
    intro:
      "A Nudi to Unicode converter turns Kannada typed in the Nudi fonts, which store Latin letters such as <code>PÀ£ÀßqÀ</code>, into Unicode Kannada such as <span lang=\"kn\">ಕನ್ನಡ</span>. Paste the text or open the file below. Only the text set in Nudi or Baraha fonts is converted.",
    mode: "kannada",
    howTitle: "How to convert a Nudi file to Unicode",
    steps: CONVERT_STEPS,
    faq: [
      {
        q: "How do I convert a Nudi file to Unicode?",
        a: "Choose <b>Open</b> and pick the Word, text or web page file. The editor reads the fonts in the file and converts the text set in Nudi or Baraha fonts. Check the marked problems, then choose <b>Save</b>.",
      },
      {
        q: "How can I tell if my text is Nudi or Unicode?",
        a: 'Paste a few words into a phone message or a web form. Unicode shows as Kannada letters everywhere. Nudi shows as Latin letters and symbols such as <code>PÀ£ÀßqÀ</code> unless the Nudi font is installed.',
      },
      UNICODE_TO_NUDI,
      {
        q: "Can it translate Nudi Kannada to English?",
        a: "No. It changes how the Kannada is stored, and the words stay Kannada. Converting to Unicode first lets translation tools and search engines read the text.",
      },
      {
        q: "Why are some letters marked in red or amber?",
        a: "Red marks a glyph the converter has no Kannada letter for. It stays in the text so nothing is lost. Amber marks signs that cannot form a syllable, often because a key was typed out of order. Select a mark to see fixes.",
      },
      UPLOAD,
      ACCURACY,
    ],
  },
  {
    path: "unicode-to-nudi/",
    lead: "Paste or type Unicode Kannada, then choose <b>Save</b> and <b>Copy as Nudi</b>.",
    og: { from: "ಕನ್ನಡ", to: "PÀ£ÀßqÀ" },
    // "unicode to nudi" 14,800; "unicode to nudi converter" 8,100
    title: "Unicode to Nudi converter: Kannada Unicode to Nudi and Baraha",
    description:
      "Convert Unicode Kannada to Nudi and Baraha ANSI font codes in your browser, for forms and printers that still need Nudi. Characters with no Nudi glyph are listed. Nothing is uploaded.",
    h1: "Unicode to Nudi converter",
    intro:
      'A Unicode to Nudi converter turns Unicode Kannada such as <span lang="kn">ಕನ್ನಡ</span> into the glyph codes the Nudi and Baraha ANSI fonts use, such as <code>PÀ£ÀßqÀ</code>. Use it when a form, a printer template or an older office system still expects Nudi. Paste or type the Kannada below, then choose <b>Save</b>.',
    mode: "kannada",
    howTitle: "How to convert Unicode to Nudi",
    steps: [
      "Paste Unicode Kannada into the sheet, open a file, or type it with <b>Baraha letters</b>.",
      "Choose <b>Save</b>, then <b>Copy as Nudi</b> to paste into Word, or <b>Nudi text</b> for a file.",
      "In Word, select the pasted text and choose a Nudi font, such as Nudi 01 e, or a Baraha font, such as BRH Kannada.",
    ],
    faq: [
      {
        q: "Which fonts can show the result?",
        a: "Nudi fonts up to version 5, and Baraha ANSI fonts such as BRH Kannada. They share 1 glyph layout. Without one of these fonts, the text shows as Latin letters and symbols.",
      },
      {
        q: "What happens to characters Nudi cannot show?",
        a: 'Characters with no glyph in the layout, such as English letters and some Old Kannada signs, are kept as they were, and the message says how many. In a Nudi font, English letters draw as Kannada shapes, so set English text in another font.',
      },
      {
        q: "Will the Nudi text convert back the same?",
        a: 'Yes, for well-formed Kannada. Converting to Nudi and back gives the same text. On 9,047 words from 35 Karnataka government pages from 2002, 9,029 encode to the exact bytes that were typed.',
      },
      {
        q: "How do I get Kannada digits?",
        a: "Digits are written as 0 to 9. The Nudi fonts whose names end in k draw them as Kannada digits.",
      },
      UPLOAD,
    ],
  },
  {
    path: "baraha-to-unicode/",
    og: { from: "¸ÀPÁðj", to: "ಸರ್ಕಾರಿ" },
    // "baraha to unicode" 170; "baraha" 2,900
    title: "Baraha to Unicode converter: BRH Kannada to Unicode",
    description:
      "Convert Kannada typed in Baraha ANSI fonts such as BRH Kannada to Unicode in your browser. Unknown glyphs are marked with fixes. Nothing is uploaded.",
    h1: "Baraha to Unicode converter",
    intro:
      "A Baraha to Unicode converter turns Kannada typed in Baraha's older ANSI fonts, such as BRH Kannada, into Unicode Kannada that any phone or website can show. Baraha ANSI fonts use the same glyph layout as Nudi, so 1 converter handles both.",
    mode: "kannada",
    howTitle: "How to convert Baraha text to Unicode",
    steps: CONVERT_STEPS,
    faq: [
      {
        q: "Which Baraha fonts does it convert?",
        a: "The ANSI fonts whose names start with BRH, such as BRH Kannada. Text typed in a Baraha Unicode font is already Unicode and needs no conversion.",
      },
      {
        q: "Many old Karnataka government pages look like PÀ£ÀßqÀ. Can I convert them?",
        a: "Yes. Open the saved page, or copy its text and paste it here. The editor reads the font names in the page and converts only the Kannada parts.",
      },
      {
        q: "Is this the same as Baraha phonetic typing?",
        a: 'No. This page converts text that was stored in Baraha fonts. To type new Kannada with English letters the Baraha way, see <a href="../english-to-kannada-typing/">English to Kannada typing</a>.',
      },
      UNICODE_TO_NUDI,
      UPLOAD,
    ],
  },
  {
    path: "kannada-typing/",
    lead: 'Start typing. With <b>Baraha letters</b> chosen, <kbd>kannaDa</kbd> becomes <span lang="kn">ಕನ್ನಡ</span> as you type.',
    og: { from: "kannaDa", to: "ಕನ್ನಡ" },
    // "kannada typing" 40,500; "type in kannada" 40,500; "kannada typing online" 9,900
    title: "Kannada typing online: type Kannada with an English keyboard",
    description:
      "Type Kannada online with an English keyboard or a Kannada keyboard. Words turn into Kannada as you type. Save as Word or copy anywhere. Free, and nothing is uploaded.",
    h1: "Kannada typing online",
    intro:
      "Kannada typing means writing Kannada script on a computer or phone, either with a Kannada keyboard layout or by typing words in English letters that turn into Kannada. This page does both, in your browser.",
    mode: "baraha",
    howTitle: "How to type in Kannada",
    steps: TYPING_STEPS,
    chart: true,
    faq: [
      {
        q: "Do I need to install a Kannada keyboard?",
        a: "No. With <b>Baraha letters</b> chosen, a normal English keyboard is enough. If your phone or computer already has a Kannada keyboard, choose <b>Kannada keys</b> and type as usual.",
      },
      {
        q: "How do I type long vowels and hard consonants?",
        a: 'Use capital letters: <kbd>A</kbd> gives <span lang="kn">ಆ</span>, <kbd>I</kbd> gives <span lang="kn">ಈ</span>, <kbd>T</kbd> gives <span lang="kn">ಟ</span> and <kbd>D</kbd> gives <span lang="kn">ಡ</span>. The chart on this page lists every letter.',
      },
      {
        q: "Can I type Kannada on my phone here?",
        a: "Yes. On phones with word suggestions, the word changes into Kannada when the keyboard commits it, usually at the space.",
      },
      {
        q: "What is ISO 15919?",
        a: 'A standard way to write Indian scripts in Latin letters with marks, such as <kbd>kannaḍa</kbd>. Choose it if you already type those letters.',
      },
      UPLOAD,
    ],
  },
  {
    path: "english-to-kannada-typing/",
    lead: 'Start typing. With <b>Baraha letters</b> chosen, <kbd>kannaDa</kbd> becomes <span lang="kn">ಕನ್ನಡ</span> as you type.',
    og: { from: "namaskAra", to: "ನಮಸ್ಕಾರ" },
    // "english to kannada typing" 49,500; "kannada transliteration" 135,000;
    // "english to kannada transliteration" 1,220,000
    title: "English to Kannada typing: type Kannada in English letters",
    description:
      "Type Kannada words in English letters and get Kannada script: namaskAra gives ನಮಸ್ಕಾರ. Free English to Kannada transliteration in your browser, with a full letter chart.",
    h1: "English to Kannada typing",
    intro:
      'English to Kannada typing lets you write Kannada words with English letters: type <kbd>namaskAra</kbd> and get <span lang="kn">ನಮಸ್ಕಾರ</span>. This is transliteration. It spells Kannada words in another script and does not translate English words.',
    mode: "baraha",
    howTitle: "How to type Kannada in English letters",
    steps: TYPING_STEPS,
    chart: true,
    faq: [
      {
        q: "Is this English to Kannada translation?",
        a: "No. You type the sound of a Kannada word, and the page writes it in Kannada script. English words are not translated.",
      },
      {
        q: "Which spelling rules does it use?",
        a: 'The Baraha phonetic scheme, which many Kannada typists already know. Capital letters give long vowels and retroflex sounds. ISO 15919 is also available for typists who use Latin letters with marks.',
      },
      {
        q: "How do I type a conjunct such as ಜ್ಞ or ಕ್ಷ?",
        a: "Type the consonants one after another: the editor joins them. The word list on this page shows common words with their spellings.",
      },
      {
        q: "Can I convert Kannada back to English letters?",
        a: "Not on this page yet. The kannadi library can, in Baraha, ISO 15919 and IAST spellings.",
      },
      UPLOAD,
    ],
  },
];
