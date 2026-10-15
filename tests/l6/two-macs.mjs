// L6 (TESTING-PLAN §5: "owner + member as two instances with isolated state dirs on this
// Mac"; DECISIONS B65; tests-sys #1, #2): two instances of the real app at once, each on
// its own throwaway home, against one local Worker (worker/dev/serve.mjs) and the L1 fake.
//
//   cargo build -p shroom-cloudfake --bin cloudfake
//   (cd app && npm run build:dev)
//   node tests/l6/two-macs.mjs
//
// The owner (a token Mac) invites Mher → the link is handed to the member's home the way
// the installer hands it over (a file, mode 600, never an argv) → the member joins and
// records (the test pattern covers the display; the only sound is the harness's beep) →
// a second take waits while the space is too busy to mint → the owner removes Mher → the
// member sees member-revoked and Save to Movies lands in its throwaway Movies → the owner
// invites again → the member re-joins from the page's Open shroom (the deep link) → the
// waiting take drains. Each instance writes its own report; this script steps them through
// empty signal files in the run's work folder. Never the installed app, never ~/.shroom(-dev),
// never a browser.
import { execFileSync, spawn } from "node:child_process";
import { chmodSync, cpSync, existsSync, mkdirSync, mkdtempSync, readFileSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createInterface } from "node:readline";
import { refuseOffTestMachine } from "../test-machine.mjs";

refuseOffTestMachine("two-macs.mjs");

const repo = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const app = process.env.SHROOM_APP ?? join(repo, "target/debug/bundle/macos/shroom-dev.app");
const fakeBin = join(repo, "target/debug/cloudfake");
for (const p of [app, fakeBin]) if (!existsSync(p)) { console.error(`missing ${p} — build it first (see the header)`); process.exit(2); }

const work = mkdtempSync(join(tmpdir(), "shroom-l6-"));
const ownerHome = join(work, "owner-home");
const memberHome = join(work, "member-home");
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

async function start(cmd, args) {
  const p = spawn(cmd, args, { stdio: ["pipe", "pipe", "inherit"] });
  const first = await new Promise((ok, fail) => {
    createInterface({ input: p.stdout }).once("line", (l) => ok(JSON.parse(l)));
    p.once("exit", () => fail(new Error(`${cmd} exited`)));
  });
  return { p, first };
}
const has = (name) => existsSync(join(work, name));
const signal = (name) => writeFileSync(join(work, name), "");
async function until(name, secs, alive) {
  for (let i = 0; i < secs * 10; i++) {
    if (has(name)) return;
    for (const [who, a] of Object.entries(alive)) if (a.exited) throw new Error(`the ${who} instance ended before ${name}`);
    await sleep(100);
  }
  throw new Error(`timed out waiting for ${name}`);
}
function launch(home, out, which, extra = []) {
  const p = spawn("open", ["-W", "-n", "--stderr", join(work, `${which}.stderr.log`), app, "--args", "--home", home, "--cloud-fake", fake.first.api, fake.first.r2, "--space-base", worker.first.url, "--drive", which, out, ...extra], { stdio: "inherit" });
  const s = { p, exited: false };
  p.once("exit", () => { s.exited = true; });
  return s;
}
const report = (f) => (existsSync(f) ? JSON.parse(readFileSync(f, "utf8")) : { ok: false, error: "no report" });

let fake;
let worker;
let result = { ok: false };
const ownerOut = join(work, "owner.json");
const memberOut = join(work, "member.json");
try {
  fake = await start(fakeBin, ["--state", "found"]);
  worker = await start(process.execPath, [join(repo, "worker/dev/serve.mjs"), "--space", "v.alpha.workers.dev=Bagrat"]);
  const owner = launch(ownerHome, ownerOut, "l6-owner");
  await until("owner-invited", 120, { owner });
  // The installer's hand-off: the file, moved (never copied through a pipe or an argv).
  mkdirSync(memberHome, { recursive: true, mode: 0o700 });
  chmodSync(memberHome, 0o700);
  renameSync(join(work, "invite.handoff"), join(memberHome, "invite"));
  chmodSync(join(memberHome, "invite"), 0o600);
  const member = launch(memberHome, memberOut, "l6-member", ["--invite"]);
  const alive = { owner, member };
  await until("member-take2-waiting", 400, alive);
  signal("go-remove");
  await until("owner-removed", 120, alive);
  signal("go-learn");
  await until("member-saved", 300, alive);
  signal("go-reinvite");
  await until("owner-reinvited", 120, alive);
  signal("go-rejoin");
  await until("member-drained", 400, alive);
  signal("go-finish");
  await Promise.all([owner, member].map((a) => (a.exited ? null : new Promise((ok) => a.p.once("exit", ok)))));
  const o = report(ownerOut);
  const m = report(memberOut);
  const failed = (r) => (r.checks ?? []).filter((c) => !c[1]);
  result = { ok: o.ok && m.ok, owner: { ok: o.ok, error: o.error, failed: failed(o), checks: o.checks?.length }, member: { ok: m.ok, error: m.error, failed: failed(m), checks: m.checks?.length, states: m.states } };
} catch (e) {
  result = { ok: false, error: String(e?.stack ?? e), owner: report(ownerOut), member: report(memberOut) };
} finally {
  // An instance still waiting on a step that will never come: ended by its own pid.
  for (const h of [ownerHome, memberHome]) {
    try {
      const { pid } = JSON.parse(readFileSync(join(h, "status.json"), "utf8"));
      // Only that very process: the pid must still be shroom running on this home.
      const cmd = execFileSync("ps", ["-p", String(pid), "-o", "command="], { encoding: "utf8" });
      if (pid && cmd.includes("shroom-dev.app") && cmd.includes(h)) process.kill(pid, "SIGTERM");
    } catch { /* not running, or never started */ }
  }
  fake?.p.stdin.end();
  worker?.p.stdin.end();
  // Keep both run logs for reading a failure; the homes (takes of the pattern, keys) go.
  for (const [who, h] of [["owner", ownerHome], ["member", memberHome]]) {
    if (existsSync(join(h, "logs/run.log"))) cpSync(join(h, "logs/run.log"), join(work, `${who}.run.log`));
    rmSync(h, { recursive: true, force: true });
  }
}
console.log(JSON.stringify({ work, ...result }, null, 1));
process.exit(result.ok ? 0 : 1);
