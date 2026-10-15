#!/bin/sh
# Builds the release configuration (DECISIONS B66; app/src-tauri/tauri.release.conf.json: `am.shroom`,
# product `shroom`, the `shroom` URL type, no test harness) into a throwaway target dir and
# copies the bundle to <out-dir>/shroom.app — for L5, never for publishing: the release key
# is Bagrat's (ARCHITECTURE §16 #26), so this signs with the dev identity.
#
# The build and every check of the bundle are the one release build's, tools/release-build.sh
# (DECISIONS B102) — the same script the tag workflow runs. What is here is this Mac's own:
# the throwaway target dir, the dev identity, and the guards below.
#
# This Mac must never have a build that claims shroom:// (a real invite would launch it,
# with ~/.shroom as its home): the copy is unregistered from LaunchServices, and the run
# fails if anything on this Mac answers for the scheme afterwards.
#
# The release carries its own encoder (DECISIONS B82): the sidecar tools/build-ffmpeg.sh
# makes is bundled as Contents/MacOS/ffmpeg, signed with the bundle, with FFmpeg's notice and
# licence in Contents/Resources/licenses/ffmpeg/ — checked there, with the bundle's macOS floor (B6).
#
# Usage: tests/l5/build-release.sh <out-dir>
set -eu
ROOT=$(cd "$(dirname "$0")/../.." && pwd)
out=$1
mkdir -p "$out"
tgt=$(mktemp -d)
said=$(mktemp)
lsregister=/System/Library/Frameworks/CoreServices.framework/Frameworks/LaunchServices.framework/Support/lsregister
handlers() {
  osascript -l JavaScript -e "ObjC.import('AppKit'); const a = \$.NSWorkspace.sharedWorkspace.URLsForApplicationsToOpenURL(\$.NSURL.URLWithString('shroom://join/v.example.workers.dev/AbCdEfGhIjKlMnOp')); const o = []; for (let i = 0; i < a.count; i++) o.push(a.objectAtIndex(i).path.js); o.join(' ')"
}
unregister() {
  for a in "$tgt/release/bundle/macos/shroom.app" "$out/shroom.app"; do "$lsregister" -u "$a" >/dev/null 2>&1 || true; done
}
cleanup() {
  unregister
  rm -rf "$tgt" "$said"
}
trap cleanup EXIT
# The one release build, into the throwaway target, signed with this Mac's dev identity. It
# says the floor probe's two closing lines, then the bundle's path — or what failed.
CARGO_TARGET_DIR="$tgt" "$ROOT/tools/release-build.sh" "$out" --identity shroom-dev >"$said" || { cat "$said"; exit 1; }
unregister
sed '$d' "$said"
for h in $(handlers); do
  h=${h%/}
  echo "something on this Mac claims shroom:// — $h"; exit 1
done
echo "$out/shroom.app"
