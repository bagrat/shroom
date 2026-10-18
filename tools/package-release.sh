#!/bin/sh
# Packages a built shroom.app as a release the installer and the updater read (DECISIONS
# B29, B31): shroom-<version>-<arch>.zip made with ditto (signatures intact) and
# manifest.txt with the version, the macOS floor and each file's SHA-256, and update.txt, the
# same file under the name the app's updater asks for (B136).
#
# Usage: tools/package-release.sh <path/to/shroom.app> <version> [out-dir] [macos-floor]
#
# The floor defaults to the bundle's own (LSMinimumSystemVersion, B6), and a floor below it
# is refused: the installer would put the app on a Mac it can't open on. A release carries
# its own encoder (B82): Contents/MacOS/ffmpeg and FFmpeg's notice and licence in
# Contents/Resources/licenses/ffmpeg/ — a release without them can't record, and is refused.
#
# It only prepares files. Publishing is a deliberate act: it prints the `gh release
# create` command for you to run (a release feeds every install and update). The usual
# road is a pushed tag — .github/workflows/release.yml builds, signs, runs this and
# publishes (docs/RELEASING.md, DECISIONS B102); the printed command is the by-hand one.
#
# A dev build is never a release (DECISIONS B62): an identifier ending in `.dev`, or a
# binary carrying the test harness's marker (the `dev-harness` feature compiles it in), is
# refused. The test lanes that package a dev build as a loopback or VM stand-in (the update
# drive, L5) say so with SHROOM_STAND_IN=1 — the files are made, and no publish command is
# printed.
set -eu

app=$1
version=$2
out=${3:-dist/release/v$version}
floor_arg=${4-}

# The shapes the installer and the updater both accept (B29, B31): a version of one to
# three whole numbers, and a whole-number macOS floor (the updater reads it as one).
case "$version" in
  ""|.*|*.|*..*|*.*.*.*|*[!0-9.]*) echo "version must look like 1.2.3"; exit 1 ;;
esac
[ -d "$app" ] || { echo "no app at $app"; exit 1; }
# The bundle's own floor (LSMinimumSystemVersion), as the whole number the updater reads.
bundle_floor=$(plutil -extract LSMinimumSystemVersion raw -o - "$app/Contents/Info.plist" 2>/dev/null || true)
case "$bundle_floor" in
  "") bundle_major= ;;
  *.0|*.0.0) bundle_major=${bundle_floor%%.*} ;;
  *[!0-9]*) echo "the bundle's macOS floor $bundle_floor isn't a whole version — the updater reads whole numbers only"; exit 1 ;;
  *) bundle_major=$bundle_floor ;;
esac
if [ $# -ge 4 ]; then floor=$floor_arg; else floor=${bundle_major:-14}; fi
case "$floor" in
  ""|*[!0-9]*) echo "the macOS floor must be a whole number, like 14"; exit 1 ;;
esac
if [ -n "$bundle_major" ] && [ "$floor" -lt "$bundle_major" ]; then
  echo "the floor $floor is below the app's own (macOS $bundle_major) — it can't open there"; exit 1
fi

id=$(plutil -extract CFBundleIdentifier raw -o - "$app/Contents/Info.plist" 2>/dev/null || true)
harness=no
for bin in "$app/Contents/MacOS/"*; do
  if LC_ALL=C grep -a -q 'shroom:dev-harness-build:never-a-release' "$bin" 2>/dev/null; then harness=yes; fi
done
stand_in=${SHROOM_STAND_IN:-}
if [ -z "$stand_in" ]; then
  case "$id" in
    "") echo "the app has no bundle identifier — not a shroom build"; exit 1 ;;
    *.dev) echo "$id is a dev build — never a release (B62); build the release config instead"; exit 1 ;;
  esac
fi
if [ -z "$stand_in" ]; then
  [ "$harness" = no ] || { echo "this build carries the test harness — never a release (B62); build it without --features dev-harness"; exit 1; }
  [ -x "$app/Contents/MacOS/ffmpeg" ] || { echo "this build carries no encoder — it can't record (B82); run tools/build-ffmpeg.sh, then build the release config"; exit 1; }
  for f in NOTICE.txt COPYING.LGPLv2.1; do
    [ -s "$app/Contents/Resources/licenses/ffmpeg/$f" ] || { echo "this build lacks FFmpeg's $f — never ship the encoder without its licence (B82)"; exit 1; }
  done
fi
codesign --verify --deep "$app" || { echo "the app's signature doesn't verify — never ship unsigned arm64 code"; exit 1; }

# Every executable in the bundle — the app and its encoder — is arm64.
for bin in "$app/Contents/MacOS/"*; do
  arch=$(lipo -archs "$bin" 2>/dev/null || true)
  case "$arch" in
    arm64) ;;
    *) echo "expected an arm64 build, got: $arch in $(basename "$bin") (B7: v1 is Apple silicon only)"; exit 1 ;;
  esac
done

mkdir -p "$out"
stage=$(mktemp -d)
trap 'rm -rf "$stage"' EXIT
ditto "$app" "$stage/shroom.app"
file="shroom-$version-arm64.zip"
ditto -c -k --keepParent "$stage/shroom.app" "$out/$file"
sum=$(shasum -a 256 "$out/$file" | cut -d ' ' -f 1)
printf 'version=%s\nmacos=%s\narm64=%s %s\n' "$version" "$floor" "$file" "$sum" > "$out/manifest.txt"
# It carries the `shroom` command (DECISIONS B217): the cask links it only then.
if [ -x "$app/Contents/MacOS/shroom-cli" ]; then echo "cli=shroom-cli" >> "$out/manifest.txt"; fi
# The same file under the name the app's updater asks for, so the release's download
# counts tell its checks from the install line's runs (DECISIONS B136).
cp "$out/manifest.txt" "$out/update.txt"

echo "packaged $out/$file"
echo "  sha256 $sum"
echo "manifest:"
sed 's/^/  /' "$out/manifest.txt"
echo
if [ -n "$stand_in" ]; then
  echo "A stand-in release for tests ($id, harness: $harness) — never publish it."
  exit 0
fi
echo "To publish (only when you mean it):"
echo "  gh release create v$version --repo bagrat/shroom $out/$file $out/manifest.txt $out/update.txt"
