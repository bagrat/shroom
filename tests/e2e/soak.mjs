// L3's soak (TESTING-PLAN §5: "a 2-hour take with pauses, CPU and memory tracked";
// DECISIONS B64; tests-sys #4): the real app records one long, pause-heavy take of the test
// pattern — paused every ~45 s for ~8 s — uploading as it goes to the L1 fake (its `sink`
// bucket counts the video's bytes without holding them), under a throwaway home. The
// pattern covers the display the whole time; the only sound is the harness's beep. Never
// the installed app, never ~/.shroom(-dev), never Cloudflare.
//
//   cargo build -p shroom-cloudfake --bin cloudfake
//   (cd app && npm run build:dev)
//   node tests/e2e/soak.mjs [minutes]     15 by default; 120 is the milestone soak
//
// Run the long one when the Mac is quiet and on power: the display is covered throughout.
import { spawn } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createInterface } from "node:readline";
import { refuseOffTestMachine } from "../test-machine.mjs";

refuseOffTestMachine("soak.mjs");

const repo = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const app = process.env.SHROOM_APP ?? join(repo, "target/debug/bundle/macos/shroom-dev.app");
const fakeBin = join(repo, "target/debug/cloudfake");
for (const p of [app, fakeBin]) if (!existsSync(p)) { console.error(`missing ${p} — build it first (see the header)`); process.exit(2); }
const minutes = Number(process.argv[2] ?? 15);
if (!Number.isInteger(minutes) || minutes < 1 || minutes > 240) { console.error("usage: node tests/e2e/soak.mjs [minutes 1–240]"); process.exit(64); }

const work = mkdtempSync(join(tmpdir(), `shroom-soak-${minutes}m-`));
const home = join(work, "home");
const fake = spawn(fakeBin, ["--state", "found"], { stdio: ["pipe", "pipe", "inherit"] });
const ep = await new Promise((ok, fail) => {
  createInterface({ input: fake.stdout }).once("line", (l) => ok(JSON.parse(l)));
  fake.once("exit", () => fail(new Error("fake exited")));
});
fake.stdin.write("sink shroom-space\n");
const out = join(work, "report.json");
const started = Date.now();
const run = spawn("open", ["-W", "-n", "--stderr", join(work, "stderr.log"), app, "--args", "--home", home, "--cloud-fake", ep.api, ep.r2, "--drive", "take", out, `soak:${minutes}`], { stdio: "inherit" });
await new Promise((ok) => run.once("exit", ok));
fake.stdin.end();
const r = existsSync(out) ? JSON.parse(readFileSync(out, "utf8")) : { ok: false, error: "no report" };
// The core's trace ring (DECISIONS B93): a harness run's log ends on it, and the dev build's
// own law check never wrote it down for a broken law over the whole soak.
const log = (() => { try { return readFileSync(join(home, "logs/run.log"), "utf8"); } catch { return ""; } })();
const ring = { written: log.includes("trace: a harness run ends"), law_breaks: (log.match(/^(?:\S+Z )?trace: law /gm) ?? []).length };
if (r.ok && (!ring.written || ring.law_breaks > 0)) Object.assign(r, { ok: false, error: `the trace ring: ${JSON.stringify(ring)}` });
const summary = {
  work,
  minutes,
  ran_minutes: Math.round((Date.now() - started) / 600) / 100,
  ok: r.ok,
  error: r.error,
  checks: r.checks,
  pauses: r.facts?.pauses,
  length_ms: r.facts?.take?.length_ms,
  segments: r.facts?.take?.segments,
  rss_kib: r.facts?.rss_kib,
  cpu_pct: r.facts?.cpu_pct,
  ring,
};
console.log(JSON.stringify(summary, null, 1));
// A pass leaves nothing (the take is the pattern, but it's big); a failure keeps the home.
if (r.ok) rmSync(work, { recursive: true, force: true });
process.exit(r.ok ? 0 : 1);
