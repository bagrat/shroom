#!/bin/sh
# The one release build (DECISIONS B66, B82, B83, B102): the release configuration
# (app/src-tauri/tauri.release.conf.json — `am.shroom`, product `shroom`, the `shroom` URL
# type, no test harness, the bundled encoder) built, signed and checked. Every lane that
# makes a release-config app runs this script, so the app CI checks is the app a tag ships:
#
#   .github/workflows/release.yml   --identity <the release identity> --leaf <its pin>
#   .github/workflows/ci.yml        --adhoc   (no identity on a pull request's runner)
#   tests/l5/build-release.sh       --identity shroom-dev   (this Mac's L5 lane, with its
#                                   own guards: a throwaway target, nothing claims shroom://)
#
# Usage: tools/release-build.sh <out-dir> (--identity <name> [--leaf <sha1>] | --adhoc)
#
#   --identity <name>  sign the bundle and its encoder with this keychain identity. The
#                      bundle's designated requirement must then be `identifier "am.shroom"
#                      and certificate leaf = H"…"` — what macOS keeps Screen Recording by
#                      (RESEARCH §5b, E1/E2) — so the same certificate keeps the grant
#                      across every update.
#   --leaf <sha1>      …and that leaf must be this certificate (40 hex digits): the tag
#                      workflow passes the repo's pin, tools/release-identity.sha1, so a
#                      release signed with any other certificate is never packaged.
#   --adhoc            sign ad hoc (`-`): no identity, no Authority, a requirement that names
#                      one build only. Enough to prove the build and its bundle; never a
#                      release — macOS would ask for Screen Recording again after every
#                      update.
#
# It builds into cargo's target dir — CARGO_TARGET_DIR when set, else ./target (the one a
# CI cache keeps) — and copies the bundle to <out-dir>/shroom.app. Beside it, it leaves:
#
#   build.log          the build's own words (its last 20 lines are printed if it fails)
#   minos-probe.tsv    every Apple name newer than 12.3 the bundle reaches (B83)
#   minos-probe.log    the probe's report
#   signature.txt      who signed the bundle, and its designated requirement
#
# On success it prints the probe's two closing lines, then <out-dir>/shroom.app. It opens
# nothing and registers nothing; the bundle left in the target dir declares the shroom URL
# type, so a Mac that must never answer for shroom:// goes through tests/l5/build-release.sh.
#
# Needs: the encoder (tools/build-ffmpeg.sh), node_modules (`npm ci`), cargo, python3, Xcode's tools.
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
ID=am.shroom
SIDECAR=$ROOT/build/ffmpeg/sidecar/ffmpeg-aarch64-apple-darwin
# The `shroom` command (crates/shroom-cli, dev/proposals/cli-agent-skill.md) rides beside
# the encoder as a second external binary, signed with the bundle: Contents/MacOS/shroom-cli.
CLI=$ROOT/build/cli/shroom-cli-aarch64-apple-darwin

usage() { echo "usage: tools/release-build.sh <out-dir> (--identity <name> [--leaf <sha1>] | --adhoc)"; exit 64; }

[ $# -ge 1 ] || usage
out=$1
shift
identity=
adhoc=
leaf=
while [ $# -gt 0 ]; do
  case "$1" in
    --identity) [ $# -ge 2 ] || usage; identity=$2; shift 2 ;;
    --adhoc) adhoc=1; shift ;;
    --leaf) [ $# -ge 2 ] || usage; leaf=$2; shift 2 ;;
    *) usage ;;
  esac
done
case "$out" in ""|-*) usage ;; esac
if [ -n "$adhoc" ]; then
  [ -z "$identity" ] && [ -z "$leaf" ] || usage
  sign=-
else
  [ -n "$identity" ] || usage
  # The name goes into a line of JSON and is matched against codesign's Authority.
  case "$identity" in
    -|*[\"\\]*) echo "'$identity' isn't a signing identity's name"; exit 64 ;;
  esac
  sign=$identity
fi
if [ -n "$leaf" ]; then
  leaf=$(printf '%s' "$leaf" | tr 'A-F' 'a-f')
  case "$leaf" in
    *[!0-9a-f]*) echo "--leaf takes a certificate's SHA-1: 40 hex digits"; exit 64 ;;
  esac
  [ ${#leaf} -eq 40 ] || { echo "--leaf takes a certificate's SHA-1: 40 hex digits"; exit 64; }
fi

[ -x "$SIDECAR" ] || { echo "no encoder to bundle — run tools/build-ffmpeg.sh first"; exit 1; }
# The encoder is the one the script pins — not a stale build, not a cache's stand-in.
"$ROOT/tools/build-ffmpeg.sh" --check >/dev/null || { echo "the encoder in build/ffmpeg isn't the one tools/build-ffmpeg.sh makes — run tools/build-ffmpeg.sh"; exit 1; }
command -v npx >/dev/null 2>&1 || { echo "node isn't on the PATH here"; exit 1; }
[ -x "$ROOT/node_modules/.bin/tauri" ] || { echo "the build tools aren't installed — run npm ci"; exit 1; }
python3 "$ROOT/tools/third-party-notices.py" --check || exit 1

mkdir -p "$out"
out=$(cd "$out" && pwd)
tgt=${CARGO_TARGET_DIR:-$ROOT/target}
case "$tgt" in /*) ;; *) tgt=$(pwd)/$tgt ;; esac
bundle=$tgt/release/bundle/macos/shroom.app

# The command, built for the bundle's floor (every binary in Contents/MacOS is, below).
floor=$(plutil -extract bundle.macOS.minimumSystemVersion raw -o - "$ROOT/app/src-tauri/tauri.conf.json")
rm -f "$CLI"
(cd "$ROOT" && MACOSX_DEPLOYMENT_TARGET="$floor" CARGO_TARGET_DIR="$tgt" cargo build --release -p shroom-cli >"$out/build-cli.log" 2>&1) || { tail -n 20 "$out/build-cli.log"; exit 1; }
mkdir -p "$(dirname "$CLI")" && cp "$tgt/release/shroom" "$CLI"

# Never a bundle from an earlier build (a restored target dir may hold one).
rm -rf "$bundle" "$out/shroom.app"
(cd "$ROOT/app" && CARGO_TARGET_DIR="$tgt" npx tauri build --bundles app \
  --config src-tauri/tauri.release.conf.json \
  --config "{\"bundle\":{\"macOS\":{\"signingIdentity\":\"$sign\"}}}" >"$out/build.log" 2>&1) || { tail -n 20 "$out/build.log"; exit 1; }
[ -d "$bundle" ] || { tail -n 20 "$out/build.log"; echo "the build left no bundle at $bundle"; exit 1; }
ditto "$bundle" "$out/shroom.app"

app=$out/shroom.app
plist=$app/Contents/Info.plist
[ "$(plutil -extract CFBundleIdentifier raw -o - "$plist")" = "$ID" ] || { echo "not the release identifier"; exit 1; }
plutil -extract CFBundleURLTypes xml1 -o - "$plist" | grep -q '<string>shroom</string>' || { echo "no shroom URL type in the release bundle"; exit 1; }
! LC_ALL=C grep -a -q 'shroom:dev-harness-build:never-a-release' "$app/Contents/MacOS/shroom" || { echo "the release build carries the harness"; exit 1; }
# The version it answers by is the tree's (the tag workflow holds the tag to it).
version=$(plutil -extract version raw -o - "$ROOT/app/src-tauri/tauri.conf.json")
[ "$(plutil -extract CFBundleShortVersionString raw -o - "$plist")" = "$version" ] || { echo "the bundle isn't version $version"; exit 1; }

# Its encoder: there, arm64, signed with the bundle, the LGPL build the script made, its
# notice and licence beside it — and the bundle's floor is the one the app was built for.
enc=$app/Contents/MacOS/ffmpeg
sig() { codesign -dvv "$1" 2>&1; }
[ -x "$enc" ] || { echo "the release bundle carries no encoder"; exit 1; }
[ "$(lipo -archs "$enc")" = arm64 ] || { echo "the bundled encoder isn't arm64"; exit 1; }
cmp -s "$enc" "$SIDECAR" && { echo "the bundled encoder was never signed"; exit 1; }
codesign --verify --strict "$enc" || { echo "the bundled encoder's signature doesn't verify"; exit 1; }
codesign --verify --deep --strict "$app" || { echo "the bundle's signature doesn't verify"; exit 1; }
sig "$app" | grep -Fxq "Identifier=$ID" || { echo "the signature doesn't name $ID"; exit 1; }
requirement=$(codesign -d -r- "$app" 2>/dev/null | sed -n 's/^#* *designated => //p')
if [ -n "$adhoc" ]; then
  # Ad hoc: no Authority to name. The bundler's own signature on both — not the linker's
  # on the encoder — and a requirement that is this one build's hash.
  sig "$enc" | grep -Fxq 'Signature=adhoc' || { echo "the bundled encoder isn't signed ad hoc, as the bundle is"; exit 1; }
  ! sig "$enc" | grep -q 'linker-signed' || { echo "the bundled encoder was never signed"; exit 1; }
  sig "$app" | grep -Fxq 'Signature=adhoc' || { echo "the bundle isn't signed ad hoc"; exit 1; }
  signed="ad hoc — no identity; never a release"
else
  sig "$enc" | grep -Fxq "Authority=$identity" || { echo "the bundled encoder isn't signed with the bundle's identity"; exit 1; }
  sig "$app" | grep -Fxq "Authority=$identity" || { echo "the bundle isn't signed with $identity"; exit 1; }
  # What macOS keeps a permission by: the identifier and the certificate, nothing else.
  got=$(printf '%s\n' "$requirement" | sed -n "s/^identifier \"$ID\" and certificate leaf = H\"\\([0-9a-f]\\{40\\}\\)\"\$/\\1/p")
  [ -n "$got" ] || { echo "the bundle's designated requirement isn't its identifier and its certificate: $requirement"; exit 1; }
  if [ -n "$leaf" ] && [ "$got" != "$leaf" ]; then
    echo "the bundle is signed with certificate $got, not the pinned $leaf"; exit 1
  fi
  signed="$identity — certificate $got"
fi
{
  echo "signed: $signed"
  echo "requirement: $requirement"
  sig "$app" | grep -E '^(Identifier|Format|CodeDirectory|Signature|Authority|Timestamp|Signed Time|TeamIdentifier)[= ]'
} >"$out/signature.txt"
"$enc" -hide_banner -L 2>&1 | grep -q 'GNU Lesser General Public' || { echo "the bundled encoder isn't the LGPL build"; exit 1; }
# The command: there, signed with the bundle, and a release build — its dev-only options
# (another home, a local space) refused as unknown.
cli=$app/Contents/MacOS/shroom-cli
[ -x "$cli" ] || { echo "the release bundle carries no shroom command"; exit 1; }
codesign --verify --strict "$cli" || { echo "the bundled shroom command's signature doesn't verify"; exit 1; }
"$cli" --version | grep -q '^shroom ' || { echo "the bundled shroom command doesn't answer --version"; exit 1; }
"$cli" --home /tmp status --json | grep -q '"error":"usage"' || { echo "the bundled shroom command takes the dev-only --home"; exit 1; }
"$enc" -hide_banner -encoders 2>/dev/null | grep -q ' h264_videotoolbox ' || { echo "the bundled encoder has no VideoToolbox H.264"; exit 1; }
for f in NOTICE.txt COPYING.LGPLv2.1; do
  [ -s "$app/Contents/Resources/licenses/ffmpeg/$f" ] || { echo "the release bundle lacks FFmpeg's $f"; exit 1; }
done
[ -s "$app/Contents/Resources/licenses/third-party/NOTICES.md" ] || { echo "the release bundle lacks the third-party notices"; exit 1; }
[ "$(plutil -extract LSMinimumSystemVersion raw -o - "$plist")" = "$floor" ] || { echo "the bundle's LSMinimumSystemVersion isn't $floor"; exit 1; }
for bin in "$app/Contents/MacOS/"*; do
  [ "$(lipo -archs "$bin")" = arm64 ] || { echo "$(basename "$bin") isn't arm64"; exit 1; }
  minos=$(otool -l "$bin" | awk '/LC_BUILD_VERSION/{f=1} f && $1=="minos"{print $2; exit}')
  [ "$minos" = "$floor" ] || { echo "$(basename "$bin") is built for macOS $minos, not the bundle's $floor"; exit 1; }
done
# Nothing the bundle reaches needs a newer macOS than its floor (B6, B83).
"$ROOT/tests/l5/minos-probe.sh" "$app" "$out/minos-probe.tsv" >"$out/minos-probe.log" || { cat "$out/minos-probe.log"; exit 1; }
tail -n 2 "$out/minos-probe.log"
echo "$app"
