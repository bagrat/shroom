#!/bin/sh
# L5: an update, on a Mac nobody sits at — and what comes back afterwards (N37; laws 5 and
# 6 as built, DEVIATIONS #38, DECISIONS B113).
#
# The update drive (app/test/update-drive.sh) runs the real app as an older version against
# a release stand-in on loopback: the mushroom gets its Frost dot, Settings · General offers
# the update, the banner names it, Update swaps a throwaway copy and shroom comes back by
# itself — Settings restored where it was, “Updated to … ✓” said once. Whether that
# restored window is in front of the app the person is in, and leaves them the keyboard,
# needs a screen that isn't locked — which an unattended run on this Mac can't count on
# (the drive then says what it couldn't look at). A throwaway clone of a golden image is
# always logged in and unlocked, so the drive runs there with that check required
# (SHROOM_HANDS=1), the person in TextEdit on a note the whole time:
#
#   - the update is found, staged, offered and taken;
#   - shroom comes back as the new version, its Settings window on top with the Dock icon,
#     shroom not the app in front and not holding the keyboard;
#   - it says “Updated to … ✓” once, and wears no Frost dot;
#   - TextEdit is still the app in front when it is over.
#
# What the clone is given, and only the clone — every line of it goes through `tart exec`,
# never to this Mac: leave to record, written into the clone's own privacy database exactly
# as tests/l5/take-vm.sh writes it (the golden images have System Integrity Protection
# off). Without it macOS has screen recording switched off for shroom, the mushroom wears
# its amber dot — “something needs you” outranks “something new from us” (N38) — and the
# Frost dot the drive waits for never shows. It is written after the app's first launch and
# nothing here captures, so macOS has no second question to ask.
#
# Keys and clicks are the drive's own, put on shroom's event queue — nothing needs
# Accessibility or sends an event to another app. The VM has no window and its sound is off
# (TESTING-PLAN's quiet guard). The clone is deleted after.
#
# Usage: tests/l5/update-vm.sh
#   needs the debug bundle and the fake:
#   (cd app && npm run build:dev) && cargo build -p shroom-cloudfake --bin cloudfake
# SHROOM_UPDATE_IMAGE=<name> clones another golden image (the Sequoia one, `shroom-clean`).
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
APP=$ROOT/target/debug/bundle/macos/shroom-dev.app
FAKE=$ROOT/target/debug/cloudfake
VM=shroom-l5-update-$$
WORK=$(mktemp -d)
TREE=/tmp/shroom
for p in "$APP" "$FAKE"; do
  [ -e "$p" ] || { echo "missing $p — build it first (see the header)"; exit 2; }
done
command -v tart >/dev/null || { echo "tart isn't installed here"; exit 1; }

cleanup() {
  tart stop "$VM" >/dev/null 2>&1 || true
  tart delete "$VM" >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

tart clone "${SHROOM_UPDATE_IMAGE:-shroom-clean-tahoe}" "$VM"
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
[ -n "$NODE" ] || { echo "the VM has no node to run the drive with"; exit 1; }
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
vm "mkdir -p $TREE && printf 'A note someone was writing.\n' > $TREE/note.txt"
to_textedit

# The tree's own layout, so the drive finds the bundle, the fake, the packager and the
# version where it looks.
vm "mkdir -p $TREE/target/debug/bundle/macos $TREE/app/test $TREE/app/src-tauri $TREE/tools $TREE/tests"
tar -cf - -C "$(dirname "$APP")" "$(basename "$APP")" | tart exec -i "$VM" tar -xf - -C "$TREE/target/debug/bundle/macos"
tar -cf - -C "$(dirname "$FAKE")" "$(basename "$FAKE")" | tart exec -i "$VM" tar -xf - -C "$TREE/target/debug"
tar -cf - -C "$ROOT/app/test" update-drive.sh region5-drive.mjs release-standin.mjs | tart exec -i "$VM" tar -xf - -C "$TREE/app/test"
tar -cf - -C "$ROOT/app/src-tauri" tauri.conf.json | tart exec -i "$VM" tar -xf - -C "$TREE/app/src-tauri"
tar -cf - -C "$ROOT/tools" package-release.sh | tart exec -i "$VM" tar -xf - -C "$TREE/tools"
tar -cf - -C "$ROOT/tests" test-machine.mjs | tart exec -i "$VM" tar -xf - -C "$TREE/tests"

# A just-booted clone is busy with its own first minutes: a minute at most for it to go
# quiet, then the app's first launch — macOS checking an app it has never seen — spent on
# one that only reads its own version and ends.
i=0
until vm 'sysctl -n vm.loadavg' | awk 'NF >= 4 { quiet = ($2 < 1.5) } END { exit !quiet }'; do
  i=$((i + 1)); [ "$i" -gt 12 ] && break
  sleep 5
done
vm "open -W -n $TREE/target/debug/bundle/macos/shroom-dev.app --args --selftest $TREE/first-launch.json" || true

fail=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fail=1; }

echo "── 1. leave to record, in the clone ──"
# The clone's own privacy database, through `tart exec` — never this Mac's. The same row
# tests/l5/take-vm.sh writes: what the Screen Recording switch says, for the bundle as it
# is signed.
tart exec -i "$VM" tee "$TREE/allow.sh" >/dev/null <<'GUEST'
set -eu
APP=$1
DB="/Library/Application Support/com.apple.TCC/TCC.db"
[ "$(csrutil status)" = "System Integrity Protection status: disabled." ] || { echo "this isn't a golden image's clone"; exit 1; }
REQ=$(codesign -dr - "$APP" 2>&1 | sed -n 's/^designated => //p')
[ -n "$REQ" ] || { echo "the bundle has no designated requirement"; exit 1; }
echo "$REQ" | csreq -r- -b /tmp/shroom-dev.csreq
HEX=$(xxd -p /tmp/shroom-dev.csreq | tr -d '\n')
sudo sqlite3 "$DB" "insert or replace into access (service, client, client_type, auth_value, auth_reason, auth_version, csreq, indirect_object_identifier, flags) values ('kTCCServiceScreenCapture', 'am.shroom.dev', 0, 2, 4, 1, X'$HEX', 'UNUSED', 0);"
GUEST
if vm "sh $TREE/allow.sh $TREE/target/debug/bundle/macos/shroom-dev.app"; then ok "the dev build is allowed to record in the clone"; else bad "the clone's privacy database didn't take the row"; fi
# The first launch came and went: the person turns back to their note.
to_textedit

echo "── 2. the update, and what comes back ──"
# Once: a drive that hangs is stopped after four minutes and counts as a failure.
if tart exec "$VM" sh -c "cd $TREE/app && PATH=$(dirname "$NODE"):\$PATH SHROOM_HANDS=1 perl -e 'alarm 240; exec @ARGV' sh test/update-drive.sh" </dev/null >"$WORK/update.out" 2>"$WORK/update.err"; then
  ok "found, staged, offered, taken — and shroom back by itself as the new version"
else
  bad "the update drive"
  # A drive that failed kept its report beside its work: bring it out before the clone goes.
  vm 'T=$(getconf DARWIN_USER_TEMP_DIR); cat "$T"shroom-r5-upd.*/out.json' >>"$WORK/update.out" || true
  sed 's/^/     /' "$WORK/update.err" | tail -n 8
fi
# What both halves of the drive saw, check by check — and that each half reported.
if node -e '
const text = require("fs").readFileSync(process.argv[1], "utf8");
const halves = {};
for (const part of text.split(/^(?=\{$)/m).filter((p) => p.startsWith("{"))) {
  let d;
  try { d = JSON.parse(part); } catch { continue; }
  const r = d.report ?? d.relaunched;
  if (!r) continue;
  halves[r.drive] = r;
  for (const [what, ok, seen] of r.checks ?? []) console.log(`     ${ok ? "ok  " : "FAIL"} ${what}${seen ? ` — ${seen}` : ""}`.slice(0, 400));
  if (r.error) console.log(`     ${r.error}`);
  for (const u of r.unseen ?? []) console.log(`     not looked at: ${u}`);
}
const whole = (r) => r && r.ok === true && (r.checks ?? []).length > 0 && (r.checks ?? []).every((c) => c[1] === true) && (r.unseen ?? []).length === 0;
const back = (halves["update-relaunched"]?.checks ?? []).find((c) => c[0] === "restored in front, without the keyboard");
process.exit(whole(halves["update"]) && whole(halves["update-relaunched"]) && back?.[1] === true ? 0 : 1);
' "$WORK/update.out"; then
  ok "both halves reported, every check held, nothing left unlooked-at — the restored window in front, the keyboard left alone"
else
  bad "a half of the drive is missing, failed a check, or couldn't look"
fi
if in_textedit; then ok "TextEdit is still the app in front"; else bad "the app in front afterwards: $(vm 'lsappinfo info -only bundleid "$(lsappinfo front)"')"; fi
exit $fail
