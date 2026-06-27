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
  videoBitrate: '4M',
  pixFmt: 'yuv420p',
  captureCursor: 1,

  // --- audio (optional; off by default in M1) ---
  audioCodec: 'aac',
  audioBitrate: '128k',

  // --- HLS segmentation ---
  // SPEC §5: "6 s default, a config constant (adjustable in code)." A forced IDR is
  // placed at every segment boundary so segments cut cleanly on a keyframe.
  segmentSeconds: 6,

  // --- output filenames (relative to the session dir; ffmpeg runs with cwd = dir) ---
  files: {
    playlist: 'stream.m3u8',
    initSegment: 'init.mp4',
    segmentPattern: 'seg_%05d.m4s',
    segmentGlob: /^seg_(\d+)\.m4s$/,
    preview: 'preview.mp4',
    events: 'events.ndjson',
    control: 'control.fifo',
    ffmpegLog: 'ffmpeg.log',
  },
};

// Reconstruct a segment filename from its integer index (matches segmentPattern).
export function segName(i) {
  return `seg_${String(i).padStart(5, '0')}.m4s`;
}
