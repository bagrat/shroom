// Capture / encode constants for the shroom recorder.
//
// This is the single home of the *validated* ffmpeg recipe parameters
// (empirically de-risked 2026-06-27 on ffmpeg 7.1.1 / macOS Darwin 25). M2
// (pause/resume) and M3 (uploader) read from here so the recipe lives in one place.

export const CONFIG = {
  // --- video ---
  framerate: 30, // avfoundation input rate AND forced output rate (-r).
  // THE KEY FIX: avfoundation reports a bogus input timebase (1000k tbr); without a
  // forced constant output rate the HLS muxer can't time-slice and wedges on a 0-byte
  // init.mp4. `-r 30` + forced keyframes at each segment boundary is what makes tee work.
  videoCodec: 'h264_videotoolbox', // macOS hardware encode
  // Resolution + bitrate are NOT here — they're the user-chosen quality preset
  // (lib/quality.mjs: Normal 1080p / 2K / 4K). avfoundation grabs native res, so
  // the recipe downscales in ffmpeg to the preset's box (never upscaling).
  pixFmt: 'yuv420p',
  captureCursor: 1,

  // --- audio (optional) ---
  audioCodec: 'aac',
  audioBitrate: '128k',
  // Audio is captured NATIVELY (the mic tap → mono f32le PCM on a fifo), then fed to
  // ffmpeg as a second input and encoded to AAC here. We do NOT capture the mic via
  // ffmpeg's avfoundation audio demuxer: its buffer handling corrupts the built-in
  // mic with real digital splices (a ~decade-old, version-independent bug). The old
  // `aresample=async=1` band-aid is gone with it — native PCM is clean and locked to
  // wall-clock, so async would only stuff silence back in. See ffmpeg.mjs and the
  // capture-empirical-findings notes for the full why.

  // --- HLS segmentation ---
  // SPEC §5: "6 s default, a config constant (adjustable in code)." A forced IDR is
  // placed at every segment boundary so segments cut cleanly on a keyframe.
  segmentSeconds: 6,

  // --- output filenames (relative to the session dir; ffmpeg runs with cwd = dir) ---
  //
  // Segments are numbered GLOBALLY and contiguously across takes (a take = one
  // recording run between pauses), via ffmpeg's -start_number. The init segment is
  // byte-identical across takes (validated), so the whole session shares one
  // init.mp4. Each take writes its own playlist + preview; `playlist` and `preview`
  // are the FINAL assembled artifacts produced at finalize (SPEC §5).
  files: {
    playlist: 'stream.m3u8', // final master, assembled at finalize
    initSegment: 'init.mp4', // shared by all takes
    segmentPattern: 'seg_%05d.m4s',
    segmentGlob: /^seg_(\d+)\.m4s$/,
    preview: 'preview.mp4', // final, concatenated at finalize
    events: 'events.ndjson',
    control: 'control.fifo',
    ffmpegLog: 'ffmpeg.log',
  },
};

// Reconstruct a segment filename from its integer index (matches segmentPattern).
export function segName(i) {
  return `seg_${String(i).padStart(5, '0')}.m4s`;
}

// Per-take artifact names (k = 0-based take index).
export function takePlaylist(k) {
  return `stream_${k}.m3u8`;
}
export function takePreview(k) {
  return `preview_${k}.mp4`;
}
