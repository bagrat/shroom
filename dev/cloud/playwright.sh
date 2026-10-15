#!/bin/bash
# Playwright's browsers for cloud sessions, installed off the critical path. The galleries
# and the page drive open headless WebKit and Chromium; the browsers plus the system
# libraries they need are ~2 min cold, so by default the SessionStart hook kicks the
# install off in the background and the session starts immediately; anything that renders
# waits for it only if it's still running. The version is the workspace's own
# (node_modules/playwright), so run dev/cloud/setup.sh (npm ci) first.
#
#   playwright.sh install   blocking, idempotent (what setup.sh uses in "setup" mode)
#   playwright.sh start     background install; returns at once (what the hook uses)
#   playwright.sh status    ready | installing | failed | missing   (exit 0 iff ready)
#   playwright.sh wait [s]  block until ready (default 600s); exit 1 on failure/timeout
set -uo pipefail
if [ "$(uname -s)" != "Linux" ]; then
  echo "not Linux — this is for cloud sessions and changes nothing on a Mac"
  exit 0
fi
HERE="$(cd "$(dirname "$0")" && pwd)"
ROOT="$(cd "$HERE/../.." && pwd)"
. "$HERE/path.sh"
STATE="$HOME/.shroom-dev/cloud"
LOG="$STATE/playwright-install.log"
PIDF="$STATE/playwright-install.pid"
OK="$STATE/playwright-ready"   # holds the version whose browsers started cleanly
mkdir -p "$STATE"

version() { node -p 'require(process.argv[1]).version' "$ROOT/node_modules/playwright/package.json" 2>/dev/null; }
ready() { local v; v=$(version) && [ -n "$v" ] && [ -f "$OK" ] && [ "$(cat "$OK")" = "$v" ]; }
running() { [ -f "$PIDF" ] && kill -0 "$(cat "$PIDF")" 2>/dev/null; }

# Both engines start headless and render a page: the marker is written on proof, not hope.
smoke() {
  (cd "$ROOT" && node --input-type=module -e '
    import { chromium, webkit } from "playwright";
    for (const engine of [webkit, chromium]) {
      const browser = await engine.launch();
      const page = await browser.newPage();
      await page.setContent("<p>ok</p>");
      if ((await page.textContent("p")) !== "ok") throw new Error(engine.name() + " rendered nothing");
      await browser.close();
    }')
}

install() {
  ready && return 0
  local v; v=$(version)
  [ -n "$v" ] || { echo "playwright isn't in node_modules — run dev/cloud/setup.sh (npm ci) first"; return 1; }
  echo "installing Playwright $v: webkit, chromium and their system libraries"
  (cd "$ROOT" && npx --no-install playwright install --with-deps webkit chromium) \
    || { echo "playwright install failed"; return 1; }
  smoke || { echo "a browser was installed but didn't start"; return 1; }
  echo "$v" > "$OK" && echo "ready"
}

case "${1:-status}" in
  install) install ;;
  start)
    ready && exit 0
    running && exit 0
    # setsid + nohup: must outlive the hook that starts it.
    setsid nohup bash "$0" install >"$LOG" 2>&1 < /dev/null &
    echo $! > "$PIDF"
    ;;
  status)
    if ready; then echo ready
    elif running; then echo installing
    elif [ -s "$LOG" ]; then echo failed; tail -3 "$LOG"; exit 1
    else echo missing; exit 1
    fi
    ;;
  wait)
    deadline=$(( $(date +%s) + ${2:-600} ))
    until ready; do
      running || { bash "$0" start; sleep 1; running || ready || { echo "playwright install failed:"; tail -3 "$LOG"; exit 1; }; }
      [ "$(date +%s)" -ge "$deadline" ] && { echo "timed out waiting for playwright"; exit 1; }
      sleep 3
    done
    echo ready
    ;;
  *) echo "usage: $0 install|start|status|wait [seconds]" >&2; exit 2 ;;
esac
