#!/bin/sh
# Compile + ad-hoc-sign the shroom macOS control app, on-device.
#
# We ship READABLE SOURCE and build it here at install — never a precompiled blob.
# A precompiled binary that captures the screen and sits near cloud creds is exactly
# the opaque thing an OSS audience shouldn't be asked to trust; on-device compile
# means the bytes running are the bytes you can read in Sources/main.swift. Even the
# icon is rendered on-device from the same mushroom mark the tray draws — no image blob.
#
# We package a real .app bundle (not a bare binary) for two reasons: TCC shows the
# bundle's icon + clean "shroom" name in the Privacy panes, and the ad-hoc signature
# (`codesign -s -`) gives it a stable cdhash so the grants persist for the life of THIS
# build (re-prompts only when an update changes it — no Apple Developer account needed).
# See scripts/recorder (model B) and the README here.
set -eu

DIR=$(cd "$(dirname "$0")" && pwd)
OUT_DIR="$DIR/build"
APP="$OUT_DIR/shroom.app"
BIN="$APP/Contents/MacOS/shroom"   # user-facing name = the TCC principal the user grants
RES="$APP/Contents/Resources"

# Require the Swift compiler (Xcode Command Line Tools). Don't install silently —
# tell the caller the one command that fixes it.
if ! command -v swiftc >/dev/null 2>&1; then
  echo "error: swiftc not found. Install the Command Line Tools first:" >&2
  echo "       xcode-select --install" >&2
  exit 1
fi

# Fresh bundle skeleton.
rm -rf "$APP"
mkdir -p "$APP/Contents/MacOS" "$RES"

# Compile every Sources/*.swift together (main.swift + Overlay.swift + …) straight into
# the bundle. AVFoundation: the shim requests the microphone itself (TCC names "shroom",
# not the parent). The bundle's Contents/Info.plist (copied below) carries the mic usage
# string + identity, so no -sectcreate is needed.
swiftc -O \
  -framework Cocoa \
  -framework CoreGraphics \
  -framework AVFoundation \
  "$DIR"/Sources/*.swift -o "$BIN"

# Render the icon on-device from the binary's own mushroom drawing, then pack the .icns.
ICONSET="$OUT_DIR/shroom.iconset"
rm -rf "$ICONSET"; mkdir -p "$ICONSET"
for s in 16 32 128 256 512; do
  "$BIN" --render-icon "$s"          "$ICONSET/icon_${s}x${s}.png"
  "$BIN" --render-icon "$((s * 2))"  "$ICONSET/icon_${s}x${s}@2x.png"
done
iconutil -c icns "$ICONSET" -o "$RES/shroom.icns"
rm -rf "$ICONSET"

# The bundle's authoritative Info.plist (mic usage string, identity, icon, LSUIElement).
cp "$DIR/Info.plist" "$APP/Contents/Info.plist"

# Ad-hoc sign the whole bundle (seals the executable + resources; stable cdhash).
codesign --force --sign - "$APP"

echo "built: $APP"
