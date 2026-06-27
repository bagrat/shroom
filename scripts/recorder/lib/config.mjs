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
  // Match the original SaaS shroom's capture policy (assets/js getDisplayMedia:
  // width max 1920, height max 1080, 30fps). avfoundation can't constrain capture
  // resolution (it grabs native — e.g. 4K), so we downscale in ffmpeg to fit a
  // 1920x1080 box, preserving aspect, never upscaling (min(target,input)), even
  // dims for yuv420p. 4K screen capture was both wasteful (4x pixels) and low
  // quality at a sane bitrate; 1080p is the right target for a Loom-style share.
  maxWidth: 1920,
  maxHeight: 1080,
  videoFilter: "scale=w='min(1920,iw)':h='min(1080,ih)':force_original_aspect_ratio=decrease:force_divisible_by=2",
  // The original rode Chrome's VP9 default (~2.5 Mbps). VP9 is ~1.4x more efficient
  // than our h264, so ~3.5 Mbps h264 ≈ that quality at 1080p screen content.
  videoBitrate: '3.5M',
  pixFmt: 'yuv420p',
  captureCursor: 1,

  // --- audio (optional; off by default in M1) ---
  audioCodec: 'aac',
  audioBitrate: '128k',
  // THE AUDIO FIX (empirically found 2026-06-27): avfoundation mic capture delivers
  // ~6% fewer samples than the wall-clock timeline (a device-clock/timestamp
  // mismatch, NOT 4K throughput — verified: audio-only capture drops too,
  // downscaling doesn't help, separate inputs make it worse). The gaps play as the
  // audio speeding up then cutting before the video ends. `aresample=async=1`
  // fills the gaps with silence so audio stays locked to the timeline, correct
  // pitch, in sync to the end. Measured: ~6% drop → 0% with this filter.
  audioFilter: 'aresample=async=1',

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
