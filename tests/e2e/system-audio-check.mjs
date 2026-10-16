// The system-audio check (spec N47, DECISIONS B172): the dev app covers the display with the
// test pattern and records two takes through the real path with the drives' beep as the
// voice and the Mac's own sound mixed in. During take 0 another process plays a 440 Hz burst
// half a second after every beep; during take 1 nothing else plays. All along, the app
// itself plays a 2 kHz tone out loud. Then this reads the takes back:
//
//   - take 0 carries both the voice (1 kHz) and the Mac's sound (440 Hz), each against the
//     picture within the heartbeat's 40 ms;
//   - shroom's own tone is in neither take;
//   - take 1 is exactly what a voice-only take was: every beep within 40 ms, and nothing
//     else on the line;
//   - the other app's steady tone never drops out in take 0 (DECISIONS B174: the mix used to
//     cut the Mac's sound at the voice's block edges).
//
//   node tests/e2e/system-audio-check.mjs [seconds=12]
//
// Uses the fresh debug bundle (SHROOM_APP overrides), launched through LaunchServices. It
// plays sound out loud: a test Mac only. Prints one JSON summary. Exit 0: all held; 2: both
// sounds are there but an offset bar was missed (a runner's three cores; a Mac decides);
// 1: it broke (a sound missing, shroom's own sound heard, the voice changed).

import { spawn, spawnSync } from "node:child_process";
import fs from "node:fs";
import os from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { readStripGray } from "./strip.mjs";
import { avOffsets, avTruth, f32 } from "./av.mjs";
import { refuseOffTestMachine } from "../test-machine.mjs";

refuseOffTestMachine("system-audio-check.mjs");

const repo = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "../..");
const app = process.env.SHROOM_APP ?? path.join(repo, "target/debug/bundle/macos/shroom-dev.app");
const seconds = Number(process.argv[2] ?? 12);
if (!Number.isFinite(seconds) || seconds < 6 || seconds > 600) { console.error("usage: node tests/e2e/system-audio-check.mjs [seconds, 6–600]"); process.exit(64); }
const tool = (name) => ["/opt/homebrew/bin", "/usr/local/bin"].map((d) => path.join(d, name)).find((p) => fs.existsSync(p)) ?? name;
const work = fs.mkdtempSync(path.join(os.tmpdir(), "shroom-system-audio-"));
const summary = { ok: false, whole: false, load: os.loadavg().map((l) => Number(l.toFixed(2))), bars: {} };

const RATE = 16_000;
const VOICE_HZ = 1_000, MAC_HZ = 440, MAC_AT_S = 0.5, SELF_HZ = 2_000, OTHER_APP_HZ = 660;
const WIN = 160, HOP = 40; // 10 ms windows every 2.5 ms
const startOf = (file, stream) => Number(spawnSync(tool("ffprobe"), ["-v", "error", "-allowed_extensions", "ALL", "-select_streams", stream, "-show_entries", "stream=start_time", "-of", "csv=p=0", file], { encoding: "utf8" }).stdout.trim().split("\n")[0]) || 0;

function sound(file) {
  const r = spawnSync(tool("ffmpeg"), ["-hide_banner", "-loglevel", "error", "-allowed_extensions", "ALL", "-i", file, "-map", "0:a:0", "-ac", "1", "-ar", String(RATE), "-f", "f32le", "-"], { maxBuffer: 1 << 30 });
  return { samples: f32(r.stdout), videoStart: startOf(file, "v:0"), audioStart: startOf(file, "a:0") };
}

const HANN = Float32Array.from({ length: WIN }, (_, i) => 0.5 - 0.5 * Math.cos((2 * Math.PI * i) / (WIN - 1)));

/** A tone's amplitude through the take, one figure per hop (Goertzel over each Hann window,
 *  so a burst's edges don't leak into the other tones' bands). */
function band(samples, hz) {
  const k = 2 * Math.cos((2 * Math.PI * hz) / RATE);
  const out = new Float32Array(Math.max(0, Math.floor((samples.length - WIN) / HOP)));
  for (let w = 0; w < out.length; w++) {
    let s1 = 0, s2 = 0;
    for (let i = 0; i < WIN; i++) { const s0 = samples[w * HOP + i] * HANN[i] + k * s1 - s2; s2 = s1; s1 = s0; }
    out[w] = (4 * Math.sqrt(Math.max(0, s1 * s1 + s2 * s2 - k * s1 * s2))) / WIN;
  }
  return out;
}

/** Windows in the steady middle of a tone (its first and last half second left out) where
 *  it falls below half its median: a dropout, or a splice the window straddles. */
function dropouts(env) {
  const edge = Math.round((0.5 * RATE) / HOP), level = median(env) / 2;
  let n = 0;
  for (let w = edge; w < env.length - edge; w++) if (env[w] < level) n++;
  return n;
}

const median = (env) => { const a = Array.from(env).sort((x, y) => x - y); return a.length ? a[Math.floor(a.length / 2)] : 0; };

/** The band as a clean on/off signal at RATE, for `avOffsets`: on where it's above `level`. */
function gate(env, length, level) {
  const out = new Float32Array(length);
  env.forEach((a, w) => { if (a > level) out.fill(0.5, w * HOP + WIN / 2 - HOP / 2, w * HOP + WIN / 2 + HOP / 2); });
  return out;
}

const peak = (env) => env.reduce((m, a) => Math.max(m, a), 0);

function counters(file) {
  const size = spawnSync(tool("ffprobe"), ["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height", "-of", "csv=p=0:s=x", file], { encoding: "utf8" });
  const [w0, h0] = (size.stdout ?? "").trim().split("\n")[0].split("x").map(Number);
  const h = 270, w = 2 * Math.round((h * w0) / h0 / 2);
  const r = spawnSync(tool("ffmpeg"), ["-hide_banner", "-loglevel", "error", "-allowed_extensions", "ALL", "-i", file, "-map", "0:v:0", "-fps_mode", "passthrough", "-vf", `scale=${w}:${h}:flags=area,format=gray`, "-f", "rawvideo", "-"], { maxBuffer: 1 << 30 });
  const c = [];
  for (let at = 0; at + w * h <= r.stdout.length; at += w * h) c.push(readStripGray(r.stdout.subarray(at, at + w * h), w, h));
  return c;
}

/** A mono 16-bit WAV of a steady sine: Apple's player's tone. */
function toneWav(file, hz, secs) {
  const rate = 48_000, n = Math.round(secs * rate);
  const b = Buffer.alloc(44 + n * 2);
  b.write("RIFF", 0); b.writeUInt32LE(36 + n * 2, 4); b.write("WAVEfmt ", 8); b.writeUInt32LE(16, 16);
  b.writeUInt16LE(1, 20); b.writeUInt16LE(1, 22); b.writeUInt32LE(rate, 24); b.writeUInt32LE(rate * 2, 28);
  b.writeUInt16LE(2, 32); b.writeUInt16LE(16, 34); b.write("data", 36); b.writeUInt32LE(n * 2, 40);
  for (let i = 0; i < n; i++) b.writeInt16LE(Math.round(0.3 * Math.sin((2 * Math.PI * hz * i) / rate) * 32_767), 44 + i * 2);
  fs.writeFileSync(file, b);
}

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

try {
  const report = path.join(work, "report.json");
  // The other apps, started here and never by shroom (a child of shroom's is shroom's own
  // sound, which ScreenCaptureKit leaves out): the dev app's player playing the burst, and
  // Apple's afplay playing a steady tone. Both stop when take 0 ends; take 1 is quiet.
  const exe = path.join(app, "Contents/MacOS", fs.readdirSync(path.join(app, "Contents/MacOS"))[0]);
  const wav = path.join(work, "other-app.wav");
  toneWav(wav, OTHER_APP_HZ, 120);
  const players = [spawn(exe, ["--play-out", String(MAC_HZ), String(MAC_AT_S * 1000), "120"], { stdio: ["ignore", "ignore", "pipe"] }), spawn("/usr/bin/afplay", [wav], { stdio: "ignore" })];
  // The burst player's output latency: its bursts leave the speaker this long after their
  // wall-clock instants, so that much of the Mac's offset is the player's, not shroom's.
  let playerSaid = "";
  players[0].stderr.on("data", (d) => { playerSaid += d; });
  const stopPlayers = () => { for (const p of players) if (p.exitCode === null) p.kill(); };
  // The wall clock against the steady one over the takes: a fresh runner's is still being set.
  const clock0 = [Date.now(), process.hrtime.bigint()];
  const check = spawn("open", ["-W", "-n", app, "--args", "--record-check", report, String(seconds), "60", "system"], { stdio: "ignore" });
  const done = new Promise((r) => check.on("exit", r));
  let ended = false;
  check.on("exit", () => { ended = true; });
  const marker = report.replace(/\.json$/, ".take0-ended");
  while (!ended && !fs.existsSync(marker)) await sleep(50);
  summary.playersRunningAtTake0End = players.map((p) => p.exitCode === null);
  stopPlayers();
  await sleep(300);
  fs.writeFileSync(report.replace(/\.json$/, ".quiet"), "");
  await done;
  stopPlayers();
  const steadyMs = Number(process.hrtime.bigint() - clock0[1]) / 1e6;
  summary.wallClockPpm = Math.round(((Date.now() - clock0[0] - steadyMs) / steadyMs) * 1e6);
  const r = JSON.parse(fs.readFileSync(report, "utf8"));
  summary.system = r.system;
  if (r.error) throw new Error(`record check: ${r.error}`);
  const dir = path.join(work, "recording");
  const files = fs.readdirSync(dir).filter((n) => /^stream_\d+\.m3u8$/.test(n)).sort().map((n) => path.join(dir, n));
  if (files.length !== 2) throw new Error(`expected two takes, found ${files.length}`);
  const expected = Math.floor(seconds / 2) - 3;

  const takes = files.map((file, i) => {
    const { samples, videoStart, audioStart } = sound(file);
    // The take's sound alone is only test tones (no room, no screen): kept for a look by ear.
    if (process.env.SHROOM_SYSTEM_AUDIO_OUT) {
      spawnSync(tool("ffmpeg"), ["-hide_banner", "-loglevel", "error", "-y", "-allowed_extensions", "ALL", "-i", file, "-map", "0:a:0", path.join(process.env.SHROOM_SYSTEM_AUDIO_OUT, `take-${i}.wav`)]);
    }
    const pic = counters(file);
    const voice = band(samples, VOICE_HZ), mac = band(samples, MAC_HZ), self = band(samples, SELF_HZ), other = band(samples, OTHER_APP_HZ);
    // shroom's tone plays all along: if it got in, it's in most of the take (its median).
    const voicePeak = peak(voice), macPeak = peak(mac), selfLevel = median(self);
    const voiceAv = avOffsets(pic, 30, gate(voice, samples.length, voicePeak / 3), RATE, { videoStart, audioStart });
    const playerLatency = Number(/device_latency_ms=([\d.]+)/.exec(playerSaid)?.[1] ?? 0) / 1000;
    summary.playerLatencyMs = playerLatency * 1000;
    const macAv = macPeak > 0.02 ? avOffsets(pic, 30, gate(mac, samples.length, macPeak / 3), RATE, { videoStart, audioStart: audioStart - MAC_AT_S - playerLatency }) : [];
    // Anything on the line that is neither the beep nor the Mac's burst, in 50 ms steps.
    let foreign = 0;
    // A window near a tone's edge (±15 ms) counts as that tone.
    const loud = (env, w) => { for (let k = Math.max(0, w - 6); k <= Math.min(env.length - 1, w + 6); k++) if (env[k] > 0.02) return true; return false; };
    for (let w = 0; w < voice.length; w += 20) {
      let e = 0;
      for (let j = w * HOP; j < w * HOP + WIN && j < samples.length; j++) e += samples[j] * samples[j];
      if (Math.sqrt(e / WIN) > 0.01 && !loud(voice, w) && !loud(mac, w) && !(i === 0 && loud(other, w))) foreign++;
    }
    // A 440 Hz hit at −500 ms is the beep leaking into the band, not the Mac's burst: only
    // hits near the burst's own slot count.
    const macNear = macAv.filter((o) => Math.abs(o.offsetMs) < 200);
    return {
      take: i,
      macHits: macNear.length,
      voice: { peak: Number(voicePeak.toFixed(3)), ...avTruth(voiceAv), offsetsMs: voiceAv.map((o) => o.offsetMs) },
      mac: { peak: Number(macPeak.toFixed(3)), ...avTruth(macNear), offsetsMs: macAv.map((o) => o.offsetMs) },
      selfLevel: Number(selfLevel.toFixed(4)),
      // Apple's player's steady tone (take 0): another app's sound, by its median level.
      otherAppLevel: Number(median(other).toFixed(4)),
      otherAppDropouts: dropouts(other),
      foreign,
      starts: [videoStart, audioStart],
    };
  });
  summary.takes = takes;
  const [t0, t1] = takes;
  const checks = {
    // Take 0: both sounds, each heard about once a second.
    // The Mac's sound reached the take at all (else "shroom's tone is absent" proves nothing).
    macArrived: (r.system?.per_take?.[0]?.samples ?? 0) > 0,
    voiceInTake0: t0.voice.beeps >= expected,
    macInTake0: t0.macHits >= expected,
    otherAppInTake0: t0.otherAppLevel > 0.02,
    // ...and steady: never cut, never silenced, for the whole take.
    otherAppSteady: t0.otherAppDropouts === 0,
    // shroom's own tone: never, at a twentieth of the Mac's burst level or more.
    selfAbsent: takes.every((t) => t.selfLevel < Math.max(0.005, t0.mac.peak / 20)),
    // Take 1: the voice as it ever was, and nothing of the Mac.
    voiceInTake1: t1.voice.beeps >= expected,
    take1HasOnlyTheVoice: t1.macHits === 0 && t1.foreign === 0,
  };
  summary.checks = checks;
  summary.whole = Object.values(checks).every(Boolean);
  const bar = (name, value, limit) => { summary.bars[name] = { value, bar: limit, ok: value !== null && value !== undefined && value <= limit }; };
  bar("voiceAvMaxMs take 0", t0.voice.maxAbsMs, 40);
  bar("macAvMaxMs take 0", t0.mac.maxAbsMs, 40);
  bar("voiceAvMaxMs take 1", t1.voice.maxAbsMs, 40);
  summary.ok = summary.whole && Object.values(summary.bars).every((b) => b.ok);
} catch (e) {
  summary.error = String(e.message ?? e).slice(0, 500);
} finally {
  // The recording is the pattern and test tones, but it's still this display: never kept.
  fs.rmSync(work, { recursive: true, force: true });
  console.log(JSON.stringify(summary, null, 2));
  process.exit(summary.ok ? 0 : summary.whole ? 2 : 1);
}
