#!/bin/sh
# L5: the Dock icon iff a window of shroom's is open (law 6, SM:133 — DEVIATIONS #38's
# narrowing rejected 2026-10-13), and its windows' keyboard (DECISIONS B105). The Dock's
# side of that the app's own drives read anywhere. The keyboard's side needs a screen that isn't
# locked — which an unattended run on this Mac can't count on (the drives then say what
# they couldn't look at). A throwaway clone of the `shroom-clean-tahoe` golden image is
# always logged in and unlocked, so the dev build's drives run there against the L1 fake,
# with every keyboard check required (SHROOM_HANDS=1):
#
#   1. the Library — asked for over another app, it is in front with the keyboard; what's
#      typed lands in its search field, ⌘A and ⌘Z work, ⌘C ⌘X ⌘V are the field's to
#      answer; shroom opened again shows its popover, the Dock icon still there; from behind another
#      app, from minimized and from hidden (⌘H) the popover's door brings it back; ⌘W
#      closes it through the core, and the keyboard goes back to the app the person was in;
#   2. Settings — the same doors, and ⌘Q quits through the core;
#   3. the setup window — ✕ retreats to the menu bar and gives the keyboard back; the
#      mushroom brings the window, its Dock icon and the keyboard back;
#   4. the launches — each way shroom starts, one at a time with the person back in their
#      note before each, and what macOS says of it while it does: a launch that shows
#      nothing, and any launch on a set-up Mac — the install script's own `open` among
#      them — is never in the Dock and never the app in front, for any moment, and no app
#      comes to the front while it starts (a sampler, and LaunchServices' own
#      notifications: app/test/launch-drive.mjs); a first run ends in the Dock, the welcome
#      in front with the keyboard; a set-up Mac opened from Finder shows the popover, with
#      the keyboard, and TextEdit stays in front.
#
# The person is in another app the whole time, one with a window of its own: TextEdit, open
# on a note before the drives start. On a just-logged-in clone only Finder is in front, and
# it has no window — nothing would ever cover shroom's window when it goes behind, and “back
# to the app the person was in” couldn't be told from “back to Finder”, which is where the
# keyboard goes when there is nowhere else. The lane holds the two window drives to it: the
# app shroom's window went behind was TextEdit, and a window of TextEdit's covered it.
#
# The drives time things, and a just-booted clone is busy with its own first minutes:
# whichever drive went first there missed one of its older timing checks (“Copied”
# reverting, the launch's popover under the mushroom) and passed on a later run. So the
# clone is left to go quiet first, and the app's first launch — macOS checking an app it
# has never seen — is spent on a launch that proves nothing.
#
# Not here: all-set's Invite someone (this clone hasn't allowed recording, so setup never
# reaches all-set in it — tests/l5/setup-vm.sh walks it, in a clone that has), a real click
# on the mushroom, the real clipboard.
#
# Keys are typed the way AppKit delivers them — posted to shroom's own event queue, at
# whichever window holds the keyboard — so nothing here needs Accessibility or sends an
# event to another app (TextEdit is opened the way a click on it opens it, and nothing is
# typed into it). The VM has no window and its sound is off (TESTING-PLAN's quiet guard).
# The clone is deleted after.
#
# Usage: tests/l5/dock-vm.sh
#   needs the debug bundle and the fake:
#   (cd app && npm run build:dev) && cargo build -p shroom-cloudfake --bin cloudfake
# SHROOM_DOCK_IMAGE=<name> clones another golden image (the Sequoia one, `shroom-clean`).
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
APP=$ROOT/target/debug/bundle/macos/shroom-dev.app
FAKE=$ROOT/target/debug/cloudfake
VM=shroom-l5-dock-$$
WORK=$(mktemp -d)
for p in "$APP" "$FAKE"; do
  [ -e "$p" ] || { echo "missing $p — build it first (see the header)"; exit 2; }
done

cleanup() {
  tart stop "$VM" >/dev/null 2>&1 || true
  tart delete "$VM" >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

tart clone "${SHROOM_DOCK_IMAGE:-shroom-clean-tahoe}" "$VM"
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
NODE=$(vm 'command -v node || ls /opt/homebrew/bin/node' | head -n 1)
[ -n "$NODE" ] || { echo "the VM has no node to run the drives with"; exit 1; }

# The other app, and its window: TextEdit on a note, in front (see the header). Opened now,
# so its own first launch is over before the clone is asked to go quiet.
in_textedit() { vm 'lsappinfo info -only bundleid "$(lsappinfo front)" | grep -q "=\"com\.apple\.TextEdit\""'; }
to_textedit() {
  vm 'open -a TextEdit /tmp/shroom/note.txt'
  i=0
  until in_textedit; do
    i=$((i + 1)); [ "$i" -gt 30 ] && { echo "TextEdit never came to the front in the VM"; exit 1; }
    sleep 2
  done
}
vm 'mkdir -p /tmp/shroom && printf "A note someone was writing.\n" > /tmp/shroom/note.txt'
to_textedit

# The tree's own layout, so the runners find the bundle and the fake where they look.
vm 'mkdir -p /tmp/shroom/target/debug/bundle/macos /tmp/shroom/app/test /tmp/shroom/tests'
tar -cf - -C "$(dirname "$APP")" "$(basename "$APP")" | tart exec -i "$VM" tar -xf - -C /tmp/shroom/target/debug/bundle/macos
tar -cf - -C "$(dirname "$FAKE")" "$(basename "$FAKE")" | tart exec -i "$VM" tar -xf - -C /tmp/shroom/target/debug
tar -cf - -C "$ROOT/app/test" region5-drive.mjs onboarding-drive.mjs launch-drive.mjs launch-watch.js | tart exec -i "$VM" tar -xf - -C /tmp/shroom/app/test
tar -cf - -C "$ROOT/tests" test-machine.mjs | tart exec -i "$VM" tar -xf - -C /tmp/shroom/tests

# Quiet first: the one-minute load under 1.5 (three minutes at most), then one launch
# that only reads its own version and ends.
i=0
until vm 'sysctl -n vm.loadavg' | awk 'NF >= 4 { quiet = ($2 < 1.5) } END { exit !quiet }'; do
  i=$((i + 1)); [ "$i" -gt 36 ] && { echo "(the VM never went quiet — going on)"; break; }
  sleep 5
done
vm 'open -W -n /tmp/shroom/target/debug/bundle/macos/shroom-dev.app --args --selftest /tmp/shroom/first-launch.json' || true
# That launch came and went in front: the person turns back to their note.
to_textedit

# What a drive's report holds against it: each failed check, each thing left unlooked-at.
explain() {
  node -e '
const s = require("fs").readFileSync(process.argv[1], "utf8");
let d;
try { d = JSON.parse(s); } catch { console.log(s.slice(-1500)); process.exit(0); }
const rows = Array.isArray(d)
  ? d.flatMap((r) => r.problems ?? [])
  : [
      ...(d.report?.checks ?? []).filter((c) => !c[1]).map((c) => `${c[0]} — ${c[2]}`),
      ...(d.report?.error ? [d.report.error] : []),
      ...(d.report?.unseen ?? []).map((u) => `not looked at: ${u}`),
    ];
for (const r of rows) console.log(`     ${r}`);
' "$1"
}

fail=0
n=0
# One drive, once: the lock keeps a repeated exec from starting a second shroom beside the
# first, and a drive that hangs is stopped after four minutes and counts as a failure.
drive() {
  what=$1; shift
  n=$((n + 1))
  if tart exec "$VM" sh -c "mkdir /tmp/shroom/ran-$n && cd /tmp/shroom/app && SHROOM_HANDS=1 perl -e 'alarm 240; exec @ARGV' '$NODE' test/$*" </dev/null >"$WORK/$n.json" 2>"$WORK/$n.err"; then
    echo "ok   $what"
  else
    echo "FAIL $what"
    explain "$WORK/$n.json"
    sed 's/^/     /' "$WORK/$n.err" | tail -n 8
    fail=1
  fi
}

# What made the last drive's “behind another app” and “back to the app the person was in”
# real, from its own report: the app shroom's window went behind was TextEdit, not Finder;
# a window of TextEdit's covered the middle of shroom's while it was behind; and that is
# the app the keyboard went back to when shroom's window closed.
behind() {
  if node -e '
const fs = require("fs");
let r;
try { r = JSON.parse(fs.readFileSync(process.argv[1], "utf8")).report; } catch { /* no report */ }
const b = r?.facts?.behind;
const back = (r?.checks ?? []).find((c) => c[0].endsWith("the keyboard goes back to the app the person was in"));
if (b?.app !== "com.apple.TextEdit" || b?.covered !== true || back?.[1] !== true) {
  console.log(`     behind: ${JSON.stringify(b ?? null)}; closed: ${back ? back[2] : "not looked at"}`);
  process.exit(1);
}
' "$WORK/$n.json"; then
    echo "ok   $1"
  else
    echo "FAIL $1"
    fail=1
  fi
}

echo "── 1. the Library, with its Dock icon ──"
drive "in front with the keyboard, typed into, back through the popover's door, closed with ⌘W" region5-drive.mjs library
behind "behind TextEdit, under TextEdit's window; closed, the keyboard went back to TextEdit"
echo "── 2. Settings, with its Dock icon ──"
drive "the same doors, and ⌘Q quits through the core" region5-drive.mjs settings
behind "behind TextEdit, under TextEdit's window; closed, the keyboard went back to TextEdit"
echo "── 3. the setup window's Dock icon, there and back ──"
drive "✕ retreats to the menu bar; the mushroom brings the window, the icon and the keyboard back" onboarding-drive.mjs doors
# What the last drive's two witnesses saw of each launch, from its own report: how closely
# the sampler looked, and LaunchServices' account — the kind of app each launch was, when.
watched() {
  node -e '
let d;
try { d = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")); } catch { process.exit(0); }
for (const r of Array.isArray(d) ? d : []) {
  const s = r.seen ?? {};
  console.log(`     ${r.launch}: ${s.looks} looks, one every ${s.every_ms} ms (the longest wait ${s.longest_ms} ms); in front before ${s.in_front?.before} and after ${s.in_front?.after}; LaunchServices: ${(s.launchservices?.said ?? []).join(" → ") || "nothing read"}`);
}
' "$WORK/$n.json"
}

# One launch at a time, the person back in their note before each. A first run comes to the
# front and ends there, and when it does macOS 15 sometimes leaves no app in front at all
# (seen in this lane with the build before DECISIONS B113 and after it): every launch that
# followed was then watched with nobody in front, its keyboard unlooked-at.
launch() {
  to_textedit
  drive "$2" launch-drive.mjs "$1"
  watched
}

echo "── 4. the launches ──"
launch login "the login item — nothing shows: never in the Dock, never in front"
launch install-update "the install script's update — nothing shows: never in the Dock, never in front"
launch install "the install script's first run — the welcome: in the Dock, in front, with the keyboard"
launch user "a first run you started — the welcome: in the Dock, in front, with the keyboard"
launch user-provisioned "a set-up Mac, opened by you — the popover under the mushroom, with the keyboard; never in the Dock, your app still in front"
launch user-provisioned-late "the same behind a slow start"
launch login-set-up "the login item on a set-up Mac — never in the Dock, never in front"
launch update-set-up "an update on a set-up Mac, opened as the install script opens it — never in the Dock, never in front"
exit $fail
