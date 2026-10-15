// The pre-merge e2e run's Mac side (TESTING-PLAN D3; .github/workflows/e2e-full.yml), on
// the dedicated test account, after the installer has put the dev app in place and the
// run has opened (tests/e2e-live's `e2e-run open`):
//
//   1. setup, through the installed app's own wizard (`--drive onboarding --e2e
//      --token-file`): the run's token pasted as a person pastes it, the gates found clear,
//      the account's name found, the space wired for real, the permissions chapter to
//      all-set — checked as the onboarding drive checks a path, plus that the space is the
//      test account's;
//   2. the heartbeat (`tests/e2e/heartbeat.mjs --e2e`) in that space: one take of the test
//      pattern → Stop → the page live → WebKit and Chromium play it → every frame and the
//      sound held to the recipe → the owner titles it → a member is invited, joins, records
//      and is removed → delete → the removed face.
//
//   3. the show pass (`--drive take show`): the take a person makes, paced so the screen's
//      recording shows it — the mushroom, the popover, Show yourself and its bubble (OBS's
//      pattern camera), Start, the count, a few seconds, Stop — then its link playing in a
//      visible browser. For a person to watch; it fails the run only if it can't publish.
//
// Every step's start and end (wall clock) go to <MAC_OUT>/chapters.json, which
// dev/e2e/captions.mjs burns into the screen's recording as captions.
//
//   node tests/e2e/full.mjs [seconds=10]
//
// Needs SHROOM_INSTALLED (the installed app), E2E_TOKEN_FILE (the run's token, mode 600 —
// read by the app and removed) and MAC_OUT (where the reports go). Prints one JSON summary;
// exits as the heartbeat does (0 all held, 2 a media bar missed, 1 the path broke).

import { spawn } from "node:child_process";
import { chromium } from "playwright";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { refuseOffTestMachine } from "../test-machine.mjs";

refuseOffTestMachine("full.mjs");

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const seconds = String(Number(process.argv[2] ?? 10));
const app = process.env.SHROOM_INSTALLED;
const tokenFile = process.env.E2E_TOKEN_FILE;
const out = process.env.MAC_OUT ?? fs.mkdtempSync(path.join(os.tmpdir(), "shroom-e2e-"));
for (const [name, v] of [["SHROOM_INSTALLED", app], ["E2E_TOKEN_FILE", tokenFile]]) {
  if (!v || !fs.existsSync(v)) {
    console.error(`full.mjs: ${name} must name an existing file`);
    process.exit(64);
  }
}
const home = path.join(os.homedir(), ".shroom-dev");
const summary = { ok: false, wizard: null, heartbeat: null };
const exited = (child) => new Promise((ok) => child.once("exit", (code) => ok(code)));
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
// What the video's captions say, and when (wall clock, ms).
const chapters = [];
const chapter = (from, to, text) => chapters.push({ from, to, text });
const writeChapters = () => fs.writeFileSync(path.join(out, "chapters.json"), JSON.stringify(chapters, null, 2));

// 1. The wizard.
const wizardFrom = Date.now();
const report = path.join(out, "wizard.json");
const said = path.join(out, "wizard.stderr.log");
await exited(spawn("open", ["-W", "-n", "--stderr", said, app, "--args", "--e2e", "--drive", "onboarding", report, "--token-file", tokenFile, "--relaunched=install"], { stdio: "inherit" }));
const problems = [];
let r = null;
try {
  r = JSON.parse(fs.readFileSync(report, "utf8"));
} catch {
  const words = fs.existsSync(said) ? fs.readFileSync(said, "utf8").split("\n").filter((l) => l.trim()).slice(-3).join(" / ").slice(0, 400) : "";
  problems.push(`shroom ended without its report${words ? ` — it said: ${words}` : ""}`);
}
if (fs.existsSync(tokenFile)) problems.push("the run's token file is still there: the app never took it");
if (r) {
  // N45: the permissions come before the wiring, which runs underneath them. The wiring row
  // shows last only while it is still going, so it may be there or not.
  const want = ["welcome", "account", "token", "card", "done", "perm-priming", "all-set", "closed"];
  const states = (r.states ?? []).filter((s, i, all) => !(s === "wiring" && all[i - 1] === "perm-priming" && all[i + 1] === "all-set"));
  if (!r.ok) problems.push(`drive: ${r.error ?? "not ok"}`);
  if (JSON.stringify(states) !== JSON.stringify(want)) problems.push(`states ${r.states.join(" → ")} (want ${want.join(" → ")}, a wiring step allowed before all-set)`);
  if (r.token_leaks.length) problems.push(`token leaks: ${r.token_leaks}`);
  if (!r.stash_empty) problems.push("the stash isn't empty");
  if (r.token_json_mode !== "600") problems.push(`token.json mode ${r.token_json_mode}`);
  if (!r.welcome_faces?.includes("first-launch")) problems.push(`welcome faces ${JSON.stringify(r.welcome_faces)} (want first-launch)`);
  if (r.status?.setup !== "all-set") problems.push(`status.json setup ${r.status?.setup}`);
  // The space the wizard wired is the test account's, under its name.
  let space = null;
  try { space = JSON.parse(fs.readFileSync(path.join(home, "space.json"), "utf8")); } catch { /* none */ }
  const id = fs.readFileSync(path.join(home, "e2e-account-id"), "utf8").trim();
  if (!space) problems.push("no space.json after setup");
  else {
    if (space.account_id !== id) problems.push("the space isn't on the test account");
    if (!/^[a-z0-9-]+\.shroom-e2e\.workers\.dev$/.test(space.host ?? "")) problems.push(`the space's host ${space.host}`);
  }
}
summary.wizard = { ok: problems.length === 0, states: r?.states?.join(" → "), host: r?.host, problems };
chapter(wizardFrom, Date.now(), "Setup: the installed app's own wizard, on the test account");
// The app's own log, for the evidence (public for three days): the token never reaches it
// (law 13, and the drive's leak check), and the account's id is scrubbed out.
try {
  const id = fs.readFileSync(path.join(home, "e2e-account-id"), "utf8").trim();
  const log = fs.readFileSync(path.join(home, "logs/run.log"), "utf8").split(id).join("<ACCOUNT_ID>");
  fs.writeFileSync(path.join(out, "run.log"), log);
} catch { /* no log */ }
console.log(JSON.stringify({ wizard: summary.wizard }, null, 2));

// 2. The heartbeat, in the wizard's space — only once setup held.
let code = 1;
if (summary.wizard.ok) {
  // What the headless browsers saw — the page playing, the owner's view, the removed face —
  // kept beside the screen's recording for a person to watch.
  const pages = path.join(out, "page");
  fs.mkdirSync(pages, { recursive: true });
  const hbFrom = Date.now();
  const hb = spawn("node", [path.join(repo, "tests/e2e/heartbeat.mjs"), seconds, "--e2e"], { stdio: ["ignore", "pipe", "inherit"], env: { ...process.env, SHROOM_INSTALLED: app, E2E_PAGE_VIDEO_DIR: pages } });
  let text = "";
  hb.stdout.on("data", (d) => { text += d; });
  code = await exited(hb);
  fs.writeFileSync(path.join(out, "heartbeat.json"), text);
  try { summary.heartbeat = JSON.parse(text); } catch { summary.heartbeat = { error: "no summary", tail: text.slice(-800) }; }
  // The heartbeat's own steps, each from the end of the one before (`at`: seconds from its start).
  const said = {
    record: "Measured take: the app records the test pattern it puts on screen",
    play: "The page plays in WebKit and Chromium, measured (headless: page/play-webkit.mp4 shows it)",
    media: "Checking every frame of what was published",
    av: "Checking the sound against the picture",
    title: "The owner titles the video on its page",
    member: "An invited person's Mac (a second copy of the app: the second mushroom) joins, records, is removed",
    delete: "Deleted from the Library; the page shows it's gone",
  };
  let from = hbFrom;
  for (const [name, step] of Object.entries(summary.heartbeat?.steps ?? {})) {
    const to = hbFrom + Number(step.at) * 1000;
    if (said[name]) chapter(from, to, said[name]);
    from = to;
  }
}

// 3. The show pass, on the same space: what a person does and sees, paced for the video.
// E2E_SHOW=0 (a Tart clone: no camera for the bubble) leaves it out.
if (summary.wizard.ok && process.env.E2E_SHOW !== "0") {
  const showFrom = Date.now();
  const showReport = path.join(out, "show.json");
  await exited(spawn("open", ["-W", "-n", "--stderr", path.join(out, "show.stderr.log"), app, "--args", "--e2e", "--drive", "take", showReport, "show"], { stdio: "inherit" }));
  let show = null;
  try { show = JSON.parse(fs.readFileSync(showReport, "utf8")); } catch { /* none */ }
  chapter(showFrom, Date.now(), "As a person: the mushroom, Show yourself (the bubble), Start, the count, Stop");
  summary.show = { ok: !!show?.ok, error: show?.error ?? (show ? undefined : "no report"), link: show?.facts?.link ?? null };
  // The link, playing where the screen's recording sees it.
  const link = summary.show.link;
  if (link) {
    const playFrom = Date.now();
    const rec = link.split("/").pop();
    const origin = link.slice(0, link.lastIndexOf("/"));
    for (const until = Date.now() + 120_000; Date.now() < until;) {
      const r = await fetch(`${origin}/m/${rec}/master.m3u8`).catch(() => null);
      if (r?.ok) break;
      await sleep(2000);
    }
    const browser = await chromium.launch({ headless: false });
    try {
      const page = await browser.newPage({ viewport: { width: 960, height: 600 } });
      await page.goto(link);
      await page.evaluate(() => { const v = document.getElementById("v"); if (v) { v.muted = true; v.play().catch(() => {}); } });
      await sleep(14_000);
      summary.show.played = await page.evaluate(() => { const v = document.getElementById("v"); return v ? v.currentTime : null; });
    } catch (e) {
      summary.show.playError = String(e.message ?? e).slice(0, 300);
    } finally {
      await browser.close();
    }
    chapter(playFrom, Date.now(), "Its link, playing in a browser");
  }
  if (!summary.show.ok) {
    console.error(`the show pass: ${summary.show.error ?? "not ok"}`);
    if (code === 0 || code === 2) code = 1;
  }
}
writeChapters();
summary.ok = summary.wizard.ok && code === 0;
writeChapters();
console.log(JSON.stringify({ ok: summary.ok, heartbeat: summary.heartbeat, show: summary.show }, null, 2));
process.exit(summary.wizard.ok ? code : 1);
