// L3 (TESTING-PLAN §5: "kill -9 at random points in capture, encode, upload, wiring →
// relaunch must derive the right state, nothing lost"; DECISIONS B64, B69; tests-sys #4):
// the real app `kill -9`ed at a random point and relaunched on the same home — against
// the L1 fake, under a throwaway home. The pattern covers the display while anything
// records, and the only sound is the harness's beep. Never the installed app, never
// ~/.shroom(-dev), never Cloudflare.
//
//   cargo build -p shroom-cloudfake --bin cloudfake
//   (cd app && npm run build:dev)
//   node tests/e2e/kill-relaunch.mjs [upload|capture|wire|all] [rounds=1]
//
// Each round draws its kill point from a seeded generator — SHROOM_SEED=<n> replays a run
// (the seed and every round's draw are printed).
//
// upload  — the fake's bucket is slowed (every media PUT 1.5 s late); a take is recorded
//           and stopped; the app is killed once a random 1–4 of its files are up (+ a
//           random 0–1.2 s) — before or after the record's claim, never live; relaunched through a 503, a
//           drop, a reset and a lost reply, the same take finishes under the same link.
// capture — a take is recording (capture and the encoder running), or paused on the
//           runner's word (a third of the rounds); the app is killed a random 1.5–14 s in
//           (paused: 0.3–6 s after the pause). The relaunch (a login launch) is silent with
//           the amber dot; the take is rebuilt whole, as long as what was recorded, never
//           published by itself; Get link publishes and uploads it.
// wire    — a fresh account walked through onboarding; one of wiring's calls, drawn at
//           random (the bucket, the player, the owner cookie's secret, the roster, the
//           page, its route, the self-check, the manifest — a seeded shuffle, so eight
//           rounds meet every one), is held 15 s by the fault seam
//           and the app is killed the moment it arrives. The person opens shroom again: the
//           plan re-derives where it stands (no token pasted, no name claimed again) and
//           wiring converges on the one clean space — one page `v`, its bindings, its
//           route answering, the space's objects, the cookie secret never replaced.
//
// After upload and capture, law 2 at the bucket: every local file of the take is there
// exactly once and byte for byte, one record, live and drained, no stray key.
import { spawn } from "node:child_process";
import { createHash } from "node:crypto";
import { existsSync, mkdtempSync, readFileSync, readdirSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createInterface } from "node:readline";
import { refuseOffTestMachine } from "../test-machine.mjs";

refuseOffTestMachine("kill-relaunch.mjs");

const repo = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const app = process.env.SHROOM_APP ?? join(repo, "target/debug/bundle/macos/shroom-dev.app");
const fakeBin = join(repo, "target/debug/cloudfake");
for (const p of [app, fakeBin]) if (!existsSync(p)) { console.error(`missing ${p} — build it first (see the header)`); process.exit(2); }
const MODES = ["upload", "capture", "wire"];
const asked = process.argv[2] ?? "upload";
const rounds = Number(process.argv[3] ?? 1);
if (![...MODES, "all"].includes(asked) || !Number.isInteger(rounds) || rounds < 1 || rounds > 20) {
  console.error("usage: node tests/e2e/kill-relaunch.mjs [upload|capture|wire|all] [rounds 1–20]");
  process.exit(64);
}
const seed = Number(process.env.SHROOM_SEED ?? Math.floor(Math.random() * 2 ** 31));
// mulberry32: small, seeded, good enough to place a kill.
let state = seed >>> 0;
const rand = () => { state = (state + 0x6d2b79f5) >>> 0; let t = state; t = Math.imul(t ^ (t >>> 15), t | 1); t ^= t + Math.imul(t ^ (t >>> 7), t | 61); return ((t ^ (t >>> 14)) >>> 0) / 4294967296; };
const between = (lo, hi) => lo + rand() * (hi - lo);

const BUCKET = "shroom-space";
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const sha = (b) => createHash("sha256").update(b).digest("hex");
const readJson = (f) => { try { return JSON.parse(readFileSync(f, "utf8")); } catch { return null; } };

async function startFake(states) {
  const fake = spawn(fakeBin, states.flatMap((s) => ["--state", s]), { stdio: ["pipe", "pipe", "inherit"] });
  const ep = await new Promise((ok, fail) => {
    createInterface({ input: fake.stdout }).once("line", (l) => ok(JSON.parse(l)));
    fake.once("exit", () => fail(new Error("fake exited")));
  });
  const root = ep.api.replace(/\/client\/v4$/, "");
  const knob = async (cmd) => {
    const res = await fetch(`${root}/_fake/${cmd.split(" ").map(encodeURIComponent).join("%20")}`, { method: "POST" });
    if (!res.ok) throw new Error(`the fake refused ${cmd}: ${await res.text()}`);
  };
  const faulted = async () => (await (await fetch(`${root}/_fake/faulted`)).json()).result;
  const keys = async (prefix) => {
    const xml = await (await fetch(`${ep.r2}/${BUCKET}?list-type=2&prefix=${encodeURIComponent(prefix)}`)).text();
    return [...xml.matchAll(/<Key>([^<]*)<\/Key>/g)].map((m) => m[1].replace(/&lt;/g, "<").replace(/&amp;/g, "&"));
  };
  const object = async (key) => {
    const res = await fetch(`${ep.r2}/${BUCKET}/${key.split("/").map(encodeURIComponent).join("/")}`);
    return res.ok ? Buffer.from(await res.arrayBuffer()) : null;
  };
  const api = async (p) => (await (await fetch(`${ep.api}${p}`, { headers: { authorization: "Bearer runner" } })).json()).result;
  return { fake, ep, root, knob, faulted, keys, object, api, stop: () => fake.stdin.end() };
}

function launcher(work, home, ep) {
  return (out, args, extra = []) => {
    const p = spawn("open", ["-W", "-n", "--stderr", `${out.replace(/\.json$/, "")}.stderr.log`, app, "--args", "--home", home, "--cloud-fake", ep.api, ep.r2, "--drive", ...args.slice(0, 1), out, ...args.slice(1), ...extra], { stdio: "inherit" });
    const s = { p, exited: false };
    p.once("exit", () => { s.exited = true; });
    return s;
  };
}

/// The first launch's early report (written before it waits to be killed).
async function earlyReport(out, run, secs = 120) {
  for (let i = 0; i < secs * 10; i++) {
    const r = readJson(out);
    if (r) return r;
    if (run.exited) throw new Error("the first launch ended before its report");
    await sleep(100);
  }
  throw new Error("the first launch never reported");
}

async function kill(pid, run) {
  process.kill(pid, "SIGKILL");
  await new Promise((ok) => (run.exited ? ok() : run.p.once("exit", ok)));
  await sleep(500);
}

/// Law 2 at the bucket, for a take on this Mac and its record.
async function law2(f, check, takeDir, rec, seeded) {
  const local = {};
  const playlist = readFileSync(join(takeDir, "stream.m3u8"), "utf8");
  const named = [...new Set([...playlist.matchAll(/URI="([^"]+)"/g)].map((m) => m[1]).concat(playlist.split("\n").filter((l) => l && !l.startsWith("#"))))];
  for (const x of [...named, "stream.m3u8"]) local[`rec/${rec}/v0/${x}`] = readFileSync(join(takeDir, x));
  if (existsSync(join(takeDir, "preview.mp4"))) local[`rec/${rec}/video.mp4`] = readFileSync(join(takeDir, "preview.mp4"));
  const remote = await f.keys(`rec/${rec}/`);
  const missing = Object.keys(local).filter((k) => !remote.includes(k));
  check("every local file of the take is in the bucket", missing.length === 0, missing.join(" "));
  const torn = [];
  for (const [k, bytes] of Object.entries(local)) {
    const got = await f.object(k);
    if (!got || sha(got) !== sha(bytes)) torn.push(`${k} (${got?.length ?? 0} up, ${bytes.length} here)`);
  }
  check("each is the local file, byte for byte — nothing torn", torn.length === 0, torn.join(" "));
  const allowed = new Set([...Object.keys(local), `rec/${rec}/master.m3u8`, `rec/${rec}/poster.jpg`, `rec/${rec}/teaser.mp4`]);
  const stray = remote.filter((k) => !allowed.has(k));
  check("no stray key under the take", stray.length === 0, stray.join(" "));
  const metas = (await f.keys("meta/")).filter((k) => !seeded.includes(k));
  check("one record — no second take, no duplicate", metas.length === 1 && metas[0] === `meta/${rec}.json`, metas.join(" "));
  const meta = JSON.parse((await f.object(`meta/${rec}.json`))?.toString() ?? "null");
  check("the record is live", meta?.state === "live", meta?.state);
  check("…and drained", meta?.progress && meta.progress.done === meta.progress.total, JSON.stringify(meta?.progress));
}

// The fake's `found` state's own video.
const SEEDED_META = ["meta/AbCdEfGhIjKlMnOp.json"];

async function upload(round, work, home, f, launch, check) {
  const k = 1 + Math.floor(rand() * 4);
  const jitter = Math.round(between(0, 1200));
  round.draw = { filesUpBeforeKill: k, jitterMs: jitter };
  await f.knob(`fault delay:1500 PUT /s3/${BUCKET}/rec/ always`);
  const out1 = join(work, "kill.json");
  const first = launch(out1, ["take", "upload-kill"]);
  const r1 = await earlyReport(out1, first);
  if (!r1.ok) throw new Error(`the first half failed: ${JSON.stringify(r1.checks)} ${r1.error ?? ""}`);
  const { rec, dir } = r1.facts;
  const pid = Number(r1.facts.pid);
  check("the pid the drive reported is the app's status seam's", readJson(join(home, "status.json"))?.pid === pid, pid);
  let up = [];
  for (let i = 0; i < 600; i++) {
    up = (await f.keys(`rec/${rec}/`)).filter((x) => !x.endsWith(".m3u8"));
    if (up.length >= k) break;
    await sleep(100);
  }
  await sleep(jitter);
  const metaBefore = JSON.parse((await f.object(`meta/${rec}.json`))?.toString() ?? "null");
  // Stop made the link (the first half's report says so); the claim may or may not be up
  // yet — the segments sent while it recorded go first — but the record isn't live.
  round.killedWith = metaBefore?.state ?? "no claim yet";
  check("killed after Stop, mid-upload: the record not live yet", metaBefore === null || metaBefore.state === "uploading", round.killedWith);
  check(`killed with part of the take up (≥ ${k})`, up.length >= k, `${up.length} files up`);
  await kill(pid, first);
  round.upAtKill = (await f.keys(`rec/${rec}/`)).length;

  await f.knob("faults-clear");
  const before = await f.faulted();
  for (const x of ["503 PUT /s3/" + BUCKET + "/rec/ 1", "drop PUT /s3/" + BUCKET + "/rec/ 1", "reset PUT /s3/" + BUCKET + "/rec/ 1", "lost-reply PUT /s3/" + BUCKET + "/rec/ 1", "delay:400 PUT /s3/ 5"]) await f.knob(`fault ${x}`);
  const out2 = join(work, "resume.json");
  const second = launch(out2, ["take", "upload-resume"]);
  await new Promise((ok) => second.p.once("exit", ok));
  const r2 = readJson(out2) ?? { ok: false, error: "no report" };
  check("the relaunch finished the upload", r2.ok, r2.error ?? JSON.stringify(r2.checks?.filter((c) => !c[1])));
  check("the relaunch kept the same link", r2.facts?.rec === rec, r2.facts?.rec);
  const met = (await f.faulted()).kinds.slice(before.kinds.length);
  check("the relaunch went through a 503, a drop, a reset and a lost reply", ["503", "drop", "reset", "lost-reply"].every((x) => met.includes(x)), met.join(" "));
  await law2(f, check, join(home, "takes", dir), rec, SEEDED_META);
  check("the take on this Mac is still whole", readdirSync(join(home, "takes", dir)).includes("uploaded"), "");
  round.rec = rec;
}

async function capture(round, work, home, f, launch, check) {
  const paused = rand() < 1 / 3;
  const t1 = Math.round(between(1500, 14000));
  const t2 = paused ? Math.round(between(300, 6000)) : 0;
  round.draw = { paused, recordForMs: t1, killAfterPauseMs: paused ? t2 : undefined };
  const out1 = join(work, "kill.json");
  const first = launch(out1, ["take", "capture-kill"]);
  const r1 = await earlyReport(out1, first);
  if (!r1.ok) throw new Error(`the first half failed: ${JSON.stringify(r1.checks)} ${r1.error ?? ""}`);
  const pid = Number(r1.facts.pid);
  const since = Number(r1.facts.recording_since_ms);
  const status = readJson(join(home, "status.json"));
  check("killed with a take open (the status seam)", status?.pid === pid && status?.take_open === true, JSON.stringify(status));
  await sleep(Math.max(0, since + t1 - Date.now()));
  let recordedMs;
  if (paused) {
    writeFileSync(join(work, "pause-now"), "");
    let at = null;
    for (let i = 0; i < 150 && !at; i++) { try { at = Number(readFileSync(join(work, "paused-at"), "utf8")); } catch { await sleep(100); } }
    if (!at) throw new Error("the take never paused");
    recordedMs = at - since;
    await sleep(t2);
  } else {
    recordedMs = Date.now() - since;
  }
  await kill(pid, first);
  round.recordedMs = recordedMs;
  check("nothing in the bucket before anyone chose to stop", (await f.keys("meta/")).filter((k) => !SEEDED_META.includes(k)).length === 0, "");

  const out2 = join(work, "resume.json");
  const second = launch(out2, ["take", "capture-resume"]);
  await new Promise((ok) => second.p.once("exit", ok));
  const r2 = readJson(out2) ?? { ok: false, error: "no report" };
  check("the relaunch: silent, amber, the banner, Get link, uploaded", r2.ok, r2.error ?? JSON.stringify(r2.checks?.filter((c) => !c[1])));
  const length = r2.facts?.take?.length_ms ?? 0;
  round.savedMs = length;
  // Paused: everything up to the pause was finalized there. Recording: the last fragment
  // (1 s) and what was still in flight may go — never more.
  const lo = recordedMs - (paused ? 1000 : 2500);
  const hi = recordedMs + 1000;
  check(`the saved take is as long as what was recorded (${lo}–${hi} ms)`, length >= lo && length <= hi, `${length} ms saved of ${recordedMs} ms recorded`);
  const rec = r2.facts?.rec;
  if (rec && r2.facts?.dir) await law2(f, check, join(home, "takes", r2.facts.dir), rec, SEEDED_META);
  round.rec = rec;
}

const WIRE_CALLS = (acc) => [
  ["the bucket", "POST", `/client/v4/accounts/${acc}/r2/buckets`],
  ["the player", "PUT", `/s3/${BUCKET}/.shroom/player/`],
  ["the owner cookie's secret", "PUT", `/s3/${BUCKET}/.shroom/cookie.key`],
  ["the roster", "PUT", `/s3/${BUCKET}/.shroom/registry/roster.json`],
  ["the page", "PUT", `/client/v4/accounts/${acc}/workers/scripts/v`],
  ["its route", "POST", `/client/v4/accounts/${acc}/workers/scripts/v/subdomain`],
  ["the self-check", "PUT", `/s3/${BUCKET}/.shroom/self-check`],
  ["the manifest", "PUT", `/s3/${BUCKET}/.shroom/manifest.json`],
];

let wireOrder = null;
async function wire(round, work, home, f, launch, check) {
  const acc = (await f.api("/accounts"))[0].id;
  // A seeded shuffle of wiring's calls: rounds 1–8 meet every one once, in a random order.
  if (!wireOrder) { wireOrder = WIRE_CALLS("").map((_, i) => i); for (let i = wireOrder.length - 1; i > 0; i--) { const j = Math.floor(rand() * (i + 1)); [wireOrder[i], wireOrder[j]] = [wireOrder[j], wireOrder[i]]; } }
  const [what, method, prefix] = WIRE_CALLS(acc)[wireOrder[(round.round - 1) % wireOrder.length]];
  const HOLD = 15_000;
  round.draw = { killedAt: what };
  await f.knob(`fault delay:${HOLD} ${method} ${prefix} 1`);
  const out1 = join(work, "first.json");
  const first = launch(out1, ["onboarding"], ["--relaunched=install"]);
  const before = (await f.faulted()).faulted;
  for (let i = 0; ; i++) {
    if ((await f.faulted()).faulted > before) break;
    if (first.exited) throw new Error("the first launch ended before wiring reached the held call");
    if (i > 2400) throw new Error(`wiring never reached ${what}`);
    await sleep(50);
  }
  const status = readJson(join(home, "status.json"));
  check("killed mid-wire (the status seam says wiring)", status?.setup === "wiring", JSON.stringify(status));
  await kill(Number(status.pid), first);
  // The held request lands (or not) as it would have; then the person opens shroom again.
  await sleep(HOLD + 1000);
  const sub = (await f.api(`/accounts/${acc}/workers/subdomain`))?.subdomain;
  const cookieBefore = await f.object(".shroom/cookie.key");
  const manifestBefore = (await f.object(".shroom/manifest.json")) !== null;
  round.atRelaunch = { subdomain: !!sub, cookie: !!cookieBefore, manifest: manifestBefore };
  check("a name was claimed before the kill", !!sub, sub);

  const out2 = join(work, "resume.json");
  const second = launch(out2, ["onboarding"], ["--resume"]);
  await new Promise((ok) => second.p.once("exit", ok));
  const r = readJson(out2) ?? { ok: false, error: "no report", states: [] };
  round.states = r.states?.join(" → ");
  round.resumedAt = r.resumed_at;
  check("the relaunch walked on to all-set", r.ok, r.error ?? "");
  check("no name asked for again", !r.states?.some((s) => s === "name" || s === "name-taken"), round.states);
  check("the relaunch re-derived: wiring again, or the space it finished", r.states?.includes(manifestBefore ? "space-found" : "wiring"), round.states);
  check("the token wasn't asked for again (no token step)", !r.states?.includes("token"), round.states);
  check("law 13: the token only in token.json", (r.token_leaks ?? ["?"]).length === 0 && r.token_json_mode === "600", JSON.stringify(r.token_leaks));
  check("status.json: all set", r.status?.setup === "all-set", r.status?.setup);
  // The one clean space.
  const host = `v.${sub}.workers.dev`;
  check("this Mac's space is the page `v` on the claimed name", r.host === host, r.host);
  const scripts = (await f.api(`/accounts/${acc}/workers/scripts`)).map((s) => s.id).sort();
  check("one page, `v` (never the fallback label: our half-wired page isn't someone else's)", JSON.stringify(scripts) === JSON.stringify(["v"]), scripts.join(" "));
  const bindings = (await f.api(`/accounts/${acc}/workers/scripts/v/settings`))?.bindings?.map((b) => `${b.type}:${b.name}`).sort() ?? [];
  check("its bindings: the bucket and the mint parent", bindings.includes("r2_bucket:BUCKET") && bindings.includes("secret_text:R2_PARENT"), bindings.join(" "));
  const ping = await fetch(`${f.root}/page/${host}/_shroom/ping`).then((x) => (x.ok ? x.json() : null)).catch(() => null);
  check("its route answers as the space", ping?.shroom === true, JSON.stringify(ping));
  const objs = (await f.keys(".shroom/")).sort();
  const want = [".shroom/cookie.key", ".shroom/manifest.json", ".shroom/player/hls.min.js", ".shroom/registry/roster.json"];
  check("the space's objects, and no self-check left behind", want.every((k) => objs.includes(k)) && !objs.includes(".shroom/self-check"), objs.join(" "));
  const cookieAfter = await f.object(".shroom/cookie.key");
  check("the owner cookie's secret written once, never replaced", !cookieBefore || (cookieAfter && sha(cookieAfter) === sha(cookieBefore)), cookieBefore ? "kept" : "written by the relaunch");
}

const RUN = { upload: [["found"], upload], capture: [["found"], capture], wire: [["fresh"], wire] };
const modes = asked === "all" ? MODES : [asked];
const results = [];
for (const mode of modes) {
  for (let n = 1; n <= rounds; n++) {
    const work = mkdtempSync(join(tmpdir(), `shroom-l3-kill-${mode}-`));
    const home = join(work, "home");
    const checks = [];
    const check = (what, ok, seen = "") => { checks.push({ what, ok: !!ok, seen: String(seen ?? "") }); };
    const round = { mode, round: n };
    const [states, fn] = RUN[mode];
    const f = await startFake(states);
    try {
      await fn(round, work, home, f, launcher(work, home, f.ep), check);
      round.ok = checks.length > 0 && checks.every((c) => c.ok);
    } catch (e) {
      round.ok = false;
      round.error = String(e?.stack ?? e).slice(0, 600);
      // A launch still waiting to be killed (the runner failed before it): ended by its pid.
      const st = readJson(join(home, "status.json"));
      if (st?.pid) try { process.kill(st.pid, "SIGKILL"); } catch { /* gone */ }
    } finally {
      f.stop();
    }
    round.failed = checks.filter((c) => !c.ok);
    round.checks = checks.length;
    if (round.ok) rmSync(work, { recursive: true, force: true }); else round.work = work;
    console.log(JSON.stringify(round));
    results.push(round);
  }
}
const ok = results.every((r) => r.ok);
console.log(JSON.stringify({ seed, ok, rounds: results.map((r) => ({ mode: r.mode, ok: r.ok, draw: r.draw })) }, null, 1));
process.exit(ok ? 0 : 1);
