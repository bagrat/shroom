// Region 3's take drives (audit b2-take) through the real app (a debug bundle) against the
// L1 fake, under a throwaway home. Never the real account, never ~/.shroom or ~/.shroom-dev.
//
//   cargo build -p shroom-cloudfake --bin cloudfake
//   (cd app && npm run build:dev — the harness is built in only on purpose)
//   node app/test/take-drive.mjs disk      the disk-full stop on a small disk image (M5)
//   node app/test/take-drive.mjs signals   bubble, exclusion, Camera, mic/permission/stall
//   node app/test/take-drive.mjs movies    Save to Movies on a take the space can't take
//   node app/test/take-drive.mjs left-out  a take from a cold start, and what it leaves out
//   node app/test/take-drive.mjs not-left-out   the reader's own proof: the count stays in
//   node app/test/take-drive.mjs camera-take    Camera as the source, a take kept (the hosted
//                                               Mac's pattern camera only: dev/mac/camera.sh)
//   node app/test/take-drive.mjs memory    what shroom holds at rest, after its windows and
//                                          after a take (DECISIONS B229; dev/mac's memory)
//
// `disk`: a 1.3 GB sparse HFS+ image (TESTING-PLAN §9 M5: detached and deleted after the
// run) mounted inside the work folder; the throwaway home lives on it, and a ballast file
// leaves the free space just above what Start needs (the margin plus a minute at Normal),
// so the take's own writes reach the floor within a minute or so. The take must end as
// `disk-full-stop` — finalized, whole, unpublished, decodable — and nothing may ever meet
// ENOSPC. `signals` needs this Mac's camera and microphone grants for shroom-dev; on a Mac with
// no camera it names the bubble and Camera as skipped and passes without them. Its camera
// take is discarded inside the drive.
//
// `left-out` (DECISIONS B114): shroom's popover and its count are made as it starts, and a
// take refuses to start when it can't find a window it leaves out. As shroom starts —
// before either has shown — both must already be found; then a take through the count to
// its end, with both lifted above the pattern and shown while it records, and every frame
// of it read back here (tests/e2e/left-out.mjs, this Mac's own ffmpeg): nothing but the
// pattern. `not-left-out` is the reader's own proof: the same drive with the count taken off
// the list (only the count: a digit and a line of copy), and the reader must find it in the
// take. Exit 3 when something couldn't be looked at: a locked screen (nothing records —
// only the first reading is made), or no ffmpeg here that can decode the take.
// SHROOM_TAKE_OUT=<dir> also leaves the take and the report there, for a reader elsewhere
// (the L5 run, tests/l5/take-vm.sh: a clean Mac has no ffmpeg of its own).
//
// The pattern covers the main display while anything records. SHROOM_APP overrides the
// bundle path.

import { execFileSync, spawn } from "node:child_process";
import { cpSync, existsSync, mkdirSync, mkdtempSync, readdirSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { createInterface } from "node:readline";
import { refuseOffTestMachine } from "../../tests/test-machine.mjs";

refuseOffTestMachine("take-drive.mjs");

const repo = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
const app = process.env.SHROOM_APP ?? join(repo, "target/debug/bundle/macos/shroom-dev.app");
const fakeBin = join(repo, "target/debug/cloudfake");
for (const p of [app, fakeBin]) if (!existsSync(p)) { console.error(`missing ${p} — build it first (see the header)`); process.exit(2); }
const which = process.argv[2];
if (!["disk", "signals", "movies", "left-out", "not-left-out", "camera-take", "camera-drag", "memory"].includes(which)) { console.error("usage: node app/test/take-drive.mjs disk|signals|movies|left-out|not-left-out|camera-take|camera-drag|memory"); process.exit(64); }

// The core's numbers (crates/shroom-core/src/derive/popover.rs): what Start needs at Normal.
const MARGIN = 1_000_000_000;
const PER_MINUTE = ((3_500 + 128) * 1000 / 8) * 60 * 2;
const ABOVE_FLOOR = 30 * 1024 * 1024;

const work = mkdtempSync(join(tmpdir(), `shroom-take-${which}-`));
let image = null;
let mount = null;
// What the app reads (statvfs: available blocks × the fragment size) — `df -k`'s Available.
const free = (p) => Number(execFileSync("df", ["-k", p], { encoding: "utf8" }).trim().split("\n").at(-1).split(/\s+/)[3]) * 1024;
let keep = false;
const cleanup = () => {
  if (mount) try { execFileSync("hdiutil", ["detach", mount, "-force"], { stdio: "ignore" }); } catch {}
  mount = null;
  if (image) rmSync(image, { force: true });
  // A failed run keeps its home (logs, takes of the test pattern) for a look; a run that
  // passed leaves nothing behind. The camera's take is discarded inside the drive either way.
  if (!keep) rmSync(work, { recursive: true, force: true });
};
process.on("exit", cleanup);

let home = join(work, "home");
if (which === "disk") {
  image = join(work, "small.sparseimage");
  mount = join(work, "mnt");
  mkdirSync(mount);
  execFileSync("hdiutil", ["create", "-size", "1300m", "-type", "SPARSE", "-fs", "HFS+", "-volname", "shroom-take-disk", image], { stdio: "ignore" });
  execFileSync("hdiutil", ["attach", image, "-mountpoint", mount, "-nobrowse", "-noautoopen"], { stdio: "ignore" });
  home = join(mount, "home");
  mkdirSync(home);
  const target = MARGIN + PER_MINUTE + ABOVE_FLOOR;
  const ballast = free(mount) - target;
  if (ballast <= 0) { console.error(`the image has only ${free(mount)} bytes free`); process.exit(1); }
  execFileSync("mkfile", [String(ballast), join(mount, "ballast")]);
  console.log(JSON.stringify({ image: "1300m HFS+", free_before_take: free(mount), target }));
}

// memory: SHROOM_MEMORY_AS_BEFORE=1 has the app behave as before DECISIONS B229, for the other half of the measure.
if (which === "memory" && process.env.SHROOM_MEMORY_AS_BEFORE === "1") {
  mkdirSync(home, { recursive: true });
  writeFileSync(join(home, "memory-as-before"), "");
}

const fake = spawn(fakeBin, ["--state", "found"], { stdio: ["pipe", "pipe", "inherit"] });
const ep = await new Promise((ok, fail) => {
  createInterface({ input: fake.stdout }).once("line", (l) => ok(JSON.parse(l)));
  fake.once("exit", () => fail(new Error("fake exited")));
});
const out = join(work, "report.json");
const run = spawn("open", ["-W", "-n", "--stderr", join(work, "stderr.log"), app, "--args", "--home", home, "--cloud-fake", ep.api, ep.r2, "--drive", "take", out, which], { stdio: "inherit" });
await new Promise((ok) => run.once("exit", ok));
fake.stdin.end();
const r = existsSync(out) ? JSON.parse(readFileSync(out, "utf8")) : { ok: false, error: "no report" };
keep = !r.ok && which === "signals";
// camera-drag: the app's own log of the take, whatever the verdict — what the recorder,
// the camera and the watchdog said.
if (which === "camera-drag") {
  const log = join(home, "logs", "run.log");
  if (existsSync(log)) console.log(readFileSync(log, "utf8").split("\n").slice(-120).join("\n"));
}
// What the pattern guard said of a slow start: how much later than its first look the
// pattern covered the display.
const said = existsSync(join(work, "stderr.log")) ? readFileSync(join(work, "stderr.log"), "utf8").split("\n").filter((l) => l.startsWith("shroom: the pattern")) : [];
// camera-take: the take's single file and its frame, handed out beside the report.
if (which === "camera-take" && process.env.SHROOM_TAKE_OUT) {
  mkdirSync(process.env.SHROOM_TAKE_OUT, { recursive: true });
  for (const name of ["camera-take.mp4", "camera-take.png"]) if (existsSync(join(work, name))) cpSync(join(work, name), join(process.env.SHROOM_TAKE_OUT, name));
}
const stays = which === "not-left-out";
if (which !== "left-out" && !stays) {
  // What the drive couldn't look at here, by name. A Mac with no camera (a hosted runner)
  // skips the bubble and Camera as the source and still passes; anything else skipped is
  // unseen (exit 3).
  const skipped = (r.facts?.skipped ?? []).map(([what, why]) => ({ what, why }));
  const unseen = skipped.filter((s) => s.why !== "no camera shroom may open on this Mac");
  console.log(JSON.stringify({ work, kept: keep, pattern: said, report: r, skipped }, null, 1));
  process.exit(!r.ok ? 1 : unseen.length ? 3 : 0);
}

// left-out: what is in the take's picture, read from every frame of it before it is deleted
// — nothing but the pattern; not-left-out: the count, which the reader must find.
const skipped = (r.facts?.skipped ?? []).map(([what, why]) => ({ what, why }));
const take = r.facts?.take_dir;
let picture = null;
if (take && existsSync(take)) {
  const handOut = process.env.SHROOM_TAKE_OUT;
  if (handOut) {
    // The picture and nothing else of the take's folder: its playlists, its init, its segments.
    mkdirSync(join(handOut, "take"), { recursive: true });
    for (const name of readdirSync(take).filter((n) => /^(stream(_\d+)?\.m3u8|init\.mp4|seg_\d+\.m4s)$/.test(n))) cpSync(join(take, name), join(handOut, "take", name));
    cpSync(out, join(handOut, "report.json"));
  }
  const reader = join(repo, "tests/e2e/left-out.mjs");
  const { canRead, pictureOf } = existsSync(reader) ? await import(reader) : { canRead: () => false };
  picture = canRead() ? pictureOf(take) : { read: false, why: handOut ? `left at ${handOut} for a reader elsewhere` : "no ffmpeg and ffprobe here to decode the take with" };
}
const unseen = skipped.length > 0 || (r.ok && picture?.read === false && !process.env.SHROOM_TAKE_OUT) || (r.ok && skipped.length === 0 && !picture);
const failed = r.checks.filter(([, ok]) => !ok).map(([what, , seen]) => ({ what, seen }));
// The picture's verdict: clean — or, for the reader's own proof, the count found in it.
const verdict = !picture || picture.read === false ? null : stays ? picture.framesUnderTheCount > 0 : picture.ok;
console.log(JSON.stringify({ ok: r.ok && !unseen && verdict !== false, failed, skipped, picture, pattern: said, report: r }, null, 1));
process.exit(!r.ok || verdict === false ? 1 : unseen ? 3 : 0);
