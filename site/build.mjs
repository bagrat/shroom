#!/usr/bin/env node
// Puts together what shroom.am serves, in site/public (never committed):
//
//   index.html, favicon.svg, favicon.png   the landing, as it is
//   _/install.sh                           installer/install.sh, as it is
//   _/agent.md                             site/agent.md, as it is
//   _/agent.html                           the same markdown, as a page for a browser
//   _/whats-new/<version>.html             each release's "What's new", from
//                                          docs/release-notes/<version>.md (DECISIONS B152)
//   _/whats-new/latest.html                the newest of them
//
// site/worker.js answers from these. Run: node site/build.mjs [out-dir]
import { copyFileSync, mkdirSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const SITE = dirname(fileURLToPath(import.meta.url));
const ROOT = resolve(SITE, "..");

const escapeHtml = (/** @type {string} */ s) =>
  s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;");

/** The agent page for a browser: the landing's look, and the markdown exactly as an agent
 * reads it — "Read it first" shows the words themselves, not a retelling of them. */
export function agentPage(/** @type {string} */ landing, /** @type {string} */ markdown) {
  const style = landing.match(/<style>[\s\S]*?<\/style>/)?.[0];
  const header = landing.match(/<header>[\s\S]*?<\/header>/)?.[0];
  if (!style || !header) throw new Error("site/index.html should carry one <style> and one <header>");
  return `<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width, initial-scale=1">
<title>shroom — for your coding agent</title>
<meta name="description" content="What your coding agent reads when you ask it to install shroom.">
<meta property="og:type" content="website">
<meta property="og:title" content="shroom — for your coding agent">
<meta property="og:url" content="https://shroom.am/agent">
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
  <p>This is what your coding agent reads when you ask it to install shroom — word for word.</p>
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
  const header = landing.match(/<header>[\s\S]*?<\/header>/)?.[0];
  const footer = landing.match(/<footer>[\s\S]*?<\/footer>/)?.[0];
  if (!style || !header || !footer) throw new Error("site/index.html should carry one <style>, one <header> and one <footer>");
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

/** @param {string} out */
export function build(out) {
  rmSync(out, { recursive: true, force: true });
  mkdirSync(join(out, "_"), { recursive: true });
  for (const f of ["index.html", "favicon.svg", "favicon.png"]) copyFileSync(join(SITE, f), join(out, f));
  copyFileSync(join(ROOT, "installer/install.sh"), join(out, "_/install.sh"));
  const markdown = readFileSync(join(SITE, "agent.md"), "utf8");
  writeFileSync(join(out, "_/agent.md"), markdown);
  const landing = readFileSync(join(SITE, "index.html"), "utf8");
  writeFileSync(join(out, "_/agent.html"), agentPage(landing, markdown));
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
