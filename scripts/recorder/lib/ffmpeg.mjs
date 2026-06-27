// Build the ffmpeg argv for the validated tee recipe: one encode fanned out to
//   - HLS / fMP4 (init + 2..N segments + ENDLIST) → the upload artifact, and
//   - a progressive faststart preview.mp4 → local file:// playback with zero JS.
//
// Output filenames are RELATIVE; the recorder spawns ffmpeg with cwd = the session
// dir (exactly as the recipe was validated). See lib/config.mjs for the constants
// and the rationale behind `-r` + forced keyframes.

import { CONFIG } from './config.mjs';

export function buildFfmpegArgs({ screenIndex, audioIndex = 'none' }) {
  const f = CONFIG.files;
  const seg = CONFIG.segmentSeconds;
  const hasAudio = audioIndex !== 'none';
  const inputSpec = `${screenIndex}:${audioIndex}`;

  const hlsOpts = [
    'f=hls',
    'hls_segment_type=fmp4',
    `hls_time=${seg}`,
    'hls_playlist_type=vod',
    `hls_fmp4_init_filename=${f.initSegment}`,
    `hls_segment_filename=${f.segmentPattern}`,
  ].join(':');
  const mp4Opts = ['f=mp4', 'movflags=+faststart'].join(':');
  const teeTarget = `[${hlsOpts}]${f.playlist}|[${mp4Opts}]${f.preview}`;

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
    args.push('-c:a', CONFIG.audioCodec, '-b:a', CONFIG.audioBitrate);
  }
  args.push('-f', 'tee', '-map', '0:v');
  if (hasAudio) args.push('-map', '0:a');
  args.push(teeTarget);

  return args;
}
