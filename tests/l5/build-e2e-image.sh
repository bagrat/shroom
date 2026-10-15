#!/bin/sh
# The e2e golden image (dev/proposals/heartbeat-in-vm.md §4, ratified 2026-10-14):
# `shroom-clean-tahoe-e2e`, the clean Tahoe image plus what the heartbeat needs in a clone
# and the clean image must not have — Homebrew's ffmpeg (with ffprobe) and cliclick, a
# silent input device (BlackHole 2ch), and
# Playwright's WebKit and Chromium at the tree's own Playwright version. Built once; every
# tests/l5/heartbeat-vm.sh run clones it and throws the clone away.
#
# The clean image stays clean: installer-vm.sh proves a fresh Mac has no ffmpeg of its own,
# so these never go into `shroom-clean-tahoe` itself.
#
# Usage: tests/l5/build-e2e-image.sh [--replace]
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
BASE=shroom-clean-tahoe
IMAGE=shroom-clean-tahoe-e2e
command -v tart >/dev/null || { echo "tart isn't installed here"; exit 1; }
tart list --quiet 2>/dev/null | grep -qx "$BASE" || { echo "no $BASE image to start from"; exit 1; }
if tart list --quiet 2>/dev/null | grep -qx "$IMAGE"; then
  [ "${1:-}" = --replace ] || { echo "$IMAGE is already here (--replace builds it again)"; exit 1; }
  tart delete "$IMAGE"
fi
PLAYWRIGHT=$(node -p "require('$ROOT/package-lock.json').packages['node_modules/playwright'].version")

VM=shroom-l5-image-$$
done_ok=
cleanup() {
  tart stop "$VM" >/dev/null 2>&1 || true
  [ -n "$done_ok" ] || tart delete "$VM" >/dev/null 2>&1 || true
}
trap cleanup EXIT

tart clone "$BASE" "$VM"
tart run "$VM" --no-graphics --no-audio >/dev/null 2>&1 &
i=0
until tart exec "$VM" true >/dev/null 2>&1; do
  i=$((i + 1)); [ "$i" -gt 90 ] && { echo "the VM never came up"; exit 1; }
  sleep 2
done
vm() { tart exec "$VM" sh -c "export PATH=/opt/homebrew/bin:/usr/local/bin:\$PATH HOMEBREW_NO_AUTO_UPDATE=1 HOMEBREW_NO_INSTALL_CLEANUP=1 HOMEBREW_NO_ENV_HINTS=1; $1" </dev/null; }

[ "$(vm 'csrutil status')" = "System Integrity Protection status: disabled." ] || { echo "$BASE isn't a golden image (SIP is on)"; exit 1; }
vm 'command -v brew >/dev/null' || { echo "$BASE has no Homebrew"; exit 1; }
node_major=$(vm 'node -p "process.versions.node.split(\".\")[0]"' || echo 0)
[ "$node_major" -ge 22 ] || { echo "$BASE's node is ${node_major:-missing}; the heartbeat wants 22"; exit 1; }

echo "── ffmpeg (with ffprobe) and cliclick ──"
vm 'brew install ffmpeg cliclick'
vm 'ffprobe -version | head -n 1 && cliclick -V'

# A silent input device, so a drive's take has its beep: the app takes the pattern's beep in
# a microphone's place only when one is named, and the clone runs --no-audio (with audio,
# Tart would hand it this Mac's own microphone — the room). BlackHole is a null loopback:
# nothing it carries reaches this Mac.
echo "── a silent input device (BlackHole 2ch) ──"
vm 'brew install --cask blackhole-2ch && sudo killall coreaudiod; sleep 3; system_profiler SPAudioDataType | grep -q BlackHole' ||
  { echo "no BlackHole input device in the clone"; exit 1; }

echo "── Playwright $PLAYWRIGHT's WebKit and Chromium ──"
vm "cd /tmp && npx -y playwright@$PLAYWRIGHT install webkit chromium"
# Both browsers really on disk (the first build lost Chromium's headless shell to a stop
# that came before the disk caught up), then a clean shutdown, not a stop.
vm 'ls ~/Library/Caches/ms-playwright/chromium_headless_shell-*/chrome-headless-shell-mac-arm64/chrome-headless-shell ~/Library/Caches/ms-playwright/webkit-*/pw_run.sh' ||
  { echo "the browsers aren't where Playwright looks"; exit 1; }
vm 'sync; sudo /sbin/shutdown -h now' || true
i=0
while tart list --format json 2>/dev/null | node -e 'const l=JSON.parse(require("fs").readFileSync(0,"utf8"));process.exit(l.some(v=>v.Name===process.argv[1]&&v.Running)?0:1)' "$VM"; do
  i=$((i + 1)); [ "$i" -gt 60 ] && { echo "the VM never shut down"; exit 1; }
  sleep 2
done
tart stop "$VM" >/dev/null 2>&1 || true
# The clone becomes the image, under its name.
tart rename "$VM" "$IMAGE"
done_ok=1
echo "built $IMAGE from $BASE, with Playwright $PLAYWRIGHT's browsers"
