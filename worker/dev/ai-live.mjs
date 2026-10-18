// Names, chapters and captions, live (N66, N67; DECISIONS B205–B209): the Worker as the app
// deploys it, with the AI binding and the minute's schedule, on the dedicated test account
// (TESTING-PLAN D3), driven the way the Macs drive it — one process that cleans up after itself:
//
//   SHROOM_E2E_TOKEN=… SHROOM_E2E_ACCOUNT_ID=… node worker/dev/ai-live.mjs <take dir> [out.json]
//
// <take dir> holds a take as shroom's encoder writes it (init.mp4, seg_00000.m4s, …).
// 1. A bucket and the Worker (`shroom-probe-ai`): src/worker.js byte for byte, the AI binding,
//    a made-up parent for this run (the setup Mac's proof), a schedule each minute.
// 2. Take A (25 pieces, 150 s) asked for as a recording Mac would: a minute at a time, then
//    done until it's named. Take B (22 pieces) asked once, then left to the schedule.
// 3. What the space made: each record, the captions, the page; Cloudflare's own meter for the
//    run (neurons by model; CPU time by invocation), read with a one-hour Account Analytics
//    Read token made here and deleted at once, never written anywhere.
// 4. The Worker, its schedule, the objects and the bucket deleted, whatever happened.

import { createHash, createHmac, randomBytes } from "node:crypto";
import { readFileSync, readdirSync, writeFileSync } from "node:fs";

const A = process.env.SHROOM_E2E_ACCOUNT_ID, T = process.env.SHROOM_E2E_TOKEN;
if (!A || !T) throw new Error("needs SHROOM_E2E_ACCOUNT_ID and SHROOM_E2E_TOKEN (the test account's)");
const [dir, outFile] = process.argv.slice(2);
if (!dir) throw new Error("usage: node worker/dev/ai-live.mjs <take dir> [out.json]");
const NAME = "shroom-probe-ai";
const api = `https://api.cloudflare.com/client/v4/accounts/${A}`;
const H = { Authorization: `Bearer ${T}` };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const sha = (b) => createHash("sha256").update(b).digest("hex");
const COMPATIBILITY_DATE = readFileSync(new URL("../../app/src-tauri/src/runtime/cloud.rs", import.meta.url), "utf8").match(/const COMPATIBILITY_DATE: &str = "([0-9-]+)";/)[1];
const PARENT = { account_id: A, bucket: NAME, key_id: randomBytes(16).toString("hex"), secret: randomBytes(32).toString("hex") };
const TAKES = { A: { id: `ProbeA${randomBytes(5).toString("hex")}`, pieces: 25 }, B: { id: `ProbeB${randomBytes(5).toString("hex")}`, pieces: 22 } };
const written = [];

async function cf(method, path, body, headers = {}, tries = 6) {
  const r = await fetch(api + path, { method, headers: { ...H, ...headers }, body });
  const j = await r.json().catch(() => ({}));
  if ((r.status === 409 && /10073/.test(JSON.stringify(j.errors))) || r.status >= 500) {
    if (tries > 0) return await sleep(2000), cf(method, path, body, headers, tries - 1);
  }
  if (!r.ok && !(method === "DELETE" && r.status === 404)) throw new Error(`${method} ${path.replace(/\?.*/, "")}: ${r.status} ${JSON.stringify(j.errors)}`);
  return j;
}
const put = async (key, bytes, type = "application/octet-stream") => { written.push(key); await cf("PUT", `/r2/buckets/${NAME}/objects/${key}`, bytes, { "content-type": type }); };
const get = async (key) => { const r = await fetch(`${api}/r2/buckets/${NAME}/objects/${key}`, { headers: H }); return r.ok ? r.text() : null; };

async function pool(items, n, fn) {
  let i = 0;
  await Promise.all(Array.from({ length: n }, async () => { while (i < items.length) { const k = i++; await fn(items[k]); } }));
}

async function up() {
  await cf("POST", "/r2/buckets", JSON.stringify({ name: NAME }), { "content-type": "application/json" }).catch((e) => { if (!/10004/.test(e.message)) throw e; });
  const files = readdirSync(dir).filter((f) => /^seg_\d{5}\.m4s$/.test(f)).sort();
  for (const t of Object.values(TAKES)) {
    await put(`rec/${t.id}/v0/init.mp4`, readFileSync(`${dir}/init.mp4`));
    await pool(files.slice(0, t.pieces), 8, (f) => put(`rec/${t.id}/v0/${f}`, readFileSync(`${dir}/${f}`)));
    const meta = { schema: 1, id: t.id, state: "live", createdAt: new Date().toISOString(), title: "Untitled — Oct 18", durationSec: t.pieces * 6, media: { hls: `rec/${t.id}/master.m3u8`, mp4: null, width: 1280, height: 720 }, owner: "m1", captions: null };
    await put(`meta/${t.id}.json`, JSON.stringify(meta), "application/json");
  }
  const fd = new FormData();
  fd.append("metadata", new Blob([JSON.stringify({
    main_module: "worker.js", compatibility_date: COMPATIBILITY_DATE,
    bindings: [{ type: "r2_bucket", name: "BUCKET", bucket_name: NAME }, { type: "secret_text", name: "R2_PARENT", text: JSON.stringify(PARENT) }, { type: "ai", name: "AI" }],
  })], { type: "application/json" }));
  fd.append("worker.js", new Blob([readFileSync(new URL("../src/worker.js", import.meta.url))], { type: "application/javascript+module" }), "worker.js");
  await cf("PUT", `/workers/scripts/${NAME}`, fd);
  await cf("PUT", `/workers/scripts/${NAME}/schedules`, JSON.stringify([{ cron: "* * * * *" }]), { "content-type": "application/json" });
  await cf("POST", `/workers/scripts/${NAME}/subdomain`, JSON.stringify({ enabled: true }), { "content-type": "application/json" });
  return `${NAME}.${(await cf("GET", "/workers/subdomain")).result.subdomain}.workers.dev`;
}

async function down() {
  await cf("DELETE", `/workers/scripts/${NAME}?force=true`).catch((e) => console.error("worker:", e.message));
  const left = [];
  let cursor = "";
  do {
    const j = await cf("GET", `/r2/buckets/${NAME}/objects?per_page=1000${cursor ? `&cursor=${cursor}` : ""}`).catch(() => ({ result: [] }));
    left.push(...(j.result || []).map((o) => o.key));
    cursor = j.result_info?.is_truncated ? j.result_info.cursor : "";
  } while (cursor);
  await pool([...new Set([...written, ...left])], 8, (k) => cf("DELETE", `/r2/buckets/${NAME}/objects/${k.split("/").map(encodeURIComponent).join("/")}`).catch(() => {}));
  await cf("DELETE", `/r2/buckets/${NAME}`).catch((e) => console.error("bucket:", e.message));
}

/** One ask, as the setup Mac signs it (the parent proof enroll takes). */
async function ask(host, body) {
  const raw = JSON.stringify(body), ts = Math.floor(Date.now() / 1000), path = "/_shroom/api/ai";
  const msg = `shroom-parent-v1\nPOST\n${host}\n${path}\n${ts}\n${sha(raw)}`;
  const auth = `ShroomParent ${ts}.${createHmac("sha256", Buffer.from(PARENT.secret, "utf8")).update(msg).digest("base64url")}`;
  for (let a = 0; ; a++) {
    const t0 = Date.now();
    const r = await fetch(`https://${host}${path}`, { method: "POST", headers: { "content-type": "application/json", authorization: auth }, body: raw });
    const text = await r.text();
    if ((r.status === 404 || r.status >= 500) && a < 8) { await sleep(3000); continue; } // a fresh workers.dev name takes a moment
    let j;
    try { j = JSON.parse(text); } catch { j = { raw: text.slice(0, 200) }; }
    return { status: r.status, ms: Date.now() - t0, ...j };
  }
}

/** Cloudflare's meter for [from, to]: AI by model, and the Worker's invocations with their CPU time. */
async function meter(from, to) {
  const J = { ...H, "content-type": "application/json" };
  const groups = (await (await fetch(`${api}/tokens/permission_groups`, { headers: H })).json()).result;
  const id = groups.find((g) => g.name === "Account Analytics Read").id;
  const m = await (await fetch(`${api}/tokens`, { method: "POST", headers: J, body: JSON.stringify({ name: "shroom-ai-live-analytics", policies: [{ effect: "allow", resources: { [`com.cloudflare.api.account.${A}`]: "*" }, permission_groups: [{ id }] }], expires_on: new Date(Date.now() + 3600e3).toISOString().replace(/\.\d+Z$/, "Z") }) })).json();
  if (!m.success) return { error: JSON.stringify(m.errors) };
  try {
    await sleep(8000);
    const q = async (query) => (await (await fetch("https://api.cloudflare.com/client/v4/graphql", { method: "POST", headers: { Authorization: `Bearer ${m.result.value}`, "content-type": "application/json" }, body: JSON.stringify({ query }) })).json());
    const ai = await q(`{ viewer { accounts(filter:{accountTag:"${A}"}) { aiInferenceAdaptiveGroups(limit:100, filter:{datetime_geq:"${from}", datetime_leq:"${to}"}) { count sum { totalNeurons totalInputTokens totalOutputTokens } dimensions { modelId } } } } }`);
    const cpu = await q(`{ viewer { accounts(filter:{accountTag:"${A}"}) { workersInvocationsAdaptive(limit:100, filter:{scriptName:"${NAME}", datetime_geq:"${from}", datetime_leq:"${to}"}) { sum { requests errors subrequests } quantiles { cpuTimeP50 cpuTimeP99 cpuTimeP999 wallTimeP50 wallTimeP99 } dimensions { status usageModel } } } } }`);
    return { ai: ai.data?.viewer?.accounts?.[0]?.aiInferenceAdaptiveGroups ?? ai.errors, invocations: cpu.data?.viewer?.accounts?.[0]?.workersInvocationsAdaptive ?? cpu.errors };
  } finally {
    await fetch(`${api}/tokens/${m.result.id}`, { method: "DELETE", headers: H });
  }
}

const report = { at: new Date().toISOString(), take: dir.replace(/\/+$/, "").split("/").pop(), asks: [] };
const stamp = (d) => d.toISOString().replace(/\.\d+Z$/, "Z");
try {
  const host = await up();
  const from = stamp(new Date(Date.now() - 60e3));
  console.log("up");
  // Take A: a minute at a time while it "records", then done until it's named.
  const a = TAKES.A;
  for (const body of [{ rec: a.id, through: 10 }, { rec: a.id, through: 20 }, { rec: a.id, through: 25, done: true }]) {
    const r = await ask(host, body);
    report.asks.push({ take: "A", body: { ...body, rec: "A" }, ...r });
    console.log("A", JSON.stringify({ ...body, rec: "A" }), "→", r.status, r.step ?? r.later ?? r.error ?? r.raw, `${r.ms} ms`);
  }
  for (let i = 0; i < 6; i++) {
    const r = await ask(host, { rec: a.id, done: true });
    report.asks.push({ take: "A", body: { done: true }, ...r });
    console.log("A done →", r.status, r.step ?? r.later ?? r.error, `${r.ms} ms`);
    if (!r.more) break;
  }
  // Take B: one ask, then the schedule each minute until it's named.
  const b = TAKES.B;
  const rb = await ask(host, { rec: b.id, done: true });
  report.asks.push({ take: "B", body: { done: true }, ...rb });
  console.log("B done →", rb.status, rb.step ?? rb.later ?? rb.error, `${rb.ms} ms`, "— the rest is the schedule's");
  const t0 = Date.now();
  let metaB;
  while (Date.now() - t0 < 9 * 60e3) {
    await sleep(20e3);
    metaB = JSON.parse((await get(`meta/${b.id}.json`)) ?? "{}");
    const tb = JSON.parse((await get(`rec/${b.id}/ai/transcript.json`)) ?? "null");
    console.log(`  ${Math.round((Date.now() - t0) / 1000)} s: B heard ${tb?.windows ?? 0} windows${tb?.done ? ", done" : ""}${metaB.named ? `, named “${metaB.title}”` : ""}`);
    if (metaB.named) break;
  }
  report.scheduleSecs = Math.round((Date.now() - t0) / 1000);
  for (const [k, t] of Object.entries(TAKES)) {
    const meta = JSON.parse((await get(`meta/${t.id}.json`)) ?? "{}");
    const tr = JSON.parse((await get(`rec/${t.id}/ai/transcript.json`)) ?? "null");
    const vtt = await (await fetch(`https://${host}/_shroom/cc/${t.id}`)).text();
    const page = await (await fetch(`https://${host}/${t.id}`)).text();
    report[k] = {
      title: meta.title, titled: !!meta.titled, named: meta.named, captions: meta.captions,
      transcript: tr && { windows: tr.windows, duration: tr.duration, done: tr.done, language: tr.language, words: tr.words.length, first: tr.words.slice(0, 12).map((w) => w[0]).join(" ") },
      cues: (vtt.match(/ --> /g) ?? []).length, vttHead: vtt.slice(0, 160),
      page: { chapters: /shroomChapters\(p,\[\[/.test(page), captions: /<track kind="captions"/.test(page), button: /class="cc"/.test(page) },
    };
    console.log(k, JSON.stringify({ title: meta.title, chapters: meta.named?.chapters, words: tr?.words.length, cues: report[k].cues, page: report[k].page }));
  }
  const to = stamp(new Date(Date.now() + 60e3));
  console.log("waiting for the meter…");
  await sleep(120e3);
  report.meter = { from, to, ...(await meter(from, to)) };
  console.log(JSON.stringify(report.meter, null, 1));
} finally {
  await down();
  console.log("down");
  if (outFile) writeFileSync(outFile, JSON.stringify(report, null, 1));
}
