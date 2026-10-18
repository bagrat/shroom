# The shroom command against the real app (spec N62, DECISIONS B215–B217): the command the
# dev bundle carries (Contents/MacOS/shroom-cli), asking the dev app over its control socket.
#
#   1. It follows its app: `status` names the dev app (am.shroom.dev) from the bundle it's in.
#   2. Under a throwaway home: the socket is this user's only (600); with General's switch
#      off (the default) every recording ask is recording_off; with it on (prefs.json, as the
#      app keeps it) and no space yet, not_set_up — the switch's gate passed and the core
#      answered.
#   3. The app not running: `record start` opens it (open -b, the app it ships in) and waits
#      for its socket — the dev app's own folder, nothing set up there on a fresh runner, the
#      switch off: recording_off.
set -u
cli="$MAC_APP/Contents/MacOS/shroom-cli"
[ -x "$cli" ] || { echo "FAIL the dev bundle carries no shroom command at $cli"; exit 1; }
home="$(getconf DARWIN_USER_TEMP_DIR)cli-socket"
failed=0
check() { if [ "$2" = "$3" ]; then echo "ok   $1 ($2)"; else echo "FAIL $1: wanted $3, got ${2:-nothing}"; failed=1; fi; }
code_of() { sed -n 's/.*"error":"\([a-z_]*\)".*/\1/p'; }
ask() { SHROOM_HOME="$home" "$cli" record "$1" --no-launch --json | code_of; }
launch() {
  open -n "$MAC_APP" --args --home "$home"
  i=0
  while [ ! -S "$home/control.sock" ] && [ "$i" -lt 120 ]; do sleep 0.5; i=$((i + 1)); done
  [ -S "$home/control.sock" ] || { echo "FAIL the app made no control socket"; tail -n 30 "$home/logs/run.log" 2>/dev/null; exit 1; }
  sleep 2
}
stop_app() { pkill -f "$MAC_APP/Contents/MacOS/shroom( |$)" 2>/dev/null; sleep 2; }

check "status names the app it ships in" "$("$cli" status --json | sed -n 's/.*"id":"\([a-z.]*\)".*/\1/p')" am.shroom.dev

rm -rf "$home" && mkdir -p "$home" && chmod 700 "$home"
launch
check "the socket is this user's only" "$(stat -f %Lp "$home/control.sock")" 600
for op in start pause resume stop; do check "record $op, with the switch off (the default)" "$(ask "$op")" recording_off; done
stop_app

printf '%s\n' '{"source":null,"show_yourself":true,"mic":null,"quality":"normal","bubble_at":null,"setup_chosen":false,"agent_recording":true}' >"$home/prefs.json"
chmod 600 "$home/prefs.json"
launch
check "record start, the switch on, no space yet" "$(ask start)" not_set_up
stop_app
grep -E "control:" "$home/logs/run.log" | tail -n 12

# Not running: the command opens the app it ships in (whichever copy LaunchServices keeps
# for am.shroom.dev) and waits for it to listen.
any_app='\.app/Contents/MacOS/shroom( |$)'
pkill -f "$any_app" 2>/dev/null; sleep 2
check "record start opens the app, which answers (the switch off)" "$("$cli" record start --json | code_of)" recording_off
check "and the app is running" "$(pgrep -f "$any_app" >/dev/null && echo yes)" yes
pkill -f "$any_app" 2>/dev/null; sleep 2
exit "$failed"
