// The surfaces drive (DEVIATIONS #39, #40): the popover, the countdown and the bubble as
// macOS has them — who has the keyboard, who is in front, which Space they sit on, whether
// a first click acts — through the real app (a debug bundle) against the L1 fake, under a
// throwaway home. It records nothing: no take is ever opened. Never the real account,
// never ~/.shroom or ~/.shroom-dev.
//
//   cargo build -p shroom-cloudfake --bin cloudfake
//   (cd app && npm run build:dev — the harness is built in only on purpose)
//   node app/test/surfaces-drive.mjs                 on this Mac, beside whatever is in front
//   node app/test/surfaces-drive.mjs --fullscreen    beside another app that goes full screen
//
// `--fullscreen` starts app/test/fullscreen-helper.js — a window that takes a Space of its
// own — and is only for the L5 VM (tests/l5/surfaces-vm.sh runs it there): on a desk
// somebody is using it would move them.
//
// A locked screen gives no window the keyboard, so the keyboard checks are skipped there
// and listed; `--strict` fails the run on anything skipped (the VM run is strict) — but
// for the bubble, which needs a camera a VM hasn't got, and for what ScreenCaptureKit lists
// for a take, which needs leave to record: those two are this Mac's to read.
// `--late-panels` has shroom make neither overlay panel as it starts, so each is made the
// late way — the path a failed start-up takes — the first time it shows.
// `--report=<path>` keeps the whole report. SHROOM_APP overrides the bundle path.

import { spawn } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createInterface } from "node:readline";
import { refuseOffTestMachine } from "../../tests/test-machine.mjs";

refuseOffTestMachine("surfaces-drive.mjs");

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../..");
const app = process.env.SHROOM_APP ?? join(repo, "target/debug/bundle/macos/shroom-dev.app");
const fakeBin = join(repo, "target/debug/cloudfake");
for (const p of [app, fakeBin]) if (!existsSync(p)) { console.error(`missing ${p} — build it first (see the header)`); process.exit(2); }
const args = process.argv.slice(2);
const fullscreen = args.includes("--fullscreen");
const strict = args.includes("--strict");
const late = args.includes("--late-panels");
const keepReport = args.find((a) => a.startsWith("--report="))?.slice("--report=".length);

const work = mkdtempSync(join(tmpdir(), "shroom-surfaces-"));
const sleep = (ms) => new Promise((ok) => setTimeout(ok, ms));
let helper = null;
let fake = null;
const cleanup = () => {
  // Only what this run started, by the handles it holds.
  if (helper && helper.exitCode === null) helper.kill();
  if (fake && fake.exitCode === null) fake.kill();
  rmSync(work, { recursive: true, force: true });
};
process.on("exit", cleanup);

if (fullscreen) {
  const dir = join(work, "helper");
  mkdirSync(dir);
  helper = spawn("osascript", ["-l", "JavaScript", join(here, "fullscreen-helper.js"), dir], { stdio: "ignore" });
  const state = () => { try { return JSON.parse(readFileSync(join(dir, "state.json"), "utf8")); } catch { return null; } };
  for (let i = 0; i < 100 && !state()?.active; i++) await sleep(150);
  if (!state()?.active) { console.error(`the other app never came forward: ${JSON.stringify(state())}`); process.exit(1); }
}

fake = spawn(fakeBin, ["--state", "found"], { stdio: ["pipe", "pipe", "inherit"] });
const ep = await new Promise((ok, fail) => {
  createInterface({ input: fake.stdout }).once("line", (l) => ok(JSON.parse(l)));
  fake.once("exit", () => fail(new Error("fake exited")));
});
const out = join(work, "report.json");
// `-g`: opened behind whatever is in front, the way a login launch is.
const run = spawn("open", ["-W", "-n", "-g", "--stderr", join(work, "stderr.log"), app, "--args", "--home", join(work, "home"), "--cloud-fake", ep.api, ep.r2, "--drive", "take", out, "surfaces", ...(late ? ["--late-panels"] : [])], { stdio: "inherit" });
await new Promise((ok) => run.once("exit", ok));
fake.stdin.end();
if (helper) {
  writeFileSync(join(work, "helper", "cmd-quit"), "");
  for (let i = 0; i < 40 && helper.exitCode === null; i++) await sleep(150);
}

const r = existsSync(out) ? JSON.parse(readFileSync(out, "utf8")) : { ok: false, error: "no report", checks: [], facts: {} };
if (keepReport && existsSync(out)) copyFileSync(out, keepReport);
// The panels as macOS drew them, where the drive could look (`looked_at`): kept beside the report.
if (keepReport) for (const name of ["popover.png", "count.png"]) if (existsSync(join(work, name))) copyFileSync(join(work, name), join(dirname(keepReport), name));
const failed = r.checks.filter(([, ok]) => !ok).map(([what, , seen]) => ({ what, seen }));
const skipped = (r.facts?.skipped ?? []).map(([what, why]) => ({ what, why }));
// What a clean VM hasn't got, and this Mac reads instead: a camera, and leave to record.
const THIS_MACS = ["no camera shroom may open on this Mac", "recording isn't allowed for shroom on this Mac"];
const ok = r.ok && !(strict && skipped.some((s) => !THIS_MACS.includes(s.why)));
console.log(JSON.stringify({ ok, macos: r.facts?.macos ?? null, passed: r.checks.length - failed.length, failed, skipped, error: r.error ?? null, steps: r.steps?.length ?? 0 }, null, 1));
process.exit(ok ? 0 : 1);
