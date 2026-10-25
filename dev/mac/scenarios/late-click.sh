# A click on a control that something late just moved does nothing (N82, B256), with real
# clicks on a real Mac. Settings · General opens while the update is still downloading (the
# release stand-in holds the download back); a hand clicks where Update will come, again
# and again, from before it comes until shroom quits for the swap. Update comes into its row
# under the pointer: the clicks of its first half-second are not taken, the next one is.
# The app's own log says when it came (`update-staged`) and when it was taken (`update`).
# Without the guard the first click after it came takes it, a click's gap (~0.1 s) later.
set -u
OUT=${MAC_OUT:-$(mktemp -d)}
bad=0
no() { echo "NO: $*"; bad=1; }
quit() { pkill -f "shroom-dev.app/Contents/MacOS/shroom( |$)" 2>/dev/null; sleep 2; true; }
w=$(mktemp -d -t shroom-late-click)
home="$(getconf DARWIN_USER_TEMP_DIR)late-click"
frame() {
  osascript -l JavaScript -e '
    ObjC.import("CoreGraphics");
    const list = ObjC.deepUnwrap(ObjC.castRefToObject($.CGWindowListCopyWindowInfo($.kCGWindowListOptionOnScreenOnly, 0))) || [];
    const w = list.find(w => /shroom/.test(w.kCGWindowOwnerName) && w.kCGWindowLayer === 0);
    w ? [w.kCGWindowBounds.X, w.kCGWindowBounds.Y, w.kCGWindowBounds.Width, w.kCGWindowBounds.Height].map(Math.round).join(" ") : ""' 2>/dev/null
}

quit
# The release: this very build, as the update drive serves it, its download held back 12 s.
mkdir -p "$w/target" "$w/src"
ditto "$MAC_APP" "$w/target/shroom-dev.app"
ditto "$MAC_APP" "$w/src/shroom-dev.app"
v=$(node -p 'require(process.argv[1]).version' "$PWD/app/src-tauri/tauri.conf.json")
SHROOM_STAND_IN=1 sh tools/package-release.sh "$w/src/shroom-dev.app" "$v" "$w/rel/download/v$v" >/dev/null
mkdir -p "$w/rel/latest/download"
cp "$w/rel/download/v$v/manifest.txt" "$w/rel/latest/download/manifest.txt"
SLOW_MS=12000 node app/test/release-standin.mjs "$w/rel" >"$w/port" 2>"$OUT/http.log" &
srv=$!
trap 'kill $srv 2>/dev/null || true; quit' EXIT
port=""
for _ in $(seq 1 50); do
  port=$(head -n 1 "$w/port" 2>/dev/null || true)
  [ -n "$port" ] && curl -fsS "http://127.0.0.1:$port/latest/download/manifest.txt" >/dev/null 2>&1 && break
  sleep 0.2
done
echo "release stand-in on ${port:-?}, the download held 12 s"

# An owner-class Mac's invented space (nothing is reached: its host is `.invalid`).
rm -rf "$home" && mkdir -p "$home" && chmod 700 "$home"
printf '%s\n' '{"account_id":"acc","label":"v","subdomain":"x","host":"late-click.invalid","bucket":"shroom-space","role":"OwnerClass","member_id":"m_AbCdEfGhIjKlMnOp","owner_name":"Ann"}' >"$home/space.json"
printf '{"v":1,"at":%s}\n' "$(($(date +%s) * 1000))" >"$home/primed.json"
chmod 600 "$home/space.json" "$home/primed.json"

open -a "$w/src/shroom-dev.app" "shroom://settings/agent" --args --home "$home" --releases "http://127.0.0.1:$port" --update-target "$w/target/shroom-dev.app" --as-version 0.0.1
for _ in $(seq 1 30); do [ -n "$(frame)" ] && break; sleep 1; done
sleep 3
read -r X Y W H <<<"$(frame)"
echo "Settings: ${X:-?},${Y:-?} ${W:-?} × ${H:-?}"
screencapture -x "$OUT/before.png"
# Where Update comes in General's Updates row: the gallery's place (435, 116 in the page),
# under the 32-point title bar.
UX=$((${X:-0} + 435)) UY=$((${Y:-0} + 32 + 116))
echo "clicking at $UX,$UY until shroom quits"
n=0
for _ in $(seq 1 250); do
  pgrep -f "shroom-dev.app/Contents/MacOS/shroom( |$)" >/dev/null || break
  cliclick -w 20 "c:$UX,$UY"
  n=$((n + 1))
  [ $((n % 40)) = 20 ] && screencapture -x "$OUT/click-$n.png"
  sleep 0.08
done
echo "$n clicks"
log="$home/logs/run.log"
cp "$log" "$OUT/run.log" 2>/dev/null
# Each line: the wall clock, then the app's own clock in ms.
staged=$(awk '/ core update-staged /{print $2; exit}' "$log" 2>/dev/null)
took=$(awk '/ core update \[/{print $2; exit}' "$log" 2>/dev/null)
echo "update-staged at ${staged:-?} ms, update taken at ${took:-?} ms"
case "$staged$took" in *[!0-9]*) no "the log's clock isn't a number: ${staged:-?} and ${took:-?}"; staged= ;; esac
if [ -z "$staged" ] || [ -z "$took" ]; then
  no "the log hasn't both: Update never came, or was never taken"
else
  gap=$((took - staged))
  echo "Update came, and was taken ${gap} ms later"
  [ "$gap" -ge 500 ] || no "Update was taken ${gap} ms after it came — inside the half-second"
  [ "$gap" -le 1500 ] || no "Update waited ${gap} ms: the clicks after the half-second weren't taken either"
fi
[ "$bad" = 0 ] && echo "verdict: the clicks of Update's first half-second weren't taken; the next one was"
exit "$bad"
