# The shroom command against the real app (spec N62, DECISIONS B215–B217): the command the
# dev bundle carries (Contents/MacOS/shroom-cli), asking the dev app over its control socket.
#
#   1. It follows its app: `status` names the dev app (am.shroom.dev) from the bundle it's in.
#   2. Under a throwaway home: the socket is this user's only (600); with no space yet every
#      recording ask is not_set_up — set up first: Settings, where the switch is, needs a
#      space (B160, B216). Given a space (an invented member's; its host never resolves),
#      with General's switch off (the default) every ask is recording_off; with it on
#      (prefs.json, as the app keeps it), a pause with nothing recording is the core's own
#      no (no_take, which the command says as “Nothing is recording.”, code refused) — the
#      switch's gate passed and the core answered. Nothing is recorded.
#   3. The app not running: `record start` opens it (open -b, the app it ships in) and waits
#      for its socket — the dev app's own folder, nothing set up there on a fresh runner:
#      not_set_up.
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
for op in start pause resume stop; do check "record $op, no space yet" "$(ask "$op")" not_set_up; done
stop_app

printf '%s\n' '{"account_id":"acc","label":"v","subdomain":"x","host":"cli-socket.invalid","bucket":"shroom-space","role":"Member","member_id":"m_AbCdEfGhIjKlMnOp","owner_name":"Ann"}' >"$home/space.json"
chmod 600 "$home/space.json"
launch
for op in start pause resume stop; do check "record $op, a space, the switch off (the default)" "$(ask "$op")" recording_off; done
stop_app

printf '%s\n' '{"source":null,"show_yourself":true,"mic":null,"quality":"normal","bubble_at":null,"setup_chosen":false,"agent_recording":true}' >"$home/prefs.json"
chmod 600 "$home/prefs.json"
launch
said=$(SHROOM_HOME="$home" "$cli" record pause --no-launch --json | sed -n 's/.*"message":"\([^"]*\)".*/\1/p')
check "record pause, the switch on, nothing recording: the core answered" "$said" "Nothing is recording."
stop_app
grep -E "control:" "$home/logs/run.log" | tail -n 12

# Not running: the command opens the app it ships in (whichever copy LaunchServices keeps
# for am.shroom.dev) and waits for it to listen.
any_app='\.app/Contents/MacOS/shroom( |$)'
pkill -f "$any_app" 2>/dev/null; sleep 2
check "record start opens the app, which answers (nothing set up there)" "$("$cli" record start --json | code_of)" not_set_up
check "and the app is running" "$(pgrep -f "$any_app" >/dev/null && echo yes)" yes
pkill -f "$any_app" 2>/dev/null; sleep 2
exit "$failed"
