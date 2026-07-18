// Why a first take can wedge (never produce init.mp4), and how to tell the causes
// apart from ffmpeg's own log. The three causes need DIFFERENT guidance, so the
// classification is deterministic and lives here — testable in isolation from the
// recorder lifecycle.
//
//   • screen_grant_inactive — screen access isn't active THIS launch. A freshly
//     (re)granted Screen Recording permission only takes effect on the NEXT launch, so
//     ffmpeg can't configure the device and it never OPENS. Actionable: record again.
//   • capture_no_frames — the screen device opened but delivered no frames (ffmpeg logs
//     "not enough frames to estimate rate" and never encodes). Screen access IS fine,
//     so "takes effect next launch" would be the wrong guidance — a plain retry is right.
//   • capture_wedged — otherwise the two-input audio path deadlocked (ffmpeg blocked on
//     the mic input it can't get). Also the default when there's no log to read.
//
// The load-bearing subtlety: "Configuration of video device failed" / "not supported by
// the input device" ALSO print as benign pixel-format fallback on a SUCCESSFUL open — so
// on their own they don't mean the grant is inactive. Gate on the "Input #0, avfoundation"
// marker (present iff the device actually opened) to avoid the false positive that
// misreported a no-frames wedge as a permission-takes-effect-next-launch problem.

const DEVICE_OPENED = /Input #0, avfoundation/i;
const CONFIG_FAILED = /Configuration of video device failed|not supported by the input device/i;
const NO_FRAMES = /not enough frames to estimate rate/i;

export function classifyWedge(logText) {
  const t = logText || '';
  const opened = DEVICE_OPENED.test(t);

  // Only a device that never opened is a genuine grant-inactive case; the config-failed
  // lines are benign when the device did open.
  if (!opened && CONFIG_FAILED.test(t)) {
    return {
      reason: 'screen_grant_inactive',
      message: 'screen capture did not start — its permission takes effect on the next launch',
    };
  }

  // Opened but no frames flowed: screen access is fine, so retry — don't send them off to
  // re-grant a permission they already have.
  if (opened && NO_FRAMES.test(t)) {
    return { reason: 'capture_no_frames', message: 'screen device opened but delivered no frames' };
  }

  return { reason: 'capture_wedged', message: 'no init segment within watchdog window' };
}
