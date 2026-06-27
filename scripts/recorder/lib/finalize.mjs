// Finalize a session: assemble the master HLS playlist and the preview MP4 from
// the per-take artifacts, then summarize. Per SPEC §5 the master playlist is
// generated only at finalize (its upload is the "go live" act) — so we build it
// ourselves from each take's playlist rather than shipping ffmpeg's live one.

import fs from 'node:fs';
import path from 'node:path';
import { spawn } from 'node:child_process';
import { CONFIG, takePlaylist, takePreview } from './config.mjs';

// Pull [{extinf, file}] entries out of one take's HLS playlist (in order).
function readTakeEntries(dir, k) {
  const p = path.join(dir, takePlaylist(k));
  if (!fs.existsSync(p)) return [];
  const entries = [];
  let dur = 0;
  for (const line of fs.readFileSync(p, 'utf8').split('\n')) {
    const m = line.match(/^#EXTINF:([\d.]+)/);
    if (m) { dur = parseFloat(m[1]); continue; }
    if (CONFIG.files.segmentGlob.test(line.trim())) {
      entries.push({ extinf: dur, file: line.trim() });
    }
  }
  return entries;
}

// Master VOD playlist: one shared EXT-X-MAP, an EXT-X-DISCONTINUITY before each
// take after the first (a pause = a timestamp discontinuity), one ENDLIST.
export function assemblePlaylist(dir, takeIndices) {
  const f = CONFIG.files;
  const perTake = takeIndices.map((k) => readTakeEntries(dir, k));
  const all = perTake.flat();
  const target = Math.max(1, Math.ceil(all.reduce((mx, e) => Math.max(mx, e.extinf), 0)));

  const out = [
    '#EXTM3U',
    '#EXT-X-VERSION:7',
    `#EXT-X-TARGETDURATION:${target}`,
    '#EXT-X-MEDIA-SEQUENCE:0',
    '#EXT-X-PLAYLIST-TYPE:VOD',
    `#EXT-X-MAP:URI="${f.initSegment}"`,
  ];
  perTake.forEach((entries, i) => {
    if (i > 0 && entries.length) out.push('#EXT-X-DISCONTINUITY');
    for (const e of entries) {
      out.push(`#EXTINF:${e.extinf.toFixed(6)},`, e.file);
    }
  });
  out.push('#EXT-X-ENDLIST', '');
  fs.writeFileSync(path.join(dir, f.playlist), out.join('\n'));

  return {
    durationSec: Math.round(all.reduce((s, e) => s + e.extinf, 0) * 100) / 100,
    segmentCount: all.length,
    segments: all.map((e) => e.file),
  };
}

// Final preview.mp4. One take → copy it (preserve faststart bytes exactly).
// Multiple takes → concat-copy via the concat demuxer (re-timestamps the joins).
export async function assemblePreview(dir, takeIndices) {
  const f = CONFIG.files;
  const present = takeIndices.filter((k) => {
    const p = path.join(dir, takePreview(k));
    return fs.existsSync(p) && fs.statSync(p).size > 0;
  });
  if (present.length === 0) return null;

  const finalPath = path.join(dir, f.preview);
  if (present.length === 1) {
    fs.copyFileSync(path.join(dir, takePreview(present[0])), finalPath);
    return f.preview;
  }

  const listFile = 'preview_concat.txt';
  fs.writeFileSync(
    path.join(dir, listFile),
    present.map((k) => `file '${takePreview(k)}'`).join('\n') + '\n',
  );
  await new Promise((resolve, reject) => {
    const p = spawn('ffmpeg', [
      '-hide_banner', '-y',
      '-f', 'concat', '-safe', '0', '-i', listFile,
      '-c', 'copy', '-movflags', '+faststart', f.preview,
    ], { cwd: dir, stdio: ['ignore', 'ignore', 'ignore'] });
    p.on('close', (code) => (code === 0 ? resolve() : reject(new Error(`preview concat exited ${code}`))));
    p.on('error', reject);
  });
  try { fs.unlinkSync(path.join(dir, listFile)); } catch {}
  return f.preview;
}

export async function finalizeSession(dir, takeIndices) {
  const f = CONFIG.files;
  const playlist = assemblePlaylist(dir, takeIndices);

  let preview = null;
  let previewError = null;
  try {
    preview = await assemblePreview(dir, takeIndices);
  } catch (e) {
    previewError = e.message;
  }

  const initSegment = fs.existsSync(path.join(dir, f.initSegment)) ? f.initSegment : null;
  const summary = {
    dir,
    playlist: f.playlist,
    preview,
    initSegment,
    segments: playlist.segments,
    segmentCount: playlist.segmentCount,
    durationSec: playlist.durationSec,
    takeCount: takeIndices.length,
    endlist: true,
    ok: Boolean(preview) && Boolean(initSegment) && playlist.segmentCount > 0,
  };
  if (previewError) summary.previewError = previewError;
  return summary;
}

// Highest global segment index currently on disk (-1 if none). Used to compute the
// next take's start_number after a take ends.
export function maxSegmentIndex(dir) {
  let mx = -1;
  for (const n of fs.readdirSync(dir)) {
    const m = n.match(CONFIG.files.segmentGlob);
    if (m) mx = Math.max(mx, Number(m[1]));
  }
  return mx;
}
