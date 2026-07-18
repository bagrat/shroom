// Resolve avfoundation capture devices BY NAME, not index.
//
// avfoundation device indices are unstable (a Continuity Camera connecting shifts
// them), so production must select the video source — and any mic — by name. We
// parse `ffmpeg -list_devices`, which prints the catalogue to stderr.
//
// The video source is EITHER a screen OR a camera (camera-as-source, not PiP — PiP
// is deferred, SPEC §4). avfoundation lists both kinds in one "video devices" list;
// we tag each with a `kind` so the picker can group them.

import { spawn } from 'node:child_process';
import fs from 'node:fs';
import os from 'node:os';
import path from 'node:path';
import { qualityCatalogue } from './quality.mjs';

// A video device is a screen if avfoundation named it "Capture screen N"; anything
// else in the video list is a camera (FaceTime, Continuity, Desk View, etc.).
export function classifyVideoKind(name) {
  return /capture screen/i.test(name) ? 'screen' : 'camera';
}

// Continuity mics (iPhone/iPad) are wireless and stall-prone — they drop audio
// samples and, sharing one capture session with the screen, hitch the video too.
// Never auto-select one; the picker can still offer it explicitly.
export function isContinuityMic(name) {
  return /\b(iphone|ipad|continuity)\b/i.test(name);
}

// Choose a sane default mic: a built-in mic first, else the first non-Continuity
// device, else whatever exists. Returns a device or null.
export function pickDefaultAudio(audioDevs = []) {
  if (!audioDevs.length) return null;
  return (
    audioDevs.find((d) => /(macbook|built-?in).*microphone|microphone.*(macbook|built-?in)/i.test(d.name)) ??
    audioDevs.find((d) => !isContinuityMic(d.name)) ??
    audioDevs[0]
  );
}

// Pure parse of `ffmpeg -list_devices` stderr → tagged device catalogue. Exported
// so the parsing/selection logic is testable without spawning ffmpeg.
// Returns { video: [{index, name, kind}], audio: [{index, name}] }.
export function parseDeviceList(stderr) {
  const video = [];
  const audio = [];
  let section = null;
  for (const line of String(stderr).split('\n')) {
    if (/AVFoundation video devices:/.test(line)) { section = 'video'; continue; }
    if (/AVFoundation audio devices:/.test(line)) { section = 'audio'; continue; }
    // Lines look like: "[AVFoundation indev @ 0x..] [1] Capture screen 0"
    // The prefix bracket holds no bare [<digits>], so this only matches the index.
    const m = line.match(/\[(\d+)\]\s+(.+?)\s*$/);
    if (!section || !m) continue;
    const index = Number(m[1]);
    const name = m[2];
    if (section === 'video') video.push({ index, name, kind: classifyVideoKind(name) });
    else audio.push({ index, name });
  }
  return { video, audio };
}

// Returns { video: [{index, name, kind}], audio: [{index, name}] }.
export async function listDevices() {
  const stderr = await new Promise((resolve) => {
    const p = spawn('ffmpeg', [
      '-hide_banner',
      '-f', 'avfoundation',
      '-list_devices', 'true',
      '-i', '',
    ]);
    let buf = '';
    p.stderr.on('data', (d) => (buf += d));
    // ffmpeg exits non-zero here (no real input) — that's expected; we only want stderr.
    p.on('close', () => resolve(buf));
    p.on('error', () => resolve(buf));
  });
  return parseDeviceList(stderr);
}

function pick(devices, query) {
  return (
    devices.find((d) => d.name === query) ??
    devices.find((d) => d.name.toLowerCase().includes(query.toLowerCase()))
  );
}

// Would the saved "last settings" still capture successfully against the devices
// present RIGHT NOW? Mirrors resolveDevices' matching so the picker only offers to
// reuse a profile when reuse would actually work — a saved mic that's since been
// unplugged (e.g. AirPods) otherwise hard-aborts the whole recording at launch.
//   video: a null/absent name defaults to the first screen; a "Capture screen N"
//     name also resolves if ANY screen exists (displays renumber). A named camera
//     must still be present.
//   audio: null / "none" / "default" always resolve (a no-mic or built-in-default
//     recording); a specifically-named mic must still be connected.
// Returns { video, audio } booleans, or null when there's no profile to check.
export function lastProfileAvailability(lastProfile, { video = [], audio = [] } = {}) {
  if (!lastProfile) return null;
  const videoName = lastProfile.video;
  const videoOk =
    !videoName ||
    Boolean(pick(video, videoName)) ||
    (/capture screen/i.test(videoName) && video.some((d) => /capture screen/i.test(d.name)));
  const audioName = lastProfile.audio;
  const audioOk =
    !audioName || audioName === 'none' || audioName === 'default' || Boolean(pick(audio, audioName));
  return { video: videoOk, audio: audioOk };
}

// videoName: the chosen video source by name (screen OR camera). Default is the
//   first screen ("Capture screen 0"); a screen request also falls back to any
//   screen, but a camera request must match (no silent fallback to a screen).
// audio: "none" | "default" | a device name/substring.
//   "default" picks a built-in mic, never the iPhone/Continuity mic.
export async function resolveDevices({ videoName = 'Capture screen 0', audio = 'none' } = {}) {
  const { video, audio: audioDevs } = await listDevices();

  let chosen = pick(video, videoName);
  // Only fall back to "any screen" when the request itself looks like a screen —
  // a missing named camera should error, not silently grab the display.
  if (!chosen && /capture screen/i.test(videoName)) {
    chosen = video.find((d) => /capture screen/i.test(d.name));
  }
  if (!chosen) {
    const list = video.map((d) => `[${d.index}] ${d.name} (${d.kind})`).join(', ') || '(none)';
    throw new Error(`Video device "${videoName}" not found. Available video devices: ${list}`);
  }

  let audioIndex = 'none';
  let audioName = null;
  if (audio && audio !== 'none') {
    const dev = audio === 'default' ? pickDefaultAudio(audioDevs) : pick(audioDevs, audio);
    if (!dev) {
      const list = audioDevs.map((d) => `[${d.index}] ${d.name}`).join(', ') || '(none)';
      throw new Error(`Audio device "${audio}" not found. Available audio devices: ${list}`);
    }
    audioIndex = dev.index;
    audioName = dev.name;
  }

  return { video: chosen, audioIndex, audioName, videoDevs: video, audioDevs };
}

// The newest prior recording's settings (quality + video + mic), read from its
// session_started event — the "use last settings?" the picker offers. No separate
// profile file: events.ndjson already durably records each recording's choices.
export function readLastProfile() {
  const base = path.join(os.homedir(), '.shroom', 'recordings');
  let dirs;
  try {
    dirs = fs.readdirSync(base, { withFileTypes: true })
      .filter((d) => d.isDirectory())
      .map((d) => ({ name: d.name, mtime: fs.statSync(path.join(base, d.name)).mtimeMs }))
      .sort((a, b) => b.mtime - a.mtime);
  } catch { return null; }
  for (const d of dirs) {
    const ev = path.join(base, d.name, 'events.ndjson');
    if (!fs.existsSync(ev)) continue;
    for (const line of fs.readFileSync(ev, 'utf8').split('\n')) {
      if (!line.trim()) continue;
      let e; try { e = JSON.parse(line); } catch { continue; }
      if (e.event === 'session_started') {
        return { quality: e.config?.quality ?? null, video: e.video?.name ?? null, audio: e.audio?.name ?? null };
      }
    }
  }
  return null;
}

// The full device-picker payload the record command reads before capture: the
// video sources (tagged screen/camera), the mics (with a recommended non-Continuity
// default), the quality catalogue with size/cost estimates, and the last profile
// annotated with whether its saved devices are still connected. Pure read — no
// capture. Shared by `record.mjs --preflight` and the one-shot record preflight.
export async function buildPreflight() {
  const { video, audio } = await listDevices();
  const def = pickDefaultAudio(audio);
  const lastProfile = readLastProfile();
  const available = lastProfileAvailability(lastProfile, { video, audio });
  return {
    video,
    audio: audio.map((d) => ({ ...d, recommended: def ? d.index === def.index : false })),
    defaultAudioName: def?.name ?? null,
    qualities: qualityCatalogue(),
    lastProfile: lastProfile ? { ...lastProfile, available } : null,
  };
}
