#!/bin/sh
# Compile + ad-hoc-sign the shroom macOS control shim, on-device.
#
# We ship READABLE SOURCE and build it here at install — never a precompiled blob.
# A precompiled binary that captures the screen and sits near cloud creds is exactly
# the opaque thing an OSS audience shouldn't be asked to trust; on-device compile
# means the bytes running are the bytes you can read in Sources/main.swift.
#
# The ad-hoc signature (`codesign -s -`) is what makes TCC work without an Apple
# Developer account: it gives the binary a stable cdhash, so the Screen-Recording
# grant the user gives persists for the life of THIS build (re-prompts only when an
# update changes the binary). See scripts/recorder (model B) and the README here.
set -eu

DIR=$(cd "$(dirname "$0")" && pwd)
SRC="$DIR/Sources/main.swift"
OUT_DIR="$DIR/build"
OUT="$OUT_DIR/shroom-shim"

# Require the Swift compiler (Xcode Command Line Tools). Don't install silently —
# tell the caller the one command that fixes it.
if ! command -v swiftc >/dev/null 2>&1; then
  echo "error: swiftc not found. Install the Command Line Tools first:" >&2
  echo "       xcode-select --install" >&2
  exit 1
fi

mkdir -p "$OUT_DIR"
swiftc -O \
  -framework Cocoa \
  -framework CoreGraphics \
  "$SRC" -o "$OUT"

# Ad-hoc sign in place (stable cdhash → persistent TCC grant for this build).
codesign --force --sign - "$OUT"

echo "built: $OUT"
