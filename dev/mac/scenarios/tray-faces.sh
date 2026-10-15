# The mushroom on every menu bar (DECISIONS B151). The dev app's --tray-probe draws each
# tray face on a real status item under a light and a dark bar, with and without Increase
# Contrast, the app's own appearance set the other way (as over a dark wallpaper in Light
# mode), and checks the mushroom stands out from the bar; it writes each drawing. Then the
# real menu bar is shot with each face under four Macs: Light and Dark mode, each over a
# black and a white wallpaper (macOS 26's bar takes its look from what is under it).
# Passes when every drawn check does; the screenshots are for the eye.
set -u
. dev/mac/lib.sh
T="$MAC_OUT/tray"
mkdir -p "$T"
rm -f "$T/want" "$T/shown" "$T/rect"

se() { limit 30 osascript -e "tell application \"System Events\" to $1"; }
# A `tell application` may raise a consent prompt: answered while the ask waits.
allow_soon() { (sleep 4; osascript -e 'tell application "System Events" to tell process "UserNotificationCenter" to click button "Allow" of window 1' >/dev/null 2>&1) & }

# Two plain wallpapers, black and white.
python3 - "$T" <<'EOF'
import struct, sys, zlib
def png(path, v):
    w, h = 64, 64
    raw = b"".join(b"\x00" + bytes([v, v, v]) * w for _ in range(h))
    chunk = lambda t, d: struct.pack(">I", len(d)) + t + d + struct.pack(">I", zlib.crc32(t + d) & 0xffffffff)
    open(path, "wb").write(b"\x89PNG\r\n\x1a\n" + chunk(b"IHDR", struct.pack(">IIBBBBB", w, h, 8, 2, 0, 0, 0)) + chunk(b"IDAT", zlib.compress(raw)) + chunk(b"IEND", b""))
png(sys.argv[1] + "/black.png", 0)
png(sys.argv[1] + "/white.png", 255)
EOF

open -n "$MAC_APP" --args --tray-probe "$T/probe.json"
for _ in $(seq 1 150); do [ -s "$T/rect" ] && break; sleep 0.2; done
if [ ! -s "$T/rect" ]; then
  echo "the probe never said where the mushroom is"
  exit 1
fi
IFS=, read -r x y w h <"$T/rect"
# The item and some of the bar either side of it.
region="$(printf '%.0f' "$(echo "$x - 60" | bc)"),$(printf '%.0f' "$y"),$(printf '%.0f' "$(echo "$w + 120" | bc)"),$(printf '%.0f' "$h")"
echo "the mushroom at $x,$y ${w}x$h; shooting $region"

for mode in light dark; do
  for paper in black white; do
    allow_soon
    se "tell appearance preferences to set dark mode to $([ $mode = dark ] && echo true || echo false)" >/dev/null
    allow_soon
    se "tell every desktop to set picture to POSIX file \"$T/$paper.png\"" >/dev/null
    sleep 2
    for face in plain paused recording attention update; do
      echo "$face" >"$T/want"
      for _ in $(seq 1 50); do [ "$(cat "$T/shown" 2>/dev/null)" = "$face" ] && break; sleep 0.1; done
      sleep 1
      screencapture -x -R "$region" "$T/bar-$mode-$paper-$face.png"
    done
    screencapture -x "$T/screen-$mode-$paper.png"
  done
done
echo done >"$T/want"
se "tell appearance preferences to set dark mode to false" >/dev/null
for _ in $(seq 1 50); do pgrep -f "$MAC_APP/Contents/MacOS" >/dev/null || break; sleep 0.2; done

node -e 'const r = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"));
  if (r.error) console.log(`error: ${r.error}`);
  for (const c of r.checks) console.log(`${c.ok ? "ok  " : "NO  "} ${c.face.padEnd(10)} on a ${c.bar.padEnd(26)} bar (app ${c.app}): ${c.template ? "a template image" : `cap ${c.cap} over ${c.pixels} px`}`);
  process.exit(!r.error && r.checks.length === 20 && r.checks.every(c => c.ok) ? 0 : 1)' "$T/probe.json"
