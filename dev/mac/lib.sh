# What dev/mac's runner-side script (mac.sh) uses — sourced, never run.
#
# TEST MACHINES ONLY. These write permission grants, the window-picker approval and files
# under the runner's temp dir, and they end processes — things nobody wants done to their
# own Mac — so sourcing this anywhere but a GitHub-hosted macOS runner, or a Tart clone that
# tests/l5/heartbeat-vm.sh has marked (SIP off, /etc/shroom-l5-clone), ends the script.
#
# The caller may set OUT (where everything lands) and TITLE (the verdict table's) before
# sourcing; write_grant reads CLIENT, CSREQ and GRANT_LOG; approve_picker reads BUNDLE.

hosted_runner() { [ "${GITHUB_ACTIONS:-}" = true ] && [ "${RUNNER_ENVIRONMENT:-}" = github-hosted ]; }
marked_clone() { [ -f /etc/shroom-l5-clone ] && /usr/bin/csrutil status 2>/dev/null | grep -q 'status: disabled\.'; }
if [ "$(uname -s)" != Darwin ] || [ -z "${RUNNER_TEMP:-}" ] || ! { hosted_runner || marked_clone; }; then
  echo "dev/mac: this runs only on a GitHub-hosted macOS runner or a marked Tart clone." >&2
  exit 64
fi

: "${OUT:=$RUNNER_TEMP/mac}"
mkdir -p "$OUT"
USER_DB="$HOME/Library/Application Support/com.apple.TCC/TCC.db"
SYS_DB="/Library/Application Support/com.apple.TCC/TCC.db"
APPROVALS="$HOME/Library/Group Containers/group.com.apple.replayd/ScreenCaptureApprovals.plist"
GRANT_LOG="$OUT/grant.log"

# One verdict: pass | fail | info, a name, what happened.
note() {
  printf '%s\t%s\t%s\n' "$1" "$2" "$(printf '%s' "$3" | tr '|\t\n' '/  ')" | tee -a "$OUT/results.tsv"
}

# macOS has no timeout(1): a command that outlives its seconds is ended by the alarm.
with_timeout() {
  perl -e 'alarm shift; exec @ARGV or die "exec: $!"' "$@"
}

# limit <seconds> <command…> — the command in a process group of its own; at the limit the
# whole group is ended and the answer is 124.
limit() {
  perl -e '
    my $seconds = shift;
    my $pid = fork();
    if (!$pid) { setpgrp(0, 0); exec @ARGV or exit 127; }
    local $SIG{ALRM} = sub { kill "TERM", -$pid; sleep 3; kill "KILL", -$pid; waitpid($pid, 0); exit 124; };
    alarm $seconds;
    waitpid($pid, 0);
    exit($? & 127 ? 128 + ($? & 127) : $? >> 8);' "$@"
}

# A grant for a program known by its path, in both tables (GitHub's image grants its own
# agent this way). <object> is the app it may send events to, for kTCCServiceAppleEvents.
grant_path() {
  local service=$1 path=$2 object=${3:-UNUSED} kind=NULL sql
  [ "$object" = UNUSED ] || kind=0
  sql="INSERT OR REPLACE INTO access (service, client, client_type, auth_value, auth_reason, auth_version, indirect_object_identifier_type, indirect_object_identifier, flags, last_modified) VALUES ('$service', '$path', 1, 2, 4, 1, $kind, '$object', 0, CAST(strftime('%s','now') AS INTEGER));"
  sqlite3 "$USER_DB" "$sql" >>"$GRANT_LOG" 2>&1
  sudo sqlite3 "$SYS_DB" "$sql" >>"$GRANT_LOG" 2>&1
}

# A bundle's designated requirement (an ad hoc signature's is implicit, printed behind a
# '#'), and that requirement as the hex of the bytes the grants table holds.
requirement_of() {
  codesign -d -r- "$1" 2>/dev/null | sed -n 's/^#* *designated => //p'
}

requirement_hex() {
  local bin
  bin=$(mktemp "$OUT/requirement.XXXXXX")
  printf '%s' "$1" | csreq -r- -b "$bin" >/dev/null 2>&1
  xxd -p "$bin" | tr -d '\n'
  rm -f "$bin"
}

write_grant() {
  local db=$1 service=$2 as_root=$3 where wrote
  local sql="INSERT OR REPLACE INTO access (service, client, client_type, auth_value, auth_reason, auth_version, csreq, indirect_object_identifier, flags, last_modified) VALUES ('$service', '$CLIENT', 0, 2, 4, 1, X'$CSREQ', 'UNUSED', 0, CAST(strftime('%s','now') AS INTEGER));"
  if [ "$as_root" = yes ]; then
    where="the system's grants"
    sudo sqlite3 "$db" "$sql" >>"$GRANT_LOG" 2>&1
    wrote=$?
  else
    where="the user's grants"
    sqlite3 "$db" "$sql" >>"$GRANT_LOG" 2>&1
    wrote=$?
  fi
  if [ "$wrote" -eq 0 ]; then
    note pass "write $service into $where" "written"
  else
    note fail "write $service into $where" "$(tail -n 2 "$GRANT_LOG")"
  fi
}

# The windows on screen, one "owner: title [layer]" a line — an alert is somebody's window.
windows() {
  osascript -l JavaScript -e 'ObjC.import("CoreGraphics");
    const list = ObjC.deepUnwrap(ObjC.castRefToObject($.CGWindowListCopyWindowInfo($.kCGWindowListOptionOnScreenOnly, 0)));
    list.map(w => w.kCGWindowOwnerName + ": " + (w.kCGWindowName || "") + " [" + w.kCGWindowLayer + "]").join("\n")' 2>&1
}

# Is a system alert on screen? Its window says so; the process that draws it lingers after.
alert_up() {
  windows | grep -q '^UserNotificationCenter:'
}

# The window-picker approval for the bundle, in the form the recorder daemon itself keeps
# for an app it never asks about again (what it made of the image's own entry for the
# runner's agent), under the key it uses for an app: the bundle's file URL.
approve_picker() {
  python3 - "$APPROVALS" "file://$BUNDLE/" <<'EOF'
import datetime, os, plistlib, sys

path, key = sys.argv[1], sys.argv[2]
try:
    with open(path, "rb") as f:
        approvals = plistlib.load(f)
except FileNotFoundError:
    approvals = {}
never = datetime.datetime(3024, 1, 1)
approvals[key] = {
    "kScreenCaptureAlertableUsageCount": 1,
    "kScreenCaptureApprovalLastAlerted": never,
    "kScreenCaptureApprovalLastUsed": datetime.datetime.now(datetime.timezone.utc).replace(tzinfo=None, microsecond=0),
    "kScreenCapturePrivacyHintDate": never,
}
os.makedirs(os.path.dirname(path), exist_ok=True)
with open(path, "wb") as f:
    plistlib.dump(approvals, f, fmt=plistlib.FMT_BINARY)
EOF
}

# Waits for a recorder to end by itself, then asks it to, then makes it.
finish() {
  local pid=$1 waited=0
  while kill -0 "$pid" 2>/dev/null && [ "$waited" -lt "$2" ]; do sleep 1; waited=$((waited + 1)); done
  if kill -0 "$pid" 2>/dev/null; then
    kill -INT "$pid" 2>/dev/null
    sleep 5
    kill -KILL "$pid" 2>/dev/null
  fi
  wait "$pid" 2>/dev/null
}

# small <out.mp4> <ffmpeg input and filter arguments…>
small() {
  ffmpeg -hide_banner -nostdin -y -loglevel error "${@:2}" -c:v libx264 -preset veryfast -crf 26 -pix_fmt yuv420p -movflags +faststart -an "$1"
}

# Is it a picture of something? Frames, seconds (ten at least, or as told), and the widest
# spread of brightness over one frame a second — a blank or one-colour recording has none.
judge() {
  local name=$1 file=$2 least=${3:-10} seconds frames spread
  if [ ! -s "$file" ]; then
    note fail "$name" "no file"
    return 1
  fi
  if ! command -v ffprobe >/dev/null; then
    note info "$name" "$(( $(stat -f %z "$file") / 1024 )) KB, unread: no ffprobe here"
    return 0
  fi
  seconds=$(ffprobe -v error -show_entries format=duration -of csv=p=0 "$file" 2>/dev/null)
  frames=$(ffprobe -v error -count_frames -select_streams v:0 -show_entries stream=nb_read_frames -of csv=p=0 "$file" 2>/dev/null)
  spread=$(ffmpeg -hide_banner -nostdin -v error -i "$file" -vf "fps=1,signalstats,metadata=mode=print:file=-" -f null - 2>/dev/null |
    awk -F= '/YMIN/ { low = $2 } /YMAX/ { if ($2 - low > widest) widest = $2 - low } END { print widest + 0 }')
  local said="${seconds:-0} s, ${frames:-0} frames, brightness spread ${spread:-0} of 255, $(( $(stat -f %z "$file") / 1024 )) KB"
  if awk -v s="${seconds:-0}" -v f="${frames:-0}" -v w="${spread:-0}" -v least="$least" 'BEGIN { exit !(s >= least && f >= 20 && w >= 60) }'; then
    note pass "$name" "$said"
  else
    note fail "$name" "$said"
    return 1
  fi
}

summary() {
  [ -s "$OUT/results.tsv" ] || { echo "No step reported anything." | tee -a "${GITHUB_STEP_SUMMARY:-/dev/null}"; return 1; }
  {
    echo "### ${TITLE:-The Mac} — macOS $(sw_vers -productVersion), $(uname -m)"
    echo
    echo "| | asked | what happened |"
    echo "|---|---|---|"
    while IFS=$'\t' read -r verdict name said; do
      case $verdict in pass) mark="✅" ;; fail) mark="❌" ;; *) mark="·" ;; esac
      echo "| $mark | $name | $said |"
    done <"$OUT/results.tsv"
  } | tee "$OUT/summary.md" >>"${GITHUB_STEP_SUMMARY:-/dev/null}"
  [ -s "$OUT/$1" ]
}
