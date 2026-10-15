#!/bin/sh
# Builds the release's encoder (DECISIONS B82): a minimal, LGPL-only FFmpeg — only what the
# app's argv names — from the official source tarball, pinned and checksum-verified, into
# the git-ignored build dir (TESTING-PLAN §9 M6). The release bundle carries the result as
# its sidecar (`Contents/MacOS/ffmpeg`, app/src-tauri/tauri.release.conf.json); the notice
# it ships beside it is tools/ffmpeg-NOTICE.txt.
#
#   tools/build-ffmpeg.sh            build (reusing a verified download)
#   tools/build-ffmpeg.sh --clean    remove the build dir first, then build from nothing
#   tools/build-ffmpeg.sh --check    build nothing: is the encoder already in build/ffmpeg
#                                    the one this script, as it is now, makes?
#
# Out: build/ffmpeg/sidecar/ffmpeg-aarch64-apple-darwin (Tauri's externalBin name), its
# license (COPYING.LGPLv2.1, from the tarball) and BUILDINFO (version, URL, SHA-256, the
# floor, the configure line, the built binary's own SHA-256). Nothing is installed anywhere
# else; a missing build tool stops the run.
#
# --check is for an encoder that came from somewhere other than this run — a CI cache, an
# old build dir: its BUILDINFO must be exactly what this script would write (the same pin,
# floor and configure line, and the digest of the binary that is there), and the binary must
# pass every check a fresh build passes (DECISIONS B102). The release build runs it.
#
# The pin: FFmpeg 8.1.2 — the version the dev builds and the media goldens were made with.
# The SHA-256 below is the tarball's as fetched from ffmpeg.org on 2026-10-03, and the same
# digest Homebrew's own formula records for the same URL (homebrew-core `ffmpeg.rb`, 8.1.2)
# — two witnesses. FFmpeg also signs each tarball (`$URL.asc`); this Mac has no gpg, so the
# digest is the gate. A different digest is never accepted: the download is deleted.
#
# What is in it (each one because an argv in crates/shroom-encode or shroom-record names it):
#   encoders  h264_videotoolbox (every take, the teaser) · aac (sound) · mjpeg (the poster) ·
#             wrapped_avframe (`-f null`, the playable check)
#   decoders  rawvideo + pcm_f32le (the take's two pipes) · h264 + aac (the poster and
#             teaser read the single-file copy; the playable check reads a frame)
#   muxers    hls (fMP4) · mp4 · tee · image2 (poster.jpg) · null
#   demuxers  rawvideo · pcm_f32le (`-f f32le`) · concat (joining takes) · mov · hls (the
#             recovery's playlist remux)
#   protocols file · pipe — no network
#   filters   scale (BGRA → yuv420p, the poster's and teaser's sizes) · aresample (a later
#             take's rate) · the CLI's own (format, aformat, null, anull, trim, atrim, …)
# Never --enable-gpl, never --enable-nonfree, never an external library: the licence is
# FFmpeg's LGPL v2.1 or later, and the run fails if the built binary says otherwise.
set -eu

VERSION=8.1.2
URL="https://ffmpeg.org/releases/ffmpeg-$VERSION.tar.xz"
SHA256=464beb5e7bf0c311e68b45ae2f04e9cc2af88851abb4082231742a74d97b524c
TRIPLE=aarch64-apple-darwin

ROOT=$(cd "$(dirname "$0")/.." && pwd)
OUT=$ROOT/build/ffmpeg

fail() { echo "build-ffmpeg: $*" >&2; exit 1; }

mode=build
case "${1-}" in
  "") ;;
  --clean) rm -rf "$OUT" ;;
  --check) mode=check ;;
  *) fail "usage: tools/build-ffmpeg.sh [--clean | --check]" ;;
esac
[ "$(uname -s)" = Darwin ] || fail "builds on macOS only"
[ "$(uname -m)" = arm64 ] || fail "builds on Apple silicon only (B7: v1 is arm64)"
if [ "$mode" = check ]; then tools="shasum plutil otool lipo"; else tools="clang make curl shasum tar xcrun plutil otool lipo"; fi
for tool in $tools; do
  command -v "$tool" >/dev/null 2>&1 || fail "needs $tool, which isn't on this Mac — nothing is installed for you; add it, then run this again"
done

# The bundle's floor (tauri.conf.json's minimumSystemVersion) is the encoder's: the sidecar
# must run wherever the app does (B6).
FLOOR=$(plutil -extract bundle.macOS.minimumSystemVersion raw -o - "$ROOT/app/src-tauri/tauri.conf.json")
case "$FLOOR" in [0-9]*.[0-9]*) ;; *) fail "unexpected macOS floor '$FLOOR' in tauri.conf.json" ;; esac

set -- \
  --disable-everything --disable-autodetect \
  --enable-pthreads --enable-videotoolbox \
  --disable-ffplay --disable-ffprobe --disable-avdevice --disable-network \
  --disable-doc --disable-debug \
  --enable-static --disable-shared \
  --enable-encoder=h264_videotoolbox,aac,mjpeg,wrapped_avframe \
  --enable-decoder=rawvideo,pcm_f32le,h264,aac \
  --enable-muxer=hls,mp4,tee,image2,null \
  --enable-demuxer=rawvideo,pcm_f32le,concat,mov,hls \
  --enable-protocol=file,pipe \
  --enable-parser=h264,aac \
  --enable-bsf=aac_adtstoasc,h264_mp4toannexb \
  --enable-filter=scale,aresample,format,aformat,null,anull,trim,atrim,setpts,asetpts \
  --arch=aarch64 --cc=clang \
  --extra-cflags="-mmacosx-version-min=$FLOOR" \
  --extra-ldflags="-mmacosx-version-min=$FLOOR"
configure_line="./configure $*"

digest() { shasum -a 256 "$1" | cut -d ' ' -f 1; }

# What BUILDINFO says of a binary with this SHA-256, built by this script as it is now.
buildinfo() {
  echo "FFmpeg $VERSION"
  echo "source $URL"
  echo "sha256 $SHA256"
  echo "macos-min $FLOOR"
  echo "configure $configure_line"
  echo "binary-sha256 $1"
}

# What every encoder must be, fresh from the build or back from a cache.
verify() {
  # The licence the binary itself reports, and nothing GPL or nonfree in what it was built with.
  "$1" -hide_banner -L 2>&1 | grep -q 'GNU Lesser General Public' || fail "the built ffmpeg doesn't report the LGPL"
  ! "$1" -hide_banner -buildconf 2>&1 | grep -Eq 'enable-(gpl|nonfree|version3)' || fail "the built ffmpeg was configured with GPL, version3 or nonfree parts"
  # One self-contained arm64 executable that links only the system's own libraries.
  [ "$(lipo -archs "$1")" = arm64 ] || fail "not an arm64-only binary"
  if otool -L "$1" | tail -n +2 | awk '{print $1}' | grep -Ev '^(/usr/lib/|/System/Library/)' | grep -q .; then
    otool -L "$1"; fail "the built ffmpeg links something outside the system"
  fi
  minos=$(otool -l "$1" | awk '/LC_BUILD_VERSION/{f=1} f && $1=="minos"{print $2; exit}')
  [ "$minos" = "$FLOOR" ] || fail "built for macOS $minos, not the app's floor $FLOOR"
  for enc in h264_videotoolbox aac mjpeg; do
    "$1" -hide_banner -encoders 2>/dev/null | grep -q " $enc " || fail "the $enc encoder is missing"
  done
}

if [ "$mode" = check ]; then
  bin=$OUT/sidecar/ffmpeg-$TRIPLE
  [ -x "$bin" ] || fail "no encoder at $bin — run tools/build-ffmpeg.sh"
  [ -s "$OUT/sidecar/COPYING.LGPLv2.1" ] || fail "the encoder's licence isn't beside it — run tools/build-ffmpeg.sh"
  [ -f "$OUT/sidecar/BUILDINFO" ] || fail "the encoder has no BUILDINFO — run tools/build-ffmpeg.sh"
  [ "$(cat "$OUT/sidecar/BUILDINFO")" = "$(buildinfo "$(digest "$bin")")" ] || fail "the encoder in build/ffmpeg isn't the one this script makes now (its BUILDINFO says otherwise) — run tools/build-ffmpeg.sh"
  verify "$bin"
  echo "checked $bin (FFmpeg $VERSION, macOS $FLOOR+, LGPL v2.1+, sha256 $(digest "$bin"))"
  exit 0
fi

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
echo "verified ffmpeg-$VERSION.tar.xz sha256 $SHA256"

rm -rf "$OUT/src" "$OUT/sidecar"
mkdir -p "$OUT/src" "$OUT/sidecar"
tar -xJf "$tarball" -C "$OUT/src"
src=$OUT/src/ffmpeg-$VERSION


# A bare environment: nothing from Homebrew (headers, libraries, pkg-config) can reach the
# build — only the Xcode toolchain and the macOS SDK.
sdk=$(xcrun --sdk macosx --show-sdk-path)
( cd "$src" && env -i HOME="$HOME" PATH=/usr/bin:/bin:/usr/sbin:/sbin SDKROOT="$sdk" \
    PKG_CONFIG_LIBDIR="$OUT/no-pkg-config" TMPDIR="${TMPDIR:-/tmp}" \
    ./configure "$@" >"$OUT/configure.log" 2>&1 ) || { tail -n 30 "$OUT/configure.log"; fail "configure failed (log: $OUT/configure.log)"; }
( cd "$src" && env -i HOME="$HOME" PATH=/usr/bin:/bin:/usr/sbin:/sbin SDKROOT="$sdk" TMPDIR="${TMPDIR:-/tmp}" \
    make -j"$(sysctl -n hw.ncpu)" ffmpeg >"$OUT/make.log" 2>&1 ) || { tail -n 30 "$OUT/make.log"; fail "make failed (log: $OUT/make.log)"; }

bin=$src/ffmpeg
# Nothing in it reaches an API newer than the floor unguarded: FFmpeg's own build makes that
# an error (-Werror=partial-availability, against -mmacosx-version-min), and this run checks
# the log for the warning besides.
! grep -q 'is only available on macOS' "$OUT/make.log" || { grep 'is only available on macOS' "$OUT/make.log" | head; fail "the encoder calls an API newer than macOS $FLOOR unguarded"; }
verify "$bin"

install -m 755 "$bin" "$OUT/sidecar/ffmpeg-$TRIPLE"
cp "$src/COPYING.LGPLv2.1" "$OUT/sidecar/COPYING.LGPLv2.1"
buildinfo "$(digest "$OUT/sidecar/ffmpeg-$TRIPLE")" >"$OUT/sidecar/BUILDINFO"
echo "built $OUT/sidecar/ffmpeg-$TRIPLE ($(du -h "$OUT/sidecar/ffmpeg-$TRIPLE" | cut -f 1 | tr -d ' '), macOS $FLOOR+, LGPL v2.1+)"
