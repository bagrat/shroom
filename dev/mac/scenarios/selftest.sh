# The dev app says what it may reach, and whether it sees the screen: three seconds of
# ScreenCaptureKit, counted. Passes when frames arrive and every one can be read. How many
# arrive is not judged: ScreenCaptureKit sends a frame when the screen changes, so the
# count follows what moves there. It was 72 while a starting shroom still bounced in the
# Dock, and about 20 since it is born a menu-bar app (DECISIONS B113) — the same Mac, the
# same three seconds.
out="$MAC_OUT/selftest.json"
open -W -n "$MAC_APP" --args --selftest "$out"
node -e 'const r = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")), s = r.screen;
  console.log(`screen: permission ${s.permission}, ${s.frames_delivered} frames in ${s.elapsed_ms} ms (${s.frames_lockable} readable), ${s.width}x${s.height}${s.error ? ", error: " + s.error : ""}; microphone ${JSON.stringify(r.mic)}; camera ${JSON.stringify(r.camera)}`);
  process.exit(s.permission && s.frames_delivered > 0 && s.frames_lockable === s.frames_delivered ? 0 : 1)' "$out"
