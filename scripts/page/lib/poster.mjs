// Poster generation: grab a representative frame from the local preview.mp4 and
// write a poster.jpg for the page's og:image (the unfurl thumbnail) and the
// <video poster>. Deterministic ffmpeg = a script's job (the determinism boundary).
//
// Best-effort: if ffmpeg is missing or there's no preview, the page still renders
// (poster falls back to a value the caller decides). Never throws into the build.

import fs from 'node:fs';
import { spawn } from 'node:child_process';

// Seek a little in so we skip a black first frame; clamp to the clip length.
function posterSeek(durationSec) {
  const d = Number(durationSec) || 0;
  if (d <= 0) return 0;
  return Math.min(2, d / 2);
}

export async function generatePoster({ previewPath, outPath, durationSec } = {}) {
  if (!previewPath || !fs.existsSync(previewPath) || fs.statSync(previewPath).size === 0) {
    return { ok: false, reason: 'no_preview' };
  }
  const seek = posterSeek(durationSec);
  const args = [
    '-hide_banner', '-loglevel', 'error', '-y',
    '-ss', String(seek), '-i', previewPath,
    '-frames:v', '1', '-q:v', '3',
    '-vf', 'scale=1280:-2',
    outPath,
  ];
  try {
    const code = await new Promise((resolve, reject) => {
      const p = spawn('ffmpeg', args, { stdio: ['ignore', 'ignore', 'ignore'] });
      p.on('close', resolve);
      p.on('error', reject);
    });
    if (code === 0 && fs.existsSync(outPath) && fs.statSync(outPath).size > 0) {
      return { ok: true, path: outPath };
    }
    return { ok: false, reason: `ffmpeg_exit_${code}` };
  } catch (e) {
    return { ok: false, reason: e.code === 'ENOENT' ? 'ffmpeg_missing' : e.message };
  }
}
