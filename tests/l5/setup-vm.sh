#!/bin/sh
# L5: setup walked to its end, on a Mac nobody sits at — and all-set's Invite someone
# (DECISIONS B117; law 6 as built, DEVIATIONS #38).
#
# The onboarding drive (app/test/onboarding-drive.mjs) walks a fresh install through the
# welcome, the plan and the permissions chapter, against the L1 fake. Past the plan's last
# step it needs a Mac where shroom may already hear and record — the chapter's warm-up
# captures the screen — and what it reads of the keyboard needs a screen that isn't locked.
# An unattended run on this Mac can count on neither. A throwaway clone of a golden image is
# always logged in and unlocked, and its privacy settings are its own: so the dev build's
# drive runs there, every keyboard check required (SHROOM_HANDS=1), the person in TextEdit
# on a note before each drive:
#
#   1. the microphone and leave to record — given to the dev build in the clone, and seen to
#      work (its own self-test captures the clone's screen for three seconds);
#   2. all-set's Invite someone (`onboarding-drive.mjs invite`): setup is over the moment it
#      is pressed, and in that same turn Settings opens — the Dock icon staying with it (law
#      6) — in front, with the keyboard, and still with it a moment later; and what is typed
#      next is the name: “Who's this for?” arrived with the caret (N28), the drive's keys
#      going where a keyboard's go, never put into the field;
#   3. every path in one command (`onboarding-drive.mjs all`): a fresh account, a taken
#      name, the e-mail gate, the card gate and a space already there, each to all-set and
#      Close; the setup window's doors; and Invite someone once more.
#
# What the clone is given, and only the clone — every line of it goes through `tart exec`,
# never to this Mac — is what tests/l5/take-vm.sh gives its own (the golden images have
# System Integrity Protection off): the dev build (`am.shroom.dev`, as signed here) written
# into the clone's privacy database as allowed to record the screen, and here to use the
# microphone too — what a person does with two switches in System Settings; and macOS's
# second question, raised by the self-test and its asker ended by its PID, so nothing of
# macOS's sits over shroom's windows later. With both given the chapter has nothing to ask
# (a drive never asks: it logs what it would have). Without the microphone's row the walk
# stops at the microphone's step.
#
# Not here: a hand's click on Invite someone (the drive posts the button's action to the
# core, as its page does), and macOS's own asks answered (TESTING-PLAN §8: Bagrat's clicks).
#
# The VM has no window and its sound is off (TESTING-PLAN's quiet guard). The clone is
# deleted after.
#
# Usage: tests/l5/setup-vm.sh
#   needs the debug bundle and the fake:
#   (cd app && npm run build:dev) && cargo build -p shroom-cloudfake --bin cloudfake
# SHROOM_SETUP_IMAGE=<name> clones another golden image (the Sequoia one, `shroom-clean`).
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
APP=target/debug/bundle/macos/shroom-dev.app
FAKE=target/debug/cloudfake
for p in "$APP" "$FAKE"; do
  [ -e "$ROOT/$p" ] || { echo "missing $p — build it first (see the header)"; exit 2; }
done
command -v tart >/dev/null || { echo "tart isn't installed here"; exit 1; }

VM=shroom-l5-setup-$$
WORK=$(mktemp -d)
TREE=/tmp/shroom-setup
cleanup() {
  tart stop "$VM" >/dev/null 2>&1 || true
  tart delete "$VM" >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

tart clone "${SHROOM_SETUP_IMAGE:-shroom-clean-tahoe}" "$VM"
tart run "$VM" --no-graphics --no-audio >/dev/null 2>&1 &
i=0
until tart exec "$VM" true >/dev/null 2>&1; do
  i=$((i + 1)); [ "$i" -gt 90 ] && { echo "the VM never came up"; exit 1; }
  sleep 2
done
vm() { tart exec "$VM" sh -c "$1" </dev/null 2>/dev/null; }

# Logged in, the desktop up, some app holding the keyboard.
i=0
until vm 'pgrep -x Dock >/dev/null && lsappinfo info -only bundleid "$(lsappinfo front)" | grep -q "=\"com\.apple\.finder\""'; do
  i=$((i + 1)); [ "$i" -gt 60 ] && { echo "the VM's desktop never came up"; exit 1; }
  sleep 2
done
# A Mac that has just started is busy with itself for a while: let it settle.
sleep 30
NODE=$(vm 'for n in /opt/homebrew/bin/node /usr/local/bin/node; do [ -x "$n" ] && { echo "$n"; break; }; done' || true)
[ -n "$NODE" ] || { echo "FAIL the clean image has no node to run the drive with"; exit 1; }
echo "macOS $(vm 'sw_vers -productVersion') in the clone"

# The other app, and its window: TextEdit on a note, in front (tests/l5/dock-vm.sh says why).
in_textedit() { vm 'lsappinfo info -only bundleid "$(lsappinfo front)" | grep -q "=\"com\.apple\.TextEdit\""'; }
to_textedit() {
  vm "open -a TextEdit $TREE/note.txt"
  i=0
  until in_textedit; do
    i=$((i + 1)); [ "$i" -gt 30 ] && { echo "TextEdit never came to the front in the VM"; exit 1; }
    sleep 2
  done
}

# The tree as the VM sees it: the bundle, the fake and the runner in the same relative
# places, so the runner runs there unchanged.
vm "mkdir -p $TREE && printf 'A note someone was writing.\n' > $TREE/note.txt"
tar -C "$ROOT" -cf - -L "$APP" "$FAKE" app/test/onboarding-drive.mjs tests/test-machine.mjs | tart exec -i "$VM" tar -C "$TREE" -xf -

fail=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fail=1; }

echo "── 1. the microphone and leave to record, in the clone ──"
# The clone's own privacy databases, through `tart exec` — never this Mac's. The screen's
# row is the one tests/l5/take-vm.sh writes; the microphone's is the same row in the
# person's own database, where macOS keeps that switch. Each says what its switch says, for
# the bundle as it is signed (its identifier and its certificate: the requirement macOS
# itself would write down).
tart exec -i "$VM" tee "$TREE/allow.sh" >/dev/null <<'GUEST'
set -eu
APP=$1
SCREEN="/Library/Application Support/com.apple.TCC/TCC.db"
MIC="$HOME/Library/Application Support/com.apple.TCC/TCC.db"
[ "$(csrutil status)" = "System Integrity Protection status: disabled." ] || { echo "this isn't a golden image's clone"; exit 1; }
REQ=$(codesign -dr - "$APP" 2>&1 | sed -n 's/^designated => //p')
[ -n "$REQ" ] || { echo "the bundle has no designated requirement"; exit 1; }
echo "$REQ" | csreq -r- -b /tmp/shroom-dev.csreq
HEX=$(xxd -p /tmp/shroom-dev.csreq | tr -d '\n')
row() { echo "insert or replace into access (service, client, client_type, auth_value, auth_reason, auth_version, csreq, indirect_object_identifier, flags) values ('$1', 'am.shroom.dev', 0, 2, 4, 1, X'$HEX', 'UNUSED', 0);"; }
sudo sqlite3 "$SCREEN" "$(row kTCCServiceScreenCapture)"
sqlite3 "$MIC" "$(row kTCCServiceMicrophone)"
GUEST
if vm "sh $TREE/allow.sh $TREE/$APP"; then ok "the dev build may hear and record in the clone"; else bad "the clone's privacy databases didn't take the rows"; fi
# The app's first launch, and its first capture from this place: macOS asks its second
# question. Its asker is ended by its PID once the self-test is over.
vm "open -W -n $TREE/$APP --args --selftest $TREE/selftest.json" || true
asker=$(vm 'pgrep -x UserNotificationCenter' || true)
for pid in $asker; do vm "kill $pid" || true; done
vm "cat $TREE/selftest.json" >"$WORK/selftest.json" || true
if node -e '
const t = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"));
const s = t.screen ?? {};
console.log(`     the microphone ${t.mic}; recording allowed ${s.permission}; ${s.frames_delivered} frames of ${s.width}×${s.height} in ${s.elapsed_ms} ms${s.error ? `; ${s.error}` : ""}`);
process.exit(t.mic === "granted" && s.permission && s.frames_delivered > 0 ? 0 : 1);
' "$WORK/selftest.json"; then ok "it does: its self-test has the microphone, and got frames of the clone's screen"; else bad "the dev build can't hear or record in the clone"; fi
if [ -n "$asker" ]; then echo "     macOS asked its second question; its asker was ended"; else echo "     macOS didn't ask its second question"; fi

# One run of the runner, once: the lock keeps a repeated exec from starting a second shroom
# beside the first; a run that hangs is stopped after five minutes and counts as a failure.
# The person is back in their note first. Then each path as the runner judged it: the walk,
# what every check of the Dock icon and the keyboard stood on, and anything held against it.
n=0
drive() {
  what=$1; shift
  n=$((n + 1))
  to_textedit
  if tart exec "$VM" sh -c "mkdir $TREE/ran-$n && cd $TREE && SHROOM_HANDS=1 perl -e 'alarm 300; exec @ARGV' '$NODE' app/test/onboarding-drive.mjs $*" </dev/null >"$WORK/$n.json" 2>"$WORK/$n.err"; then
    ok "$what"
  else
    bad "$what"
    sed 's/^/     /' "$WORK/$n.err" | tail -n 8
  fi
  node -e '
const s = require("fs").readFileSync(process.argv[1], "utf8");
let d;
try { d = JSON.parse(s); } catch { console.log(s.slice(-1500)); process.exit(0); }
for (const r of d) {
  console.log(`     ${r.ok ? "ok  " : "FAIL"} ${r.path}: ${r.states ?? "no walk"}`);
  for (const door of r.doors ?? []) console.log(`          ${door}`.slice(0, 400));
  for (const u of r.unseen ?? []) console.log(`          not looked at: ${u}`);
  for (const p of r.problems ?? []) console.log(`          ${p}`.slice(0, 400));
}
' "$WORK/$n.json"
}
# The last run's results held to what the lane promises of them: these paths, in this order,
# every one ok — and of Invite someone, all four of its checks made, by name.
held() {
  node -e '
let d;
try { d = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")); } catch { process.exit(1); }
const want = process.argv.slice(2);
const paths = d.map((r) => r.path);
if (paths.join(" ") !== want.join(" ") || !d.every((r) => r.ok)) process.exit(1);
const invite = d.find((r) => r.path === "invite");
const checks = ["Invite someone: setup is over, and Settings carries on with the Dock icon — ", "Settings takes over in front with the keyboard — ", "and keeps it a moment later — ", "and what\u0027s typed next is the name: “Who\u0027s this for?” arrived with the caret — "];
process.exit((invite.doors ?? []).length === checks.length && checks.every((c, i) => invite.doors[i].startsWith(c)) && !(invite.unseen ?? []).length ? 0 : 1);
' "$WORK/$n.json" "$@"
}

echo "── 2. all-set's Invite someone: Settings carries on, with the Dock icon and the keyboard ──"
drive "Invite someone ends setup: Settings in front with the keyboard, the Dock icon with it" invite
if held invite; then ok "all four looked at: the Dock icon staying, Settings in front with the keyboard, keeping it, and the name typed straight in"; else bad "Invite someone's checks weren't all made"; fi

echo "── 3. every path, in one command ──"
drive "a fresh account, a taken name, both gates, a space already there, the doors, Invite someone" all
if held fresh taken email-gate card-gate found doors invite; then ok "seven paths of seven, each with its own result"; else bad "not every path came back ok"; fi
exit $fail
