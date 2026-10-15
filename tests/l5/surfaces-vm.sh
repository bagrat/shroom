#!/bin/sh
# L5 for the overlays (DEVIATIONS #39, #40): the surfaces drive on a clean Mac whose screen
# is unlocked and whose desk is nobody's — beside another app that goes full screen.
#
# The popover a click on the mushroom opens has the keyboard at once and never brings
# shroom forward; a first click on a control acts; Esc and the mushroom close it; it stays
# open, and follows, when the other app takes a Space of its own; it opens over that
# full-screen app; and the count plays there, with the keyboard, and never moves you —
# when it ends you are where you were. Before any of that, where the overlays meet a
# set-up shroom's windows (DEVIATIONS #38): the first popover has the keyboard, its door
# brings the Library in front with the keyboard, Esc on a popover over the Library gives the
# keyboard back to it, ⌘H with the popover open hides the Library and leaves the popover,
# the door brings the Library back, and ⌘W hands the keyboard to the app you were in. Then,
# with the Library open behind the other app: the popover and the first count still come
# over it, and the Library stays put. A click on the Dock, or on a window in Mission Control (brought up for the
# moment), closes nothing; a click in a corner of the screen does. And nobody new has a
# window on screen at the end: nothing was asked of macOS.
# (`app/src-tauri/src/take_drive/surfaces.rs` has the whole list.)
#
# Everything below happens inside a throwaway, headless clone of a golden image
# (`shroom-clean-tahoe` unless another is named), deleted after: the dev bundle (`am.shroom.dev`) built in this tree, the
# L1 fake, a throwaway home. No account, no camera, no recording — and the VM has no
# window and its sound is off (TESTING-PLAN's quiet guard).
#
#   cargo build -p shroom-cloudfake --bin cloudfake
#   (cd app && npm run build:dev)
#   tests/l5/surfaces-vm.sh [report.json]
#
# SHROOM_SURFACES_VM=<name> runs in a VM that is already up, and leaves it as it found it.
# SHROOM_SURFACES_IMAGE=<name> clones another golden image (the Sequoia one, `shroom-clean`).
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
APP=target/debug/bundle/macos/shroom-dev.app
FAKE=target/debug/cloudfake
KEEP=${1:-}
[ -d "$ROOT/$APP" ] && [ -x "$ROOT/$FAKE" ] || { echo "missing $APP or $FAKE — build them first (see the header)"; exit 2; }
command -v tart >/dev/null || { echo "tart isn't installed here"; exit 1; }

OWN=0
VM=${SHROOM_SURFACES_VM:-}
if [ -z "$VM" ]; then
  OWN=1
  VM=shroom-surfaces-l5-$$
fi
TREE=/tmp/shroom-surfaces
cleanup() {
  if [ "$OWN" = 1 ]; then
    tart stop "$VM" >/dev/null 2>&1 || true
    tart delete "$VM" >/dev/null 2>&1 || true
  else
    tart exec "$VM" rm -rf "$TREE" </dev/null >/dev/null 2>&1 || true
  fi
}
trap cleanup EXIT

if [ "$OWN" = 1 ]; then
  tart clone "${SHROOM_SURFACES_IMAGE:-shroom-clean-tahoe}" "$VM"
  tart run "$VM" --no-graphics --no-audio >/dev/null 2>&1 &
fi
i=0
until tart exec "$VM" true >/dev/null 2>&1; do
  i=$((i + 1)); [ "$i" -gt 90 ] && { echo "the VM never came up"; exit 1; }
  sleep 2
done
vm() { tart exec "$VM" sh -c "$1" </dev/null 2>/dev/null; }
# A Mac that has just started is busy with itself for a while: let it settle.
[ "$OWN" = 1 ] && sleep 30

NODE=$(vm 'for n in /opt/homebrew/bin/node /usr/local/bin/node; do [ -x "$n" ] && { echo "$n"; break; }; done' || true)
[ -n "$NODE" ] || { echo "FAIL the clean image has no node to run the drive with"; exit 1; }

# The tree as the VM sees it: the bundle, the fake, the runner and the other app — the
# same relative places, so the runner runs there unchanged.
vm "rm -rf $TREE && mkdir -p $TREE"
tar -C "$ROOT" -cf - -L "$APP" "$FAKE" app/test/surfaces-drive.mjs app/test/fullscreen-helper.js tests/test-machine.mjs | tart exec -i "$VM" tar -C "$TREE" -xf -

code=0
tart exec "$VM" sh -c "cd $TREE && exec $NODE app/test/surfaces-drive.mjs --fullscreen --strict --report=$TREE/report.json" </dev/null || code=$?
if [ -n "$KEEP" ]; then
  vm "cat $TREE/report.json" >"$KEEP" || true
fi
exit "$code"
