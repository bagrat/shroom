#!/bin/bash
# The Mac in the loop, on the Mac: what .github/workflows/mac.yml runs on a GitHub-hosted
# runner to try a branch's dev app the way it can only be tried on macOS.
#
#   prepare <bundle>      the dev app the workflow built is granted the screen, the
#                         microphone and the camera, and its window-picker approval is
#                         written — before anything on this Mac has captured
#   run <scenario…>       each scenario (dev/mac/scenarios/<name>.sh) with the screen
#                         recorded and the windows listed every other second while it runs
#   ssh-up <key file>     an SSH door for the session that asked: this user's own sshd on
#                         loopback, behind a quick tunnel; connect.json says where
#   hold <minutes>        keeps the Mac, recording, until the session lets go or time is up
#   shot                  a still of the screen; prints its path (for a session over SSH)
#   release               lets go of a held Mac (for a session over SSH)
#   summary               the verdicts as one table, and a line that counts the noes; fails
#                         only when nothing reported at all
#
# Everything lands in $RUNNER_TEMP/mac, which the workflow uploads: summary.md, results.tsv
# and a folder per scenario — video.mp4, output.log, windows.log, load.txt and whatever it
# wrote.
#
# What the grants and the approval are, and why they are written this way, was asked of a
# runner first: dev/mac/proof.sh (retired 2026-10-13; at tag v0.1.5), and the commits that brought it.
#
# It runs only on a GitHub-hosted runner (lib.sh refuses anywhere else).
set -u

HERE=$(cd "$(dirname "$0")" && pwd)
ROOT=$(cd "$HERE/../.." && pwd)
TITLE="The dev app on a Mac"
. "$HERE/lib.sh"
DOOR="$RUNNER_TEMP/mac-ssh"         # the door's own files: never uploaded
CONNECT="$RUNNER_TEMP/mac-connect"  # where it is: uploaded for the session, by itself

# ── prepare ───────────────────────────────────────────────────────────────────────────
prepare() {
  BUNDLE=$(cd "$1" && pwd)
  printf '%s\n' "$BUNDLE" >"$OUT/bundle"
  CLIENT=$(plutil -extract CFBundleIdentifier raw -o - "$BUNDLE/Contents/Info.plist")
  local requirement
  requirement=$(requirement_of "$BUNDLE")
  CSREQ=$(requirement_hex "$requirement")
  codesign -dvv "$BUNDLE" >"$OUT/signature.txt" 2>&1
  note info "the dev app" "$CLIENT — $requirement"
  if [ -z "$CSREQ" ]; then
    note fail "the dev app's requirement" "none could be read"
    return 1
  fi
  write_grant "$USER_DB" kTCCServiceMicrophone no
  write_grant "$USER_DB" kTCCServiceCamera no
  write_grant "$USER_DB" kTCCServiceScreenCapture no
  write_grant "$SYS_DB" kTCCServiceScreenCapture yes
  # The whole entry, under the bundle's file URL, before anything has captured: a bare date
  # is replaced by an alerted entry at the app's first capture.
  approve_picker >>"$GRANT_LOG" 2>&1
  killall cfprefsd replayd >>"$GRANT_LOG" 2>&1
  sleep 2
  plutil -p "$APPROVALS" >"$OUT/approvals.txt" 2>&1
  if grep -qF "file://$BUNDLE/" "$OUT/approvals.txt"; then
    note pass "the window-picker approval" "written"
  else
    note fail "the window-picker approval" "not in the file: $(tail -n 2 "$GRANT_LOG")"
  fi
}

# ── the watch ─────────────────────────────────────────────────────────────────────────
# The runner's own recorder and, every other second, the windows on screen — into <dir>,
# with what the Mac was busy with as it began (load.txt). Both are kept light: the Mac has
# three cores, and what it spends watching it can't spend on the app.
watch_start() {
  : >"$1/windows.log"
  # MAC_WATCH=0 (a Tart clone): no screen video — the recorder there has no grant of its
  # own, and the heartbeat's verdict is read from the published stream, not this video.
  if [ "${MAC_WATCH:-1}" = 0 ]; then WATCH=""; SAMPLER=""; return; fi
  { uptime; top -l 2 -n 12 -o cpu -stats pid,command,cpu,mem | tail -n 13; } >"$1/load.txt" 2>&1 &
  ffmpeg -hide_banner -nostdin -y -f avfoundation -capture_cursor 1 -framerate 30 -i "Capture screen 0:none" -t 3900 \
    -vf "fps=12" -c:v libx264 -preset ultrafast -crf 26 -pix_fmt yuv420p -movflags +faststart -an \
    "$1/video.mp4" >"$1/video.log" 2>&1 &
  WATCH=$!
  (
    while :; do
      {
        echo "@ $(date -u +%H:%M:%S)"
        windows
      } >>"$1/windows.log" 2>&1
      sleep 2
    done
  ) &
  SAMPLER=$!
  sleep 2
}

watch_stop() {
  [ -n "${WATCH:-}" ] || return 0
  kill "$SAMPLER" 2>/dev/null
  wait "$SAMPLER" 2>/dev/null
  kill -INT "$WATCH" 2>/dev/null
  finish "$WATCH" 20
}

# ── run ───────────────────────────────────────────────────────────────────────────────
run_one() {
  local name=$1 script="$HERE/scenarios/$1.sh" dir="$OUT/$1" seconds=${MAC_LIMIT:-600} ended said
  mkdir -p "$dir"
  if [ ! -f "$script" ]; then
    note fail "$name" "this branch has no dev/mac/scenarios/$name.sh"
    return
  fi
  watch_start "$dir"
  (cd "$ROOT" && MAC_APP="$BUNDLE" SHROOM_APP="$BUNDLE" MAC_OUT="$dir" limit "$seconds" bash "$script") >"$dir/output.log" 2>&1
  ended=$?
  sleep 1
  if alert_up; then
    screencapture -x "$dir/alert.png" >/dev/null 2>&1
    note info "$name: a system alert was left on screen" "alert.png"
  fi
  watch_stop
  # Anything of the app's still running is ended, so the next scenario starts clean.
  pgrep -f "$BUNDLE/Contents/MacOS/" | while read -r pid; do kill "$pid" 2>/dev/null; done
  said=$(tail -n 3 "$dir/output.log" | tr '\n' ' ' | cut -c1-320)
  case $ended in
    0) note pass "$name" "$said" ;;
    124) note fail "$name" "stopped at its $seconds s limit — $said" ;;
    *) note fail "$name" "ended with $ended — $said" ;;
  esac
  [ "${MAC_WATCH:-1}" = 0 ] || judge "$name: the recording of it" "$dir/video.mp4" 3
}

run() {
  BUNDLE=$(cat "$OUT/bundle" 2>/dev/null)
  if [ ! -d "${BUNDLE:-}" ]; then
    note fail "run" "nothing was prepared"
    return 1
  fi
  # The build has only just ended, and the Mac is still catching its breath: a window
  # opened now paints seconds late (a drive's own four seconds can pass on a blank one).
  # Up to forty seconds for the load to come down.
  local name waited=0
  while [ "$waited" -lt 40 ] && awk -v l="$(sysctl -n vm.loadavg | awk '{ print $2 }')" 'BEGIN { exit !(l > 4) }'; do
    sleep 5
    waited=$((waited + 5))
  done
  note info "the Mac's load as the scenarios began" "$(sysctl -n vm.loadavg), after waiting $waited s"
  for name in "$@"; do run_one "$name"; done
  # What each scenario said is in the table; `summary` is where the job passes or fails.
  return 0
}

# ── the SSH door ──────────────────────────────────────────────────────────────────────
# This user's own sshd, started from the job — so what a session runs through it stands
# where the job's steps stand: the same login session, the same grants. It listens on
# loopback only, takes the one key the session made for this run and nothing else, and is
# reached through a quick tunnel whose address is in connect.json. The runner holds no
# secret, and is thrown away after the job.
ssh_up() {
  local key me host waited=0
  key=$(head -n 1 "$1" 2>/dev/null)
  me=$(id -un)
  if ! printf '%s' "$key" | grep -Eq '^ssh-ed25519 [A-Za-z0-9+/=]+( [A-Za-z0-9@._-]+)?$'; then
    note fail "the SSH door" "no usable public key came with the request (an ssh-ed25519 one)"
    return 1
  fi
  mkdir -p "$DOOR" "$CONNECT" && chmod 700 "$DOOR"
  printf '%s\n' "$key" >"$DOOR/authorized_keys"
  chmod 600 "$DOOR/authorized_keys"
  ssh-keygen -q -t ed25519 -N '' -C mac-loop -f "$DOOR/host_key"
  cat >"$DOOR/sshd_config" <<EOF
Port 2222
ListenAddress 127.0.0.1
HostKey $DOOR/host_key
PidFile $DOOR/sshd.pid
AuthorizedKeysFile $DOOR/authorized_keys
AllowUsers $me
PubkeyAuthentication yes
PasswordAuthentication no
KbdInteractiveAuthentication no
UsePAM no
StrictModes no
Subsystem sftp /usr/libexec/sftp-server
SetEnv PATH="$PATH" GITHUB_ACTIONS=true RUNNER_ENVIRONMENT=github-hosted RUNNER_TEMP="$RUNNER_TEMP" GITHUB_WORKSPACE="$ROOT" CARGO_INCREMENTAL=0 HOMEBREW_NO_AUTO_UPDATE=1
EOF
  # Should macOS answer for sshd itself rather than for the runner's agent, it is granted too.
  for service in kTCCServiceScreenCapture kTCCServiceAccessibility kTCCServicePostEvent kTCCServiceListenEvent kTCCServiceMicrophone kTCCServiceSystemPolicyAllFiles; do
    grant_path "$service" /usr/sbin/sshd
  done
  grant_path kTCCServiceAppleEvents /usr/sbin/sshd com.apple.systemevents

  nohup /usr/sbin/sshd -D -e -f "$DOOR/sshd_config" >"$DOOR/sshd.log" 2>&1 &
  while ! nc -z 127.0.0.1 2222 2>/dev/null && [ "$waited" -lt 15 ]; do sleep 1; waited=$((waited + 1)); done
  if ! nc -z 127.0.0.1 2222 2>/dev/null; then
    note fail "the SSH door" "sshd isn't listening: $(tail -n 2 "$DOOR/sshd.log")"
    return 1
  fi
  nohup cloudflared tunnel --no-autoupdate --url ssh://127.0.0.1:2222 >"$DOOR/cloudflared.log" 2>&1 &
  waited=0
  while [ "$waited" -lt 60 ]; do
    host=$(grep -Eo 'https://[a-z0-9-]+\.trycloudflare\.com' "$DOOR/cloudflared.log" | head -n 1 | sed 's#https://##')
    [ -z "$host" ] || break
    sleep 1
    waited=$((waited + 1))
  done
  if [ -z "${host:-}" ]; then
    note fail "the SSH door" "no tunnel address: $(tail -n 2 "$DOOR/cloudflared.log")"
    return 1
  fi
  printf '{"host":"%s","user":"%s","host_key":"%s","workspace":"%s","out":"%s"}\n' \
    "$host" "$me" "$(cut -d' ' -f1,2 "$DOOR/host_key.pub")" "$ROOT" "$OUT" >"$CONNECT/connect.json"
  note pass "the SSH door" "open at $host"
}

hold() {
  local minutes=$1 dir="$OUT/hold" began end
  mkdir -p "$dir"
  rm -f "$OUT/release"
  watch_start "$dir"
  began=$(date +%s)
  end=$((began + minutes * 60))
  while [ ! -e "$OUT/release" ] && [ "$(date +%s)" -lt "$end" ]; do sleep 2; done
  watch_stop
  if [ -e "$OUT/release" ]; then
    note info "held for a session" "let go after $(( $(date +%s) - began )) s"
  else
    note info "held for a session" "its $minutes minutes were up"
  fi
  [ ! -s "$DOOR/sshd.log" ] || grep -E 'Accepted|Failed|Invalid|refused' "$DOOR/sshd.log" | sed -E 's/SHA256:[A-Za-z0-9+/]+/SHA256:…/' >"$dir/door.log"
  judge "the recording of the held session" "$dir/video.mp4" 3
}

shot() {
  mkdir -p "$OUT/shots"
  local file
  file="$OUT/shots/$(date -u +%H%M%S).png"
  screencapture -x -C "$file" && echo "$file"
}

release() {
  : >"$OUT/release"
  echo "let go"
}

# The job is green whenever the Mac ran what was asked: a scenario's no is the table's to
# say, never the job's. Only sessions ask for this Mac, and they read the table; a red job
# for every no was an email for every no.
verdicts() {
  summary results.tsv || return 1
  local noes total line
  noes=$(grep -c '^fail' "$OUT/results.tsv")
  total=$(grep -Ec '^(pass|fail)' "$OUT/results.tsv")
  if [ "$noes" -eq 0 ]; then
    line="All $total checks passed."
  else
    line="$noes of $total checks said no: $(awk -F'\t' '$1 == "fail" { printf "%s%s", sep, $2; sep = ", " }' "$OUT/results.tsv")."
    echo "::warning title=The Mac's verdict::$line"
  fi
  printf '\n**%s** The job is green either way — this table is the verdict.\n' "$line" | tee -a "$OUT/summary.md" >>"${GITHUB_STEP_SUMMARY:-/dev/null}"
  echo "$line"
}

case ${1:-} in
  prepare) [ -d "${2:-}" ] || { echo "usage: mac.sh prepare <the dev app's bundle>" >&2; exit 64; }; prepare "$2" ;;
  run) shift; run "$@" ;;
  ssh-up) ssh_up "${2:-}" ;;
  hold) hold "${2:-10}" ;;
  shot | release) "$1" ;;
  summary) verdicts ;;
  *) echo "usage: mac.sh prepare <bundle> | run <scenario…> | ssh-up <key file> | hold <minutes> | shot | release | summary" >&2; exit 64 ;;
esac
