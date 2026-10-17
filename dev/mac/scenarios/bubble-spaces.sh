# The bubble keeps moving across Spaces. In the 2026-10-17 rehearsal the bubble froze on a
# swipe into a full-screen Space and stayed frozen on every Space after it, the camera's
# light still on — and a screen take records the bubble, so the take froze with it.
# The hosted Mac's camera (dev/mac/camera.sh: OBS's virtual camera on the moving test
# pattern), then the dev app's --bubble-probe held 40 s, reading the bubble's picture off
# the screen once a second. While it holds, TextEdit goes full screen (a Space of its own),
# and the Spaces are switched back and forth with ⌃← / ⌃→, ending on the full-screen one.
# Passes when the bubble's picture still moves through the hold's last third.
set -u
. dev/mac/lib.sh
bash dev/mac/camera.sh up || exit 1
B="$MAC_OUT/bubble"
mkdir -p "$B"
out="$B/probe.json"
rm -f "$out" "$B/probe.ready"

se() { limit 30 osascript -e "tell application \"System Events\" to $1" >>"$B/keys.txt" 2>&1; }
# A `tell application` may raise a consent prompt: answered while the ask waits.
allow_soon() { (sleep 4; osascript -e 'tell application "System Events" to tell process "UserNotificationCenter" to click button "Allow" of window 1' >/dev/null 2>&1) & }
shot() { screencapture -x "$B/$1.png" >/dev/null 2>&1; }

# OBS's own window plays the pattern too: hidden, so only the bubble moves where it sits.
se 'set visible of process "OBS" to false'
open -n "$MAC_APP" --args --bubble-probe "$out" --hold 40
for _ in $(seq 1 100); do [ -e "$B/probe.ready" ] && break; sleep 0.2; done
[ -e "$B/probe.ready" ] || { echo "the bubble never showed"; exit 1; }
shot 0-bubble

# A full-screen Space: a TextEdit document, made full screen.
printf 'A full-screen Space for the bubble to cross.\n' >"$B/space.txt"
open -a TextEdit "$B/space.txt"
sleep 3
allow_soon
se 'keystroke "f" using {control down, command down}'
sleep 4
shot 1-fullscreen
se 'key code 123 using {control down}'   # ⌃← back to the desktop's Space
sleep 3
shot 2-back
se 'key code 124 using {control down}'   # ⌃→ into the full-screen Space again
sleep 3
shot 3-again
se 'key code 123 using {control down}'
sleep 2
se 'key code 124 using {control down}'
sleep 2
shot 4-ended

# The probe exits once it has written its report.
for _ in $(seq 1 300); do [ -s "$out" ] && break; sleep 0.2; done
shot 5-end
osascript -e 'tell application "TextEdit" to quit saving no' >/dev/null 2>&1 || pkill -x TextEdit
node -e 'const r = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"));
  const c = (r.samples || []).map(s => s.change == null ? "-" : s.change.toFixed(1));
  console.log(`bubble on ${r.camera ? r.camera.name : "no camera"}: moving through the end ${r.moving}; running ${r.seen && r.seen.running}; change a second apart: ${c.join(" ")}`);
  process.exit(r.ok ? 0 : 1)' "$out"
