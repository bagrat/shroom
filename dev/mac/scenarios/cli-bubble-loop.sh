# The command's start with the bubble on, round after round: one early-build e2e lost such a
# take as capture began (2026-10-18; the app came back, its take saved as interrupted at
# 0:00), and the run after passed. Each round goes as the e2e's command leg has it: the app
# freshly opened, `record start` with the count, the take held a few seconds, stopped. Ten
# rounds with the label and five without (`--no-label`), so a failure can be told to be the
# label's or the start's own.
#
# Each round's end is read from the run log (the core's `started`, or how it went wrong, and
# the bubble shown or not). The run log, the command's log and any crash report shroom left
# during the run are kept in the evidence.
#
# The home is a throwaway one with an invented member's space (its host never resolves:
# nothing is uploaded), the bubble on (dev/mac/camera.sh: OBS's virtual camera on the test
# pattern) and General's switch on, as prefs.json keeps them. Each take stays on this
# runner and goes with the home.
set -u
bash dev/mac/camera.sh up || exit 1
OUT=${MAC_OUT:-$(mktemp -d)}
cli="$MAC_APP/Contents/MacOS/shroom-cli"
home="$(getconf DARWIN_USER_TEMP_DIR)cli-bubble-loop"
log="$home/logs/run.log"
began=$(date +%s)
fails=0
app_re="$MAC_APP/Contents/MacOS/shroom( |$)"
alive() { pgrep -f "$app_re" >/dev/null; }
quit() {
  pkill -f "$app_re" 2>/dev/null
  for _ in $(seq 1 20); do alive || break; sleep 0.5; done
}
ask() { SHROOM_HOME="$home" "$cli" record "$@" --no-launch --json; }
field() { sed -n "s/.*\"$1\":\"\\([^\"]*\\)\".*/\\1/p"; }
launch() {
  rm -f "$home/control.sock"
  open -n "$MAC_APP" --args --home "$home"
  for _ in $(seq 1 120); do [ -S "$home/control.sock" ] && break; sleep 0.5; done
  [ -S "$home/control.sock" ]
}
at() { if [ -f "$log" ]; then wc -l <"$log" | tr -d ' '; else echo 0; fi; }
# What the run log says since line $1 about the take's start: `started`, or how it ended.
said() {
  tail -n +"$(($1 + 1))" "$log" 2>/dev/null |
    grep -oE 'core (started|start-failed|started-unasked|capture-died)|recorder start failed: .*|bubble: (shown|the camera didn.t open).*' |
    cut -c1-120 | tr '\n' ';'
}

[ -x "$cli" ] || { echo "NO: the dev bundle carries no shroom command at $cli"; exit 1; }
quit
rm -rf "$home" && mkdir -p "$home" && chmod 700 "$home"
printf '%s\n' '{"account_id":"acc","label":"v","subdomain":"x","host":"cli-bubble-loop.invalid","bucket":"shroom-space","role":"Member","member_id":"m_AbCdEfGhIjKlMnOp","owner_name":"Ann"}' >"$home/space.json"
printf '{"v":1,"at":%s}\n' "$(($(date +%s) * 1000))" >"$home/primed.json"
printf '%s\n' '{"source":null,"show_yourself":true,"mic":null,"quality":"normal","bubble_at":null,"setup_chosen":true,"agent_recording":true}' >"$home/prefs.json"
chmod 600 "$home/space.json" "$home/primed.json" "$home/prefs.json"

round() { # $1: the round's number, then the start's own options
  local n=$1 from j start ended k
  shift
  quit
  # Each round opens shroom fresh, as the e2e's leg does right before its start; the last
  # round's stop couldn't reach its (invented) space, and its popover holds until seen.
  if ! launch; then echo "round $n: NO, the app made no control socket"; fails=$((fails + 1)); return; fi
  from=$(at)
  j=$(ask start "$@")
  start=$(printf '%s' "$j" | field take)
  # The count (3 s), then up to 12 s for capture to begin or fail.
  for k in $(seq 1 30); do
    alive || break
    said "$from" | grep -qE 'core (started|start-failed|started-unasked|capture-died)|recorder start failed' && break
    sleep 0.5
  done
  sleep 3
  if ! alive; then
    ended="the app died"
  else
    ended=$(ask stop | field take)
  fi
  sleep 3
  local what
  what=$(said "$from")
  if [ "$start" = countdown ] && printf '%s' "$what" | grep -q 'core started' && ! printf '%s' "$what" | grep -qE 'start-failed|started-unasked|capture-died|recorder start failed' && [ "$ended" = stopping ]; then
    echo "round $n ($*): ok — $what stop: $ended"
  else
    echo "round $n ($*): NO — start: ${start:-$j}; $what stop: ${ended:-none}"
    fails=$((fails + 1))
  fi
}

echo "── ten rounds: record start, the count, the label"
for i in $(seq 1 10); do round "$i"; done
echo "── five rounds: record start --no-label"
for i in $(seq 11 15); do round "$i" --no-label; done
quit

# The evidence: the run log and the command's log, and any crash report shroom left.
cp "$log" "$OUT/run.log" 2>/dev/null
cp "$home/logs/cli.log" "$OUT/cli.log" 2>/dev/null
crashes=0
for f in "$HOME"/Library/Logs/DiagnosticReports/shroom*.ips /Library/Logs/DiagnosticReports/shroom*.ips; do
  [ -f "$f" ] || continue
  [ "$(stat -f %m "$f")" -ge "$began" ] || continue
  cp "$f" "$OUT/" && crashes=$((crashes + 1))
  echo "crash report: $(basename "$f")"
  sed -n '1,/"threads"/p' "$f" | grep -oE '"(exception|termination|asi|faultingThread)"[^}]*' | head -n 4 | cut -c1-300 | sed 's/^/  /'
done
echo "rounds that failed: $fails of 15; crash reports: $crashes"
[ "$fails" = 0 ] && [ "$crashes" = 0 ] && echo "verdict: fifteen starts from the command with the bubble on, each recorded and stopped"
[ "$fails" = 0 ] && [ "$crashes" = 0 ]
