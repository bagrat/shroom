// What each naming step costs in CPU at Cloudflare's edge (N66; DECISIONS B206 c: every step
// must fit the free plan's 10 ms), on the dedicated test account (TESTING-PLAN D3): the Worker
// as the app deploys it, with the AI binding and no schedule, driven one step a minute, so
// each minute of Cloudflare's own meter holds exactly one invocation; one process that cleans
// up after itself:
//
//   SHROOM_E2E_TOKEN=… SHROOM_E2E_ACCOUNT_ID=… node worker/dev/ai-cpu.mjs <take dir> [out.json]
//
// <take dir> holds a take as shroom's encoder writes it (init.mp4, seg_00000.m4s, …). The meter
// is read with a one-hour Account Analytics Read token made here and deleted at once.

import { createHash, createHmac, randomBytes } from "node:crypto";
import { readFileSync, readdirSync, writeFileSync } from "node:fs";

const A = process.env.SHROOM_E2E_ACCOUNT_ID, T = process.env.SHROOM_E2E_TOKEN;
if (!A || !T) throw new Error("needs SHROOM_E2E_ACCOUNT_ID and SHROOM_E2E_TOKEN (the test account's)");
const [dir, outFile] = process.argv.slice(2);
if (!dir) throw new Error("usage: node worker/dev/ai-cpu.mjs <take dir> [out.json]");
const NAME = "shroom-probe-ai-cpu";
const api = `https://api.cloudflare.com/client/v4/accounts/${A}`;
const H = { Authorization: `Bearer ${T}` };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const sha = (b) => createHash("sha256").update(b).digest("hex");
const COMPATIBILITY_DATE = readFileSync(new URL("../../app/src-tauri/src/runtime/cloud.rs", import.meta.url), "utf8").match(/const COMPATIBILITY_DATE: &str = "([0-9-]+)";/)[1];
const PARENT = { account_id: A, bucket: NAME, key_id: randomBytes(16).toString("hex"), secret: randomBytes(32).toString("hex") };
const TAKES = { A: `CpuA${randomBytes(6).toString("hex")}`, B: `CpuB${randomBytes(6).toString("hex")}` };
const PIECES = 25;
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
async function pool(items, n, fn) {
  let i = 0;
  await Promise.all(Array.from({ length: n }, async () => { while (i < items.length) { const k = i++; await fn(items[k]); } }));
}

async function up() {
  await cf("POST", "/r2/buckets", JSON.stringify({ name: NAME }), { "content-type": "application/json" }).catch((e) => { if (!/10004/.test(e.message)) throw e; });
  const files = readdirSync(dir).filter((f) => /^seg_\d{5}\.m4s$/.test(f)).sort().slice(0, PIECES);
  for (const id of Object.values(TAKES)) {
    await put(`rec/${id}/v0/init.mp4`, readFileSync(`${dir}/init.mp4`));
    await pool(files, 8, (f) => put(`rec/${id}/v0/${f}`, readFileSync(`${dir}/${f}`)));
    await put(`meta/${id}.json`, JSON.stringify({ schema: 1, id, state: "live", createdAt: new Date().toISOString(), title: "Untitled — Oct 18", durationSec: PIECES * 6, media: { hls: `rec/${id}/master.m3u8`, mp4: null, width: 1280, height: 720 }, owner: "m1", captions: null }), "application/json");
  }
  const fd = new FormData();
  fd.append("metadata", new Blob([JSON.stringify({
    main_module: "worker.js", compatibility_date: COMPATIBILITY_DATE,
    bindings: [{ type: "r2_bucket", name: "BUCKET", bucket_name: NAME }, { type: "secret_text", name: "R2_PARENT", text: JSON.stringify(PARENT) }, { type: "ai", name: "AI" }],
  })], { type: "application/json" }));
  fd.append("worker.js", new Blob([readFileSync(new URL("../src/worker.js", import.meta.url))], { type: "application/javascript+module" }), "worker.js");
  await cf("PUT", `/workers/scripts/${NAME}`, fd);
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

async function hit(host, step) {
  const path = step.ask ? "/_shroom/api/ai" : step.get;
  const init = { method: step.ask ? "POST" : "GET", headers: {} };
  if (step.ask) {
    const raw = JSON.stringify(step.ask), ts = Math.floor(Date.now() / 1000);
    const msg = `shroom-parent-v1\nPOST\n${host}\n${path}\n${ts}\n${sha(raw)}`;
    init.headers = { "content-type": "application/json", authorization: `ShroomParent ${ts}.${createHmac("sha256", Buffer.from(PARENT.secret, "utf8")).update(msg).digest("base64url")}` };
    init.body = raw;
  }
  const t0 = Date.now();
  const r = await fetch(`https://${host}${path}`, init);
  const text = await r.text();
  return { status: r.status, ms: Date.now() - t0, answer: step.ask ? text.slice(0, 120) : `${text.length} bytes` };
}

async function meter(from, to) {
  const J = { ...H, "content-type": "application/json" };
  const groups = (await (await fetch(`${api}/tokens/permission_groups`, { headers: H })).json()).result;
  const id = groups.find((g) => g.name === "Account Analytics Read").id;
  const m = await (await fetch(`${api}/tokens`, { method: "POST", headers: J, body: JSON.stringify({ name: "shroom-ai-cpu-analytics", policies: [{ effect: "allow", resources: { [`com.cloudflare.api.account.${A}`]: "*" }, permission_groups: [{ id }] }], expires_on: new Date(Date.now() + 3600e3).toISOString().replace(/\.\d+Z$/, "Z") }) })).json();
  if (!m.success) return { error: JSON.stringify(m.errors) };
  try {
    await sleep(8000);
    const r = await (await fetch("https://api.cloudflare.com/client/v4/graphql", { method: "POST", headers: { Authorization: `Bearer ${m.result.value}`, "content-type": "application/json" }, body: JSON.stringify({ query: `{ viewer { accounts(filter:{accountTag:"${A}"}) { workersInvocationsAdaptive(limit:100, filter:{scriptName:"${NAME}", datetime_geq:"${from}", datetime_leq:"${to}"}, orderBy:[datetimeMinute_ASC]) { sum { requests errors subrequests cpuTimeUs } dimensions { datetimeMinute status } } } } }` }) })).json();
    return r.data?.viewer?.accounts?.[0]?.workersInvocationsAdaptive ?? r.errors;
  } finally {
    await fetch(`${api}/tokens/${m.result.id}`, { method: "DELETE", headers: H });
  }
}

const STEPS = [
  { what: "ping", get: "/_shroom/ping" },
  { what: "page, not yet named", get: `/${TAKES.A}` },
  { what: "hear window 0", ask: { rec: TAKES.A, through: 10 } },
  { what: "hear window 1", ask: { rec: TAKES.A, through: 20 } },
  { what: "hear window 2 (the last, partial)", ask: { rec: TAKES.A, through: 25, done: true } },
  { what: "name", ask: { rec: TAKES.A, done: true } },
  { what: "page, named (chapters, captions)", get: `/${TAKES.A}` },
  { what: "captions", get: `/_shroom/cc/${TAKES.A}` },
  { what: "hear window 0, another take", ask: { rec: TAKES.B, through: 10 } },
  { what: "hear window 1, another take", ask: { rec: TAKES.B, through: 20 } },
];

const report = { at: new Date().toISOString(), take: dir.replace(/\/+$/, "").split("/").pop(), steps: [] };
try {
  const host = await up();
  console.log("up");
  // The fresh name answers after a moment.
  for (let i = 0; i < 30 && (await fetch(`https://${host}/_shroom/ping`).then((r) => r.status, () => 0)) !== 200; i++) await sleep(3000);
  for (const step of STEPS) {
    // Into the next minute, a few seconds in: that minute holds this step alone.
    await sleep(60e3 - (Date.now() % 60e3) + 5e3);
    const minute = new Date(Date.now() - (Date.now() % 60e3)).toISOString().replace(/\.\d+Z$/, "Z");
    const r = await hit(host, step);
    report.steps.push({ minute, what: step.what, ...r });
    console.log(minute, step.what, r.status, `${r.ms} ms`, r.answer);
  }
  const from = report.steps[0].minute, to = new Date(Date.now() + 60e3).toISOString().replace(/\.\d+Z$/, "Z");
  console.log("waiting for the meter…");
  await sleep(150e3);
  const rows = await meter(from, to);
  report.meter = rows;
  for (const s of report.steps) {
    const row = Array.isArray(rows) ? rows.find((x) => x.dimensions.datetimeMinute === s.minute) : null;
    s.cpuMs = row ? row.sum.cpuTimeUs / 1000 : null;
    s.invocations = row?.sum.requests ?? null;
    s.subrequests = row?.sum.subrequests ?? null;
    console.log(`${s.what.padEnd(36)} cpu ${s.cpuMs ?? "?"} ms  (${s.invocations ?? "?"} invocation, ${s.subrequests ?? "?"} subrequests)`);
  }
} finally {
  await down();
  console.log("down");
  if (outFile) writeFileSync(outFile, JSON.stringify(report, null, 1));
}
