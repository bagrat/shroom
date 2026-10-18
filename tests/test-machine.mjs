// Where a dev build may be launched (dev/proposals/heartbeat-in-vm.md §3): a test machine
// only — the GitHub-hosted Mac or a Tart clone of a golden image, both with System
// Integrity Protection off (their privacy grants are written straight into TCC.db). Bagrat's
// Mac keeps SIP on and carries only signed releases (the official one, or an early-channel
// pre-release, DECISIONS B220): nothing dev is installed or launched on it, so every script that would is refused there before it starts.

import { spawnSync } from "node:child_process";

export function onTestMachine() {
  if (process.platform !== "darwin") return false;
  if (process.env.GITHUB_ACTIONS === "true" && process.env.RUNNER_ENVIRONMENT === "github-hosted") return true;
  const r = spawnSync("/usr/bin/csrutil", ["status"], { encoding: "utf8" });
  return /status: disabled\./.test(r.stdout ?? "");
}

export function refuseOffTestMachine(what) {
  if (onTestMachine()) return;
  console.error(
    `${what} launches a dev build, and this isn't a test machine. Run it on the hosted Mac ` +
      "(dev/mac/loop.sh, or the e2e-full workflow) or, when that can't settle it, in a Tart " +
      "clone (tests/l5/) — never on Bagrat's Mac (dev/proposals/heartbeat-in-vm.md §3).",
  );
  process.exit(64);
}
