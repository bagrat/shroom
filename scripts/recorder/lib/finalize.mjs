// Summarize a finished session dir: what artifacts exist, segment list, duration,
// and whether the HLS playlist was sealed with #EXT-X-ENDLIST. Pure inspection —
// no mutation. Used to build the `finalized` event and to decide recorder exit code.

import fs from 'node:fs';
import path from 'node:path';
import { CONFIG } from './config.mjs';

export function summarize(dir) {
  const f = CONFIG.files;
  const playlistPath = path.join(dir, f.playlist);
  const previewPath = path.join(dir, f.preview);
  const initPath = path.join(dir, f.initSegment);

  const exists = (p) => fs.existsSync(p);
  const nonEmpty = (p) => exists(p) && fs.statSync(p).size > 0;

  const segFiles = fs
    .readdirSync(dir)
    .filter((n) => f.segmentGlob.test(n))
    .sort();
  const segmentIndices = segFiles
    .map((n) => Number(n.match(f.segmentGlob)[1]))
    .sort((a, b) => a - b);

  const result = {
    dir,
    playlist: exists(playlistPath) ? f.playlist : null,
    preview: nonEmpty(previewPath) ? f.preview : null,
    initSegment: exists(initPath) ? f.initSegment : null,
    segments: segFiles,
    segmentIndices,
    segmentCount: segFiles.length,
    durationSec: 0,
    endlist: false,
  };

  if (result.playlist) {
    const txt = fs.readFileSync(playlistPath, 'utf8');
    let dur = 0;
    for (const line of txt.split('\n')) {
      const m = line.match(/^#EXTINF:([\d.]+)/);
      if (m) dur += parseFloat(m[1]);
    }
    result.durationSec = Math.round(dur * 100) / 100;
    result.endlist = /#EXT-X-ENDLIST/.test(txt);
  }

  // A session is "good" when both artifacts finalized cleanly.
  result.ok =
    Boolean(result.preview) &&
    Boolean(result.initSegment) &&
    result.endlist &&
    result.segmentCount > 0;

  return result;
}
