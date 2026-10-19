// The early title's trigger (B205 f), measured: a title from the first 30, 45, 60 and 90 s of a
// talk beside the one from all of it, with the Worker's own words, call and model, on the
// dedicated test account (TESTING-PLAN D3), through a probe Worker's AI binding (the account's
// key has no Workers AI power of its own, as a space's hasn't) that's deleted after. Lean: one
// hearing and a few small calls a talk.
//
//   SHROOM_E2E_TOKEN=… SHROOM_E2E_ACCOUNT_ID=… node worker/dev/ai-early.mjs [out.json]
//
// The talks: the public how-to video (site/how-to.mp4, heard once by Whisper turbo), and the
// two invented scripts the proposal measured (dev/proposals/ai-titles-chapters/poc/results).

import { execFileSync } from "node:child_process";
import { randomBytes } from "node:crypto";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { AI_MODELS, aiAskInput, aiSentences, aiText, aiTitle } from "../src/worker.js";

const A = process.env.SHROOM_E2E_ACCOUNT_ID, T = process.env.SHROOM_E2E_TOKEN;
if (!A || !T) throw new Error("needs SHROOM_E2E_ACCOUNT_ID and SHROOM_E2E_TOKEN (the test account's)");
const [outFile] = process.argv.slice(2);
const repo = new URL("../../", import.meta.url).pathname;
const CUTS = [30, 45, 60, 90];
// Neurons a million tokens, from Workers AI's prices ($0.011 a thousand neurons).
const RATES = { [AI_MODELS.name]: { in: 0.35 / 0.011 * 1000, out: 0.75 / 0.011 * 1000 } };

const NAME = "shroom-probe-ai-early";
const KEY = randomBytes(16).toString("hex");
const api = `https://api.cloudflare.com/client/v4/accounts/${A}`;
const PROBE = `export default { async fetch(r, env) {
  if (r.method !== "POST" || r.headers.get("x-key") !== ${JSON.stringify(KEY)}) return new Response("", { status: 404 });
  const { model, input } = await r.json();
  try { return Response.json({ ok: true, result: await env.AI.run(model, input) }); } catch (e) { return Response.json({ ok: false, error: String(e?.message ?? e) }, { status: 500 }); }
} };`;
let host = "";

async function cf(method, path, body, headers = {}) {
  const r = await fetch(api + path, { method, headers: { Authorization: `Bearer ${T}`, ...headers }, body });
  const j = await r.json().catch(() => ({}));
  if (!r.ok && !(method === "DELETE" && r.status === 404)) throw new Error(`${method} ${path}: ${r.status} ${JSON.stringify(j.errors)}`);
  return j;
}
async function up() {
  const fd = new FormData();
  fd.append("metadata", new Blob([JSON.stringify({ main_module: "probe.js", compatibility_date: "2026-09-01", bindings: [{ type: "ai", name: "AI" }] })], { type: "application/json" }));
  fd.append("probe.js", new Blob([PROBE], { type: "application/javascript+module" }), "probe.js");
  await cf("PUT", `/workers/scripts/${NAME}`, fd);
  await cf("POST", `/workers/scripts/${NAME}/subdomain`, JSON.stringify({ enabled: true }), { "content-type": "application/json" });
  host = `${NAME}.${(await cf("GET", "/workers/subdomain")).result.subdomain}.workers.dev`;
  for (let i = 0; i < 30; i++) {
    const r = await fetch(`https://${host}/`, { method: "POST", headers: { "x-key": "-" } }).catch(() => null);
    if (r?.status === 404) return;
    await new Promise((ok) => setTimeout(ok, 3000));
  }
  throw new Error("the probe never answered");
}
const down = () => cf("DELETE", `/workers/scripts/${NAME}?force=true`).catch((e) => console.error("probe:", e.message));

async function run(model, input) {
  const r = await fetch(`https://${host}/`, { method: "POST", headers: { "x-key": KEY, "content-type": "application/json" }, body: JSON.stringify({ model, input }) });
  const j = await r.json();
  if (!j.ok) throw new Error(`${model}: ${r.status} ${j.error}`);
  return j.result;
}

/** The how-to video's words, [text, start, end], heard once. */
async function heard() {
  const dir = mkdtempSync(join(tmpdir(), "ai-early-"));
  execFileSync("ffmpeg", ["-hide_banner", "-loglevel", "error", "-i", join(repo, "site/how-to.mp4"), "-vn", "-ac", "1", "-ar", "16000", "-b:a", "48k", join(dir, "a.mp3")]);
  const out = await run(AI_MODELS.hear, { audio: readFileSync(join(dir, "a.mp3")).toString("base64") });
  const words = [];
  for (const seg of out.segments ?? []) for (const w of seg.words ?? []) words.push([String(w.word).trim(), +w.start, +w.end]);
  return { words, seconds: out.transcription_info?.duration ?? words.at(-1)?.[2] ?? 0 };
}

/** An invented script, [[start, sentence], …], spread a word at a time over each sentence's span. */
function spread(lines) {
  const words = [];
  lines.forEach(([s, text], i) => {
    const e = i + 1 < lines.length ? lines[i + 1][0] : s + text.split(" ").length * 0.4;
    const ws = text.split(" ");
    const step = Math.min(0.45, (e - s) / ws.length);
    ws.forEach((w, j) => words.push([w, +(s + j * step).toFixed(2), +(s + (j + 0.9) * step).toFixed(2)]));
  });
  return { words, seconds: words.at(-1)[2] };
}

async function title(words, chapters) {
  const model = AI_MODELS.name;
  const out = await run(model, aiAskInput(model, aiSentences(words), chapters));
  const text = aiText(out);
  const json = JSON.parse(text.slice(text.indexOf("{"), text.lastIndexOf("}") + 1));
  const u = out.usage ?? {};
  const neurons = ((u.prompt_tokens ?? 0) * RATES[model].in + (u.completion_tokens ?? 0) * RATES[model].out) / 1e6;
  return { title: aiTitle(json.title), tokensIn: u.prompt_tokens, tokensOut: u.completion_tokens, neurons: Math.round(neurons * 10) / 10 };
}

const invented = JSON.parse(readFileSync(join(repo, "dev/proposals/ai-titles-chapters/poc/results/transcripts-invented.json"), "utf8"));
const report = { at: new Date().toISOString(), model: AI_MODELS.name, talks: {} };
try {
  await up();
  const talks = { "how-to (real)": await heard(), walkthrough: spread(invented.walkthrough), onboarding: spread(invented.onboarding) };
  for (const [name, { words, seconds }] of Object.entries(talks)) {
    const row = (report.talks[name] = { seconds: Math.round(seconds), words: words.length, full: await title(words, seconds >= 90), early: {} });
    for (const cut of CUTS.filter((c) => c < seconds)) {
      const first = words.filter(([, s]) => s < cut);
      row.early[cut] = first.length >= 20 ? { words: first.length, ...(await title(first, false)) } : { words: first.length, title: null };
    }
    console.log(`${name} (${row.seconds} s): full “${row.full.title}”`);
    for (const [cut, r] of Object.entries(row.early)) console.log(`  first ${cut} s, ${r.words} words: “${r.title ?? "(too few words)"}”${r.neurons ? `, ${r.neurons} neurons` : ""}`);
  }
} finally {
  await down();
  if (outFile) writeFileSync(outFile, JSON.stringify(report, null, 1));
}
