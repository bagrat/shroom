# Does the bubble stay in place while the Spaces swipe? Bagrat saw it vanish and come back
# on every switch into or out of a full-screen Space (2026-10-17). The hosted Mac's camera
# (dev/mac/camera.sh), TextEdit full screen, then the bubble's window as shipped (stationary
# on all Spaces, B187) and as 0.1.21 had it (not stationary): for each, the dev app's
# --bubble-probe, and a swipe out of the full-screen Space and back, the screen shot as fast
# as it goes through each swipe. The shots and a clip of each swipe land in the evidence
# (bubble-swipe/<variant>/), for the eye: whether the bubble stays through the swipe (on
# 2026-10-17 the stationary one stayed in every shot; the other was gone from the desktop
# once shown on the full-screen Space). The shipped one's probe verdict (the picture still
# moves at its end, with OBS's own window hidden) is the scenario's.
set -u
. dev/mac/lib.sh
bash dev/mac/camera.sh up || exit 1
S="$MAC_OUT/bubble-swipe"
mkdir -p "$S"
se() { limit 30 osascript -e "tell application \"System Events\" to $1" >>"$S/keys.txt" 2>&1; }
allow_soon() { (sleep 4; osascript -e 'tell application "System Events" to tell process "UserNotificationCenter" to click button "Allow" of window 1' >/dev/null 2>&1) & }

# Shoots the screen as fast as it goes for 3 s around a key, then makes a clip of it.
swipe() { # <dir> <name> <key code>
  local d="$1/$2" i=0
  mkdir -p "$d"
  ( while [ $i -lt 24 ]; do screencapture -x -t jpg "$d/$(printf %03d $i).jpg" >/dev/null 2>&1; i=$((i + 1)); done ) &
  local shots=$!
  sleep 0.3
  se "key code $3 using {control down}"
  wait $shots
  ffmpeg -hide_banner -nostdin -y -loglevel error -framerate 8 -i "$d/%03d.jpg" -vf "scale=1024:-2" -c:v libx264 -pix_fmt yuv420p "$1/$2.mp4"
}

printf 'A full-screen Space for the bubble to cross.\n' >"$S/space.txt"
open -a TextEdit "$S/space.txt"
sleep 3
allow_soon
se 'keystroke "f" using {control down, command down}'
sleep 4
# OBS's own window plays the pattern too: hidden, so only the bubble moves where it sits.
se 'set visible of process "OBS" to false'

fails=0
for v in shipped not-stationary; do
  args=()
  case $v in
    not-stationary) args=(--not-stationary) ;;
  esac
  d="$S/$v"
  mkdir -p "$d"
  open -n "$MAC_APP" --args --bubble-probe "$d/probe.json" --hold 30 ${args[@]+"${args[@]}"}
  for _ in $(seq 1 100); do [ -e "$d/probe.ready" ] && break; sleep 0.2; done
  [ -e "$d/probe.ready" ] || { note fail "bubble-swipe: $v" "the bubble never showed"; fails=$((fails + 1)); continue; }
  swipe "$d" 1-out 123   # ⌃← out of the full-screen Space
  sleep 1
  swipe "$d" 2-in 124    # ⌃→ back into it
  for _ in $(seq 1 200); do [ -s "$d/probe.json" ] && break; sleep 0.2; done
  if node -e 'const r = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"));
    console.log(`'"$v"': moving ${r.moving}; change ${(r.samples || []).map(s => s.change == null ? "-" : s.change.toFixed(1)).join(" ")}`);
    process.exit(r.ok ? 0 : 1)' "$d/probe.json"; then
    note pass "bubble-swipe: $v" "the picture still moves after the swipes"
  elif [ $v = shipped ]; then
    note fail "bubble-swipe: $v" "the picture stopped, or the bubble left"
    fails=$((fails + 1))
  else
    note info "bubble-swipe: $v" "the picture stopped, or the bubble left"
  fi
done
osascript -e 'tell application "TextEdit" to quit saving no' >/dev/null 2>&1 || pkill -x TextEdit
[ $fails -eq 0 ]
