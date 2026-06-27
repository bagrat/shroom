// Capture quality presets — the deterministic data behind the `/shroom:record`
// quality picker. The user picks a preset (judgment); these numbers (resolution,
// bitrate, and the size/cost estimates that inform the choice) are fixed here.
//
// "Normal" = the original SaaS shroom's 1080p/30 policy (assets/js getDisplayMedia).
// 2K/4K trade file size (the product's one real cost — storage, SPEC §3) for
// fidelity. avfoundation can't constrain capture resolution, so the recipe
// downscales in ffmpeg to fit the preset's box (never upscaling).

export const QUALITY = {
  normal: { label: 'Normal (1080p)', maxWidth: 1920, maxHeight: 1080, bitrateMbps: 3.5 },
  '2k':   { label: '2K (1440p)',     maxWidth: 2560, maxHeight: 1440, bitrateMbps: 6 },
  '4k':   { label: '4K (2160p)',     maxWidth: 3840, maxHeight: 2160, bitrateMbps: 12 },
};

export const DEFAULT_QUALITY = 'normal';

// Audio adds a small constant; R2 storage is the only real cost (egress is free,
// SPEC §3). ~$0.015/GB-month.
const AUDIO_MBPS = 0.128;
const R2_USD_PER_GB_MONTH = 0.015;

export function resolveQuality(key) {
  return QUALITY[key] ? key : DEFAULT_QUALITY;
}

// The ffmpeg downscale filter for a preset: fit within the box, preserve aspect,
// never upscale (min(target, input)), even dims for yuv420p.
export function scaleFilter(key) {
  const q = QUALITY[resolveQuality(key)];
  return `scale=w='min(${q.maxWidth},iw)':h='min(${q.maxHeight},ih)':force_original_aspect_ratio=decrease:force_divisible_by=2`;
}

export function ffmpegBitrate(key) {
  return `${QUALITY[resolveQuality(key)].bitrateMbps}M`;
}

// Eyeball estimates for the picker: MB/min, GB/hour, and monthly R2 storage cost
// per hour of recording. Rounded for human display, not billing.
export function estimate(key) {
  const q = QUALITY[resolveQuality(key)];
  const totalMbps = q.bitrateMbps + AUDIO_MBPS;
  const mbPerMin = (totalMbps / 8) * 60;
  const gbPerHour = (mbPerMin * 60) / 1024;
  return {
    mbPerMin: Math.round(mbPerMin),
    gbPerHour: Number(gbPerHour.toFixed(2)),
    usdPerHourMonth: Number((gbPerHour * R2_USD_PER_GB_MONTH).toFixed(3)),
  };
}

// The full catalogue for the preflight JSON the command surfaces in the picker.
export function qualityCatalogue() {
  return Object.keys(QUALITY).map((key) => ({
    key,
    label: QUALITY[key].label,
    resolution: `${QUALITY[key].maxWidth}x${QUALITY[key].maxHeight}`,
    bitrateMbps: QUALITY[key].bitrateMbps,
    ...estimate(key),
  }));
}
