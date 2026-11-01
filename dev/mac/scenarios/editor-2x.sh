# The editor's drive on a Retina screen: the hosted Mac's own is 1× only, so a virtual
# display at 2× (dev/mac/hidpi.m) is made the main one first, and the drive runs on it — its
# shots at twice the pixels. What the ruler's time label left behind showed at 2× (Bagrat's
# Mac), not on the runner's 1× screen (2026-10-25).
set -u
OUT=${MAC_OUT:-$(mktemp -d)}
tool="$(getconf DARWIN_USER_TEMP_DIR)hidpi"
clang -fobjc-arc -framework Foundation -framework CoreGraphics dev/mac/hidpi.m -o "$tool" || { echo "NO: hidpi.m didn't build"; exit 1; }
"$tool" >"$OUT/hidpi.txt" 2>"$OUT/hidpi.log" &
display=$!
for _ in $(seq 1 20); do [ -s "$OUT/hidpi.txt" ] && break; sleep 1; done
cat "$OUT/hidpi.txt" "$OUT/hidpi.log"
grep -q "over 2560 × 1600 pixels, main yes" "$OUT/hidpi.txt" || { echo "NO: no 2× main display"; kill "$display" 2>/dev/null; exit 1; }
screencapture -x "$OUT/screen-2x.png"
(cd app && node test/editor-drive.mjs)
said=$?
kill "$display" 2>/dev/null
exit "$said"
