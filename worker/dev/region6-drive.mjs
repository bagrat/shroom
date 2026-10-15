// Region 6's drives (join, members, switch, token, stale) through the real app against the local
// Worker (worker/dev/serve.mjs, three spaces) and the L1 Cloudflare fake, under a throwaway
// home. Run from the repo: node worker/dev/region6-drive.mjs <join|members|switch|token|stale>
// `stale` (DECISIONS B90): the fake hands an R2_PARENT placed on a Worker to the local Worker's
// binding (`--relay-parent`), so the app's own re-place of a stale binding is driven end to end.
// Needs the debug bundle (cd app && npm run build:dev — the harness is built in only on purpose) and the fake
// (cargo build -p shroom-cloudfake). Never touches the installed app or ~/.shroom. The
// throwaway home goes when the run ends — a take there carries room audio from the mic —
// keeping only the report and the run log beside it.
import { spawn } from "node:child_process";
import { cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createInterface } from "node:readline";
import { refuseOffTestMachine } from "../../tests/test-machine.mjs";

refuseOffTestMachine("region6-drive.mjs");

const repo = new URL("../..", import.meta.url).pathname.replace(/\/$/, "");
const app = join(repo, "target/debug/bundle/macos/shroom-dev.app");
const which = process.argv[2];
if (!["join", "members", "switch", "token", "stale"].includes(which)) {
  console.error("usage: node worker/dev/region6-drive.mjs <join|members|switch|token|stale>");
  process.exit(64);
}
const work = mkdtempSync(join(tmpdir(), `shroom-r6-${which}-`));
const home = join(work, "home");

/** Starts a server that prints one JSON line, then serves until its stdin closes. */
async function start(cmd, args) {
  const p = spawn(cmd, args, { stdio: ["pipe", "pipe", "inherit"] });
  const first = await new Promise((ok, fail) => {
    createInterface({ input: p.stdout }).once("line", (l) => ok(JSON.parse(l)));
    p.once("exit", () => fail(new Error(`${cmd} exited`)));
  });
  return { p, first };
}

// The status seam (ARCHITECTURE §11.2, DECISIONS B80): every status.json the app wrote is
// in its run log as written (`seam: {…}` — no secret in it), so the join states a guiding
// agent reads are checked in order, not sampled.
const SEAM = { join: ["member-welcome", "joining", "done", "all-set"], switch: ["join-switch-confirm", "joining", "all-set"] };
function seamChecks(log) {
  const seen = log.split("\n").filter((l) => l.includes(" seam: ")).map((l) => JSON.parse(l.slice(l.indexOf(" seam: ") + 7)));
  const checks = [];
  const check = (what, ok, detail) => checks.push({ what, ok: Boolean(ok), detail: String(detail ?? "") });
  const steps = seen.map((s) => `${s.setup}/${s.join?.invite}/${s.join?.role}`).filter((x, i, a) => a[i - 1] !== x);
  const setups = seen.map((s) => s.setup);
  let from = 0;
  const inOrder = (SEAM[which] ?? []).every((want) => {
    const i = setups.indexOf(want, from);
    if (i >= 0) from = i;
    return i >= 0;
  });
  check(`status.json walks ${SEAM[which]?.join(" → ")} in order`, inOrder, steps.join(" → "));
  check("every write is the v2 shape, with the installer's fields", seen.length > 0 && seen.every((s) => s.v === 2 && Number.isInteger(s.pid) && typeof s.take_open === "boolean" && s.permission && s.join), `${seen.length} writes`);
  check("the permission fields use the seam's words", seen.every((s) => ["granted", "not-granted", "unknown"].includes(s.permission.screen) && ["granted", "denied", "restricted", "not-asked", "unknown"].includes(s.permission.mic) && ["done", "ahead"].includes(s.permission.chapter)), JSON.stringify(seen.at(-1)?.permission));
  const raw = log.split("\n").filter((l) => l.includes(" seam: ")).join("\n");
  check("no host, name or link in the seam", !/workers\.dev|Bagrat|Ani|Mher|join\//.test(raw), raw.match(/workers\.dev|Bagrat|Ani|Mher|join\//)?.[0]);
  if (which === "join") {
    const welcome = seen.find((s) => s.setup === "member-welcome");
    check("member-welcome: the invite's id and kind, no role yet", welcome?.join.invite === "member-welcome" && welcome?.join.invite_kind === "member" && welcome?.join.role === "none", JSON.stringify(welcome?.join));
    check("joining is the invite's state too", seen.some((s) => s.setup === "joining" && s.join.invite === "joining"), "joining");
    const done = seen.find((s) => s.setup === "done");
    check("joined: a member, the invite behind it, row six ahead", done?.join.role === "member" && done?.join.invite === "none" && done?.permission.chapter === "ahead", JSON.stringify(done));
    check("all set: the chapter done", seen.some((s) => s.setup === "all-set" && s.join.role === "member" && s.permission.chapter === "done"), "all-set");
    check("the first video's take: recording, then none", seen.some((s) => s.take === "recording" && s.take_open) && seen.at(-1)?.take === "none", seen.map((s) => s.take).filter((x, i, a) => a[i - 1] !== x).join(" → "));
  }
  if (which === "switch") {
    check("before the invite: a member of its space", seen[0]?.join.role === "member" && seen[0]?.join.invite === "none", JSON.stringify(seen[0]?.join));
    check("the switch confirm names the invite's state and kind", seen.some((s) => s.setup === "join-switch-confirm" && s.join.invite === "join-switch-confirm" && s.join.invite_kind === "member" && s.join.role === "member"), "join-switch-confirm");
    const after = seen.slice(setups.indexOf("joining"));
    check("joined: all set in the new space, the invite behind it", setups.includes("joining") && after.some((s) => s.setup === "all-set" && s.join.invite === "none" && s.join.role === "member"), "all-set");
    check("after Leave: no space", seen.at(-1)?.join.role === "none", JSON.stringify(seen.at(-1)?.join));
  }
  return checks;
}

let fake;
let worker;
let r = { ok: false, error: "the drive didn't report" };
try {
  worker = await start(process.execPath, [join(repo, "worker/dev/serve.mjs"), "--space", "v.alpha.workers.dev=Bagrat", "--space", "v.beta.workers.dev=Ani", "--space", "v.myshroom.workers.dev=Bagrat"]);
  // The stale drive: a parent the app places on the fake's Worker reaches the local Worker.
  fake = await start(join(repo, "target/debug/cloudfake"), ["--state", "found", ...(which === "stale" ? ["--relay-parent", worker.first.url] : [])]);
  const out = join(work, "report.json");
  const args = ["--home", home, "--cloud-fake", fake.first.api, fake.first.r2, "--space-base", worker.first.url, "--drive", which, out];
  // The join drive is the installer's launch: it opens shroom with `--invite` (no secret).
  if (which === "join") args.push("--invite");
  const run = spawn("open", ["-W", "-n", app, "--args", ...args], { stdio: "inherit" });
  await new Promise((ok) => run.once("exit", ok));
  r = JSON.parse(readFileSync(out, "utf8"));
} finally {
  fake?.p.stdin.end();
  worker?.p.stdin.end();
  // Keep the run log for a failed run's reading; everything else in the home goes.
  if (existsSync(join(home, "logs/run.log"))) {
    mkdirSync(join(work, "logs"), { recursive: true });
    cpSync(join(home, "logs/run.log"), join(work, "logs/run.log"));
  }
  rmSync(home, { recursive: true, force: true });
}
const seam = SEAM[which] && existsSync(join(work, "logs/run.log")) ? seamChecks(readFileSync(join(work, "logs/run.log"), "utf8")) : [];
if (SEAM[which] && seam.length === 0) seam.push({ what: "the run log holds the seam's writes", ok: false, detail: "no run.log" });
const seamOk = seam.every((c) => c.ok);
console.log(JSON.stringify({ work, report: r, seam }, null, 1));
process.exit(r.ok && seamOk ? 0 : 1);
