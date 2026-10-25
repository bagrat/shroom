#!/bin/sh
# The Rust workspace's tests, as CI runs them (DECISIONS B102).
#
#   rust-tests.sh               everything — `cargo test --workspace`, as on a dev Mac
#   rust-tests.sh --no-encoder  for a machine where VideoToolbox can't encode (ci.yml asks
#                               first): everything except the tests that make a recording
#   rust-tests.sh --ci          GitHub's Mac runner (DECISIONS B116, ruled 2026-10-13): every
#                               crate but the Linux lane's (below), the recorder's test files
#                               one at a time (four encoders on three cores is what starves
#                               them) and without its two A/V timing bars, which run apart:
#   rust-tests.sh --timing-bars the recorder's A/V timing bars alone, one at a time — on CI a
#                               miss is a warning, and a Mac decides; on a Mac every bar is
#                               held, by the plain run above
#   rust-tests.sh --ci-linux    GitHub's Linux runner: the crates with no platform in them
#                               (LINUX_CRATES), whole — the state machine's regions, the spec,
#                               the cloud client, the editor's model. They test the same on
#                               either OS, and Linux has the faster runner; --ci-linux plus
#                               --ci is every crate. It refuses if one of them grows a
#                               platform branch: that crate goes back to the Mac.
#
# The media tests encode H.264 through VideoToolbox and read the result back with ffmpeg and
# ffprobe; they fail by design where that can't happen, so they are left out by name, never
# by accident. With --no-encoder:
#
#   - every crate but the four media crates runs whole;
#   - the media crates (below) run their unit tests, and each integration test file that
#     never names ffmpeg or ffprobe; the files that do are printed as NOT RUN.
#
# Never run anywhere, on any machine, by this script: what needs a Screen Recording or
# microphone grant, a second Mac or a VM (the heartbeat, tests/e2e, tests/l5, tests/l6 —
# they are node and shell lanes, not cargo tests), and the live Cloudflare tests on the
# test account, which skip themselves unless SHROOM_E2E=1 (e2e.yml sets it).
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
cd "$ROOT"
MEDIA_CRATES="shroom-encode shroom-record shroom-upload shroom-devtool"
# No `cfg(target_os …)`, no media: what CI tests on Linux (tools/ci-route.py reads this line).
LINUX_CRATES="shroom-core shroom-spec shroom-edit shroom-cloud shroom-cloudfake shroom-storage shroom-testpattern"

# The recorder's A/V timing bars: sound against picture, which a starved runner misses
# while every take still comes out whole and readable.
TIMING_BARS="beeps_land_on_their_seconds a_take_whose_picture_lags_its_sound_still_ends"

case "${1-}" in
  "") exec cargo test --workspace --locked --no-fail-fast ;;
  --ci)
    status=0
    excludes=
    for crate in $LINUX_CRATES; do excludes="$excludes --exclude $crate"; done
    # shellcheck disable=SC2086
    cargo test --workspace --locked --no-fail-fast --exclude shroom-record $excludes || status=$?
    skips=
    for t in $TIMING_BARS; do skips="$skips --skip $t"; done
    # shellcheck disable=SC2086
    cargo test -p shroom-record --locked --no-fail-fast -- --test-threads=1 $skips || status=$?
    exit "$status"
    ;;
  --ci-linux)
    packages=
    for crate in $LINUX_CRATES; do
      if grep -rlE 'target_os|target_vendor|cfg\((not\()?(unix|windows)' "crates/$crate" --include='*.rs'; then
        echo "rust-tests.sh: crates/$crate has a platform branch now (above) — take it out of LINUX_CRATES, so the Mac tests it" >&2
        exit 1
      fi
      packages="$packages -p $crate"
    done
    # shellcheck disable=SC2086
    exec cargo test --locked --no-fail-fast $packages
    ;;
  --timing-bars)
    # shellcheck disable=SC2086
    exec cargo test -p shroom-record --locked --no-fail-fast --test av_sync --test interruptions -- --test-threads=1 --exact $TIMING_BARS
    ;;
  --no-encoder) ;;
  *) echo "usage: rust-tests.sh [--ci | --ci-linux | --timing-bars | --no-encoder]" >&2; exit 64 ;;
esac

excludes=
for crate in $MEDIA_CRATES; do excludes="$excludes --exclude $crate"; done
# shellcheck disable=SC2086
cargo test --workspace --locked --no-fail-fast $excludes

left_out=
for crate in $MEDIA_CRATES; do
  targets=
  [ ! -f "crates/$crate/src/lib.rs" ] || targets="$targets --lib"
  if [ -f "crates/$crate/src/main.rs" ] || [ -d "crates/$crate/src/bin" ]; then targets="$targets --bins"; fi
  for file in "crates/$crate/tests/"*.rs; do
    [ -f "$file" ] || continue
    name=$(basename "$file" .rs)
    if grep -Eq 'ffmpeg|ffprobe' "$file"; then
      left_out="$left_out $crate/tests/$name.rs"
    else
      targets="$targets --test $name"
    fi
  done
  # shellcheck disable=SC2086
  cargo test -p "$crate" --locked --no-fail-fast $targets
done

echo
echo "NOT RUN here (they make a recording, and this machine can't encode):"
for file in $left_out; do echo "  crates/$file"; done
if [ -n "${GITHUB_STEP_SUMMARY:-}" ]; then
  {
    echo "### Media tests were not run"
    echo
    echo "VideoToolbox couldn't encode on this runner, so these test files did not run — run them on a Mac (\`cargo test --workspace\`):"
    echo
    for file in $left_out; do echo "- \`crates/$file\`"; done
  } >>"$GITHUB_STEP_SUMMARY"
fi
