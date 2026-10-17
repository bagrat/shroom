// The onboarding drive: the real app (a debug bundle), walked through the welcome, the
// plan and the permissions chapter like a person, against the L1 Cloudflare fake, under a
// throwaway home. Never the real account, never ~/.shroom or ~/.shroom-dev.
//
//   cargo build -p shroom-cloudfake --bin cloudfake
//   (cd app && npm run build:dev — the harness is built in only on purpose)
//   node app/test/onboarding-drive.mjs [fresh|taken|email-gate|card-gate|found|doors|invite|all]...
//
// Each path starts the fake in its state, runs `--drive onboarding`, clears a gate from
// the "dashboard" (the fake's stdin) when the drive reports one, and checks the state
// sequence, the first launch's face (the app is opened with the install script's
// `--relaunched=install`), the token's one home (token.json, mode 600), the empty stash, the Dock (law 6)
// and that the quiet shell only logged. SHROOM_APP overrides the bundle path. Each path has
// its own result: a launch that ended without its report is that path's failure, in the
// app's own last words, and the others still run.
//
// The setup window's Dock icon (law 6, DEVIATIONS #38) has two paths of its own. `doors`:
// ✕ at the last step retreats to the menu bar and gives the keyboard back, and the mushroom
// brings the window, the icon and the keyboard back — the walk ends there, before the Mac's
// grants are needed, so a clean Mac can run it. `invite`: all-set's Invite someone ends the
// walk — Settings carries on with no icon, in front, with the keyboard, and what is typed
// next lands in “Who's this for?” with no click first (N28). The keyboard's side
// needs an unlocked screen: on a locked one the report says what went unseen, and
// SHROOM_HANDS=1 makes that a failure (the L5 lanes: tests/l5/dock-vm.sh for `doors`).
//
// Every path but `doors` walks the permissions chapter to all-set, and a drive never asks
// macOS for anything: it needs a Mac where the dev build may already hear and record — this
// one, or a clean VM given both (tests/l5/setup-vm.sh runs every path there, strict).

import { spawn } from "node:child_process";
import { existsSync, mkdtempSync, readFileSync, rmSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createInterface } from "node:readline";
import { refuseOffTestMachine } from "../../tests/test-machine.mjs";

refuseOffTestMachine("onboarding-drive.mjs");

const repo = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const app = process.env.SHROOM_APP ?? join(repo, "target/debug/bundle/macos/shroom-dev.app");
const fakeBin = join(repo, "target/debug/cloudfake");
for (const p of [app, fakeBin]) if (!existsSync(p)) { console.error(`missing ${p} — build it first (see the header)`); process.exit(2); }

const head = ["welcome", "account", "token", "card"];
const tail = ["done", "perm-priming", "all-set", "closed"];
// A claimed name goes straight to done: wiring starts underneath the moment the name ticks
// (N45), and its own last step shows only if the permissions chapter outruns it — never
// against the fake, which wires at once.
const PATHS = {
  fresh: { fake: ["fresh"], states: [...head, "name", ...tail] },
  taken: { fake: ["taken:acme-team"], drive: ["--taken", "acme-team"], states: [...head, "name", "name-taken", "name", ...tail] },
  "email-gate": { fake: ["email-gate"], states: [...head, "gate-email", "card", "name", ...tail] },
  "card-gate": { fake: ["card-gate"], states: [...head, "gate-card", "card", "name", ...tail] },
  found: { fake: ["found"], states: [...head, "space-found", ...tail], host: "v.myshroom.workers.dev" },
  doors: { fake: ["found"], drive: ["--doors"], states: [...head, "space-found", "done", "closed", "done", "closed"], host: "v.myshroom.workers.dev", ends: "done", checks: 2 },
  invite: { fake: ["found"], drive: ["--invite-someone"], states: [...head, "space-found", ...tail], host: "v.myshroom.workers.dev", checks: 1 },
};
let asked = process.argv.slice(2);
if (asked.length === 0) asked = ["fresh"];
if (asked.includes("all")) asked = Object.keys(PATHS);

async function drive(name) {
  const path = PATHS[name];
  if (!path) throw new Error(`unknown path ${name}`);
  const work = mkdtempSync(join(tmpdir(), "shroom-onb-"));
  const fake = spawn(fakeBin, path.fake.flatMap((s) => ["--state", s]), { stdio: ["pipe", "pipe", "inherit"] });
  const endpoints = await new Promise((ok, fail) => {
    createInterface({ input: fake.stdout }).once("line", (l) => ok(JSON.parse(l)));
    fake.once("exit", () => fail(new Error("the fake exited")));
  });
  const marker = join(work, "gate");
  const out = join(work, "report.json");
  const said = join(work, "stderr.log");
  // The person clears the gate in the dashboard a little after shroom shows it (so at
  // least one 5 s poll sees it still up). GATE_HOLD_MS holds it longer, for a screen
  // recording to show the gate's face (dev/mac/scenarios/gate-face.sh).
  const hold = Number(process.env.GATE_HOLD_MS) > 0 ? Number(process.env.GATE_HOLD_MS) : 6000;
  let cleared = false;
  const watch = setInterval(() => {
    if (!cleared && existsSync(marker)) {
      cleared = true;
      setTimeout(() => fake.stdin.write("clear-gates\n"), hold);
    }
  }, 200);
  try {
    // Asynchronously: the gate watcher above needs the event loop while the app runs.
    const run = spawn("open", ["-W", "-n", "--stderr", said, app, "--args", "--home", join(work, "home"), "--cloud-fake", endpoints.api, endpoints.r2, "--drive", "onboarding", out, "--gate-marker", marker, ...(path.drive ?? []), "--relaunched=install"], { stdio: "inherit" });
    await new Promise((ok) => run.once("exit", ok));
  } finally {
    clearInterval(watch);
    fake.stdin.end();
  }
  // A launch that ended without its report is this path's failure, in the app's own last
  // words (`open` hands them to nobody unless asked) — never the runner's end: the paths
  // before it keep their results, and the ones after it still run.
  let r;
  try {
    r = JSON.parse(readFileSync(out, "utf8"));
  } catch {
    const words = existsSync(said) ? readFileSync(said, "utf8").split("\n").filter((l) => l.trim()).slice(-3).join(" / ").slice(0, 400) : "";
    rmSync(work, { recursive: true, force: true });
    return { path: name, ok: false, problems: [`shroom ended without its report${words ? ` — it said: ${words}` : ""}`] };
  }
  const problems = [];
  if (!r.ok) problems.push(`drive: ${r.error ?? "not ok"}`);
  if (JSON.stringify(r.states) !== JSON.stringify(path.states)) problems.push(`states ${r.states.join(" → ")} (want ${path.states.join(" → ")})`);
  if (r.token_leaks.length) problems.push(`token leaks: ${r.token_leaks}`);
  if (!r.stash_empty) problems.push("the stash isn't empty");
  if (r.token_json_mode !== "600") problems.push(`token.json mode ${r.token_json_mode}`);
  if (!r.dock_with_window?.startsWith("regular") || !r.dock_after_close?.startsWith("accessory")) problems.push(`dock ${r.dock_with_window} / ${r.dock_after_close}`);
  // The doors, each as the drive saw it — and nothing left unlooked-at where the run was
  // promised a keyboard.
  const doors = r.doors ?? [];
  if (doors.length < (path.checks ?? 0)) problems.push(`the doors weren't walked (${doors.length} checks)`);
  for (const [what, ok, saw] of doors) if (!ok) problems.push(`${what} — ${saw}`);
  if (process.env.SHROOM_HANDS === "1" && r.unseen?.length) problems.push(`not looked at: ${r.unseen.join("; ")}`);
  if (!r.host || (path.host && r.host !== path.host)) problems.push(`host ${r.host}`);
  const ended = path.ends ?? "all-set";
  if (r.status?.setup !== ended) problems.push(`status.json setup ${r.status?.setup}`);
  // The status seam's v2 shape (ARCHITECTURE §11.2, DECISIONS B80) at the end: the token's
  // Mac, its chapter done, no invite, no take, the grants in the seam's words.
  const st = r.status ?? {};
  if (ended === "all-set" && (st.v !== 2 || st.take !== "none" || st.take_open !== false || st.join?.role !== "token-holder" || st.join?.invite !== "none" || st.join?.standing !== "fine" || st.permission?.chapter !== "done" || !["granted", "not-granted"].includes(st.permission?.screen) || !["granted", "denied", "restricted", "not-asked"].includes(st.permission?.mic))) {
    problems.push(`status.json ${JSON.stringify(st)}`);
  }
  // Every write, from the run log (`seam: {…}`): the plan's states in the drive's order, and
  // never a host or the token in the seam.
  const log = existsSync(join(work, "home/logs/run.log")) ? readFileSync(join(work, "home/logs/run.log"), "utf8") : "";
  const seam = log.split("\n").filter((l) => l.includes(" seam: ")).map((l) => l.slice(l.indexOf(" seam: ") + 7));
  const setups = seam.map((l) => JSON.parse(l).setup);
  let from = 0;
  const missing = path.states.filter((s) => s !== "closed").filter((want) => {
    const i = setups.indexOf(want, from);
    if (i >= 0) from = i;
    return i < 0;
  });
  if (missing.length) problems.push(`the seam never said ${missing.join(", ")} in order (${[...new Set(setups)].join(" → ")})`);
  if (seam.some((l) => /workers\.dev|acme-team|token":|cfut_|"secret/.test(l))) problems.push("the seam carries a host or a secret");
  // Opened as the install script opens a fresh install: the welcome is the first launch's.
  if (!r.welcome_faces?.includes("first-launch")) problems.push(`welcome faces ${JSON.stringify(r.welcome_faces)} (want first-launch)`);
  const unexpected = r.would.filter((w) => w !== "register the login item");
  if (unexpected.length) problems.push(`the quiet shell would also: ${unexpected.join("; ")}`);
  rmSync(work, { recursive: true, force: true });
  return { path: name, ok: problems.length === 0, states: r.states.join(" → "), host: r.host, ...(doors.length ? { doors: doors.map(([what, , saw]) => `${what} — ${saw}`) } : {}), ...(r.unseen?.length ? { unseen: r.unseen } : {}), problems };
}

const results = [];
// One path at a time, each with its own result: a path the runner couldn't walk at all is
// that path's failure too.
for (const name of asked) results.push(await drive(name).catch((e) => ({ path: name, ok: false, problems: [e.message] })));
console.log(JSON.stringify(results, null, 2));
process.exit(results.every((r) => r.ok) ? 0 : 1);
