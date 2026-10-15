#!/bin/sh
# L5: a recording, on a Mac nobody sits at (DECISIONS B114).
#
# A take can only be made on a screen that isn't locked, by an app that has leave to record —
# which an unattended run on this Mac can't count on (a locked screen gives a capture no
# frames). A throwaway clone of a golden image is always logged in and unlocked, and its
# privacy settings are its own: so the dev build's take drives run there, against the L1
# fake, and the take one of them makes is read back here.
#
#   1. leave to record — given to the dev build in the clone, and seen to work (its own
#      self-test captures the clone's screen for three seconds);
#   2. a take from a cold start, and what it leaves out (`take-drive.mjs left-out`): as
#      shroom starts — before its popover or its count has ever shown — ScreenCaptureKit
#      already finds both, so the check a take makes before it records would pass; then
#      Return, the count to its end, and the take starts. Both are lifted above the pattern
#      for the run and shown again while it records: a capture that leaves nothing out sees
#      them there each time. The take is brought out of the clone and every frame of it is
#      decoded by this Mac's own ffmpeg (tests/e2e/left-out.mjs): nothing but the pattern;
#   3. Stop, and a file that plays (`take-drive.mjs movies`): a take the space can't have,
#      saved to the throwaway home's own Movies, twice;
#   4. the reader's own proof (`take-drive.mjs not-left-out`): the same drive with the count
#      taken off the list of what a take leaves out — and the reader finds it in the take;
#   5. the overlays, where shroom may record (`surfaces-drive.mjs`, the drive
#      tests/l5/surfaces-vm.sh runs where it may not): Return, pressed in the popover — a
#      key for its panel, answered by its page — starts the count. The page's Return re-reads
#      what macOS allows, so on a Mac that may not record it can only say so; here it counts,
#      twice (once over an app in a full-screen Space), and the drive cancels each count
#      before it could end: no take. And ScreenCaptureKit lists both panels once they have
#      shown.
#
# What the clone is given, and only the clone — every line of it goes through `tart exec`,
# never to this Mac: the golden images have System Integrity Protection off, so
#   - the dev build (`am.shroom.dev`, as signed here) is written into the clone's privacy
#     database as allowed to record the screen — what a person does in System Settings;
#   - macOS then asks its second question (may this app record without the system's own
#     picker?) the first time the app captures from a given place, and writes down that it
#     has asked; the lane lets the self-test raise it, and ends the asker's process by its
#     PID. On macOS 26 the capture runs either way, and it isn't asked again;
#   - the encoder built in this tree (tools/build-ffmpeg.sh) is put where a dev build looks
#     for one: a clean Mac has no ffmpeg of its own.
# Not here: the bubble (no camera), the microphone (the VM's sound is off), Camera as the
# source — `node app/test/take-drive.mjs signals` on this Mac, its screen unlocked, has those.
#
# The VM has no window and its sound is off (TESTING-PLAN's quiet guard). The clone is
# deleted after, and the take with it; the copy read here is deleted too.
#
# Usage: tests/l5/take-vm.sh
#   needs the debug bundle, the fake and the encoder:
#   (cd app && npm run build:dev) && cargo build -p shroom-cloudfake --bin cloudfake
#   tools/build-ffmpeg.sh        (once; the release build needs it too)
# SHROOM_TAKE_IMAGE=<name> clones another golden image (the Sequoia one, `shroom-clean`).
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
APP=target/debug/bundle/macos/shroom-dev.app
FAKE=target/debug/cloudfake
ENCODER=build/ffmpeg/sidecar/ffmpeg-aarch64-apple-darwin
for p in "$APP" "$FAKE" "$ENCODER"; do
  [ -e "$ROOT/$p" ] || { echo "missing $p — build it first (see the header)"; exit 2; }
done
command -v tart >/dev/null || { echo "tart isn't installed here"; exit 1; }

VM=shroom-l5-take-$$
WORK=$(mktemp -d)
TREE=/tmp/shroom-take
cleanup() {
  tart stop "$VM" >/dev/null 2>&1 || true
  tart delete "$VM" >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

tart clone "${SHROOM_TAKE_IMAGE:-shroom-clean-tahoe}" "$VM"
tart run "$VM" --no-graphics --no-audio >/dev/null 2>&1 &
i=0
until tart exec "$VM" true >/dev/null 2>&1; do
  i=$((i + 1)); [ "$i" -gt 90 ] && { echo "the VM never came up"; exit 1; }
  sleep 2
done
vm() { tart exec "$VM" sh -c "$1" </dev/null 2>/dev/null; }

# Logged in, the desktop up.
i=0
until vm 'pgrep -x Dock >/dev/null && lsappinfo info -only bundleid "$(lsappinfo front)" | grep -q "=\"com\.apple\.finder\""'; do
  i=$((i + 1)); [ "$i" -gt 60 ] && { echo "the VM's desktop never came up"; exit 1; }
  sleep 2
done
# A Mac that has just started is busy with itself for a while: let it settle.
sleep 30
NODE=$(vm 'for n in /opt/homebrew/bin/node /usr/local/bin/node; do [ -x "$n" ] && { echo "$n"; break; }; done' || true)
[ -n "$NODE" ] || { echo "FAIL the clean image has no node to run the drives with"; exit 1; }
echo "macOS $(vm 'sw_vers -productVersion') in the clone"

# The tree as the VM sees it: the bundle, the fake and the runner in the same relative
# places, so the runner runs there unchanged — and the encoder where a dev build looks.
vm "mkdir -p $TREE"
tar -C "$ROOT" -cf - -L "$APP" "$FAKE" app/test/take-drive.mjs app/test/surfaces-drive.mjs app/test/fullscreen-helper.js tests/test-machine.mjs | tart exec -i "$VM" tar -C "$TREE" -xf -
tart exec -i "$VM" tee "$TREE/ffmpeg" <"$ROOT/$ENCODER" >/dev/null
vm "chmod +x $TREE/ffmpeg && { test -e /opt/homebrew/bin/ffmpeg || sudo cp $TREE/ffmpeg /opt/homebrew/bin/ffmpeg; }"

fail=0
ok() { echo "ok   $1"; }
bad() { echo "FAIL $1"; fail=1; }

echo "── 1. leave to record, in the clone ──"
# The clone's own privacy database, through `tart exec` — never this Mac's. The row says
# what the Screen Recording switch says, for the bundle as it is signed (its identifier and
# its certificate: the requirement macOS itself would write down).
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
if vm "sh $TREE/allow.sh $TREE/$APP"; then ok "the dev build is allowed to record in the clone"; else bad "the clone's privacy database didn't take the row"; fi
# The first capture from this place: macOS asks its second question. Its asker is ended by
# its PID once the self-test is over, so nothing of macOS's sits over the pattern later.
vm "open -W -n $TREE/$APP --args --selftest $TREE/selftest.json" || true
asker=$(vm 'pgrep -x UserNotificationCenter' || true)
for pid in $asker; do vm "kill $pid" || true; done
vm "cat $TREE/selftest.json" >"$WORK/selftest.json" || true
if node -e '
const s = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")).screen;
console.log(`     allowed ${s.permission}; ${s.frames_delivered} frames of ${s.width}×${s.height} in ${s.elapsed_ms} ms${s.error ? `; ${s.error}` : ""}`);
process.exit(s.permission && s.frames_delivered > 0 ? 0 : 1);
' "$WORK/selftest.json"; then ok "it records there: its self-test got frames of the clone's screen"; else bad "the dev build got no frames in the clone"; fi
if [ -n "$asker" ]; then echo "     macOS asked its second question; its asker was ended"; else echo "     macOS didn't ask its second question"; fi

# One drive, once; a drive that hangs is stopped after five minutes and counts as a failure.
n=0
drive() {
  what=$1; shift
  n=$((n + 1))
  if tart exec "$VM" sh -c "cd $TREE && SHROOM_TAKE_OUT=$TREE/out-$n perl -e 'alarm 300; exec @ARGV' '$NODE' app/test/take-drive.mjs $*" </dev/null >"$WORK/$n.json" 2>"$WORK/$n.err"; then
    ok "$what"
  else
    bad "$what"
    sed 's/^/     /' "$WORK/$n.err" | tail -n 8
  fi
  # What the drive's report holds: each check, and what it saw.
  node -e '
const s = require("fs").readFileSync(process.argv[1], "utf8");
let d;
try { d = JSON.parse(s); } catch { console.log(s.slice(-1500)); process.exit(0); }
const r = d.report ?? {};
for (const [what, ok, seen] of r.checks ?? []) console.log(`     ${ok ? "ok  " : "FAIL"} ${what}${seen ? ` — ${seen}` : ""}`.slice(0, 400));
if (r.error) console.log(`     ${r.error}`);
for (const sk of d.skipped ?? []) console.log(`     not looked at: ${sk.what} (${sk.why})`);
' "$WORK/$n.json"
}

echo "── 2. a take from a cold start, and what it leaves out ──"
drive "both panels found before either has shown; the count runs to its end and the take starts; a capture that leaves nothing out sees them over the pattern" left-out
# The take's picture, read here: the clone has no ffmpeg that can.
mkdir -p "$WORK/out"
if tart exec "$VM" tar -C "$TREE/out-$n" -cf - take </dev/null 2>/dev/null | tar -C "$WORK/out" -xf - 2>/dev/null && picture=$(node "$ROOT/tests/e2e/left-out.mjs" "$WORK/out/take"); then
  ok "every frame of that take, decoded here: nothing but the pattern"
else
  bad "the take's picture: something is over the pattern, or it couldn't be read"
fi
echo "     ${picture:-no take came out of the clone}"

echo "── 3. Stop, and a file that plays ──"
drive "a take the space can't have is saved to Movies, twice, and plays" movies

echo "── 4. the reader's own proof: the count taken off the list is found in the take ──"
drive "the same drive, the count no longer left out" not-left-out
mkdir -p "$WORK/in"
if tart exec "$VM" tar -C "$TREE/out-$n" -cf - take </dev/null 2>/dev/null | tar -C "$WORK/in" -xf - 2>/dev/null && found=$(node "$ROOT/tests/e2e/left-out.mjs" "$WORK/in/take" --the-count-stays-in); then
  ok "the reader finds the count in that take's picture"
else
  bad "the reader didn't find a count that wasn't left out"
fi
echo "     ${found:-no take came out of the clone}"

echo "── 5. the overlays, where shroom may record ──"
# Strict, beside the app that goes full screen, as the surfaces lane runs it. What only a Mac
# that may record can show is then read from its report: each Return in the popover started
# the count, and ScreenCaptureKit's list was looked at, not skipped.
if tart exec "$VM" sh -c "cd $TREE && perl -e 'alarm 300; exec @ARGV' '$NODE' app/test/surfaces-drive.mjs --fullscreen --strict --report=$TREE/surfaces.json" </dev/null >"$WORK/surfaces.out" 2>"$WORK/surfaces.err"; then
  ok "the surfaces drive, strict: $(node -e 'const r = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")); console.log(`${r.passed} checks`)' "$WORK/surfaces.out" 2>/dev/null || echo "its checks")"
else
  bad "the surfaces drive"
  tail -n 30 "$WORK/surfaces.out" | sed 's/^/     /'
  sed 's/^/     /' "$WORK/surfaces.err" | tail -n 8
fi
vm "cat $TREE/surfaces.json" >"$WORK/surfaces.json" || true
if node -e '
let r;
try { r = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8")); } catch { console.log("     no report came out of the clone"); process.exit(1); }
const checks = r.checks ?? [];
const by_return = checks.filter(([what]) => what.endsWith("Return, pressed in the popover, goes through its page to the core"));
const listed = checks.filter(([what]) => what.startsWith("a take would start: ScreenCaptureKit finds every window"));
for (const [what, ok, seen] of [...by_return, ...listed]) console.log(`     ${ok ? "ok  " : "FAIL"} ${what} — ${seen}`.slice(0, 400));
const counted = by_return.length === 2 && by_return.every(([, ok, seen]) => ok && seen.startsWith("the count started"));
process.exit(counted && listed.length === 1 && listed[0][1] ? 0 : 1);
' "$WORK/surfaces.json"; then
  ok "Return in the popover started the count, both times; ScreenCaptureKit lists both panels"
else
  bad "Return in the popover, or ScreenCaptureKit's list, where shroom may record"
fi
exit $fail
