#!/usr/bin/env bash
# The dev build's `shroom` command (DECISIONS B217), built and put where the bundler takes
# it from: build/cli/shroom-cli-<this Mac's triple>, the dev bundle's
# Contents/MacOS/shroom-cli (app/src-tauri/tauri.dev-cli.conf.json). `npm run build:dev`
# runs it first, so every dev build — the hosted Mac's, the full e2e's — carries the command
# the real install line links onto PATH. The release's is tools/release-build.sh's, built for
# the bundle's floor.
#
#   tools/dev-cli.sh
set -eu
ROOT=$(cd "$(dirname "$0")/.." && pwd)
triple=$(rustc -vV | sed -n 's/^host: //p')
tgt=${CARGO_TARGET_DIR:-$ROOT/target}
case "$tgt" in /*) ;; *) tgt=$ROOT/$tgt ;; esac
(cd "$ROOT" && cargo build -q -p shroom-cli)
mkdir -p "$ROOT/build/cli"
cp "$tgt/debug/shroom-cli" "$ROOT/build/cli/shroom-cli-$triple"
