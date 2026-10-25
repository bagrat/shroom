// The editor's drive (N63) through the real app against the L1 fake, under a throwaway home.
// Run from app/: node test/editor-drive.mjs (dev/mac/scenarios/editor.sh runs it on the
// hosted Mac). Needs the debug bundle (npm run build:dev in app/) and the fake
// (cargo build -p shroom-cloudfake --bin cloudfake). Never touches the installed app.
//
// It makes the drive's test video with this Mac's ffmpeg — 40 s of a test pattern with its
// running clock burned in and a speech-like sound with pauses, faststart, and its poster —
// starts the fake and the app (`--drive editor <out> <dir>`), and answers what the drive
// asks in <dir> that only this process may do (the app never drives the pointer or takes
// pictures of the screen itself): `ask-<n>.json` → `done-<n>`,
//
//   { "do": "shot", "window": <id>, "name": "editor-open" }   screencapture -l <id> <name>.png
//   { "do": "drag", "focus": [x, y], "from": [x, y], "to": [x, y] }   cliclick: a click, a drag
//   { "do": "click", "at": [x, y] }                                    cliclick: a click
//   { "do": "move", "at": [x, y] }                                     cliclick: the pointer there
//   { "do": "sweep", "press": bool, "from": [x, y], "to": [x, y] }      cliclick: the pointer along, in 2 px steps (held down)
//   { "do": "same", "a": <name>, "b": <name> }                         ffmpeg: the two shots' pixels, the same? (else <a>-diff.png)
//
// Points are from the main display's top-left. The screenshots, the copy the drive read
// back from the bucket and the report land in $MAC_OUT when the scenario sets it. A drive
// that passes leaves nothing in the temp folder; one that fails keeps its throwaway home —
// the run log is in it — and says where.
import { execFileSync, spawn, spawnSync } from "node:child_process";
import { copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, renameSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { createInterface } from "node:readline";
import { refuseOffTestMachine } from "../../tests/test-machine.mjs";

refuseOffTestMachine("editor-drive.mjs");

const repo = new URL("../..", import.meta.url).pathname.replace(/\/$/, "");
const app = process.env.MAC_APP ?? join(repo, "target/debug/bundle/macos/shroom-dev.app");
const fakeBin = join(repo, "target/debug/cloudfake");
const work = mkdtempSync(join(tmpdir(), "shroom-editor-"));
const dir = join(work, "drive");
mkdirSync(dir);

// The test video: what a take of this Mac's preview.mp4 is, made here.
const ffmpeg = process.env.SHROOM_FFMPEG ?? ["/opt/homebrew/bin/ffmpeg", "/usr/local/bin/ffmpeg"].find((p) => existsSync(p)) ?? "ffmpeg";
const SECONDS = 40;
const sound =
  "aevalsrc='0.8*sin(2*PI*(170+50*sin(2*PI*0.7*t))*t)*abs(sin(2*PI*2.3*t))*(0.2+0.8*abs(sin(2*PI*0.47*t+2*sin(2*PI*0.11*t))))" +
  `*gt(sin(2*PI*0.21*t+1)\\,-0.4)*not(between(t\\,9\\,11))':s=48000:d=${SECONDS}`;
execFileSync(ffmpeg, ["-v", "error", "-y", "-f", "lavfi", "-i", `testsrc2=size=1280x720:rate=30:duration=${SECONDS}`, "-f", "lavfi", "-i", sound,
  "-c:v", "libx264", "-preset", "veryfast", "-crf", "28", "-pix_fmt", "yuv420p", "-g", "30", "-c:a", "aac", "-b:a", "96k", "-movflags", "+faststart", "-shortest", join(dir, "preview.mp4")]);
execFileSync(ffmpeg, ["-v", "error", "-y", "-i", join(dir, "preview.mp4"), "-frames:v", "1", "-vf", "scale=1200:-1", "-q:v", "3", join(dir, "poster.jpg")]);

const fake = spawn(fakeBin, ["--state", "found"], { stdio: ["pipe", "pipe", "inherit"] });
const ep = await new Promise((ok, fail) => {
  createInterface({ input: fake.stdout }).once("line", (l) => ok(JSON.parse(l)));
  fake.once("exit", () => fail(new Error("fake exited")));
});
const out = join(work, "report.json");

// The drive's asks, one at a time, in order.
const px = (v) => String(Math.round(v));
const at = ([x, y]) => `${px(x)},${px(y)}`;
const cliclick = (...cmds) => {
  const r = spawnSync("cliclick", ["-w", "60", ...cmds], { encoding: "utf8" });
  if (r.status !== 0) throw new Error(`cliclick ${cmds.join(" ")}: ${r.error ?? r.stderr}`);
};
// A drag's moves are drags (the button held: `dm:`), never plain moves — a page that saw
// plain moves never sees the button come up, and keeps the pointer. An older cliclick
// without `dm:` gets plain moves.
const dragMoves = (points) => {
  try {
    cliclick(...points.map((p) => `dm:${at(p)}`));
    return true;
  } catch {
    return false;
  }
};
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
async function answer(ask) {
  if (ask.do === "shot") {
    if (!/^[a-z0-9-]+$/.test(ask.name) || !Number.isInteger(ask.window)) throw new Error("not a shot this runner takes");
    const file = join(dir, `${ask.name}.png`);
    spawnSync("screencapture", ["-x", "-o", `-l${ask.window}`, file]);
    if (!existsSync(file)) throw new Error("screencapture wrote nothing");
    return;
  }
  if (ask.do === "drag") {
    cliclick(`c:${at(ask.focus)}`);
    await sleep(400);
    const [[x1, y1], [x2, y2]] = [ask.from, ask.to];
    const points = [1, 2, 3, 4, 5, 6].map((i) => [x1 + ((x2 - x1) * i) / 6, y1 + ((y2 - y1) * i) / 6]);
    cliclick(`m:${at(ask.from)}`, `dd:${at(ask.from)}`);
    if (!dragMoves(points)) cliclick(...points.map((p) => `m:${at(p)}`));
    cliclick(`du:${at(ask.to)}`);
    return;
  }
  if (ask.do === "click") {
    cliclick(`c:${at(ask.at)}`);
    return;
  }
  if (ask.do === "move") {
    cliclick(`m:${at([ask.at[0] - 3, ask.at[1]])}`, `m:${at(ask.at)}`);
    return;
  }
  if (ask.do === "sweep") {
    // Slow, as a hand goes: every 2 px, each move its own event.
    const [[x1, y1], [x2, y2]] = [ask.from, ask.to];
    const n = Math.max(1, Math.round(Math.abs(x2 - x1) / 2));
    const points = Array.from({ length: n }, (_, i) => [x1 + ((x2 - x1) * (i + 1)) / n, y1 + ((y2 - y1) * (i + 1)) / n]);
    cliclick(`m:${at(ask.from)}`);
    if (!ask.press) {
      cliclick("-w", "8", ...points.map((p) => `m:${at(p)}`));
      return;
    }
    cliclick(`dd:${at(ask.from)}`);
    if (!dragMoves(points)) cliclick(...points.map((p) => `m:${at(p)}`));
    cliclick(`du:${at(ask.to)}`);
    return;
  }
  if (ask.do === "same") {
    if (![ask.a, ask.b].every((n) => /^[a-z0-9-]+$/.test(n))) throw new Error("not shots this runner compares");
    const [a, b] = [ask.a, ask.b].map((n) => join(dir, `${n}.png`));
    const md5 = (f) => execFileSync(ffmpeg, ["-v", "error", "-i", f, "-f", "md5", "-"], { encoding: "utf8" }).trim();
    if (md5(a) === md5(b)) return;
    // Where they differ, made bright, beside them.
    spawnSync(ffmpeg, ["-v", "error", "-y", "-i", a, "-i", b, "-lavfi", "[0][1]blend=all_mode=difference,lutrgb=r=min(255\\,val*16):g=min(255\\,val*16):b=min(255\\,val*16)", "-frames:v", "1", join(dir, `${ask.a}-diff.png`)]);
    throw new Error(`${ask.a} and ${ask.b} differ (${ask.a}-diff.png)`);
  }
  throw new Error(`unknown ask ${JSON.stringify(ask)}`);
}
let next = 1;
let busy = false;
const asks = setInterval(async () => {
  if (busy) return;
  const file = join(dir, `ask-${next}.json`);
  if (!existsSync(file)) return;
  busy = true;
  let said = "ok";
  try {
    await answer(JSON.parse(readFileSync(file, "utf8")));
  } catch (e) {
    said = String(e.message ?? e).slice(0, 300);
  }
  // Written whole, then put in place: the drive reads it the moment it's there.
  writeFileSync(join(dir, `done-${next}.tmp`), said);
  renameSync(join(dir, `done-${next}.tmp`), join(dir, `done-${next}`));
  next += 1;
  busy = false;
}, 150);

const run = spawn("open", ["-W", "-n", app, "--args", "--home", join(work, "home"), "--cloud-fake", ep.api, ep.r2, "--drive", "editor", out, dir], { stdio: "inherit" });
await new Promise((ok) => run.once("exit", ok));
clearInterval(asks);
fake.stdin.end();

// What it saw, beside the scenario's recording.
const keep = process.env.MAC_OUT;
if (keep) {
  mkdirSync(keep, { recursive: true });
  for (const f of readdirSync(dir).filter((f) => f.endsWith(".png") || f === "edit-1.mp4" || f === "preview.mp4")) copyFileSync(join(dir, f), join(keep, f));
  if (existsSync(out)) copyFileSync(out, join(keep, "report.json"));
}
let r;
try {
  if (!existsSync(out)) throw new Error("the drive wrote no report");
  r = JSON.parse(readFileSync(out, "utf8"));
} catch (e) {
  console.error(`${e.message}\nkept for a look: ${work}`);
  process.exit(1);
}
const failed = r.ok === false;
for (const [what, ok, seen] of r.checks ?? []) console.log(`${ok ? "pass" : "FAIL"} ${what}${ok ? "" : ` — ${seen}`}`);
if (r.error) console.log(`error: ${r.error}`);
console.log(JSON.stringify({ ...(failed ? { work } : {}), report: r }, null, 1));
if (failed) console.error(`kept for a look: ${work}`);
else rmSync(work, { recursive: true, force: true });
console.log(`editor drive: ${(r.checks ?? []).filter((c) => c[1]).length}/${(r.checks ?? []).length} checks passed${r.error ? `, then: ${r.error}` : ""}`);
process.exit(failed ? 1 : 0);
