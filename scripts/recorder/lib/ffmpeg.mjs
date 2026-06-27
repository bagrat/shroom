// Build the ffmpeg argv for one TAKE of the validated tee recipe: one encode fanned
// out to HLS / fMP4 (the upload artifact) and a progressive faststart preview.
//
// A take is a single recording run between pauses (M2). Segments are numbered
// globally via `startNumber` so they stay contiguous across takes and never
// collide; each take writes its own per-take playlist + preview, assembled into
// the final stream.m3u8 / preview.mp4 at finalize. The init segment is shared
// (byte-identical across takes — validated).
//
// Output filenames are RELATIVE; the recorder spawns ffmpeg with cwd = the session
// dir (exactly as the recipe was validated). See lib/config.mjs for the constants
// and the rationale behind `-r` + forced keyframes.

import { CONFIG, takePlaylist, takePreview } from './config.mjs';

export function buildFfmpegArgs({ videoIndex, audioIndex = 'none', startNumber = 0, take = 0 }) {
  const f = CONFIG.files;
  const seg = CONFIG.segmentSeconds;
  const hasAudio = audioIndex !== 'none';
  const inputSpec = `${videoIndex}:${audioIndex}`;

  const hlsOpts = [
    'f=hls',
    'hls_segment_type=fmp4',
    `hls_time=${seg}`,
    'hls_playlist_type=vod',
    `start_number=${startNumber}`,
    `hls_fmp4_init_filename=${f.initSegment}`,
    `hls_segment_filename=${f.segmentPattern}`,
  ].join(':');
  const mp4Opts = ['f=mp4', 'movflags=+faststart'].join(':');
  const teeTarget = `[${hlsOpts}]${takePlaylist(take)}|[${mp4Opts}]${takePreview(take)}`;

  const args = [
    '-hide_banner',
    '-f', 'avfoundation',
    '-framerate', String(CONFIG.framerate),
    '-capture_cursor', String(CONFIG.captureCursor),
    '-i', inputSpec,
    '-c:v', CONFIG.videoCodec,
    '-b:v', CONFIG.videoBitrate,
    '-pix_fmt', CONFIG.pixFmt,
    '-r', String(CONFIG.framerate),
    '-force_key_frames', `expr:gte(t,n_forced*${seg})`,
  ];
  if (hasAudio) {
    // -af keeps avfoundation's drifting mic clock glued to the timeline (see
    // CONFIG.audioFilter) — without it ~6% of audio drops as gaps. Applies before
    // the tee, so both the HLS and preview branches get the corrected audio.
    args.push('-af', CONFIG.audioFilter, '-c:a', CONFIG.audioCodec, '-b:a', CONFIG.audioBitrate);
  }
  args.push('-f', 'tee', '-map', '0:v');
  if (hasAudio) args.push('-map', '0:a');
  args.push(teeTarget);

  return args;
}
