// Build the ffmpeg argv for one TAKE of the validated tee recipe: one encode fanned
// out to HLS / fMP4 (the upload artifact) and a progressive faststart preview.
//
// TWO INPUTS (the native-audio design):
//   input 0 = the screen (or camera) via avfoundation, VIDEO ONLY (`<index>:none`).
//   input 1 = the microphone as clean native PCM (mono f32le) arriving on a fifo,
//             captured by the native mic tap — NOT ffmpeg's avfoundation audio, whose
//             buffer handling corrupts the built-in mic (real digital splices; a
//             ~decade-old bug, version-independent). See capture-empirical-findings.
// The tee maps `0:v` + `1:a`, so both the HLS branch and the instant preview carry
// the clean audio. The recorder owns the mic tap and the fifo hand-off (record.mjs);
// this module just assembles the argv that reads them.
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
import { scaleFilter, ffmpegBitrate, DEFAULT_QUALITY } from './quality.mjs';

// audio: null for a video-only take, else { fifo, rate, itsoffset? } — the PCM fifo
//   the native mic tap feeds, its sample rate (from the tap's probe), and an optional
//   constant A/V shift knob (seconds; rarely needed — sync measured tight).
export function buildFfmpegArgs({ videoIndex, audio = null, startNumber = 0, take = 0, quality = DEFAULT_QUALITY }) {
  const f = CONFIG.files;
  const seg = CONFIG.segmentSeconds;

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
    // input 0: screen/camera, VIDEO ONLY. avfoundation's audio demuxer is the bug;
    // audio arrives on input 1 as clean native PCM instead.
    '-f', 'avfoundation',
    '-framerate', String(CONFIG.framerate),
    '-capture_cursor', String(CONFIG.captureCursor),
    '-i', `${videoIndex}:none`,
  ];
  if (audio) {
    // input 1: the native mic tap's PCM on the fifo. -itsoffset (optional) shifts
    // audio to hunt a constant lead/lag; -thread_queue_size keeps the live input
    // from starving while ffmpeg warms up the avfoundation video (~1s).
    if (audio.itsoffset != null) args.push('-itsoffset', String(audio.itsoffset));
    args.push('-thread_queue_size', '4096', '-f', 'f32le', '-ar', String(audio.rate), '-ac', '1', '-i', audio.fifo);
  }

  args.push(
    // Downscale to the chosen quality preset's box (avfoundation captures native
    // res). Never upscales — a smaller source passes through.
    '-vf', scaleFilter(quality),
    '-c:v', CONFIG.videoCodec,
    '-b:v', ffmpegBitrate(quality),
    '-pix_fmt', CONFIG.pixFmt,
    '-r', String(CONFIG.framerate),
    '-force_key_frames', `expr:gte(t,n_forced*${seg})`,
  );
  if (audio) {
    // No aresample/async: that was the band-aid for avfoundation's drifting mic
    // clock. Native PCM is clean and locked to wall-clock, so async would only stuff
    // silence back in. Just encode the PCM to AAC before the tee.
    args.push('-c:a', CONFIG.audioCodec, '-b:a', CONFIG.audioBitrate);
  }
  args.push('-f', 'tee', '-map', '0:v');
  if (audio) args.push('-map', '1:a');
  args.push(teeTarget);

  return args;
}
