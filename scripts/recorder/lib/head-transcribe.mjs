// Mid-record head transcription (SPEC §7, Thread 2). Once roughly a minute of audio
// has been captured, transcribe just the HEAD so a good auto-title is ready the
// instant the user stops — no transcription wait at /stop. The full transcript still
// runs after stop (transcribe.mjs) for chapters + search.
//
// This is strictly READ-ONLY over already-written HLS segments — it never touches
// the live capture recipe. The live preview MP4 is faststart (moov at close), so it
// isn't decodable mid-record; the segments are. Pipeline:
//   first ~maxSeconds of complete segments → a temp closed VOD playlist
//   → ffmpeg decodes it to a small mono 16 kHz wav → whisper → head-transcript.json.

import fs from 'node:fs';
import path from 'node:path';
import { spawn } from 'node:child_process';

import { CONFIG, segName } from './config.mjs';
import { runTranscribe, DEFAULT_MODEL } from '../../transcribe/lib/transcribe.mjs';
import { spawnWhisper } from '../../transcribe/lib/whisper.mjs';

// The head transcript lands beside the full one but under its own name, so the two
// never clobber each other. The agent reads this at /stop to suggest a title.
export const HEAD_TRANSCRIPT_FILE = 'head-transcript.json';

// How much of the opening to transcribe for the title. A minute is plenty — for this
// medium the speaker states the topic up front (validated 2026-06-30 against the real
// library: a head-only title matched the full-transcript title in every titleable case).
export const HEAD_SECONDS = 60;

// How many segments cover `seconds` at the configured segment length.
export function segmentsForSeconds(seconds) {
  return Math.ceil(seconds / CONFIG.segmentSeconds);
}

function fileNonEmpty(p) {
  try { return fs.statSync(p).size > 0; } catch { return false; }
}

// Build a temp closed (VOD + ENDLIST) playlist over the first `need` segments
// covering ~maxSeconds. A segment is only safe to read once the NEXT one exists, so
// we require need+1 segments on disk and list the first `need`. Returns the playlist
// path, or null when there aren't enough complete segments yet (or no init). Pure +
// synchronous; safe to call repeatedly — it just no-ops until enough has been written.
export function buildHeadPlaylist(dir, { maxSeconds = 60, fileName = 'head.m3u8' } = {}) {
  const init = CONFIG.files.initSegment;
  if (!fileNonEmpty(path.join(dir, init))) return null;

  const need = segmentsForSeconds(maxSeconds);
  let count = 0;
  for (let i = 0; i <= need; i++) { // need+1: confirm the need-th segment is closed
    if (!fileNonEmpty(path.join(dir, segName(i)))) break;
    count++;
  }
  if (count <= need) return null; // not enough closed segments yet

  const seg = CONFIG.segmentSeconds;
  const lines = [
    '#EXTM3U',
    '#EXT-X-VERSION:7',
    `#EXT-X-TARGETDURATION:${seg}`,
    '#EXT-X-MEDIA-SEQUENCE:0',
    '#EXT-X-PLAYLIST-TYPE:VOD',
    `#EXT-X-MAP:URI="${init}"`,
  ];
  for (let i = 0; i < need; i++) lines.push(`#EXTINF:${seg.toFixed(6)},`, segName(i));
  lines.push('#EXT-X-ENDLIST', '');

  const pl = path.join(dir, fileName);
  fs.writeFileSync(pl, lines.join('\n'));
  return pl;
}

// Decode the head playlist to a small mono 16 kHz wav (whisper's native rate — tiny
// + fast). `spawnFn` is injectable for tests. Resolves the wav path or rejects.
export function extractHeadAudio(dir, playlist, { maxSeconds = 60, fileName = 'head.wav', spawnFn = spawn } = {}) {
  return new Promise((resolve, reject) => {
    const out = path.join(dir, fileName);
    const child = spawnFn('ffmpeg', [
      '-hide_banner', '-y',
      '-i', path.basename(playlist),
      '-t', String(maxSeconds),
      '-vn', '-ac', '1', '-ar', '16000',
      fileName,
    ], { cwd: dir, stdio: ['ignore', 'ignore', 'ignore'] });
    child.on('error', reject);
    child.on('close', (code) => (code === 0 ? resolve(out) : reject(new Error(`ffmpeg head extract exited ${code}`))));
  });
}

function cleanup(files) {
  for (const f of files) { try { fs.rmSync(f); } catch { /* best-effort */ } }
}

// Full head pipeline. Best-effort + soft-skip like transcribe.mjs: it returns
// { ok:false, reason } when there aren't enough segments yet, the decode fails, or
// whisper is absent/empty — it never throws into the recorder. Writes
// HEAD_TRANSCRIPT_FILE on success. Seams (`spawnFn`, `runWhisper`) are injectable.
export async function transcribeHead({
  dir,
  maxSeconds = 60,
  model = DEFAULT_MODEL,
  whisperBin = 'whisper',
  spawnFn = spawn,
  runWhisper,
  log = () => {},
} = {}) {
  const playlist = buildHeadPlaylist(dir, { maxSeconds });
  if (!playlist) {
    log('head_transcribe_skipped', { reason: 'not_enough_segments' });
    return { ok: false, reason: 'not_enough_segments' };
  }

  let wav;
  try {
    wav = await extractHeadAudio(dir, playlist, { maxSeconds, spawnFn });
  } catch (e) {
    cleanup([playlist]);
    log('head_transcribe_skipped', { reason: 'extract_failed', message: e.message });
    return { ok: false, reason: 'extract_failed', message: e.message };
  }

  const whisper = runWhisper ?? ((a) => spawnWhisper({ ...a, bin: whisperBin }));
  const res = await runTranscribe({
    audioPath: wav,
    outDir: dir,
    outFile: HEAD_TRANSCRIPT_FILE,
    model,
    runWhisper: whisper,
    log,
  });
  cleanup([playlist, wav]);

  if (res.ok) log('head_transcribed', { chars: res.transcript.length, transcriptPath: res.transcriptPath });
  return res;
}
