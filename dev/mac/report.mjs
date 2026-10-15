// The Mac in the loop, as one page: what `loop.sh` brought back from each run — every
// scenario's verdict, the checks that said no and its video — and what the session that ran
// them adds by hand (dev/mac/MANUAL.md §4). The page is published as a private artifact,
// its media beside it; nothing here reaches the network but `gh api` for each run's commit.
//
//   node dev/mac/report.mjs --out <dir> [--notes <notes.json>] <run dir>…
//
// A run dir is the folder `loop.sh` keeps a run in (its `run` file and `evidence/`;
// `loop.sh run` prints it). `<dir>` gets index.html and media/, ready to publish. The
// notes are the session's own words, all optional:
//
//   { "title": "…", "lede": "…", "summary": ["…"],
//     "scenarios": { "<run id>/<scenario>": { "note": "…", "issues": [3], "label": "expected" } },
//     "sections": [ { "title": "…", "intro": "…",
//                     "items": [ { "check": "…", "verdict": "pass|fail|partial|skip",
//                                  "note": "…", "issues": [4], "media": ["shot.png"] } ] } ],
//     "owed": [ { "what": "…", "where": "…" } ],
//     "issues": { "3": "the issue's title" } }
//
// Media paths in the notes are relative to the notes file. Videos get a poster frame when
// ffmpeg is on PATH.

import { execFileSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, readdirSync, readFileSync, writeFileSync } from "node:fs";
import { basename, dirname, extname, join, resolve } from "node:path";

const args = process.argv.slice(2);
const opt = (name) => {
  const i = args.indexOf(name);
  if (i < 0) return undefined;
  const v = args[i + 1];
  args.splice(i, 2);
  return v;
};
const out = opt("--out");
const notesPath = opt("--notes");
if (!out || args.length === 0) {
  console.error("usage: node dev/mac/report.mjs --out <dir> [--notes <notes.json>] <run dir>…");
  process.exit(64);
}
const notes = notesPath ? JSON.parse(readFileSync(notesPath, "utf8")) : {};
const notesDir = notesPath ? dirname(resolve(notesPath)) : process.cwd();
const repo = (() => {
  try {
    const url = execFileSync("git", ["remote", "get-url", "origin"], { encoding: "utf8" }).trim();
    return url.replace(/\.git$/, "").replace(/^.*[:/]([^/:]+\/[^/:]+)$/, "$1");
  } catch {
    return "";
  }
})();
mkdirSync(join(out, "media"), { recursive: true });
const hasFfmpeg = (() => {
  try {
    execFileSync("ffmpeg", ["-version"], { stdio: "ignore" });
    return true;
  } catch {
    return false;
  }
})();

const esc = (s) => String(s ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]);

/** A copy of `src` under media/ as `name`; its published path. */
function media(src, name) {
  copyFileSync(src, join(out, "media", name));
  return `media/${name}`;
}

/** A frame from the middle of a video, for its poster (when ffmpeg is here). */
function poster(video, name) {
  if (!hasFfmpeg) return null;
  try {
    const seconds = Number(execFileSync("ffprobe", ["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", video], { encoding: "utf8" })) || 2;
    execFileSync("ffmpeg", ["-v", "error", "-y", "-ss", String(seconds / 2), "-i", video, "-frames:v", "1", "-vf", "scale=640:-2", join(out, "media", name)]);
    return `media/${name}`;
  } catch {
    return null;
  }
}

/** Every JSON value in a drive's output: one that starts a line and ends on the first line
 * from there that closes it (a one-line value, or a printed one whose last line is `}`/`]`). */
function jsonIn(text) {
  const found = [];
  const lines = text.split("\n");
  for (let i = 0; i < lines.length; i++) {
    if (!/^[[{]/.test(lines[i])) continue;
    for (let j = i; j < lines.length; j++) {
      if (j > i && !/^[}\]]/.test(lines[j])) continue;
      try {
        found.push(JSON.parse(lines.slice(i, j + 1).join("\n")));
        i = j;
        break;
      } catch {}
    }
  }
  return found;
}

/** The checks a drive's report holds, whatever its shape: [what, ok, seen] rows, cases with
 * an `ok` and a name, bars with an `ok`, and `failed` / `skipped` lists. */
function checksIn(values) {
  const checks = new Map();
  const skipped = new Map();
  // A drive that reports a count rather than its checks (`{ passed: 90, failed: […] }`).
  let counted = null;
  const add = (what, ok, seen) => {
    const was = checks.get(what);
    if (!was || (was.ok && !ok)) checks.set(what, { what, ok, seen: typeof seen === "string" ? seen : JSON.stringify(seen ?? "") });
  };
  const walk = (v, key) => {
    if (Array.isArray(v)) {
      if (v.length >= 2 && typeof v[0] === "string" && typeof v[1] === "boolean") return add(v[0], v[1], v[2]);
      for (const x of v) {
        if (key === "failed" && x && typeof x === "object" && typeof x.what === "string") add(x.what, false, x.seen);
        else if (key === "skipped" && x && typeof x === "object" && typeof x.what === "string") skipped.set(x.what, x.why ?? "");
        else if (key === "skipped" && Array.isArray(x) && typeof x[0] === "string") skipped.set(x[0], x[1] ?? "");
        else walk(x);
      }
      return;
    }
    if (!v || typeof v !== "object") return;
    if (typeof v.passed === "number" && Array.isArray(v.failed)) counted = { passed: v.passed, total: v.passed + v.failed.length };
    const name = ["launch", "path", "case", "name"].map((k) => v[k]).find((n) => typeof n === "string");
    if (typeof v.ok === "boolean" && name) add(name, v.ok, (v.problems ?? []).join("; ") || v.error || "");
    for (const [k, x] of Object.entries(v)) {
      if (x && typeof x === "object" && !Array.isArray(x) && typeof x.ok === "boolean" && "bar" in x) add(k, x.ok, `${x.value} (bar ${x.bar})`);
      else walk(x, k);
    }
  };
  for (const v of values) walk(v);
  const list = [...checks.values()];
  const tally = list.length ? { passed: list.filter((c) => c.ok).length, total: list.length } : counted;
  return { checks: list, tally, skipped: [...skipped.entries()].map(([what, why]) => ({ what, why })) };
}

function ghRun(id) {
  try {
    return JSON.parse(execFileSync("gh", ["api", `repos/${repo}/actions/runs/${id}`, "--jq", "{sha: .head_sha, branch: .head_branch, at: .created_at, conclusion: .conclusion}"], { encoding: "utf8", stdio: ["ignore", "pipe", "ignore"] }));
  } catch {
    return {};
  }
}

// ── the runs ─────────────────────────────────────────────────────────────────────────────
const runs = args.map((dir) => {
  const id = readFileSync(join(dir, "run"), "utf8").trim();
  const ev = join(dir, "evidence");
  const rows = readFileSync(join(ev, "results.tsv"), "utf8").trim().split("\n").map((l) => l.split("\t"));
  const said = (name) => rows.find((r) => r[1] === name);
  const load = rows.find((r) => r[1] === "the Mac's load as the scenarios began")?.[2] ?? "";
  const scenarios = readdirSync(ev, { withFileTypes: true })
    .filter((d) => d.isDirectory() && d.name !== "hold" && existsSync(join(ev, d.name, "output.log")))
    .map((d) => d.name)
    .sort((a, b) => rows.findIndex((r) => r[1] === a) - rows.findIndex((r) => r[1] === b))
    .map((name) => {
      const row = said(name) ?? ["info", name, ""];
      const log = readFileSync(join(ev, name, "output.log"), "utf8");
      const code = row[0] === "pass" ? 0 : Number(/ended with (\d+)/.exec(row[2])?.[1] ?? 1);
      const { checks, tally, skipped } = checksIn(jsonIn(log));
      const video = join(ev, name, "video.mp4");
      const note = notes.scenarios?.[`${id}/${name}`] ?? {};
      // With no checks to show, the line that says most: the last error, else the last line.
      const lines = log.trim().split("\n").filter((l) => l.trim());
      const line = [...lines].reverse().find((l) => /\b(Error|error|refused|failed)\b/.test(l)) ?? lines.at(-1) ?? "";
      return {
        name,
        code,
        checks,
        tally,
        skipped,
        line: checks.length ? "" : line.trim(),
        video: existsSync(video) ? media(video, `${id}-${name}.mp4`) : null,
        poster: existsSync(video) ? poster(video, `${id}-${name}.jpg`) : null,
        recording: said(`${name}: the recording of it`)?.[2] ?? "",
        ...note,
      };
    });
  const hold = existsSync(join(ev, "hold", "video.mp4")) ? { video: media(join(ev, "hold", "video.mp4"), `${id}-hold.mp4`), poster: poster(join(ev, "hold", "video.mp4"), `${id}-hold.jpg`), said: said("held for a session")?.[2] ?? "" } : null;
  return { id, ...ghRun(id), load, scenarios, hold };
});

// ── the page ─────────────────────────────────────────────────────────────────────────────
const VERDICT = { pass: ["passed", "pass"], fail: ["failed", "fail"], partial: ["partly", "warn"], skip: ["not run", "skip"] };
const pill = (label, tone) => `<span class="pill ${tone}">${esc(label)}</span>`;
const issueLinks = (list) => (list ?? []).map((n) => `<a class="issue" href="https://github.com/${repo}/issues/${n}" title="${esc(notes.issues?.[n] ?? "")}">#${n}</a>`).join(" ");
const runLink = (id) => `<a href="https://github.com/${repo}/actions/runs/${id}">run ${esc(id)}</a>`;
const video = (src, posterSrc, label) => (src ? `<video controls playsinline preload="${posterSrc ? "none" : "metadata"}" ${posterSrc ? `poster="${posterSrc}"` : ""} src="${src}" aria-label="${esc(label)}"></video>` : "");

function scenarioCard(r, s) {
  const [label, tone] = s.label ? [s.label, "warn"] : s.code === 0 ? ["passed", "pass"] : s.code === 2 || s.code === 3 ? [`exit ${s.code}`, "warn"] : ["failed", "fail"];
  const failed = s.checks.filter((c) => !c.ok);
  const tally = s.tally ? `<span class="tally">${s.tally.passed} of ${s.tally.total} checks${s.skipped.length ? ` · ${s.skipped.length} skipped` : ""}</span>` : "";
  return `<article class="card" id="${esc(`${r.id}-${s.name}`)}">
  <header><h3>${esc(s.name)}</h3>${pill(label, tone)}${tally}</header>
  ${failed.length ? `<ul class="said">${failed.map((c) => `<li><span class="what">${esc(c.what)}</span> <span class="seen">${esc(c.seen)}</span></li>`).join("")}</ul>` : ""}
  ${!s.checks.length && s.line ? `<p class="said mono">${esc(s.line)}</p>` : ""}
  ${s.skipped.length ? `<p class="skipped">Skipped: ${s.skipped.map((k) => `${esc(k.what)} (${esc(k.why)})`).join("; ")}</p>` : ""}
  ${s.note || s.issues ? `<p class="note">${esc(s.note ?? "")} ${issueLinks(s.issues)}</p>` : ""}
  ${video(s.video, s.poster, `${s.name}, ${r.id}`)}
</article>`;
}

function runBlock(r) {
  const sha = r.sha ? `<code>${esc(r.sha.slice(0, 7))}</code>` : "";
  return `<section class="run">
  <h2 class="runhead">${runLink(r.id)} <span class="meta">${sha} ${r.branch ? `on <code>${esc(r.branch)}</code>` : ""}${r.at ? ` · ${esc(r.at.slice(11, 16))} UTC` : ""} · load ${esc(r.load.replace(/\{\s*|\s*\}/g, "").trim())}</span></h2>
  <div class="grid">${r.scenarios.map((s) => scenarioCard(r, s)).join("\n")}</div>
  ${r.hold ? `<article class="card wide"><header><h3>the held session</h3>${pill("held", "skip")}<span class="tally">${esc(r.hold.said)}</span></header>${video(r.hold.video, r.hold.poster, `the held session, ${r.id}`)}</article>` : ""}
</section>`;
}

function sectionBlock(sec) {
  const items = (sec.items ?? []).map((it) => {
    const [label, tone] = VERDICT[it.verdict] ?? [it.verdict ?? "", "skip"];
    const shown = (it.media ?? []).map((m) => {
      const src = media(resolve(notesDir, m), basename(m));
      return [".mp4", ".mov", ".webm"].includes(extname(m)) ? video(src, null, it.check) : `<img loading="lazy" src="${src}" alt="${esc(it.check)}">`;
    });
    return `<article class="card"><header><h3>${esc(it.check)}</h3>${pill(label, tone)}</header>${it.note ? `<p class="note">${esc(it.note)} ${issueLinks(it.issues)}</p>` : ""}${shown.join("")}</article>`;
  });
  return `<section><h2>${esc(sec.title)}</h2>${sec.intro ? `<p class="intro">${esc(sec.intro)}</p>` : ""}<div class="grid">${items.join("\n")}</div></section>`;
}

const all = runs.flatMap((r) => r.scenarios);
const tally = { pass: all.filter((s) => s.code === 0).length, warn: all.filter((s) => s.code !== 0 && (s.label || s.code === 2 || s.code === 3)).length };
tally.fail = all.length - tally.pass - tally.warn;
const shas = [...new Set(runs.map((r) => r.sha?.slice(0, 7)).filter(Boolean))];
const title = notes.title ?? "shroom on a hosted Mac";

const html = `<title>${esc(title)}</title>
<style>
/* Layout: one column of runs; each run a grid of scenario cards, the video under its verdict. Nord, as shroom.am. */
:root {
  --bg: #2E3440; --raised: #3B4252; --line: #434C5E; --text: #ECEFF4; --muted: #D8DEE9; --quiet: #a9b3c4;
  --frost: #88C0D0; --pass: #A3BE8C; --fail: #BF616A; --warn: #EBCB8B; --skip: #81A1C1;
  --sans: system-ui, -apple-system, "Segoe UI", sans-serif;
  --mono: ui-monospace, "SF Mono", SFMono-Regular, Menlo, monospace;
  color-scheme: dark;
}
@media (prefers-color-scheme: light) { :root:not([data-theme="dark"]) {
  --bg: #ECEFF4; --raised: #FFFFFF; --line: #D8DEE9; --text: #2E3440; --muted: #3B4252; --quiet: #4C566A;
  --frost: #4C6F96; --pass: #4F7A3A; --fail: #A3434D; --warn: #8A6A10; --skip: #4C6F96; color-scheme: light; } }
:root[data-theme="light"] {
  --bg: #ECEFF4; --raised: #FFFFFF; --line: #D8DEE9; --text: #2E3440; --muted: #3B4252; --quiet: #4C566A;
  --frost: #4C6F96; --pass: #4F7A3A; --fail: #A3434D; --warn: #8A6A10; --skip: #4C6F96; color-scheme: light; }
body { background: var(--bg); color: var(--text); font: 15px/1.5 var(--sans); }
.page { max-width: 1120px; margin: 0 auto; padding-inline: 16px; padding-block: 28px 56px; display: grid; gap: 36px; }
h1 { font-size: clamp(26px, 5vw, 36px); line-height: 1.15; margin: 0; letter-spacing: -0.01em; text-wrap: balance; }
h2 { font-size: 19px; margin: 0 0 14px; text-wrap: balance; }
h3 { font: 600 15px/1.3 var(--mono); margin: 0; }
p { margin: 0; max-width: 68ch; }
a { color: var(--frost); }
a:focus-visible, video:focus-visible { outline: 2px solid var(--frost); outline-offset: 2px; }
code, .mono { font-family: var(--mono); font-size: 0.92em; }
.top { display: grid; gap: 12px; }
.eyebrow { font: 600 12px/1 var(--mono); letter-spacing: 0.08em; text-transform: uppercase; color: var(--quiet); }
.lede { color: var(--muted); }
.counts { display: flex; flex-wrap: wrap; gap: 8px; }
.summary { display: grid; gap: 8px; padding-left: 1.1em; margin: 0; color: var(--muted); max-width: 72ch; }
.runhead { display: flex; flex-wrap: wrap; align-items: baseline; gap: 6px 12px; overflow-wrap: anywhere; }
.meta { font: 400 13px/1.4 var(--mono); color: var(--quiet); }
.grid { display: grid; grid-template-columns: repeat(auto-fill, minmax(min(100%, 330px), 1fr)); gap: 14px; }
.card { background: var(--raised); border: 1px solid var(--line); border-radius: 10px; padding: 14px; display: grid; gap: 10px; align-content: start; min-width: 0; overflow-wrap: anywhere; }
.card.wide { margin-top: 14px; }
.card header { display: flex; flex-wrap: wrap; align-items: center; gap: 6px 10px; }
.pill { font: 600 12px/1 var(--mono); padding: 5px 8px; border-radius: 999px; letter-spacing: 0.02em; }
.pill.pass { color: var(--pass); background: color-mix(in srgb, var(--pass) 16%, transparent); }
.pill.fail { color: var(--fail); background: color-mix(in srgb, var(--fail) 16%, transparent); }
.pill.warn { color: var(--warn); background: color-mix(in srgb, var(--warn) 16%, transparent); }
.pill.skip { color: var(--skip); background: color-mix(in srgb, var(--skip) 16%, transparent); }
.tally { font: 400 13px/1.3 var(--mono); color: var(--quiet); font-variant-numeric: tabular-nums; }
.said { margin: 0; padding: 0; list-style: none; display: grid; gap: 6px; font-size: 13.5px; }
.said li { border-left: 2px solid var(--fail); padding-left: 8px; overflow-wrap: anywhere; }
.said .what { font-weight: 600; }
.said .seen { font-family: var(--mono); color: var(--muted); font-size: 12.5px; }
p.said { color: var(--muted); overflow-wrap: anywhere; font-size: 12.5px; }
.skipped, .intro { color: var(--quiet); font-size: 13.5px; }
.note { color: var(--muted); font-size: 14px; }
.issue { font-family: var(--mono); font-size: 13px; white-space: nowrap; }
video, .card img { width: 100%; max-width: 100%; border-radius: 6px; border: 1px solid var(--line); background: #000; aspect-ratio: 4 / 3; }
.card img { aspect-ratio: auto; height: auto; }
.owed { display: grid; gap: 8px; padding-left: 1.1em; margin: 0; max-width: 72ch; }
.owed .where { color: var(--quiet); font-size: 13.5px; }
footer { color: var(--quiet); font-size: 13px; }
@media (prefers-reduced-motion: reduce) { * { scroll-behavior: auto; } }
</style>
<main class="page">
  <header class="top">
    <span class="eyebrow">The Mac in the loop · macOS 26.6.2 · 1024×768</span>
    <h1>${esc(title)}</h1>
    ${notes.lede ? `<p class="lede">${esc(notes.lede)}</p>` : ""}
    <div class="counts">${pill(`${tally.pass} passed`, "pass")}${tally.warn ? pill(`${tally.warn} expected or timing`, "warn") : ""}${tally.fail ? pill(`${tally.fail} failed`, "fail") : ""}<span class="tally">${all.length} scenario runs · ${runs.length} runs${shas.length ? ` · ${shas.map((s) => `<code>${s}</code>`).join(", ")}` : ""}</span></div>
    ${notes.summary?.length ? `<ul class="summary">${notes.summary.map((s) => `<li>${esc(s)}</li>`).join("")}</ul>` : ""}
  </header>
  ${runs.map(runBlock).join("\n")}
  ${(notes.sections ?? []).map(sectionBlock).join("\n")}
  ${notes.owed?.length ? `<section><h2>Still owed to a real Mac</h2><ul class="owed">${notes.owed.map((o) => `<li>${esc(o.what)} ${o.where ? `<span class="where">${esc(o.where)}</span>` : ""}</li>`).join("")}</ul></section>` : ""}
  <footer>Built by <code>dev/mac/report.mjs</code> from what <code>dev/mac/loop.sh</code> brought back. The videos are the runner's own screen.</footer>
</main>
`;
writeFileSync(join(out, "index.html"), html);
console.log(`${join(out, "index.html")}: ${runs.length} runs, ${all.length} scenarios (${tally.pass} passed, ${tally.warn} expected or timing, ${tally.fail} failed)`);
