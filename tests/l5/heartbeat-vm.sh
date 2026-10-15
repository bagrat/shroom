#!/bin/sh
# The heartbeat in a Tart clone on Bagrat's Mac — the last resort of
# dev/proposals/heartbeat-in-vm.md §3, for what the hosted Mac's E2E full run can't settle
# (a timing bar it reports but doesn't judge, a long take). The same harness as that run:
# the dev build installed by the real one-line installer (dev/e2e/install.sh), granted
# (dev/mac/mac.sh prepare), set up through its own wizard and driven through the heartbeat
# (dev/mac/mac.sh run e2e-full) — in a throwaway clone of `shroom-clean-tahoe-e2e`
# (tests/l5/build-e2e-image.sh), headless and silent. Nothing is installed or launched on
# this Mac.
#
# What differs from the hosted run, and why:
#   - the bars are judged (MAC_JUDGE_BARS=1) — two of them against the clone's own alarm:
#     skipped frames and Stop → the keyed page miss the product's bar in every clone
#     (DEVIATIONS #49: the virtual display, the guest's own network stack), so a miss there
#     is said, and only a gross one — past 2.5 % or 3 s (MAC_CLONE_ALARMS) — fails;
#   - no screen video (MAC_WATCH=0) and no show pass (E2E_SHOW=0): the clone's recorder has
#     no grant of its own and the clone has no camera; the verdict is read from the
#     published stream, as always;
#   - the run opens and closes HERE (tests/e2e-live's e2e-run, the account token from the
#     repo's .secrets/): only the run's own token — two hours, the product's two powers —
#     and its space id enter the clone, the token as a mode-600 file the app removes once
#     read. The account token never leaves this Mac.
#
# The clone is marked as a test machine (/etc/shroom-l5-clone, after its SIP is seen off),
# which dev/mac/lib.sh and dev/e2e/install.sh require. The run has its own space on the
# test account (DECISIONS B131): a hosted run going beside it is never touched, nor it.
#
# It runs the COMMITTED tree (git archive HEAD) with this checkout's build of it:
#   (cd app && npm run build:dev) && cargo build -p shroom-e2e-live --bin e2e-run
#
# Usage: tests/l5/heartbeat-vm.sh [seconds=10]
#   Evidence lands in build/l5-heartbeat/<time>/ (gitignored): results.tsv, summary.md,
#   e2e-full/ (heartbeat.json, the drives' reports, the page's views).
set -eu

ROOT=$(cd "$(dirname "$0")/../.." && pwd)
SECONDS_=${1:-10}
IMAGE=${SHROOM_HB_IMAGE:-shroom-clean-tahoe-e2e}
APP=target/debug/bundle/macos/shroom-dev.app
E2E_RUN=$ROOT/target/debug/e2e-run
for p in "$APP" target/debug/e2e-run node_modules/playwright .secrets/e2e-account-token .secrets/e2e-account-id; do
  [ -e "$ROOT/$p" ] || { echo "missing $p — see the header"; exit 2; }
done
command -v tart >/dev/null || { echo "tart isn't installed here"; exit 1; }
tart list --quiet 2>/dev/null | grep -qx "$IMAGE" || { echo "no $IMAGE image — tests/l5/build-e2e-image.sh builds it"; exit 1; }
[ -z "$(git -C "$ROOT" status --porcelain -- app crates tests dev worker installer tools)" ] ||
  echo "note: uncommitted changes aren't in the run — it ships HEAD"

VM=shroom-l5-hb-$$
WORK=$(mktemp -d)
EVIDENCE=$ROOT/build/l5-heartbeat/$(date +%Y%m%d-%H%M%S)
TREE=/tmp/shroom-hb
opened=
cleanup() {
  if [ -n "$opened" ]; then
    SHROOM_E2E=1 "$E2E_RUN" close "$WORK/space-id" || echo "the run's close failed — a later run reaps its space after 45 min"
  fi
  tart stop "$VM" >/dev/null 2>&1 || true
  tart delete "$VM" >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT INT TERM

tart clone "$IMAGE" "$VM"
tart run "$VM" --no-graphics --no-audio >/dev/null 2>&1 &
i=0
until tart exec "$VM" true >/dev/null 2>&1; do
  i=$((i + 1)); [ "$i" -gt 90 ] && { echo "the VM never came up"; exit 1; }
  sleep 2
done
vm() { tart exec "$VM" sh -c "export PATH=/opt/homebrew/bin:/usr/local/bin:\$PATH; $1" </dev/null; }

# Logged in, the desktop up, then a while to settle (as take-vm.sh).
i=0
until vm 'pgrep -x Dock >/dev/null && lsappinfo info -only bundleid "$(lsappinfo front)" | grep -q "=\"com\.apple\.finder\""' 2>/dev/null; do
  i=$((i + 1)); [ "$i" -gt 60 ] && { echo "the VM's desktop never came up"; exit 1; }
  sleep 2
done
sleep 30
[ "$(vm 'csrutil status')" = "System Integrity Protection status: disabled." ] || { echo "the clone's SIP is on — not a golden image"; exit 1; }
vm 'sudo touch /etc/shroom-l5-clone'
vm 'ls ~/Library/Caches/ms-playwright/chromium_headless_shell-*/chrome-headless-shell-mac-arm64/chrome-headless-shell ~/Library/Caches/ms-playwright/webkit-*/pw_run.sh' >/dev/null 2>&1 ||
  { echo "$IMAGE has no Playwright browsers — tests/l5/build-e2e-image.sh --replace"; exit 1; }
echo "macOS $(vm 'sw_vers -productVersion') in the clone"

# The committed tree, this checkout's build of it, its node packages.
vm "mkdir -p $TREE"
git -C "$ROOT" archive HEAD | tart exec -i "$VM" tar -C "$TREE" -xf -
tar -C "$ROOT" -cf - "$APP" node_modules | tart exec -i "$VM" tar -C "$TREE" -xf -
# The account's id where the app's --e2e looks for it (not a secret, kept private anyway).
tart exec -i "$VM" sh -c 'umask 077; mkdir -p "$HOME/.shroom-dev" && cat >"$HOME/.shroom-dev/e2e-account-id"' <"$ROOT/.secrets/e2e-account-id"

# The run opens here: pinned, its own space id and token; the id goes where the app reads it.
opened=1
SHROOM_E2E=1 "$E2E_RUN" open "$WORK/run-token" "$WORK/space-id"
tart exec -i "$VM" sh -c 'umask 077; cat >"$HOME/.shroom-dev/e2e-space-id"' <"$WORK/space-id"
tart exec -i "$VM" sh -c "umask 077; cat >$TREE/run-token" <"$WORK/run-token"
rm -f "$WORK/run-token"

# In the clone: the hosted run's own steps.
in_tree() { vm "cd $TREE && export RUNNER_TEMP=$TREE/runner && mkdir -p \$RUNNER_TEMP && $1"; }
in_tree "bash dev/e2e/install.sh $APP $TREE/releases" || echo "the install step failed — the run goes on to say where"
in_tree 'bash dev/mac/mac.sh prepare "$HOME/Applications/shroom.app"' || true
# Two bars no VM holds (DEVIATIONS #49): said against the product's bar, judged against the
# clone's own alarm (a gross regression); the other five judged as they are.
ALARMS=skippedFramesPct=2.5,stopToKeyedPageMs=3000
in_tree "MAC_WATCH=0 MAC_JUDGE_BARS=1 MAC_CLONE_ALARMS=$ALARMS E2E_SHOW=0 E2E_TOKEN_FILE=$TREE/run-token MAC_LIMIT=1800 bash dev/mac/mac.sh run e2e-full" || true
in_tree 'bash dev/mac/mac.sh summary' || true

# The evidence comes out; the run's token never does (the app removed it).
mkdir -p "$EVIDENCE"
tart exec "$VM" tar -C "$TREE/runner/mac" -cf - . </dev/null 2>/dev/null | tar -C "$EVIDENCE" -xf - 2>/dev/null || true
echo "evidence: $EVIDENCE"
if [ -s "$EVIDENCE/e2e-full/heartbeat.json" ]; then
  node -e '
const h = JSON.parse(require("fs").readFileSync(process.argv[1], "utf8"));
const alarms = { skippedFramesPct: 2.5, stopToKeyedPageMs: 3000 }; // DEVIATIONS #49, as ALARMS above
for (const [k, b] of Object.entries(h.bars ?? {})) {
  const alarm = alarms[k];
  const mark = b.ok ? "ok  " : alarm !== undefined && b.value <= alarm ? "said" : "MISS";
  console.log(`  ${mark} ${k} ${b.value} (bar ${b.bar}${alarm !== undefined ? `, clone alarm ${alarm}` : ""})`);
}
console.log(`  path ${h.pathOk ? "held" : "broke"}${h.error ? ` — ${h.error}` : ""}; load ${h.load}`);
' "$EVIDENCE/e2e-full/heartbeat.json"
fi
[ -s "$EVIDENCE/results.tsv" ] || { echo "FAIL nothing came out of the clone"; exit 1; }
if grep -q '^fail' "$EVIDENCE/results.tsv"; then
  grep '^fail' "$EVIDENCE/results.tsv" | cut -f2,3 | sed 's/^/FAIL /'
  exit 1
fi
echo "PASS the heartbeat in a clone: installed, set up, recorded, played, a member, delete — every judged bar held"
