// The record check (TESTING-PLAN §4's media truth, without the cloud): the dev app covers the
// main display with the test pattern, records two takes with a pause through the real path,
// and reports; this script reads every frame's counter back from the joined playlist and
// holds the figures to the plan's bars — then deletes the recording, whatever happened.
//
//   node tests/e2e/record-check.mjs [seconds=6] [loopback]
//
// Uses the fresh debug bundle (target/debug/bundle/macos/shroom-dev.app; SHROOM_APP
// overrides), launched through LaunchServices under its own grants. Records only the
// pattern (no microphone). With `loopback`, the takes' sound is the BlackHole loopback
// through the native input path (CoreAudio, the recorder's sound writer) carrying the
// drives' beep on the pattern's clock — A/V through ScreenCaptureKit and a real input
// device, with no room (DECISIONS B59 e); nothing but the beep may be on the line. Bars:
// repeated and skipped frames ≤ 0.5 % each, the last take's end + finalize ≤ 200 ms,
// capture ≤ ~1 core; with the loopback, every beep within 40 ms of the picture and no
// drift (≤ 15 ms, last third against first). Prints one JSON summary. Exit 0: all held;
// 2: the recording is whole but a bar was missed (`load` is the Mac's load average — a bar
// missed under build load is re-measured idle, never loosened); 1: it broke.

import { spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { readStripGray, frameTruth, STALLS_COL } from "./strip.mjs";
import { avOffsets, avTruth, f32, onsets } from "./av.mjs";
import { refuseOffTestMachine } from "../test-machine.mjs";

refuseOffTestMachine("record-check.mjs");

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const app = process.env.SHROOM_APP ?? path.join(repo, "target/debug/bundle/macos/shroom-dev.app");
const seconds = Number(process.argv[2] ?? 6);
const loopback = process.argv.includes("loopback");
if (!Number.isFinite(seconds) || seconds < 2 || seconds > 600) { console.error("usage: node tests/e2e/record-check.mjs [seconds, 2–600]"); process.exit(64); }
const tool = (name) => ["/opt/homebrew/bin", "/usr/local/bin"].map((d) => path.join(d, name)).find((p) => fs.existsSync(p)) ?? name;
const work = fs.mkdtempSync(path.join(os.tmpdir(), "shroom-record-check-"));
const summary = { ok: false, whole: false, load: os.loadavg().map((l) => Number(l.toFixed(2))), bars: {} };

const RATE = 16_000;
const startOf = (file, stream) => Number(spawnSync(tool("ffprobe"), ["-v", "error", "-allowed_extensions", "ALL", "-select_streams", stream, "-show_entries", "stream=start_time", "-of", "csv=p=0", file], { encoding: "utf8" }).stdout.trim().split("\n")[0]) || 0;

// A take's sound, mono at RATE, and where it starts against its picture.
function sound(file) {
  const r = spawnSync(tool("ffmpeg"), ["-hide_banner", "-loglevel", "error", "-allowed_extensions", "ALL", "-i", file, "-map", "0:a:0", "-ac", "1", "-ar", String(RATE), "-f", "f32le", "-"], { maxBuffer: 1 << 30 });
  return { samples: f32(r.stdout), videoStart: startOf(file, "v:0"), audioStart: startOf(file, "a:0") };
}

// Sound on the line that isn't the beep: any 50 ms outside a beep (and its 150 ms after
// the onset) above -40 dBFS.
function foreign(samples) {
  const beeps = onsets(samples, RATE).map((i) => [i, i + 0.15 * RATE]);
  const win = RATE / 20;
  let n = 0;
  for (let at = RATE / 2; at + win <= samples.length - RATE / 2; at += win) {
    if (beeps.some(([a, b]) => at + win > a && at < b)) continue;
    let e = 0;
    for (let i = at; i < at + win; i++) e += samples[i] * samples[i];
    if (Math.sqrt(e / win) > 0.01) n++;
  }
  return n;
}

function counters(playlist) {
  const size = spawnSync(tool("ffprobe"), ["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height", "-of", "csv=p=0:s=x", playlist], { encoding: "utf8" });
  const [w0, h0] = (size.stdout ?? "").trim().split("\n")[0].split("x").map(Number);
  const h = 270, w = 2 * Math.round((h * w0) / h0 / 2);
  // Per take: ffmpeg's HLS reader doesn't re-base at a discontinuity, the players do.
  const out = [];
  const stalls = [];
  const files = [];
  for (const take of fs.readdirSync(path.dirname(playlist)).filter((n) => /^stream_\d+\.m3u8$/.test(n)).sort()) {
    const file = path.join(path.dirname(playlist), take);
    const r = spawnSync(tool("ffmpeg"), ["-hide_banner", "-loglevel", "error", "-allowed_extensions", "ALL", "-i", file, "-map", "0:v:0", "-fps_mode", "passthrough", "-vf", `scale=${w}:${h}:flags=area,format=gray`, "-f", "rawvideo", "-"], { maxBuffer: 1 << 30 });
    const c = [], st = [];
    for (let at = 0; at + w * h <= r.stdout.length; at += w * h) {
      const frame = r.stdout.subarray(at, at + w * h);
      c.push(readStripGray(frame, w, h));
      st.push(readStripGray(frame, w, h, STALLS_COL));
    }
    out.push(c);
    stalls.push(st);
    files.push(file);
  }
  return { takes: out, stalls, files, size: [w0, h0] };
}

try {
  const report = path.join(work, "report.json");
  spawnSync("open", ["-W", "-n", app, "--args", "--record-check", report, String(seconds), "60", ...(loopback ? ["loopback"] : [])], { stdio: "ignore" });
  const r = JSON.parse(fs.readFileSync(report, "utf8"));
  if (r.error) throw new Error(`record check: ${r.error}`);
  const playlist = path.join(work, "recording/stream.m3u8");
  const { takes, stalls, files, size } = counters(playlist);
  const truths = takes.map((c, i) => frameTruth(c, 60, 30, stalls[i]));
  const all = { frames: 0, repeated: 0, skipped: 0, unreadable: 0, backwards: 0, repeatedByCapture: 0, skippedByCapture: 0, patternHolds: 0 };
  for (const t of truths) for (const k of Object.keys(all)) all[k] += t[k];
  const pct = (n) => Number(((n / Math.max(all.frames, 1)) * 100).toFixed(3));
  Object.assign(summary, {
    size, takes: truths, finalizeMs: r.finalize_ms, captureCores: r.capture_cores, cpuSeconds: r.cpu_seconds,
    stats: r.takes.map((t) => ({ received: t.frames_received, written: t.frames_written, repeats: t.repeats, stale: t.stale_ticks, late: t.late_ticks, overflow: t.backlog_overflow, backlogPeak: t.backlog_peak, firstFrameMs: t.first_frame_ms, audio: t.audio })),
  });
  let avOk = true;
  if (loopback) {
    const per = files.map((file, i) => {
      const { samples, videoStart, audioStart } = sound(file);
      const offsets = avOffsets(takes[i], 30, samples, RATE, { videoStart, audioStart });
      return { offsets, foreign: foreign(samples), videoStart, audioStart };
    });
    const all = per.flatMap((p) => p.offsets);
    summary.av = { loopback: r.loopback, input: r.mic?.name, ...avTruth(all), perTake: per.map((p) => ({ ...avTruth(p.offsets), foreign: p.foreign, starts: [p.videoStart, p.audioStart], offsetsMs: p.offsets.map((o) => o.offsetMs) })) };
    avOk = r.loopback?.output_gaps === 0 && per.every((p) => p.foreign === 0 && p.offsets.length >= Math.floor(seconds / 2) - 2);
    summary.av.whole = avOk;
  }
  summary.whole = truths.length === 2 && all.frames > 0 && all.unreadable === 0 && all.backwards === 0 && avOk;
  const bar = (name, value, limit) => { summary.bars[name] = { value, bar: limit, ok: value !== null && value !== undefined && value <= limit }; };
  // The bar is capture's: a hold the pattern itself made (its own missed refreshes, drawn
  // beside its counter) was on the screen twice and is kept faithfully (DECISIONS B59 f).
  summary.raw = { repeatedPct: pct(all.repeated), skippedPct: pct(all.skipped), patternHolds: all.patternHolds };
  bar("repeatedFramesPct", pct(all.repeatedByCapture), 0.5);
  bar("skippedFramesPct", pct(all.skippedByCapture), 0.5);
  bar("finalizeMs", r.finalize_ms, 200);
  bar("captureCores", r.capture_cores === undefined || r.capture_cores === null ? null : Number(r.capture_cores.toFixed(3)), 1.0);
  if (loopback) {
    bar("avOffsetMaxMs", summary.av.maxAbsMs, 40);
    // Drift needs a take long enough for thirds to mean something: each take ≥ 30 s.
    if (seconds / 2 >= 30) for (const [i, p] of summary.av.perTake.entries()) bar(`avDriftMs take ${i}`, p.driftMs, 15);
  }
  summary.ok = summary.whole && Object.values(summary.bars).every((b) => b.ok);
} catch (e) {
  summary.error = String(e.message ?? e).slice(0, 500);
} finally {
  // The recording is the pattern, but it's still a recording of this display: never kept.
  fs.rmSync(work, { recursive: true, force: true });
  console.log(JSON.stringify(summary, null, 2));
  process.exit(summary.ok ? 0 : summary.whole ? 2 : 1);
}
