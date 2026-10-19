#!/usr/bin/env node
// Puts together what shroom.am serves, in site/public (never committed):
//
//   index.html, favicon.svg, favicon.png   the landing, as it is
//   how-to-poster.webp                     the how-to video's still (DECISIONS B194)
//   _/next.html                            site/next.html, as it is: the landing with the
//                                          voice-cut hero, at the hidden /next
//   _/how-to.mp4                           the how-to video, served at /how-to.mp4 by
//                                          worker.js, in pieces (DECISIONS B194)
//   og.png                                 the link preview card (tools/render-og-card.mjs)
//   _/install.sh                           installer/install.sh, as it is
//   _/agent.md                             site/agent.md, as it is
//   _/agent.html                           the same markdown, as a page for a browser
//   _/cli.md, _/cli.html                   site/cli.md — the guide to the command line — the same two ways
//   _/whats-new/<version>.html             each release's "What's new", from
//                                          docs/release-notes/<version>.md (DECISIONS B152)
//   _/whats-new/latest.html                the newest of them
//   _/faq.html, _/privacy.html, _/terms.html, _/feedback.html
//                                          site/pages/<name>.html in the landing's look
//   clean.js                               site/clean.js, as it is: the feedback page
//                                          cleans a log with it before anything leaves
//
// site/worker.js answers from these. Run: node site/build.mjs [out-dir]
import { copyFileSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { FEEDBACK_QUESTIONS } from "./report.js";

const SITE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(SITE, "..");

/** The link preview card every page names (tools/render-og-card.mjs draws site/og.png). */
export const OG_IMAGE = `<meta property="og:image" content="https://shroom.am/og.png">
<meta property="og:image:width" content="1200">
<meta property="og:image:height" content="630">
<meta property="og:image:alt" content="shroom — Record. Share. Pay-as-you-go.">
<meta name="twitter:card" content="summary_large_image">`;

/** The other pages' header and footer. The landing has neither since N59 (DECISIONS B191): its
 * links sit at its foot. The header is the landing's as it was, Send feedback beside GitHub
 * (N55); the footer holds the landing's foot, the same links and the visits line (B192). */
export const HEADER = `<header>
  <div class="wrap">
    <a class="brand" href="/"><svg xmlns="http://www.w3.org/2000/svg" viewBox="18 20 84 90" aria-hidden="true">
<path class="stem" d="M50,68 C49,82 46,92 46,96 C46,101 52,104 60,104 C68,104 74,101 74,96 C74,92 71,82 70,68 Z" fill="#E5E9F0"/>
<path d="M24,64 C24,42 40,26 60,26 C80,26 96,42 96,64 C86,75 34,75 24,64 Z" fill="#88C0D0"/>
<circle cx="33" cy="58" r="3.4" fill="#ECEFF4"/>
<circle cx="88" cy="58" r="2.2" fill="#ECEFF4"/>
<circle cx="50" cy="44" r="6" fill="#ECEFF4"/>
<circle cx="72" cy="48" r="3.6" fill="#ECEFF4"/>
<circle cx="64" cy="58" r="2.6" fill="#ECEFF4"/>
</svg>shroom</a>
    <nav><a href="/feedback">Send feedback</a><a href="https://github.com/bagrat/shroom">GitHub</a></nav>
  </div>
</header>`;

export const FOOTER = `<footer><div class="wrap links">
  <nav><a href="/feedback">Send feedback</a><span class="sep"> · </span><a href="https://github.com/bagrat/shroom">GitHub</a><span class="sep"> · </span><a href="/faq">Questions and limits</a><span class="sep"> · </span><a href="/privacy">Privacy</a><span class="sep"> · </span><a href="/terms">Terms</a></nav>
  <p>We count visits to this page — no cookies, and nothing that tells us who you are.</p>
</div></footer>`;

const escapeHtml = (/** @type {string} */ s) =>
  s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

/** What frames the two pages for coding agents: the install page (/agent) and the guide to
 * the command line (/cli, dev/proposals/cli-agent-skill.md). */
const AGENT_PAGE = {
  path: "/agent",
  title: "shroom — for your coding agent",
  description: "What your coding agent reads when you ask it to install shroom.",
  intro: "This is what your coding agent reads when you ask it to install shroom — word for word.",
};
export const CLI_PAGE = {
  path: "/cli",
  title: "shroom — your coding agent and your videos",
  description: "What your coding agent reads when you ask it to use shroom for you.",
  intro: "This is what your coding agent reads when you ask it to do something with your shroom videos — word for word.",
};

/** The agent page for a browser: the landing's look, and the markdown exactly as an agent
 * reads it — "Read it first" shows the words themselves, not a retelling of them. */
export function agentPage(/** @type {string} */ landing, /** @type {string} */ markdown, page = AGENT_PAGE) {
  const style = landing.match(/<style>[\s\S]*?<\/style>/)?.[0];
  const header = HEADER;
  if (!style) throw new Error("site/index.html should carry one <style>");
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${page.title}</title>
<meta name="description" content="${page.description}">
<meta property="og:type" content="website">
<meta property="og:title" content="${page.title}">
<meta property="og:url" content="https://shroom.am${page.path}">
<meta property="og:description" content="${page.description}">
${OG_IMAGE}
<link rel="icon" type="image/svg+xml" href="favicon.svg">
<link rel="icon" type="image/png" sizes="180x180" href="favicon.png">
<link rel="apple-touch-icon" href="favicon.png">
<meta name="theme-color" content="#2b2f36" media="(prefers-color-scheme: dark)">
<meta name="theme-color" content="#f6f7f9" media="(prefers-color-scheme: light)">
${style.replace("</style>", `  main.agent { gap: 16px; padding-block: 24px 48px; }
  .agent p { color: var(--muted); margin: 0; }
  .agent pre { margin: 0; white-space: pre-wrap; overflow-wrap: anywhere; background: var(--well); border: 1px solid var(--line); border-radius: 12px; padding: 18px; font: 14px/1.6 var(--mono); color: var(--text); }
</style>`)}
</head>
<body>
${header}

<main class="wrap agent">
  <p>${page.intro}</p>
  <pre>${escapeHtml(markdown)}</pre>
</main>
</body>
</html>
`;
}

/** The release notes' versions, newest first.
 * @param {string} dir */
export function noteVersions(dir) {
  const key = (/** @type {string} */ v) => v.split(".").map(Number);
  return readdirSync(dir)
    .filter((f) => /^\d+\.\d+\.\d+\.md$/.test(f))
    .map((f) => f.slice(0, -3))
    .sort((a, b) => {
      const [x, y] = [key(a), key(b)];
      return y[0] - x[0] || y[1] - x[1] || y[2] - x[2];
    });
}

/** A release's "What's new" for a browser: the landing's look, and the notes' list. The notes
 * are a list, one change a line ("- …"); any other line is a short paragraph above it. Their
 * words are escaped, never read as markup (tools/release-notes.mjs checks them).
 * @param {string} landing @param {string} version @param {string} notes */
export function whatsNewPage(landing, version, notes) {
  const style = landing.match(/<style>[\s\S]*?<\/style>/)?.[0];
  const header = HEADER, footer = FOOTER;
  if (!style) throw new Error("site/index.html should carry one <style>");
  const lines = notes.split("\n").map((l) => l.trim()).filter(Boolean);
  const items = lines.filter((l) => /^[-*] /.test(l)).map((l) => `    <li>${escapeHtml(l.slice(2).trim())}</li>`);
  const paras = lines.filter((l) => !/^[-*] /.test(l)).map((l) => `  <p class="lead">${escapeHtml(l)}</p>\n`);
  const title = `What's new in shroom ${version}`;
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${escapeHtml(title)}</title>
<meta name="description" content="${escapeHtml(title)}.">
<meta property="og:type" content="website">
<meta property="og:title" content="${escapeHtml(title)}">
<meta property="og:url" content="https://shroom.am/whats-new/${version}">
<meta property="og:description" content="${escapeHtml(title)}.">
${OG_IMAGE}
<link rel="icon" type="image/svg+xml" href="/favicon.svg">
<link rel="icon" type="image/png" sizes="180x180" href="/favicon.png">
<link rel="apple-touch-icon" href="/favicon.png">
<meta name="theme-color" content="#2b2f36" media="(prefers-color-scheme: dark)">
<meta name="theme-color" content="#f6f7f9" media="(prefers-color-scheme: light)">
${style.replace("</style>", `  main.whats-new { gap: 28px; }
  .whats-new .lead { color: var(--muted); margin: 0; }
  .whats-new .more { color: var(--quiet); font-size: 14px; margin: 0; }
</style>`)}
</head>
<body>
${header}

<main class="wrap whats-new">
  <div class="hero">
    <h1>What's new</h1>
    <p class="punch">shroom ${version}</p>
  </div>
${paras.join("")}  <ul class="points">
${items.join("\n")}
  </ul>
  <p class="more">Every change, in detail, is <a href="https://github.com/bagrat/shroom/releases">on GitHub</a>.</p>
</main>

${footer}
</body>
</html>
`;
}

/** The pages under site/pages, each a fragment: its <title>, its description, then what
 * goes in <main> (an <h1> and a punch line first, as on the landing).
 * @type {string[]} */
export const PAGES = ["faq", "privacy", "terms", "feedback"];

/** The feedback form's Turnstile site key (public by design: it names the widget, and only
 * the Worker's TURNSTILE_SECRET can check a token): the invisible widget “shroom.am
 * feedback”, for shroom.am alone. Emptied, the form says sending is off. */
export const TURNSTILE_SITEKEY = "0x4AAAAAAFYi2j_R2IFjqx_7";

/** The feedback page's fragment with its questions and its site key filled in.
 * @param {string} fragment */
export function feedbackFragment(fragment, sitekey = TURNSTILE_SITEKEY) {
  const fields = FEEDBACK_QUESTIONS.map(
    (q, i) => `<div class="field">
    <label for="fb-q${i + 1}">${i + 1}. ${escapeHtml(q)}</label>
    <textarea id="fb-q${i + 1}" maxlength="4000"></textarea>
  </div>`,
  ).join("\n  ");
  return fragment
    .replace("{{questions}}", fields)
    .replace("{{questions-json}}", JSON.stringify(FEEDBACK_QUESTIONS).replace(/</g, "\\u003c"))
    .replace("{{sitekey}}", sitekey.replace(/[^A-Za-z0-9_-]/g, ""));
}

/** One of site/pages in the landing's look, with the pages' header and footer.
 * @param {string} landing @param {string} name @param {string} fragment */
export function docPage(landing, name, fragment) {
  const style = landing.match(/<style>[\s\S]*?<\/style>/)?.[0];
  const header = HEADER, footer = FOOTER;
  if (!style) throw new Error("site/index.html should carry one <style>");
  const title = fragment.match(/<title>([^<]*)<\/title>/)?.[1];
  const description = fragment.match(/<meta name="description" content="([^"]*)">/)?.[1];
  if (!title || !description) throw new Error(`site/pages/${name}.html should carry a <title> and a description`);
  const body = fragment
    .replace(/<!--[\s\S]*?-->\s*/g, "")
    .replace(/<title>[^<]*<\/title>\s*/, "")
    .replace(/<meta name="description"[^>]*>\s*/, "")
    .trim();
  // The hero: the <h1>, then the lines right under it (a punch line, a date).
  const [, hero, rest] = body.match(/^(<h1(?: [^>]*)?>[^<]*<\/h1>(?:\s*<p class="(?:punch|fine)"(?: [^>]*)?>[\s\S]*?<\/p>)*)\s*([\s\S]*)$/) ?? [];
  if (!hero) throw new Error(`site/pages/${name}.html should open with an <h1>`);
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>${title}</title>
<meta name="description" content="${description}">
<meta property="og:type" content="website">
<meta property="og:title" content="${title}">
<meta property="og:url" content="https://shroom.am/${name}">
<meta property="og:description" content="${description}">
${OG_IMAGE}
<link rel="icon" type="image/svg+xml" href="/favicon.svg">
<link rel="icon" type="image/png" sizes="180x180" href="/favicon.png">
<link rel="apple-touch-icon" href="/favicon.png">
<meta name="theme-color" content="#2b2f36" media="(prefers-color-scheme: dark)">
<meta name="theme-color" content="#f6f7f9" media="(prefers-color-scheme: light)">
${style.replace("</style>", `  main.doc { gap: 20px; }
  .doc h2 { margin-top: 20px; }
  .doc h3 { margin: 8px 0 0; color: var(--muted); }
  .qa li { display: flex; flex-direction: column; gap: 4px; }
  .qa li b { font-weight: 600; }
  .qa li span, .limits li, .prose li { color: var(--muted); }
  .prose li b { color: var(--text); }
  .doc code { font: 0.92em var(--mono); background: var(--well); padding: 1px 5px; border-radius: 5px; overflow-wrap: anywhere; }
  .doc .hero .fine { margin: 0; }
  /* What's coming: its words and the pill on one line; the words wrap beside it. */
  .limits li.soon { display: flex; align-items: center; justify-content: space-between; gap: 12px; color: var(--text); }
  .limits li.soon span { flex: 1 1 auto; min-width: 0; color: var(--text); }
  .pill { flex: none; white-space: nowrap; padding: 2px 10px; border: 1px solid var(--frost); border-radius: 999px; background: color-mix(in srgb, var(--frost) 14%, transparent); color: var(--frost); font: 600 12px/1.6 var(--sans); font-style: normal; letter-spacing: 0.02em; }
  .doc .fine { margin-top: 12px; }
</style>`)}
</head>
<body>
${header}

<main class="wrap doc">
  <div class="hero">
    ${hero.replace(/\n\s*/g, "\n    ")}
  </div>
${rest.split("\n").map((l) => (l ? `  ${l}` : l)).join("\n")}
</main>

${footer}
</body>
</html>
`;
}

/** @param {string} out */
export function build(out) {
  rmSync(out, { recursive: true, force: true });
  mkdirSync(join(out, "_"), { recursive: true });
  for (const f of ["index.html", "favicon.svg", "favicon.png", "og.png", "how-to-poster.webp"]) copyFileSync(join(SITE, f), join(out, f));
  copyFileSync(join(ROOT, "installer/install.sh"), join(out, "_/install.sh"));
  copyFileSync(join(SITE, "how-to.mp4"), join(out, "_/how-to.mp4"));
  copyFileSync(join(SITE, "next.html"), join(out, "_/next.html"));
  const markdown = readFileSync(join(SITE, "agent.md"), "utf8");
  writeFileSync(join(out, "_/agent.md"), markdown);
  const landing = readFileSync(join(SITE, "index.html"), "utf8");
  writeFileSync(join(out, "_/agent.html"), agentPage(landing, markdown));
  const cli = readFileSync(join(SITE, "cli.md"), "utf8");
  writeFileSync(join(out, "_/cli.md"), cli);
  writeFileSync(join(out, "_/cli.html"), agentPage(landing, cli, CLI_PAGE));
  for (const name of PAGES) {
    const fragment = readFileSync(join(SITE, "pages", `${name}.html`), "utf8");
    writeFileSync(join(out, `_/${name}.html`), docPage(landing, name, name === "feedback" ? feedbackFragment(fragment) : fragment));
  }
  copyFileSync(join(SITE, "clean.js"), join(out, "clean.js"));
  const notes = join(ROOT, "docs/release-notes");
  const versions = noteVersions(notes);
  mkdirSync(join(out, "_/whats-new"), { recursive: true });
  for (const v of versions) {
    const page = whatsNewPage(landing, v, readFileSync(join(notes, `${v}.md`), "utf8"));
    writeFileSync(join(out, `_/whats-new/${v}.html`), page);
    if (v === versions[0]) writeFileSync(join(out, "_/whats-new/latest.html"), page);
  }
  return out;
}

if (process.argv[1] && resolve(process.argv[1]) === fileURLToPath(import.meta.url)) {
  const out = build(resolve(process.argv[2] ?? join(SITE, "public")));
  console.log(`shroom.am, built in ${out}`);
}
