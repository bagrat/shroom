// Resolve avfoundation capture devices BY NAME, not index.
//
// avfoundation device indices are unstable (a Continuity Camera connecting shifts
// them), so production must select the screen — and any mic — by name. We parse
// `ffmpeg -list_devices`, which prints the catalogue to stderr.

import { spawn } from 'node:child_process';

// Returns { video: [{index, name}], audio: [{index, name}] }.
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

  const video = [];
  const audio = [];
  let section = null;
  for (const line of stderr.split('\n')) {
    if (/AVFoundation video devices:/.test(line)) { section = video; continue; }
    if (/AVFoundation audio devices:/.test(line)) { section = audio; continue; }
    // Lines look like: "[AVFoundation indev @ 0x..] [1] Capture screen 0"
    // The prefix bracket holds no bare [<digits>], so this only matches the device index.
    const m = line.match(/\[(\d+)\]\s+(.+?)\s*$/);
    if (section && m) section.push({ index: Number(m[1]), name: m[2] });
  }
  return { video, audio };
}

function pick(devices, query) {
  return (
    devices.find((d) => d.name === query) ??
    devices.find((d) => d.name.toLowerCase().includes(query.toLowerCase()))
  );
}

// screenName: device name (default "Capture screen 0").
// audio: "none" | "default" | a device name/substring.
export async function resolveDevices({ screenName = 'Capture screen 0', audio = 'none' } = {}) {
  const { video, audio: audioDevs } = await listDevices();

  const screen =
    pick(video, screenName) ?? video.find((d) => /capture screen/i.test(d.name));
  if (!screen) {
    const list = video.map((d) => `[${d.index}] ${d.name}`).join(', ') || '(none)';
    throw new Error(`Screen device "${screenName}" not found. Available video devices: ${list}`);
  }

  let audioIndex = 'none';
  let audioName = null;
  if (audio && audio !== 'none') {
    const dev = audio === 'default' ? audioDevs[0] : pick(audioDevs, audio);
    if (!dev) {
      const list = audioDevs.map((d) => `[${d.index}] ${d.name}`).join(', ') || '(none)';
      throw new Error(`Audio device "${audio}" not found. Available audio devices: ${list}`);
    }
    audioIndex = dev.index;
    audioName = dev.name;
  }

  return { screen, audioIndex, audioName, video, audioDevs };
}
