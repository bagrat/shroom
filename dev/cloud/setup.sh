#!/bin/bash
# Cloud-session provisioning for working on shroom v1 (Claude Code cloud, Ubuntu x86_64).
# The environment's "Setup script" field just runs this file, so changes land by
# editing the repo — not the UI. It is also re-run (cheaply) by the SessionStart hook,
# because a cached environment skips the UI setup script until its cache expires.
#
# Idempotent: every step checks before installing, so a warm machine is a no-op in
# ~1s. Never fails the session: a step that can't install warns and moves on.
# Linux only: on a Mac it says so and leaves — it installs nothing there.
#
# What it gives you (v1's Linux-runnable part; capture, the app bundle and the encoder
# are the Mac's — the shroom-cloud skill's §4 has the list):
#   node >= 22 (preinstalled)          the Worker's tests, the webviews' build, the galleries
#   Rust >= Cargo.toml's rust-version  the crates' tests (and a C linker for them)
#   ffmpeg + ffprobe                   the test stream behind the page's faces
#   node_modules (npm ci)              vitest, workerd, vite, svelte-check, Playwright
#   ssh + ssh-keygen                   dev/mac/loop.sh --hold (a held Mac)
#   Playwright's browsers and the crates' sources — the slow two, see the note below
set -uo pipefail

# Nothing here needs a live test token, and npm ci runs dependencies' install scripts:
# keep them out of their reach whoever launched this (the first is retired with
# main-account testing; the e2e account's token is read from the environment by its own
# workflow steps).
unset SHROOM_TEST_TOKEN_B64 SHROOM_E2E_TOKEN

log() { printf '[shroom-cloud-setup] %s\n' "$*"; }
warn() { printf '[shroom-cloud-setup] WARN: %s\n' "$*" >&2; }

if [ "$(uname -s)" != "Linux" ]; then
  log "not Linux — this provisions cloud sessions and changes nothing on a Mac"
  exit 0
fi

HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"

# No checkout around this file (copied out, or a partial checkout): nothing to set up.
# Skip the steps that read the workspace rather than failing the session's setup.
if [ ! -f "$ROOT/Cargo.toml" ] || [ ! -f "$ROOT/package.json" ]; then
  log "no shroom checkout at $ROOT — skipping setup"
  exit 0
fi
. "$HERE/path.sh"

as_root() { if [ "$(id -u)" = 0 ]; then "$@"; else sudo -n "$@"; fi; }
APT_UPDATED=""
apt_install() {
  if [ -z "$APT_UPDATED" ]; then as_root apt-get update -qq || return 1; APT_UPDATED=1; fi
  as_root env DEBIAN_FRONTEND=noninteractive apt-get install -y -qq "$@"
}

# Node >= 22 — the image ships it; path.sh already reached for /opt/node22 if PATH's is old.
major() { "$1" -v 2>/dev/null | sed 's/^v\([0-9]*\).*/\1/'; }
if [ "$(major node || echo 0)" -lt 22 ] 2>/dev/null; then
  warn "no Node >= 22 found — the Worker's tests, the build and the galleries need it"
fi

# ffmpeg + ffprobe — preinstalled today; install if a future image drops them. Any stock
# build will do: nothing that runs here encodes the way the app does (that's VideoToolbox).
if ! command -v ffmpeg >/dev/null 2>&1 || ! command -v ffprobe >/dev/null 2>&1; then
  log "installing ffmpeg"
  apt_install ffmpeg || warn "ffmpeg install failed"
fi

# A C compiler and linker — rustc links through `cc`, and one crate (ring) compiles C.
if ! command -v cc >/dev/null 2>&1; then
  log "installing build-essential"
  apt_install build-essential || warn "build-essential install failed"
fi

# ssh + ssh-keygen — dev/mac/loop.sh's --hold makes a key and opens the Mac's door with them.
if ! command -v ssh-keygen >/dev/null 2>&1 || ! command -v ssh >/dev/null 2>&1; then
  log "installing openssh-client"
  apt_install openssh-client || warn "openssh-client install failed"
fi

# Rust — at least the workspace's rust-version, read from Cargo.toml so it can't drift.
need=$(sed -n 's/^rust-version *= *"\(.*\)"/\1/p' "$ROOT/Cargo.toml" | head -1)
rust_ok() {
  command -v cargo >/dev/null 2>&1 || return 1
  local have; have=$(cargo -V 2>/dev/null | awk '{print $2}')
  [ -n "$have" ] && [ "$(printf '%s\n%s\n' "${need:-0}" "$have" | sort -V | head -1)" = "${need:-0}" ]
}
if ! rust_ok; then
  if command -v rustup >/dev/null 2>&1; then
    log "Rust is missing or older than ${need:-the workspace needs}; installing stable with rustup"
    { rustup toolchain install stable --profile minimal >/dev/null 2>&1 && rustup default stable >/dev/null 2>&1; } \
      || warn "rustup couldn't install stable"
  else
    log "installing Rust (rustup, minimal profile) into ~/.cargo"
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs 2>/dev/null \
      | sh -s -- -y --profile minimal --no-modify-path >/dev/null 2>&1 || warn "Rust install failed"
  fi
  . "$HERE/path.sh"
  rust_ok || warn "no Rust >= ${need:-?} — the crates' tests won't run"
fi

# node_modules — the workspace's locked dev dependencies (app, worker, tests/e2e).
if [ ! -d "$ROOT/node_modules" ] || [ "$ROOT/package-lock.json" -nt "$ROOT/node_modules/.package-lock.json" ]; then
  log "installing node_modules (npm ci)"
  (cd "$ROOT" && npm ci --no-audit --no-fund >/dev/null 2>&1) || warn "npm ci failed"
fi

# The slow two — deferred by default: the SessionStart hook starts them in the background
# so sessions start immediately. Playwright's browsers and their system libraries
# (dev/cloud/playwright.sh, ~2 min cold) and the crates' sources (cargo fetch, ~2 min
# cold: the workspace pins two macOS-only git dependencies that cargo fetches on every
# platform before it will build anything). SHROOM_CLOUD_HEAVY=setup does both here
# instead, baking them into the environment cache (the setup step then takes a few
# minutes, but only when the cache rebuilds). SHROOM_CLOUD_HEAVY=0 disables both.
if [ "${SHROOM_CLOUD_HEAVY:-defer}" = "setup" ]; then
  bash "$HERE/playwright.sh" install >/dev/null 2>&1 || warn "Playwright install failed"
  if command -v cargo >/dev/null 2>&1; then
    (cd "$ROOT" && cargo fetch --locked --target "$(rustc -vV | sed -n 's/^host: //p')" >/dev/null 2>&1) \
      || warn "cargo fetch failed"
  fi
fi

exit 0
