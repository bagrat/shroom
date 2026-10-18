// Whisper's prime and its voice-activity filter (B205 a), checked the way the hearing step sends
// sound — AAC in ADTS frames — on the dedicated test account (TESTING-PLAN D3), through a probe
// Worker's AI binding that's deleted after. Lean: a minute or two a clip, three ways each.
//
//   SHROOM_E2E_TOKEN=… SHROOM_E2E_ACCOUNT_ID=… node worker/dev/ai-prime.mjs [out.json]
//
// The clips: a silent minute, a room's noise, two made-up pieces of music (no one's recording),
// and the first 90 s of the public how-to video (site/how-to.mp4). The voice cuts' tester
// measured the cue itself (dev/proposals/voice-cut/poc, on its branch); this checks what the
// hearing step would write into minutes nobody speaks in, and that speech keeps its words and times.

import { execFileSync } from "node:child_process";
import { randomBytes } from "node:crypto";
import { mkdtempSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { AI_HEAR_PRIME, AI_MODELS } from "../src/worker.js";

const A = process.env.SHROOM_E2E_ACCOUNT_ID, T = process.env.SHROOM_E2E_TOKEN;
if (!A || !T) throw new Error("needs SHROOM_E2E_ACCOUNT_ID and SHROOM_E2E_TOKEN (the test account's)");
const [outFile] = process.argv.slice(2);
const repo = new URL("../../", import.meta.url).pathname;

const NAME = "shroom-probe-ai-prime";
const KEY = randomBytes(16).toString("hex");
const api = `https://api.cloudflare.com/client/v4/accounts/${A}`;
const PROBE = `export default { async fetch(r, env) {
  if (r.method !== "POST" || r.headers.get("x-key") !== ${JSON.stringify(KEY)}) return new Response("", { status: 404 });
  const { model, audio, options } = await r.json();
  const bytes = Uint8Array.from(atob(audio), (c) => c.charCodeAt(0));
  const body = new ReadableStream({ start(c) { c.enqueue(bytes); c.close(); } });
  try { return Response.json({ ok: true, result: await env.AI.run(model, { audio: { body, contentType: "audio/aac" }, ...options }) }); } catch (e) { return Response.json({ ok: false, error: String(e?.message ?? e) }, { status: 500 }); }
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

async function hear(aac, options) {
  const r = await fetch(`https://${host}/`, { method: "POST", headers: { "x-key": KEY, "content-type": "application/json" }, body: JSON.stringify({ model: AI_MODELS.hear, audio: aac.toString("base64"), options }) });
  const j = await r.json();
  if (!j.ok) throw new Error(`${r.status} ${j.error}`);
  const words = [];
  for (const seg of j.result.segments ?? []) for (const w of seg.words ?? []) words.push([String(w.word).trim(), +w.start, +w.end]);
  return words;
}

/** A clip as the hearing step sends it: AAC-LC, 48 kHz mono, in ADTS frames. */
function clip(dir, name, input) {
  const out = join(dir, `${name}.aac`);
  execFileSync("ffmpeg", ["-hide_banner", "-loglevel", "error", ...input, "-ac", "1", "-ar", "48000", "-c:a", "aac", "-b:a", "128k", "-f", "adts", out]);
  return readFileSync(out);
}

const dir = mkdtempSync(join(tmpdir(), "ai-prime-"));
const CLIPS = {
  silence: ["-f", "lavfi", "-i", "anullsrc=r=48000:cl=mono", "-t", "60"],
  "room noise": ["-f", "lavfi", "-i", "anoisesrc=color=pink:amplitude=0.02:r=48000", "-t", "60"],
  "music (chords and a beat)": ["-f", "lavfi", "-i", "aevalsrc='0.15*sin(2*PI*220*t)+0.12*sin(2*PI*277.2*t)+0.12*sin(2*PI*329.6*t)+0.3*sin(2*PI*55*t)*exp(-12*mod(t,0.5))':s=48000", "-t", "60"],
  "music (a slow pad)": ["-f", "lavfi", "-i", "aevalsrc='(0.12*sin(2*PI*196*t)+0.1*sin(2*PI*246.9*t)+0.1*sin(2*PI*293.7*t))*(0.6+0.4*sin(2*PI*0.2*t))':s=48000", "-t", "60"],
  "how-to (speech)": ["-i", join(repo, "site/how-to.mp4"), "-vn", "-t", "90"],
};
const VARIANTS = { "no prime": {}, "VAD only": { vad_filter: true }, "prime + VAD": { vad_filter: true, initial_prompt: AI_HEAR_PRIME } };
const report = { at: new Date().toISOString(), model: AI_MODELS.hear, prime: AI_HEAR_PRIME, clips: {} };
try {
  await up();
  for (const [name, input] of Object.entries(CLIPS)) {
    const aac = clip(dir, name.replace(/\W+/g, "-"), input);
    const row = (report.clips[name] = {});
    for (const [v, options] of Object.entries(VARIANTS)) {
      const words = await hear(aac, options);
      row[v] = { words: words.length, text: words.map((w) => w[0]).join(" ").slice(0, 200), times: words };
      console.log(`${name} · ${v}: ${words.length} words${words.length ? ` — “${row[v].text.slice(0, 80)}”` : ""}`);
    }
  }
  // Speech: the same words, on the same clock.
  const sp = report.clips["how-to (speech)"];
  const base = sp["no prime"].times;
  for (const v of ["VAD only", "prime + VAD"]) {
    const other = sp[v].times;
    const shifts = [];
    let j = 0;
    for (const [w, s] of base) {
      const k = other.findIndex((o, i) => i >= j && o[0].toLowerCase().replace(/\W/g, "") === w.toLowerCase().replace(/\W/g, "") && Math.abs(o[1] - s) < 2);
      if (k >= 0) { shifts.push(other[k][1] - s); j = k + 1; }
    }
    shifts.sort((a, b) => a - b);
    const q = (p) => shifts.length ? Math.round(shifts[Math.floor(p * (shifts.length - 1))] * 100) / 100 : null;
    report[`speech ${v}`] = { paired: shifts.length, of: base.length, median: q(0.5), p05: q(0.05), p95: q(0.95) };
    console.log(`speech · ${v}: ${shifts.length} of ${base.length} words paired; shift median ${q(0.5)} s, 5–95% ${q(0.05)}…${q(0.95)} s`);
  }
  for (const c of Object.values(report.clips)) for (const v of Object.values(c)) delete v.times;
} finally {
  await down();
  if (outFile) writeFileSync(outFile, JSON.stringify(report, null, 1));
}
