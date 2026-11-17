// Writes the landing pages into a built web/dist: node web/site/build.mjs <dist> [site URL]
// Each page is the editor template with its own head, intro, how-to and FAQ.
// Canonical links, og:url and the sitemap need the site URL, so they are only
// written when it is given (or set as KANNADI_SITE_URL).
import { readFileSync, mkdirSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { pathToFileURL } from "node:url";

import { CHART, EXAMPLE_WORDS, PAGES } from "./pages.mjs";

const dist = resolve(process.argv[2] ?? "web/dist");
const site = siteUrl(process.argv[3] || process.env.KANNADI_SITE_URL || "");
const hasSite = site !== "";

/** The site URL with a trailing slash, or "" when none is given. */
function siteUrl(value) {
  if (!value) return "";
  const url = new URL(value.endsWith("/") ? value : `${value}/`);
  if (url.protocol !== "https:" || url.search || url.hash || url.username || url.password) {
    throw new Error(`The site URL must be a plain https address: ${value}`);
  }
  return url.href;
}

const wasm = await import(pathToFileURL(join(dist, "pkg", "kannadi_wasm.js")).href);
wasm.initSync({ module: readFileSync(join(dist, "pkg", "kannadi_wasm_bg.wasm")) });

const template = readFileSync(join(dist, "index.html"), "utf8");
const DEFAULT_LEAD = "Paste Nudi or Baraha text, open a TXT, Word or HTML file, or start typing.";

const escape = (s) => s.replace(/[<>&"]/g, (c) => ({ "<": "&lt;", ">": "&gt;", "&": "&amp;", '"': "&quot;" })[c]);
const plain = (html) =>
  html
    .replace(/<[^>]+>/g, "")
    .replace(/&lt;/g, "<")
    .replace(/&gt;/g, ">")
    .replace(/&quot;/g, '"')
    .replace(/&amp;/g, "&");

/** Replaces the text between <!-- page:NAME --> and <!-- /page:NAME -->. */
function fill(html, name, content) {
  const open = `<!-- page:${name} -->`;
  const close = `<!-- /page:${name} -->`;
  const start = html.indexOf(open);
  const end = html.indexOf(close);
  if (start < 0 || end < start) throw new Error(`template has no ${name} slot`);
  return html.slice(0, start + open.length) + "\n" + content + "\n" + html.slice(end);
}

/** Makes the template's relative asset links work from a page `depth` folders down. */
function rebase(html, root) {
  if (!root) return html;
  return html.replace(/(?<![\w-])(href|src)="(?![a-z]+:|#|\/)([^"]*)"/g, (_, attr, url) =>
    `${attr}="${url === "./" || url === "" ? root : root + url}"`,
  );
}

function head(page) {
  const url = hasSite ? site + page.path : null;
  // Social sites ignore relative image URLs, so the card needs the site URL.
  const image = hasSite ? `${site}og/${page.path ? page.path.slice(0, -1) : "home"}.png` : null;
  const app = {
    "@context": "https://schema.org",
    "@type": "SoftwareApplication",
    name: page.h1,
    description: plain(page.description),
    applicationCategory: "UtilitiesApplication",
    operatingSystem: "Any",
    inLanguage: ["en", "kn"],
    isAccessibleForFree: true,
    offers: { "@type": "Offer", price: "0", priceCurrency: "INR" },
    author: { "@type": "Person", name: "Prasad Subrahmanya", url: "https://prasad.tech" },
    ...(url ? { url } : {}),
  };
  const faq = {
    "@context": "https://schema.org",
    "@type": "FAQPage",
    mainEntity: page.faq.filter((f) => !f.shared || page.path === "").map(({ q, a }) => ({
      "@type": "Question",
      name: q,
      acceptedAnswer: { "@type": "Answer", text: plain(a) },
    })),
  };
  const json = (o) => JSON.stringify(o, null, 2).replace(/</g, "\\u003c");
  return [
    `<title>${escape(page.title)}</title>`,
    `<meta name="description" content="${escape(page.description)}">`,
    url ? `<link rel="canonical" href="${escape(url)}">` : "",
    `<meta property="og:type" content="website">`,
    `<meta property="og:title" content="${escape(page.title)}">`,
    `<meta property="og:description" content="${escape(page.description)}">`,
    url ? `<meta property="og:url" content="${escape(url)}">` : "",
    image ? `<meta property="og:image" content="${escape(image)}">` : "",
    image ? `<meta property="og:image:width" content="1200">` : "",
    image ? `<meta property="og:image:height" content="630">` : "",
    `<meta name="twitter:card" content="${image ? "summary_large_image" : "summary"}">`,
    `<script type="application/ld+json">\n${json(app)}\n</script>`,
    `<script type="application/ld+json">\n${json(faq)}\n</script>`,
  ]
    .filter(Boolean)
    .join("\n");
}

function chart(kind) {
  const spell = (k) => [wasm.transliterate(k, "kannada", "baraha"), wasm.transliterate(k, "kannada", "iso15919")];
  const groups = Object.entries(CHART)
    .map(([name, letters]) => {
      const cells = letters
        .map((k) => {
          const [b, i] = spell(k);
          return `<li><span class="chart-kn" lang="kn">${k}</span><kbd>${escape(b)}</kbd><span class="chart-iso">${escape(i)}</span></li>`;
        })
        .join("");
      return `<h3>${escape(name)}</h3>\n<ul class="chart" role="list">${cells}</ul>`;
    })
    .join("\n");
  const rows = EXAMPLE_WORDS.map((k) => {
    const [b, i] = spell(k);
    return `<tr><td lang="kn">${k}</td><td><kbd>${escape(b)}</kbd></td><td>${escape(i)}</td></tr>`;
  }).join("");
  if (kind === "words") {
    return `<section aria-labelledby="chart-heading">
<h2 id="chart-heading">Common Kannada words in English letters</h2>
<p>Type the spelling in the middle column to get the word. Capitals matter: <kbd>D</kbd> is <span lang="kn">ಡ</span> and <kbd>d</kbd> is <span lang="kn">ದ</span>.</p>
<table class="words"><thead><tr><th scope="col">Kannada</th><th scope="col">Type in Baraha</th><th scope="col">ISO 15919</th></tr></thead><tbody>${rows}</tbody></table>
</section>`;
  }
  return `<section aria-labelledby="chart-heading">
<h2 id="chart-heading">Kannada letters in English letters</h2>
<p>Each letter with its Baraha spelling, which this page uses, and its ISO 15919 spelling.</p>
${groups}
</section>`;
}

function prose(page, root) {
  const steps = page.steps.map((s) => `<li>${s}</li>`).join("\n");
  const faq = page.faq.map(({ q, a }) => `<h3>${escape(q)}</h3>\n<p>${a}</p>`).join("\n");
  const links = PAGES.filter((p) => p !== page)
    .map((p) => `<li><a href="${p.path ? root + p.path : root || "./"}">${escape(p.h1)}</a></li>`)
    .join("\n");
  // FAQ links are written relative to a page 1 folder down.
  const relink = (html) => (root ? html : html.replace(/href="\.\.\//g, 'href="'));
  return relink(`<section aria-labelledby="how-heading">
<h2 id="how-heading">${escape(page.howTitle)}</h2>
<ol>
${steps}
</ol>
</section>
${page.chart ? chart(page.chart) : ""}
<section aria-labelledby="faq-heading">
<h2 id="faq-heading">Questions</h2>
${faq}
</section>
<nav aria-labelledby="more-heading" class="more">
<h2 id="more-heading">More Kannada tools</h2>
<ul>
${links}
</ul>
</nav>`);
}

for (const page of PAGES) {
  const root = page.path ? "../".repeat(page.path.split("/").length - 1) : "";
  let html = rebase(template, root);
  html = fill(html, "head", head(page));
  html = fill(html, "intro", `<h1>${escape(page.h1)}</h1>\n<p>${page.intro}</p>`);
  html = fill(html, "lead", `<p class="empty-lead">${page.lead ?? DEFAULT_LEAD}</p>`);
  html = fill(html, "prose", prose(page, root));
  html = html.replace(/<html lang="en"[^>]*>/, `<html lang="en"${page.mode !== "kannada" ? ` data-default-mode="${page.mode}"` : ""}>`);
  mkdirSync(join(dist, page.path), { recursive: true });
  writeFileSync(join(dist, page.path, "index.html"), html);
}

const robots = ["User-agent: *", "Allow: /"];
if (hasSite) {
  const urls = PAGES.map((p) => `  <url><loc>${escape(site + p.path)}</loc></url>`).join("\n");
  writeFileSync(
    join(dist, "sitemap.xml"),
    `<?xml version="1.0" encoding="UTF-8"?>\n<urlset xmlns="http://www.sitemaps.org/schemas/sitemap/0.9">\n${urls}\n</urlset>\n`,
  );
  robots.push(`Sitemap: ${site}sitemap.xml`);
}
writeFileSync(join(dist, "robots.txt"), robots.join("\n") + "\n");

const llms = [
  "# kannadi Kannada editor",
  "",
  "> A free Kannada editor that runs in the browser. It converts Kannada text typed in the old Nudi and Baraha ANSI fonts to Unicode, marks every glyph it could not convert with fixes, converts Unicode back to Nudi glyph codes, types Kannada from English letters (Baraha or ISO 15919), and saves as DOCX, TXT, HTML or Nudi text. Nothing is uploaded.",
  "",
  ...PAGES.map((p) => `- [${p.h1}](${hasSite ? site + p.path : p.path || "./"}): ${plain(p.description)}`),
  "",
  "Source: https://github.com/openkannada/kannadi (MIT).",
];
writeFileSync(join(dist, "llms.txt"), llms.join("\n") + "\n");

console.log(`Wrote ${PAGES.length} pages${hasSite ? ` and a sitemap for ${site}` : "; no site URL, so no canonical links or sitemap"}`);
