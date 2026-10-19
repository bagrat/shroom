// The pre-merge e2e run's command leg (spec N62; DECISIONS B215–B217): the `shroom` command
// an agent runs, in the space the installed app's own wizard wired on the test account —
// after the show pass, before the run closes (tests/e2e/full.mjs, step 4).
//
//   1. On PATH: the install line linked ~/.local/bin/shroom to the installed app's own
//      command, and a login shell finds it (B217).
//   2. It follows its app: `status` names the dev app and the wizard's space (B215).
//   3. The app running (with --e2e, as every instance of the run), `record start` is
//      refused as recording_off: General's switch is off by default (B216). The refusal
//      carries the link to the switch (`settings_link`), and the link, opened with the app
//      (`open -a`: a dev bundle claims no scheme), opens Settings there and flips nothing.
//   4. The switch on (prefs.json, as the app keeps it), relaunched: `record start` starts
//      the count, `pause` pauses, `resume` resumes, `stop` stops — each the click it stands
//      for — and the stop answers with its link, which goes live.
//   5. The video through the page: `info`, `rename`, then `delete`, which asks first
//      (needs_confirmation), dry-runs, and deletes on --yes; the link then says it's gone.
//   6. Before the delete, an edit (spec N72): `edit` reads it, `edit cut 1 2` saves a cut
//      through the page and asks the app for the final video, and the app makes it.
//
//   node tests/e2e/cli-leg.mjs
//
// Needs SHROOM_INSTALLED (the installed dev app). Prints one JSON summary; exit 0 when every
// step held, 1 when one didn't (`problems` says which).

import { execFileSync, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { refuseOffTestMachine } from "../test-machine.mjs";

refuseOffTestMachine("cli-leg.mjs");

const app = process.env.SHROOM_INSTALLED;
if (!app || !fs.existsSync(app)) {
  console.error("cli-leg.mjs: SHROOM_INSTALLED must name the installed app");
  process.exit(64);
}
const home = path.join(os.homedir(), ".shroom-dev");
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const problems = [];
const steps = {};
const started = Date.now();
const step = (name) => { steps[name] = { at: ((Date.now() - started) / 1000).toFixed(1) }; };
const check = (what, ok, seen) => { if (!ok) problems.push(`${what}${seen === undefined ? "" : ` — ${typeof seen === "string" ? seen : JSON.stringify(seen)}`}`); return ok; };

// 1. On PATH, through the user's own login shell.
step("path");
const link = path.join(os.homedir(), ".local/bin/shroom");
const target = path.join(app, "Contents/MacOS/shroom-cli");
check("the install line linked ~/.local/bin/shroom to the app's command", fs.existsSync(link) && fs.readlinkSync(link) === target, fs.existsSync(link) ? fs.readlinkSync(link) : "no link");
const shell = process.env.SHELL || "/bin/zsh";
const found = spawnSync(shell, ["-lc", "command -v shroom"], { encoding: "utf8" }).stdout.trim();
check(`a login shell (${path.basename(shell)}) finds shroom on PATH`, found === link, found || "not found");
const cli = fs.existsSync(link) ? link : target;

/** One command, as an agent runs it: `--json`, its exit code and its one object. */
function shroom(...args) {
  const r = spawnSync(cli, [...args, "--json"], { encoding: "utf8", timeout: 60_000 });
  let json = null;
  try { json = JSON.parse(r.stdout); } catch { /* not JSON */ }
  return { code: r.status, json: json ?? { raw: (r.stdout + r.stderr).slice(0, 300) } };
}

// The app's own process — its binary, with or without arguments — never the command's.
const appProcess = `${app}/Contents/MacOS/shroom( |$)`;
const appPids = () => spawnSync("pgrep", ["-f", appProcess], { encoding: "utf8" }).stdout.trim();
async function quitApp() {
  spawnSync("pkill", ["-f", appProcess]);
  for (let i = 0; i < 20 && appPids(); i++) await sleep(500);
}
/** The app, as the run starts every instance (its client pinned to the test account), up
 * and listening for the command. */
async function openApp() {
  execFileSync("open", ["-n", app, "--args", "--e2e"]);
  for (let i = 0; i < 60; i++) {
    const s = shroom("status").json;
    if (s?.app?.running && fs.existsSync(path.join(home, "control.sock"))) return s;
    await sleep(500);
  }
  return null;
}
/** Asks again while the app is between two steps (the count, a pause settling). */
async function settled(...args) {
  for (let i = 0; i < 30; i++) {
    const r = shroom(...args);
    if (r.code !== 6) return r;
    await sleep(500);
  }
  return shroom(...args);
}

let host = null;
let take = null;
try {
  // 2. It follows its app.
  step("status");
  await quitApp();
  const space = JSON.parse(fs.readFileSync(path.join(home, "space.json"), "utf8"));
  host = space.host;
  const status = await openApp();
  check("the app came up and listens", !!status, "no status.json or no control.sock in 60 s");
  check("status names the dev app", status?.app?.id === "am.shroom.dev", status?.app);
  check("status names the wizard's space, ready", status?.space?.host === host && status?.space?.ready === true, status?.space);
  const list = shroom("list");
  check("list answers", list.code === 0 && Array.isArray(list.json.videos), list.json);

  // 3. The switch off, the default: refused, and nothing starts.
  step("switch-off");
  const off = shroom("record", "start");
  check("record start with the switch off is recording_off (exit 10)", off.code === 10 && off.json.error === "recording_off", off);
  check("the refusal carries the link to the switch", off.json.settings_link === "shroom://settings/agent", off.json);
  step("settings-link");
  const runLog = path.join(home, "logs", "run.log");
  const taken = () => (fs.existsSync(runLog) ? fs.readFileSync(runLog, "utf8").split("\n").filter((l) => l.includes("core settings-link ")).length : 0);
  const before = taken();
  spawnSync("open", ["-a", app, off.json.settings_link ?? "shroom://settings/agent"]);
  let tookIt = false;
  for (const until = Date.now() + 15_000; !tookIt && Date.now() < until; await sleep(500)) tookIt = taken() > before;
  check("the link opens Settings at the switch", tookIt, `${taken()} after ${before}`);
  const prefsNow = (() => { try { return JSON.parse(fs.readFileSync(path.join(home, "prefs.json"), "utf8")); } catch { return {}; } })();
  check("the link flips nothing", prefsNow.agent_recording !== true && shroom("record", "start").json.error === "recording_off", prefsNow);

  // 4. The switch on, as the app keeps it; relaunched.
  step("switch-on");
  await quitApp();
  const prefsFile = path.join(home, "prefs.json");
  const prefs = fs.existsSync(prefsFile) ? JSON.parse(fs.readFileSync(prefsFile, "utf8")) : {};
  fs.writeFileSync(prefsFile, JSON.stringify({ ...prefs, agent_recording: true }), { mode: 0o600 });
  check("the app came back up", !!(await openApp()));
  step("record");
  const start = shroom("record", "start");
  check("record start starts the count", start.code === 0 && start.json.take === "countdown", start);
  await sleep(6000);
  const pause = await settled("record", "pause");
  check("record pause pauses", pause.code === 0 && pause.json.take === "paused", pause);
  await sleep(1500);
  const resume = await settled("record", "resume");
  check("record resume goes on", resume.code === 0 && resume.json.take === "recording", resume);
  await sleep(4000);
  const stop = await settled("record", "stop");
  check("record stop answers with its link", stop.code === 0 && typeof stop.json.link === "string" && stop.json.link.startsWith(`https://${host}/`), stop);
  take = stop.json.link ? stop.json.link.split("/").pop() : null;
  step("live");
  let live = false;
  for (const until = Date.now() + 120_000; take && Date.now() < until;) {
    const r = await fetch(`https://${host}/m/${take}/master.m3u8`).catch(() => null);
    if (r?.ok) { live = true; break; }
    await sleep(2000);
  }
  check("its page went live", live);

  // 5. Through the page: read, rename, delete — asked first. The playlist answers before the
  // record says the video is done (live, with its length): asked again until it does.
  step("rename");
  let info = shroom("info", take ?? "-");
  for (const until = Date.now() + 60_000; take && !(info.json.video?.state === "live" && info.json.video?.duration_sec > 0) && Date.now() < until;) {
    await sleep(2000);
    info = shroom("info", take);
  }
  check("info finds it, with a length", info.code === 0 && info.json.video?.state === "live" && info.json.video?.duration_sec > 0, info);
  const renamed = shroom("rename", take ?? "-", "Recorded by the shroom command");
  check("rename saves the title", renamed.code === 0 && renamed.json.title === "Recorded by the shroom command", renamed);

  // 6. An edit (spec N72): read, a second cut out — saved through the page, the app asked
  // whether the video is open in its editor, then for the final video — and the app making it.
  step("edit");
  const unedited = shroom("edit", take ?? "-");
  check("edit says what plays, not edited yet", unedited.code === 0 && unedited.json.edit?.edited === false && unedited.json.edit?.plays_sec > 2, unedited);
  const cut = shroom("edit", take ?? "-", "cut", "1", "2");
  check("edit cut saves it and asks the app for the final video", cut.code === 0 && cut.json.saved === true && cut.json.final === "asked" && Math.abs(cut.json.edit?.plays_sec - (unedited.json.edit?.plays_sec - 1)) < 0.01, cut);
  let made = null;
  for (const until = Date.now() + 240_000; take && Date.now() < until; await sleep(3000)) {
    const r = shroom("edit", take);
    if (r.json.edit?.final_video === "made") { made = r; break; }
  }
  check("the app made the final video", made?.json.edit?.cuts?.[0]?.id === "m1", made ?? "not made in 4 min");
  step("delete");
  const asked = shroom("delete", take ?? "-");
  check("delete without --yes asks first (needs_confirmation, exit 7)", asked.code === 7 && asked.json.error === "needs_confirmation", asked);
  const dry = shroom("delete", take ?? "-", "--dry-run");
  check("delete --dry-run names it", dry.code === 0 && dry.json.would_delete?.[0]?.id === take, dry);
  const gone = shroom("delete", take ?? "-", "--yes");
  check("delete --yes deletes it", gone.code === 0 && gone.json.deleted?.[0] === take, gone);
  const after = shroom("list");
  check("it's off the list", after.code === 0 && !after.json.videos?.some((v) => v.id === take), after.json.videos?.map((v) => v.id));
  const page = take ? await fetch(`https://${host}/${take}`).catch(() => null) : null;
  check("its link says it's gone", page?.status === 404, page?.status);
} catch (e) {
  problems.push(`the leg broke: ${String(e.message ?? e).slice(0, 300)}`);
} finally {
  await quitApp();
}
step("done");
const ok = problems.length === 0;
console.log(JSON.stringify({ ok, steps, take, problems }, null, 2));
process.exit(ok ? 0 : 1);
