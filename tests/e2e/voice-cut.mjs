// Voice cuts from capture to the owner page (spec N65), on the dedicated test account, after
// the e2e run's wizard has wired the run's space (tests/e2e/full.mjs): the space turned to
// listen as a person turns it (the live drive's `--ai`: N66's banner and its confirm), then one take
// whose sound is the Mac's own stock voice saying a slip, “shroom cut”, and the line again,
// twice, in place of the beep (`--voice`); Stop → the page live → the space hears the take →
// this Mac plans the cuts, asks the space's Clef how far back the second redo reaches, and
// lands them → the owner page, open as the owner, shows the strip. Then the take is deleted.
// Playing the page with its cuts skipped is the editor's player (N63, its own drives); here
// the cut take's sound is rendered from the strip's own keep, for a person to listen to.
//
//   node tests/e2e/voice-cut.mjs
//
// Needs SHROOM_INSTALLED (the installed dev app), MAC_OUT (where the evidence goes:
// voice-cut/summary.json, strip.webm — the owner page as the cuts land —, strip.png,
// before.wav and after.wav — the take's sound as recorded, and cut where the strip says with
// the editor's 30 ms fades at each join —, and voice-cut.mp4, the strip over the cut sound)
// and a build whose live drive turns the AI on (`--ai`, N66). Prints one JSON summary. Exit 0:
// every check held; 1: one didn't, or the path broke; 64: this build can't turn the AI on.
//
// No one's voice: the speech is macOS's `say`. No secret: the owner's one-time key reaches
// this script in a mode-600 file beside the drive's report, as the heartbeat's does.

import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { chromium } from "playwright";
import { refuseOffTestMachine } from "../test-machine.mjs";

refuseOffTestMachine("voice-cut.mjs");
// Voice cuts need the space to listen: a build whose live drive can't turn the AI on is refused.
const repo = path.resolve(path.dirname(new URL(import.meta.url).pathname), "../..");
if (!fs.readFileSync(path.join(repo, "app/src-tauri/src/harness.rs"), "utf8").includes("pub fn drive_ai(")) {
  console.error("voice-cut.mjs: this build's live drive can't turn the AI on (`--ai`, N66); voice cuts need the space to listen");
  process.exit(64);
}

const installed = process.env.SHROOM_INSTALLED ?? path.join(os.homedir(), "Applications/shroom-dev.app");
const out = path.join(process.env.MAC_OUT ?? fs.mkdtempSync(path.join(os.tmpdir(), "shroom-e2e-")), "voice-cut");
fs.mkdirSync(out, { recursive: true });
const work = fs.mkdtempSync(path.join(os.tmpdir(), "shroom-voice-cut-"));
const DRIVE_PROC = `${path.basename(installed)}/Contents/MacOS/shroom`;
const SECONDS = 34;
const SEEN = { width: 1024, height: 640 };
// What the take says (Bagrat's words on the tester, 2026-10-18): a broken-off restart, then a
// line said again in other words. [[slnc ms]] is `say`'s own pause.
const SPEECH = [
  "[[slnc 600]] Hi, this is a quick test of shroom.", "[[slnc 700]] Today, I will show you how to share, um,",
  "[[slnc 900]] shroom cut.", "[[slnc 1000]] Today, I'll show you how to share a recording in one click.",
  "[[slnc 800]] You press stop and the link is pasted.", "[[slnc 900]] Shroom cut.",
  "[[slnc 1000]] When you want to get your link, you just press stop, and it is copied for you.", "[[slnc 600]] That's it.",
].join(" ");
// What the strip must strike: the first redo's slip, and (by Clef) the line said again.
const FIRST = /will show you how to share.*cut/i;
const SECOND = /press stop and the link is pasted.*cut/i;
const summary = { ok: false, checks: {}, ms: {} };
const t0 = Date.now();
const check = (name, ok, seen) => { summary.checks[name] = { ok: !!ok, seen }; };
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const tool = (name) => ["/opt/homebrew/bin", "/usr/local/bin"].map((d) => path.join(d, name)).find((p) => fs.existsSync(p)) ?? name;
const run = (cmd, args) => {
  const r = spawnSync(cmd, args, { encoding: "utf8", timeout: 300_000 });
  if (r.status !== 0) throw new Error(`${path.basename(cmd)}: ${(r.stderr || r.error || "").toString().slice(0, 400)}`);
  return r.stdout;
};
const readJson = (f) => { try { return JSON.parse(fs.readFileSync(f, "utf8")); } catch { return null; } };
// A system alert an earlier step left up (the Local Network prompt a visible browser raises)
// sits over the drive's pattern, and the drive then refuses to record. Each is answered with
// its button that isn't Allow: nothing here needs the local network. How many were up.
const alerts = () => spawnSync("/usr/bin/osascript", [
  "-e", 'tell application "System Events"',
  "-e", 'if exists process "UserNotificationCenter" then return count windows of process "UserNotificationCenter"',
  "-e", "return 0",
  "-e", "end tell",
], { encoding: "utf8" }).stdout.trim();
async function answerAlerts() {
  let answered = 0;
  for (let i = 0; i < 5 && Number(alerts()) > 0; i++) {
    spawnSync("/usr/bin/osascript", ["-e", 'tell application "System Events" to tell process "UserNotificationCenter" to click (first button of window 1 whose name is not "Allow")']);
    answered += 1;
    await sleep(1000);
  }
  return answered;
}

let driving = null;
try {
  // 1. The speech, in the Mac's own stock voice, as a 48 kHz WAV.
  const aiff = path.join(work, "speech.aiff");
  const wav = path.join(work, "speech.wav");
  run("/usr/bin/say", ["-o", aiff, SPEECH]);
  run("/usr/bin/afconvert", ["-f", "WAVE", "-d", "LEI16@48000", "-c", "1", aiff, wav]);
  const said = Number(run(tool("ffprobe"), ["-v", "error", "-show_entries", "format=duration", "-of", "csv=p=0", wav]).trim());
  check("the speech fits the take", said > 10 && said < SECONDS - 3, `${said.toFixed(1)} s of speech in a ${SECONDS} s take`);
  fs.copyFileSync(wav, path.join(out, "speech.wav"));

  // 2–3. The live drive: the space turned to listen as a person turns it (the popover's AI
  // banner, Turn on…, its confirm's Turn on), then the take — the pattern on the screen, the
  // speech in place of the microphone. It stops, waits for the upload, leaves the owner's key
  // beside its report and holds there until this script asks for the delete, while this
  // Mac's voice lane runs. (A drive of the AI alone, `--drive ai`, is the fake's only.)
  spawnSync("pkill", ["-f", DRIVE_PROC]);
  summary.alertsAnswered = await answerAlerts();
  const report = path.join(work, "drive.json");
  driving = spawn("open", ["-W", "-n", installed, "--args", "--e2e", "--drive", report, String(SECONDS), "--voice", wav, "--ai"], { stdio: "ignore" });
  let drive = null;
  for (const until = Date.now() + (SECONDS + 300) * 1000; Date.now() < until;) {
    drive = readJson(report);
    if (drive && (drive.phase !== "recording" || drive.error)) break;
    if (driving.exitCode !== null) break;
    await sleep(250);
  }
  const listens = (drive?.steps ?? []).some(([step]) => step === "AI is on");
  check("the space listens (the AI turned on)", listens, listens ? "on" : drive?.error ?? "no step said so");
  if (!drive || drive.error || drive.phase !== "live") throw new Error(`the take: ${drive?.error ?? drive?.phase ?? "no report"}`);
  const stopAt = drive.stop_at_ms;
  summary.ms.stopToLive = drive.stop_to_uploaded_ms;
  const origin = `https://${drive.host}`;
  const pageUrl = `${origin}/${drive.rec}`;
  check("the take went live", true, pageUrl.replace(drive.rec, "<id>"));

  // 4. The status says cuts are coming, then gives their tag (what the owner page polls).
  const status = async () => (await fetch(`${origin}/_shroom/status/${drive.rec}`, { cache: "no-store" }).then((r) => r.json()).catch(() => ({})));
  let coming = false, tag = null;
  for (const until = Date.now() + 300_000; Date.now() < until && !tag;) {
    const s = await status();
    if (s.cuts === "coming") coming = true;
    else if (s.cuts) tag = s.cuts;
    if (!tag) await sleep(1000);
  }
  check("the status said cuts were coming", coming, coming ? "coming" : "never seen");
  check("the cuts landed", !!tag, tag ? "a tag" : "no tag within 5 minutes of go-live");
  if (tag) summary.ms.stopToLanded = Date.now() - stopAt;

  // 5. The owner page, open as the owner (the app's one-time key): the strip under the player.
  const k = fs.readFileSync(path.join(path.dirname(report), "owner-key"), "utf8").trim();
  const browser = await chromium.launch({ headless: false });
  try {
    const context = await browser.newContext({ viewport: SEEN, recordVideo: { dir: work, size: SEEN } });
    const page = await context.newPage();
    await page.goto(`${pageUrl}?k=${encodeURIComponent(k)}`);
    await page.waitForSelector("#cuts:not([hidden]) li", { timeout: 120_000 });
    summary.ms.stopToStrip = Date.now() - stopAt;
    const rows = await page.$$eval("#cuts li", (lis) => lis.map((li) => ({ cls: li.className, words: li.querySelector("q .w")?.textContent ?? "", at: li.querySelector(".at")?.textContent ?? "" })));
    summary.rows = rows;
    const on = rows.filter((r) => r.cls === "cut on");
    check("the strip strikes the first slip", on.some((r) => FIRST.test(r.words)), rows.map((r) => `${r.cls}: ${r.words}`).join(" | "));
    check("the strip strikes the line said again (Clef)", on.some((r) => SECOND.test(r.words)), rows.map((r) => `${r.cls}: ${r.words}`).join(" | "));
    await page.screenshot({ path: path.join(out, "strip.png") });
    const { keep } = await (await page.request.get(`${origin}/_shroom/edits/${drive.rec}`)).json();
    summary.keep = keep;
    const kept = keep.reduce((s, [a, b]) => s + (b - a), 0);
    check("what plays is shorter by the cuts", kept > 5 && kept < SECONDS - 3, `${kept.toFixed(1)} s kept of a ${SECONDS} s take`);
    // A moment on the strip, for whoever watches.
    await sleep(3000);
    await context.close();
    fs.copyFileSync(await page.video().path(), path.join(out, "strip.webm"));

    // 6. What a person hears: the take's sound, cut where the strip says, each join faded
    // 30 ms out and 30 ms in as the editor's copy does; and the strip over it.
    const sound = path.join(work, "sound.wav");
    run(tool("ffmpeg"), ["-hide_banner", "-loglevel", "error", "-y", "-i", `${origin}/m/${drive.rec}/master.m3u8`, "-vn", "-ac", "1", "-ar", "48000", sound]);
    fs.copyFileSync(sound, path.join(out, "before.wav"));
    const F = 0.03;
    const parts = keep.map(([a, b], i) => `[0:a]atrim=${a}:${b},asetpts=PTS-STARTPTS${i ? `,afade=t=in:d=${F}` : ""}${i < keep.length - 1 ? `,afade=t=out:st=${Math.max(0, b - a - F)}:d=${F}` : ""}[p${i}]`);
    const cutWav = path.join(out, "after.wav");
    run(tool("ffmpeg"), ["-hide_banner", "-loglevel", "error", "-y", "-i", sound, "-filter_complex", `${parts.join(";")};${keep.map((_, i) => `[p${i}]`).join("")}concat=n=${keep.length}:v=0:a=1[o]`, "-map", "[o]", cutWav]);
    run(tool("ffmpeg"), ["-hide_banner", "-loglevel", "error", "-y", "-loop", "1", "-i", path.join(out, "strip.png"), "-i", cutWav, "-shortest",
      "-c:v", "libx264", "-preset", "veryfast", "-tune", "stillimage", "-pix_fmt", "yuv420p", "-c:a", "aac", "-movflags", "+faststart", path.join(out, "voice-cut.mp4")]);
    check("the evidence is kept", fs.existsSync(path.join(out, "voice-cut.mp4")), "strip.webm, strip.png, before.wav, after.wav, voice-cut.mp4");
  } finally {
    await browser.close();
  }

  // 7. Delete, through the Library like a person (the drive does it on this word).
  fs.writeFileSync(path.join(path.dirname(report), "delete-now"), "");
  await Promise.race([new Promise((ok) => driving.once("exit", ok)), sleep(120_000)]);
  const final = readJson(report);
  check("the take was deleted", final?.phase === "deleted" && !final?.error, final?.error ?? final?.phase);
  // How the lane went, from the app's own log (scores, never words).
  const log = fs.readFileSync(path.join(os.homedir(), ".shroom-dev/logs/run.log"), "utf8").split("\n").filter((l) => /\bvoice: /.test(l));
  summary.lane = log.slice(-12);
  summary.ok = Object.values(summary.checks).every((c) => c.ok);
} catch (e) {
  summary.error = String(e.message ?? e).slice(0, 1500);
} finally {
  if (driving && driving.exitCode === null) spawnSync("pkill", ["-f", DRIVE_PROC]);
  summary.seconds = Math.round((Date.now() - t0) / 1000);
  fs.writeFileSync(path.join(out, "summary.json"), JSON.stringify(summary, null, 2));
  fs.rmSync(work, { recursive: true, force: true });
  console.log(JSON.stringify(summary));
  process.exit(summary.ok ? 0 : 1);
}
