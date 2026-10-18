# The shroom command against the real app (dev/proposals/cli-agent-skill.md, spec N62): the
# dev app under a throwaway home, asked over its control socket by the command itself.
# Off by default: every recording ask is refused as recording_off. With General's switch on
# (prefs.json, as the app keeps it) and no space yet: not_set_up — the switch's gate passed
# and the core answered. Also: the socket is this user's only (600).
set -u
cargo build -q -p shroom-cli || { echo "FAIL the command didn't build"; exit 1; }
cli=target/debug/shroom-cli
home="$(getconf DARWIN_USER_TEMP_DIR)cli-socket"
failed=0
check() { if [ "$2" = "$3" ]; then echo "ok   $1 ($2)"; else echo "FAIL $1: wanted $3, got ${2:-nothing}"; failed=1; fi; }
ask() { SHROOM_HOME="$home" "$cli" record "$1" --no-launch --json | sed -n 's/.*"error":"\([a-z_]*\)".*/\1/p'; }
launch() {
  open -n "$MAC_APP" --args --home "$home"
  i=0
  while [ ! -S "$home/control.sock" ] && [ "$i" -lt 120 ]; do sleep 0.5; i=$((i + 1)); done
  [ -S "$home/control.sock" ] || { echo "FAIL the app made no control socket"; tail -n 30 "$home/logs/run.log" 2>/dev/null; exit 1; }
  sleep 2
}
stop_app() { pkill -f "$MAC_APP/Contents/MacOS/" 2>/dev/null; sleep 2; }

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
exit "$failed"
