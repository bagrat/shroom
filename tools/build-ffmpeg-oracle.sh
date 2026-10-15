#!/bin/sh
# Builds the media tests' oracle for a Mac with no ffmpeg of its own — CI's runner
# (DECISIONS B102).
#
# The media tests encode with the ffmpeg on PATH (or SHROOM_FFMPEG) and read the result back
# with the ffmpeg and ffprobe on PATH — an independent oracle (crates/shroom-encode's
# locate::under_test). They fail by design where there is none. On this Mac that pair is
# Homebrew's; a hosted runner has neither, so this script makes it: FFmpeg's own `ffmpeg`
# and `ffprobe`, whole (every built-in codec, format and filter — the tests' sources are
# `lavfi` ones the release's minimal encoder doesn't carry), from the very tarball
# tools/build-ffmpeg.sh pins — the version and SHA-256 are read from that script, so there
# is one pin. Never shipped: the release carries only the minimal encoder.
#
#   tools/build-ffmpeg-oracle.sh           build (reusing a verified download)
#   tools/build-ffmpeg-oracle.sh --check   build nothing: is the pair already in
#                                          build/ffmpeg-oracle the one this script makes now?
#
# Out: build/ffmpeg-oracle/bin/ffmpeg and ffprobe (put that folder on PATH), and BUILDINFO.
# LGPL like the encoder (never --enable-gpl or --enable-nonfree), no external library, a
# bare build environment. Nothing is installed anywhere else.
set -eu

ROOT=$(cd "$(dirname "$0")/.." && pwd)
PINS=$ROOT/tools/build-ffmpeg.sh
OUT=$ROOT/build/ffmpeg-oracle

fail() { echo "build-ffmpeg-oracle: $*" >&2; exit 1; }
pin() { sed -n "s/^$1=\\(.*\\)\$/\\1/p" "$PINS" | head -n 1 | tr -d '"'; }
VERSION=$(pin VERSION)
SHA256=$(pin SHA256)
[ -n "$VERSION" ] && [ ${#SHA256} -eq 64 ] || fail "couldn't read FFmpeg's pin from tools/build-ffmpeg.sh"
URL="https://ffmpeg.org/releases/ffmpeg-$VERSION.tar.xz"

mode=build
case "${1-}" in
  "") ;;
  --check) mode=check ;;
  *) fail "usage: tools/build-ffmpeg-oracle.sh [--check]" ;;
esac
[ "$(uname -s)" = Darwin ] || fail "builds on macOS only"
[ "$(uname -m)" = arm64 ] || fail "builds on Apple silicon only"

set -- \
  --disable-autodetect --enable-videotoolbox --enable-zlib \
  --disable-ffplay --disable-doc --disable-debug \
  --enable-static --disable-shared \
  --arch=aarch64 --cc=clang
configure_line="./configure $*"

digest() { shasum -a 256 "$1" | cut -d ' ' -f 1; }
buildinfo() {
  echo "FFmpeg $VERSION"
  echo "source $URL"
  echo "sha256 $SHA256"
  echo "configure $configure_line"
  echo "ffmpeg-sha256 $(digest "$OUT/bin/ffmpeg")"
  echo "ffprobe-sha256 $(digest "$OUT/bin/ffprobe")"
}
# What the tests ask of the pair.
verify() {
  "$OUT/bin/ffmpeg" -hide_banner -version | head -n 1 | grep -q "^ffmpeg version $VERSION " || fail "the built ffmpeg isn't $VERSION"
  "$OUT/bin/ffprobe" -hide_banner -version | head -n 1 | grep -q "^ffprobe version $VERSION " || fail "the built ffprobe isn't $VERSION"
  ! "$OUT/bin/ffmpeg" -hide_banner -buildconf 2>&1 | grep -Eq 'enable-(gpl|nonfree|version3)' || fail "the built ffmpeg was configured with GPL, version3 or nonfree parts"
  for enc in h264_videotoolbox aac mjpeg; do
    "$OUT/bin/ffmpeg" -hide_banner -encoders 2>/dev/null | grep -q " $enc " || fail "the $enc encoder is missing"
  done
  for f in testsrc2 sine; do
    "$OUT/bin/ffmpeg" -hide_banner -filters 2>/dev/null | grep -q " $f " || fail "the $f test source is missing"
  done
  "$OUT/bin/ffmpeg" -hide_banner -devices 2>/dev/null | grep -q ' lavfi ' || fail "the lavfi input is missing"
}

if [ "$mode" = check ]; then
  [ -x "$OUT/bin/ffmpeg" ] && [ -x "$OUT/bin/ffprobe" ] || fail "no oracle in $OUT/bin — run tools/build-ffmpeg-oracle.sh"
  [ -f "$OUT/BUILDINFO" ] && [ "$(cat "$OUT/BUILDINFO")" = "$(buildinfo)" ] || fail "the oracle in build/ffmpeg-oracle isn't the one this script makes now — run tools/build-ffmpeg-oracle.sh"
  verify
  echo "checked $OUT/bin (ffmpeg and ffprobe $VERSION)"
  exit 0
fi

for tool in clang make curl shasum tar xcrun; do
  command -v "$tool" >/dev/null 2>&1 || fail "needs $tool, which isn't on this Mac — nothing is installed for you; add it, then run this again"
done

# The tarball: the encoder build's verified download if it is there, else fetched — and
# never accepted under any other digest.
tarball=$ROOT/build/ffmpeg/dl/ffmpeg-$VERSION.tar.xz
if [ ! -f "$tarball" ] || [ "$(digest "$tarball")" != "$SHA256" ]; then
  mkdir -p "$OUT/dl"
  tarball=$OUT/dl/ffmpeg-$VERSION.tar.xz
  if [ ! -f "$tarball" ] || [ "$(digest "$tarball")" != "$SHA256" ]; then
    rm -f "$tarball" "$tarball.part"
    echo "fetching $URL"
    curl -fsSL --proto '=https' --tlsv1.2 -o "$tarball.part" "$URL" || fail "couldn't fetch $URL"
    got=$(digest "$tarball.part")
    if [ "$got" != "$SHA256" ]; then
      rm -f "$tarball.part"
      fail "the tarball's SHA-256 is $got, not the pinned $SHA256 — deleted, nothing built"
    fi
    mv "$tarball.part" "$tarball"
  fi
fi
echo "verified ffmpeg-$VERSION.tar.xz sha256 $SHA256"

rm -rf "$OUT/src" "$OUT/bin" "$OUT/BUILDINFO"
mkdir -p "$OUT/src" "$OUT/bin"
tar -xJf "$tarball" -C "$OUT/src"
src=$OUT/src/ffmpeg-$VERSION

# A bare environment, as the encoder's build: only the Xcode toolchain and the macOS SDK.
sdk=$(xcrun --sdk macosx --show-sdk-path)
( cd "$src" && env -i HOME="$HOME" PATH=/usr/bin:/bin:/usr/sbin:/sbin SDKROOT="$sdk" \
    PKG_CONFIG_LIBDIR="$OUT/no-pkg-config" TMPDIR="${TMPDIR:-/tmp}" \
    ./configure "$@" >"$OUT/configure.log" 2>&1 ) || { tail -n 30 "$OUT/configure.log"; fail "configure failed (log: $OUT/configure.log)"; }
( cd "$src" && env -i HOME="$HOME" PATH=/usr/bin:/bin:/usr/sbin:/sbin SDKROOT="$sdk" TMPDIR="${TMPDIR:-/tmp}" \
    make -j"$(sysctl -n hw.ncpu)" ffmpeg ffprobe >"$OUT/make.log" 2>&1 ) || { tail -n 30 "$OUT/make.log"; fail "make failed (log: $OUT/make.log)"; }

install -m 755 "$src/ffmpeg" "$OUT/bin/ffmpeg"
install -m 755 "$src/ffprobe" "$OUT/bin/ffprobe"
verify
buildinfo >"$OUT/BUILDINFO"
# The source tree is hundreds of megabytes of objects nobody reads again.
rm -rf "$OUT/src"
echo "built $OUT/bin/ffmpeg and ffprobe ($VERSION) — put $OUT/bin on PATH"
