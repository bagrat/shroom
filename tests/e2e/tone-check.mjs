// The tone capture (TESTING-PLAN §3, the POC's audio bug as a standing regression;
// DECISIONS B59 e): the dev app records one take whose sound is a continuous 997 Hz tone
// played into the BlackHole loopback and heard back through the native input path — the
// device, the recorder's sound writer, the encoder — under a still synthetic picture (no
// screen is captured or covered; no room is heard). This script reads the recording back:
//
//   - sample gaps: the tone's phase, block by block (10 ms), against the absolute sample
//     index — a sample lost or added anywhere moves it (7.5° per sample at 48 kHz); a
//     stretch of silence (padding) drops its level;
//   - the duration deficit: the sound's length against the picture's and the wall clock's
//     (the POC's avfoundation bug lost seconds an hour);
//   - nothing else on the line: each block's residual once the tone is taken out.
//
//   node tests/e2e/tone-check.mjs [seconds=1800]
//
// Then deletes the recording, whatever happened. Prints one JSON summary. Exit 0: whole and
// every bar held; 2: whole, a bar missed (`load` says how busy the Mac was); 1: it broke.

import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { f32 } from "./av.mjs";
import { ToneReader, toneTruth } from "./tone.mjs";
import { refuseOffTestMachine } from "../test-machine.mjs";

refuseOffTestMachine("tone-check.mjs");

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const app = process.env.SHROOM_APP ?? path.join(repo, "target/debug/bundle/macos/shroom-dev.app");
const seconds = Number(process.argv[2] ?? 1800);
if (!Number.isFinite(seconds) || seconds < 10 || seconds > 7200) { console.error("usage: node tests/e2e/tone-check.mjs [seconds, 10–7200]"); process.exit(64); }
const tool = (name) => ["/opt/homebrew/bin", "/usr/local/bin"].map((d) => path.join(d, name)).find((p) => fs.existsSync(p)) ?? name;
const work = fs.mkdtempSync(path.join(os.tmpdir(), "shroom-tone-check-"));
const summary = { ok: false, whole: false, seconds, load: os.loadavg().map((l) => Number(l.toFixed(2))), bars: {} };
const TONE_HZ = 997;
const LEVEL = 0.5;
// The device's start (B58 b's padding) and the take's tail are left out of the gap count.
const EDGE_S = 0.5;

const probe = (file, stream, entries) => spawnSync(tool("ffprobe"), ["-v", "error", "-allowed_extensions", "ALL", "-select_streams", stream, "-show_entries", entries, "-of", "json", file], { encoding: "utf8" });

// Reads the sound block by block as ffmpeg decodes it (a 30-minute take is ~300 MB of
// samples — never held whole).
function analyse(file) {
  return new Promise((resolve, reject) => {
    const rate = Number(JSON.parse(probe(file, "a:0", "stream=sample_rate").stdout).streams[0].sample_rate);
    const reader = new ToneReader(TONE_HZ, rate);
    const ff = spawn(tool("ffmpeg"), ["-hide_banner", "-loglevel", "error", "-allowed_extensions", "ALL", "-i", file, "-map", "0:a:0", "-ac", "1", "-f", "f32le", "-"]);
    let carry = Buffer.alloc(0), err = "";
    ff.stderr.on("data", (d) => { err += d; });
    ff.stdout.on("data", (chunk) => {
      const buf = carry.length ? Buffer.concat([carry, chunk]) : chunk;
      const n = Math.floor(buf.length / 4);
      reader.push(f32(buf.subarray(0, n * 4)));
      carry = Buffer.from(buf.subarray(n * 4));
    });
    ff.on("close", (code) => (code === 0 ? resolve({ reader, rate }) : reject(new Error(`decoding the sound: ${err.slice(0, 300)}`))));
  });
}

try {
  const report = path.join(work, "report.json");
  spawnSync("open", ["-W", "-n", app, "--args", "--record-check", report, String(seconds), "60", "tone"], { stdio: "ignore" });
  const r = JSON.parse(fs.readFileSync(report, "utf8"));
  if (r.error) throw new Error(`tone check: ${r.error}`);
  const take = path.join(work, "recording/stream_0.m3u8");
  const { reader, rate } = await analyse(take);
  const blocks = reader.blocks, samples = reader.samples;
  const frames = r.takes[0]?.frames_written ?? 0;
  const wall = r.wall_seconds?.[0] ?? null;
  const truth = toneTruth(blocks, LEVEL, { edge: Math.ceil(EDGE_S * 100) });
  const { dropouts, phaseJumps: jumps, foreign } = truth;
  const soundS = samples / rate;
  const pictureS = frames / 30;
  Object.assign(summary, {
    input: r.mic?.name, rate, loopback: r.loopback, wallSeconds: wall, soundSeconds: Number(soundS.toFixed(3)), pictureSeconds: Number(pictureS.toFixed(3)),
    writer: r.takes[0]?.audio, blocks: blocks.length, ...truth,
    level: Number((blocks.reduce((s, b) => s + b[0], 0) / Math.max(1, blocks.length)).toFixed(3)),
  });
  summary.whole = blocks.length > 2 * EDGE_S * 100 && foreign === 0 && r.loopback?.output_gaps === 0;
  const bar = (name, value, limit) => { summary.bars[name] = { value, bar: limit, ok: value !== null && value !== undefined && value <= limit }; };
  bar("sampleGaps", dropouts + jumps, 0);
  // The picture's length is the wall clock's: the frame clock writes a frame per 1/30 s of
  // wall time from the take's first frame, whatever arrived (`wallSeconds` adds the start's
  // wait for that frame and the end's).
  bar("deficitVsWallMs", Number((Math.abs(soundS - pictureS) * 1000).toFixed(1)), 40);
  summary.ok = summary.whole && Object.values(summary.bars).every((b) => b.ok);
} catch (e) {
  summary.error = String(e.message ?? e).slice(0, 500);
} finally {
  fs.rmSync(work, { recursive: true, force: true });
  console.log(JSON.stringify(summary, null, 2));
  process.exit(summary.ok ? 0 : summary.whole ? 2 : 1);
}
