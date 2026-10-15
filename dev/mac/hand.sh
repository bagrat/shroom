#!/bin/bash
# By hand on a held Mac (dev/mac/MANUAL.md §1): the dev app under a throwaway home against
# the Cloudflare fake, TextEdit as the other app, and the pointer and keys a person would
# use. Run from the repo's root there — `loop.sh ssh 'dev/mac/hand.sh …'` does.
#
#   fake [state]            the fake in a state (found, the default; fresh, …), kept as the
#                           current one: seed and start use it
#   seed <home>             a set-up Mac in that home (the launch drive seeds it, then exits)
#   start <home> [args…]    the app opened as a person opens it, on that home
#   quit                    ends the dev app
#   textedit                TextEdit on a note of its own, in front
#   fullscreen on|off       TextEdit's window in or out of full screen
#   front                   the app in front
#   dock                    the dev app's kind, as LaunchServices has it: UIElement is no
#                           Dock icon, Foreground is one
#   watch <seconds>         looks at the dev app's kind and the app in front every few ms
#                           for that long, then counts what it saw (start a launch with &)
#   wins                    the windows on screen, with their frames
#   tray                    the mushroom's position and size
#   click <x> <y> | move <x> <y>
#   key esc|return|cmdtab|missioncontrol|selectall|paste
#   type <text>             typed as keystrokes
#   still <name>            the screen as $RUNNER_TEMP/mac/shots/<name>.png (loop.sh get it)
#   paste-token             a random, made-up token pasted where the caret is; the
#                           clipboard is cleared after
#   allow                   answers a consent prompt that a `tell application` raised
#
# RUNNER ONLY: it ends the dev app and drives the pointer. Nothing real goes in — the fake
# takes any token, and the one pasted here is random and never kept.
set -u

if [ "${GITHUB_ACTIONS:-}" != true ] || [ "${RUNNER_ENVIRONMENT:-}" != github-hosted ] || [ "$(uname -s)" != Darwin ] || [ -z "${RUNNER_TEMP:-}" ]; then
  echo "dev/mac: this runs only on a GitHub-hosted macOS runner." >&2
  exit 64
fi

APP=$(cat "$RUNNER_TEMP/mac/bundle")
W="$RUNNER_TEMP/hand"
SHOTS="$RUNNER_TEMP/mac/shots"
HOMES="$(getconf DARWIN_USER_TEMP_DIR)"
mkdir -p "$W" "$SHOTS"

ep() { node -p 'JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"))[process.argv[2]]' "$W/fake.json" "$1"; }
se() { osascript -e "tell application \"System Events\" to $1"; }
kind() {
  local asn
  asn=$(lsappinfo find bundleid=am.shroom.dev 2>/dev/null)
  [ -z "$asn" ] || lsappinfo info -only ApplicationType "$asn" 2>/dev/null | sed 's/.*=//'
}
in_front() { lsappinfo info -only bundleid "$(lsappinfo front)" 2>/dev/null | sed 's/.*=//'; }

case ${1:-} in
  fake)
    state=${2:-found}
    if [ ! -s "$W/fake-$state.json" ]; then
      # It serves until its input closes: the sleep holds it open past the hold.
      nohup sh -c "sleep 3600 | '$PWD/target/debug/cloudfake' --state '$state' >'$W/fake-$state.json' 2>'$W/fake-$state.err'" >/dev/null 2>&1 &
      for _ in $(seq 1 50); do [ -s "$W/fake-$state.json" ] && break; sleep 0.2; done
    fi
    cp "$W/fake-$state.json" "$W/fake.json"
    cat "$W/fake.json"
    ;;
  seed)
    open -W -n -g "$APP" --args --home "$HOMES$2" --cloud-fake "$(ep api)" "$(ep r2)" --drive launch "$W/$2-seed.json" provisioned
    ls "$HOMES$2"
    ;;
  start)
    home=$2
    shift 2
    open -n "$APP" --args --home "$HOMES$home" --cloud-fake "$(ep api)" "$(ep r2)" "$@"
    ;;
  quit) pkill -f "$APP/Contents/MacOS/" && echo ended || echo "none running" ;;
  textedit)
    mkdir -p "$W/te"
    [ -f "$W/te/note.txt" ] || printf 'The other app.\n' >"$W/te/note.txt"
    open -a TextEdit "$W/te/note.txt"
    ;;
  fullscreen)
    [ "${2:-on}" = on ] && v=true || v=false
    se "tell process \"TextEdit\" to set value of attribute \"AXFullScreen\" of window 1 to $v"
    ;;
  front) in_front ;;
  dock) kind ;;
  watch)
    end=$(($(date +%s) + ${2:-8}))
    while [ "$(date +%s)" -lt "$end" ]; do
      k=$(kind)
      [ -z "$k" ] || echo "kind $k"
      echo "front $(in_front)"
    done | sort | uniq -c
    ;;
  wins)
    osascript -l JavaScript -e 'ObjC.import("CoreGraphics");
      const list = ObjC.deepUnwrap(ObjC.castRefToObject($.CGWindowListCopyWindowInfo($.kCGWindowListOptionOnScreenOnly, 0)));
      list.filter(w => !["Window Server", "Dock", "Control Center"].includes(w.kCGWindowOwnerName))
        .map(w => { const b = w.kCGWindowBounds; return `${w.kCGWindowOwnerName}: "${w.kCGWindowName || ""}" layer ${w.kCGWindowLayer} at ${b.X},${b.Y} ${b.Width}x${b.Height}`; }).join("\n")'
    ;;
  tray) se 'tell process "shroom-dev" to get {position, size} of menu bar item 1 of menu bar 2' ;;
  click) cliclick "c:$2,$3" ;;
  move) cliclick "m:$2,$3" ;;
  key)
    case $2 in
      esc) se 'key code 53' ;;
      return) se 'key code 36' ;;
      cmdtab) se 'key code 48 using command down' ;;
      missioncontrol) se 'key code 160' ;;
      selectall) se 'key code 0 using command down' ;;
      paste) se 'key code 9 using command down' ;;
      *) echo "key esc|return|cmdtab|missioncontrol|selectall|paste" >&2; exit 64 ;;
    esac
    ;;
  type) se "keystroke \"$2\"" ;;
  still) screencapture -x "$SHOTS/$2.png" && echo "$SHOTS/$2.png" ;;
  paste-token)
    printf 'cfat_%s' "$(LC_ALL=C tr -dc A-Za-z0-9 </dev/urandom | head -c 40)" | pbcopy
    se 'key code 9 using command down'
    sleep 0.5
    printf '' | pbcopy
    ;;
  allow) se 'tell process "UserNotificationCenter" to click button "Allow" of window 1' ;;
  *) sed -n '2,32p' "$0" | sed 's/^# \{0,1\}//'; exit 64 ;;
esac
