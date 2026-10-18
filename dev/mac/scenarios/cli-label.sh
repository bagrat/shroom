# The shroom command's start without the count, and the agent label (spec N62, DECISIONS
# B216; Bagrat, 2026-10-18), through the dev app's real recorder, filmed:
#
#   1. `record start --no-countdown` records at once (no count on screen) and the label
#      shows top right of the display; the take is stopped after a few seconds. A frame of
#      the take, decoded here, has nothing where the label was on screen: the label's own
#      rectangle (as the window server has it) is lit in the screenshot and dark in the
#      take — this Mac's desktop is black, and nothing else of shroom's is there.
#   2. `record start --no-countdown --no-label`: a take with no label anywhere.
#
# The home is a throwaway one with an invented member's space (its host never resolves:
# nothing is uploaded) and General's switch on, as prefs.json keeps it. Each take stays on
# this runner and goes with the home.
set -u
OUT=${MAC_OUT:-$(mktemp -d)}
cli="$MAC_APP/Contents/MacOS/shroom-cli"
home="$(getconf DARWIN_USER_TEMP_DIR)cli-label"
bad=0
no() { echo "NO: $*"; bad=1; }
quit() { pkill -f "$MAC_APP/Contents/MacOS/shroom( |$)" 2>/dev/null; sleep 2; true; }
ffmpeg=$(command -v ffmpeg || ls /opt/homebrew/bin/ffmpeg /usr/local/bin/ffmpeg 2>/dev/null | head -n 1)
ask() { SHROOM_HOME="$home" "$cli" record "$@" --no-launch --json; }
field() { sed -n "s/.*\"$1\":\"\\([^\"]*\\)\".*/\\1/p"; }
# The label's window on screen, as the window server has it: "x y w h" in points (top-left
# origin), or nothing.
label_rect() {
  osascript -l JavaScript -e '
    ObjC.import("CoreGraphics");
    const list = ObjC.deepUnwrap(ObjC.castRefToObject($.CGWindowListCopyWindowInfo($.kCGWindowListOptionOnScreenOnly, 0))) || [];
    const w = list.find(w => /shroom/.test(w.kCGWindowOwnerName) && Math.round(w.kCGWindowBounds.Width) === 380 && Math.round(w.kCGWindowBounds.Height) === 30);
    w ? [w.kCGWindowBounds.X, w.kCGWindowBounds.Y, w.kCGWindowBounds.Width, w.kCGWindowBounds.Height].map(Math.round).join(" ") : ""' 2>/dev/null
}
# How many pixels of `file`'s rectangle (x y w h, in its own pixels) are bright: the label's
# words are near-white.
bright() {
  "$ffmpeg" -hide_banner -loglevel error -i "$1" -frames:v 1 -vf "crop=$4:$5:$2:$3,format=gray" -f rawvideo - 2>/dev/null |
    od -An -v -tu1 | tr -s ' ' '\n' | awk '$1 >= 160 { n++ } END { print n + 0 }'
}
size_of() { "$ffmpeg" -hide_banner -i "$1" 2>&1 | sed -n 's/.*Video:.* \([0-9][0-9]*\)x\([0-9][0-9]*\)[ ,].*/\1 \2/p' | head -n 1; }
# The main display's width in points: what the window server's rectangles are measured in.
points_w() { osascript -l JavaScript -e 'ObjC.import("CoreGraphics"); $.CGDisplayBounds($.CGMainDisplayID()).size.width' 2>/dev/null; }
launch() {
  open -n "$MAC_APP" --args --home "$home"
  for _ in $(seq 1 120); do [ -S "$home/control.sock" ] && break; sleep 0.5; done
  [ -S "$home/control.sock" ] || { echo "NO: the app made no control socket"; exit 1; }
  sleep 2
}

[ -x "$cli" ] || { echo "NO: the dev bundle carries no shroom command at $cli"; exit 1; }
[ -n "$ffmpeg" ] || { echo "NO: no ffmpeg here to read the take"; exit 1; }
quit
rm -rf "$home" && mkdir -p "$home" && chmod 700 "$home"
printf '%s\n' '{"account_id":"acc","label":"v","subdomain":"x","host":"cli-label.invalid","bucket":"shroom-space","role":"Member","member_id":"m_AbCdEfGhIjKlMnOp","owner_name":"Ann"}' >"$home/space.json"
printf '{"v":1,"at":%s}\n' "$(($(date +%s) * 1000))" >"$home/primed.json"
printf '%s\n' '{"source":null,"show_yourself":false,"mic":null,"quality":"normal","bubble_at":null,"setup_chosen":true,"agent_recording":true}' >"$home/prefs.json"
chmod 600 "$home/space.json" "$home/primed.json" "$home/prefs.json"
launch

echo "── record start --no-countdown: at once, with the label"
j=$(ask start --no-countdown); echo "  $j"
[ "$(printf '%s' "$j" | field take)" = recording ] || no "the start didn't answer recording at once"
rect=""
for _ in $(seq 1 20); do rect=$(label_rect); [ -n "$rect" ] && break; sleep 0.25; done
[ -n "$rect" ] && echo "  the label is on screen at $rect" || no "no label on screen"
sleep 1.5
screencapture -x "$OUT/label-on.png"
sleep 3
j=$(ask stop); echo "  $j"
[ "$(printf '%s' "$j" | field take)" = stopping ] || no "the stop didn't answer stopping"
sleep 4
take=$(ls -td "$home"/takes/*/ 2>/dev/null | head -n 1)
playlist="${take}stream.m3u8"
if [ -n "$rect" ] && [ -f "$playlist" ]; then
  set -- $rect
  read -r sw sh <<EOF
$(size_of "$OUT/label-on.png")
EOF
  "$ffmpeg" -hide_banner -loglevel error -allowed_extensions ALL -ss 2 -i "$playlist" -frames:v 1 "$OUT/take-frame.png"
  read -r vw vh <<EOF
$(size_of "$OUT/take-frame.png")
EOF
  # The label's rectangle, in each picture's own pixels (the screen's points scaled to it).
  pw=$(points_w); pw=${pw:-$sw}
  sx=$(awk -v a="$sw" -v p="$pw" 'BEGIN { print a / p }'); vx=$(awk -v a="$vw" -v p="$pw" 'BEGIN { print a / p }')
  on_screen=$(bright "$OUT/label-on.png" "$(awk -v v="$1" -v s="$sx" 'BEGIN{printf "%d", v*s}')" "$(awk -v v="$2" -v s="$sx" 'BEGIN{printf "%d", v*s}')" "$(awk -v v="$3" -v s="$sx" 'BEGIN{printf "%d", v*s}')" "$(awk -v v="$4" -v s="$sx" 'BEGIN{printf "%d", v*s}')")
  in_take=$(bright "$OUT/take-frame.png" "$(awk -v v="$1" -v s="$vx" 'BEGIN{printf "%d", v*s}')" "$(awk -v v="$2" -v s="$vx" 'BEGIN{printf "%d", v*s}')" "$(awk -v v="$3" -v s="$vx" 'BEGIN{printf "%d", v*s}')" "$(awk -v v="$4" -v s="$vx" 'BEGIN{printf "%d", v*s}')")
  echo "  the label's rectangle: $on_screen bright pixels on screen (${sw}×${sh}), $in_take in the take (${vw}×${vh})"
  [ "${on_screen:-0}" -ge 40 ] || no "the screenshot shows no label where the window server has it"
  [ "${in_take:-0}" -le 4 ] || no "the label is in the take"
else
  no "no take to read (${take:-none})"
fi

echo "── record start --no-countdown --no-label"
# The first take's stop couldn't reach its (invented) space, and its popover holds until the
# person sees it: a start now would rightly be not_ready. A fresh launch starts clear.
quit
launch
j=$(ask start --no-countdown --no-label); echo "  $j"
[ "$(printf '%s' "$j" | field take)" = recording ] || no "the start didn't answer recording at once"
sleep 2
[ -z "$(label_rect)" ] && echo "  no label on screen" || no "a label showed with --no-label"
screencapture -x "$OUT/label-off.png"
sleep 2
ask stop >/dev/null
sleep 3
grep -E "control:|core cli" "$home/logs/run.log" | tail -n 8 | sed 's/^/  log: /'
quit
[ "$bad" = 0 ] && echo "verdict: --no-countdown records at once; the label shows on screen and not in the take; --no-label shows none"
exit "$bad"
