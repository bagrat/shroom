// The launch drive: the real app (a debug bundle) started each way something starts it,
// under a throwaway home — and what each launch opens by itself (N9, N41). Nothing is
// clicked. Never ~/.shroom or ~/.shroom-dev; no cloud, no browser, no login item.
//
//   (cd app && npm run build:dev — the harness is built in only on purpose)
//   node app/test/launch-drive.mjs
//
// - the login item: macOS starts shroom with no arguments and marks the launch event
//   (keyAELaunchedAsLogInItem). The drive launches the bundle through NSWorkspace with
//   exactly that launch event — the closest this Mac gets to a login without logging out
//   (the real login path is Bagrat's checkpoint, DECISIONS B45) — silent, tray-only.
// - the install script's update lane (`--relaunched=install-update`): silent, tray-only.
// - the install script's fresh install (`--relaunched=install`): the welcome, with the
//   first launch's face, taking the keyboard.
// - a launch the person did (no flag, no mark): a tray click — the welcome on a fresh Mac.
// - the same on a set-up Mac (`provisioned`, against the L1 fake's `found` space): config
//   opens in the boot's first Scene, while macOS is still laying the just-made mushroom
//   out — and the popover must hang under it (centred, on its screen), shown once, never
//   left in the fallback corner (N41, DECISIONS B46). No Dock icon comes with it: once set
//   up, shroom lives in the menu bar (law 6, DEVIATIONS #38).
// - the same once more behind a slow start: shroom's main thread kept busy for 1.2 s after
//   the mushroom is made, so the first Scene meets a mushroom macOS is only then bringing in.
// - the login item and the install script's update lane again, on a set-up Mac — the update
//   opened exactly as installer/install.sh opens it (`open <app> --args …`: no -g, no -n).
//
// And through every one of them, what macOS itself says of shroom while it starts — never
// only what shroom says of itself once it is up. A launch that shows nothing (the login
// item, an update) and a launch on a set-up Mac must not be in the Dock, or the app in
// front, or own the menu bar, for any moment of it; a first run must end up all three,
// the welcome with the keyboard; a launch you did on a set-up Mac gives its popover the
// keyboard while your app stays in front. Two witnesses, each started before the launch:
//   - a sampler (launch-watch.js): the app's Dock presence (its activation policy as macOS
//     has it), whether it is the app in front, whether the menu bar is its own — a look
//     every 4–6 ms. A handful of looks at the very moments macOS announces the launch come
//     later than that (40–160 ms measured: AppKit is taking the news in);
//   - LaunchServices' own notifications (`lsappinfo listen`), which have no gaps: the kind
//     of app the launch was announced as — the Dock shows a `Foreground` one from that
//     moment, before the app has run a line — every change of kind, and every coming to
//     the front. Only lines that speak of this copy of shroom are kept.
// And the app the person is in keeps the front through a launch that must stay quiet: any
// app coming to the front while shroom starts fails it too (each launch also says which app
// was in front before it and after).
// Either one seeing a moment in the Dock or in front fails the launch. Where one of them
// couldn't look (no notifications read, looks more than 15 ms apart, a locked screen for
// the keyboard), the launch says so, and SHROOM_HANDS=1 (tests/l5/dock-vm.sh) fails on it.
//
// A locked screen hides the mushroom too, so "under the mushroom" is one more thing a
// launch can't be looked at for there. The lock screen is over the menu bar, and macOS
// counts no menu-bar item as seen behind it, any app's (measured 2026-10-13 with a bare
// status item of another process's: on this Mac behind its lock; and in a macOS 26.6.2
// clone, seen before its screen was locked and not seen after — where this same launch was
// under the mushroom unlocked, and in the corner locked). shroom then does what N41 says
// for a mushroom nobody can see: the popover takes the corner. Such a launch lists "under
// the mushroom" as not looked at — SHROOM_HANDS=1 fails on that as on the keyboard — and
// is still held to the rest: its popover shows, in the corner, where it was put; it goes
// when dismissed; a click on the mushroom brings it back at once, to the same place.
// SHROOM_APP overrides the bundle path.

import { execFileSync, spawn } from "node:child_process";
import { createInterface } from "node:readline";
import { existsSync, mkdtempSync, readFileSync, realpathSync, rmSync, statSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { refuseOffTestMachine } from "../../tests/test-machine.mjs";

refuseOffTestMachine("launch-drive.mjs");

const here = dirname(fileURLToPath(import.meta.url));
const repo = resolve(here, "../..");
const app = process.env.SHROOM_APP ?? join(repo, "target/debug/bundle/macos/shroom-dev.app");
if (!existsSync(app)) { console.error(`missing ${app} — build it first (see the header)`); process.exit(2); }
const bundleId = execFileSync("plutil", ["-extract", "CFBundleIdentifier", "raw", "-o", "-", join(app, "Contents/Info.plist")], { encoding: "utf8" }).trim();
const sleep = (ms) => new Promise((ok) => setTimeout(ok, ms));
const exited = (child) => new Promise((ok) => child.once("exit", ok));

// NSWorkspace's launch with a chosen launch event: `oapp` carrying keyAEPropData =
// keyAELaunchedAsLogInItem, as a login launch's does.
const JXA = `
ObjC.import("AppKit");
function run(argv) {
  const app = argv[0];
  const args = argv.slice(1);
  const ev = $.NSAppleEventDescriptor.appleEventWithEventClassEventIDTargetDescriptorReturnIDTransactionID(0x61657674, 0x6f617070, $.NSAppleEventDescriptor.nullDescriptor, -1, 0);
  ev.setParamDescriptorForKeyword($.NSAppleEventDescriptor.descriptorWithEnumCode(0x6c676974), 0x70726474);
  const config = $.NSMutableDictionary.dictionary;
  config.setObjectForKey($(args), $.NSWorkspaceLaunchConfigurationArguments);
  config.setObjectForKey(ev, $.NSWorkspaceLaunchConfigurationAppleEvent);
  const err = Ref();
  const running = $.NSWorkspace.sharedWorkspace.launchApplicationAtURLOptionsConfigurationError($.NSURL.fileURLWithPath(app), $.NSWorkspaceLaunchNewInstance | $.NSWorkspaceLaunchWithoutActivation, config, err);
  return running.isNil() ? "failed" : String(running.processIdentifier);
}`;

// `quiet`: a launch that must never be in the Dock or in front. `shows`: a first run — the
// welcome, its Dock icon, in front with the keyboard.
const CASES = {
  login: { via: "login-event", flags: [], want: { kind: "LoginItem", popover: null, onboarding: null, tray: "idle-unprovisioned", activates: false, dock: "accessory" }, quiet: true },
  "install-update": { via: "open", flags: ["--relaunched=install-update"], want: { kind: "InstallUpdate", popover: null, onboarding: null, tray: "idle-unprovisioned", activates: false, dock: "accessory" }, quiet: true },
  install: { via: "open", flags: ["--relaunched=install"], want: { kind: "Install { join: false }", onboarding: "welcome", activates: true, dock: "regular" }, faces: ["first-launch"], shows: true },
  user: { via: "open", flags: [], want: { kind: "User", onboarding: "welcome", activates: true, dock: "regular" }, noFaces: ["first-launch"], shows: true },
  "user-provisioned": { via: "open", flags: [], provisioned: true, want: { kind: "User", popover: "config", onboarding: null, activates: true, dock: "accessory" }, anchored: true, quiet: true, keyboard: true },
  // The same, with a start-up that keeps shroom's main thread for 1.2 s after the mushroom is
  // made (the harness's `late`; some starts take that long by themselves): the first Scene
  // is waiting when the event loop first turns, and macOS is only then bringing the mushroom
  // into the menu bar. The popover must still hang from where the mushroom comes to rest.
  "user-provisioned-late": { via: "open", flags: [], provisioned: true, late: true, want: { kind: "User", popover: "config", onboarding: null, activates: true, dock: "accessory" }, anchored: true, quiet: true, keyboard: true },
  "login-set-up": { via: "login-event", flags: [], provisioned: true, want: { kind: "LoginItem", popover: null, onboarding: null, activates: false, dock: "accessory" }, quiet: true },
  "update-set-up": { via: "installer", flags: ["--relaunched=install-update"], provisioned: true, want: { kind: "InstallUpdate", popover: null, onboarding: null, activates: false, dock: "accessory" }, quiet: true },
};

// The L1 fake (a loopback stand-in for Cloudflare's API and R2), for the set-up Mac.
async function startFake() {
  const fake = spawn(join(repo, "target/debug/cloudfake"), ["--state", "found"], { stdio: ["pipe", "pipe", "inherit"] });
  const ep = await new Promise((ok, fail) => {
    createInterface({ input: fake.stdout }).once("line", (l) => ok(JSON.parse(l)));
    fake.once("exit", () => fail(new Error("the fake exited")));
  });
  return { fake, ep };
}

// The popover hangs from the mushroom: its panel shows, where the presenter put it, which
// is under the mushroom as macOS draws it now (within a point).
function anchorProblems(r) {
  const p = [];
  const near = (a, b) => Math.abs(a - b) <= 1;
  if (!r.panel || !r.panel[2]) p.push(`panel ${JSON.stringify(r.panel)} (want shown)`);
  if (!r.placed || !r.placed.under) p.push(`placed ${JSON.stringify(r.placed)} (want under the mushroom)`);
  if (!r.mushroom || !r.mushroom[1]) p.push(`mushroom ${JSON.stringify(r.mushroom)} (want seen)`);
  if (r.panel && r.expected && !(near(r.panel[0], r.expected[0]) && near(r.panel[1], r.expected[1]))) {
    p.push(`panel corner ${JSON.stringify(r.panel.slice(0, 2))} (want ${JSON.stringify(r.expected.slice(0, 2))}, under the mushroom)`);
  }
  if (r.panel && r.placed && !(near(r.panel[0], r.placed.x) && near(r.panel[1], r.placed.y))) {
    p.push(`panel corner ${JSON.stringify(r.panel.slice(0, 2))} moved from where it was placed ${JSON.stringify([r.placed.x, r.placed.y])}`);
  }
  if (!r.expected) p.push("no mushroom to measure against");
  // Dismissed, it goes; a click on the mushroom brings it back, under the mushroom again.
  if (!r.dismissed || r.dismissed[2]) p.push(`dismissed panel ${JSON.stringify(r.dismissed)} (want hidden)`);
  if (!r.reopened || !r.reopened[2]) p.push(`reopened panel ${JSON.stringify(r.reopened)} (want shown)`);
  if (!r.reopened_placed || !r.reopened_placed.under || r.reopened_placed.held_ms !== 0) p.push(`reopened placed ${JSON.stringify(r.reopened_placed)} (want under, at once)`);
  if (r.reopened && r.expected && !(near(r.reopened[0], r.expected[0]) && near(r.reopened[1], r.expected[1]))) {
    p.push(`reopened corner ${JSON.stringify(r.reopened.slice(0, 2))} (want ${JSON.stringify(r.expected.slice(0, 2))})`);
  }
  return p;
}

// The same popover behind a locked screen, where nobody can see the mushroom (the header
// has the measurement): it shows, in the corner (N41), where the presenter put it; it goes
// when dismissed; and a click on the mushroom brings it back at once, to the same place.
function cornerProblems(r) {
  const p = [];
  const near = (a, b) => Math.abs(a - b) <= 1;
  if (!r.panel || !r.panel[2]) p.push(`panel ${JSON.stringify(r.panel)} (want shown)`);
  if (!r.placed || r.placed.under) p.push(`placed ${JSON.stringify(r.placed)} (want the corner: the mushroom is behind the lock screen)`);
  if (r.panel && r.placed && !(near(r.panel[0], r.placed.x) && near(r.panel[1], r.placed.y))) {
    p.push(`panel corner ${JSON.stringify(r.panel.slice(0, 2))} moved from where it was placed ${JSON.stringify([r.placed.x, r.placed.y])}`);
  }
  if (!r.dismissed || r.dismissed[2]) p.push(`dismissed panel ${JSON.stringify(r.dismissed)} (want hidden)`);
  if (!r.reopened || !r.reopened[2]) p.push(`reopened panel ${JSON.stringify(r.reopened)} (want shown)`);
  if (!r.reopened_placed || r.reopened_placed.under || r.reopened_placed.held_ms !== 0) p.push(`reopened placed ${JSON.stringify(r.reopened_placed)} (want the corner, at once)`);
  if (r.reopened && r.panel && !(near(r.reopened[0], r.panel[0]) && near(r.reopened[1], r.panel[1]))) {
    p.push(`reopened corner ${JSON.stringify(r.reopened.slice(0, 2))} (want ${JSON.stringify(r.panel.slice(0, 2))}, where it was)`);
  }
  return p;
}

async function waitFor(file, ms) {
  const until = Date.now() + ms;
  while (Date.now() < until) {
    if (existsSync(file)) {
      try { return JSON.parse(readFileSync(file, "utf8")); } catch { /* still being written */ }
    }
    await sleep(200);
  }
  throw new Error(`no report in ${ms / 1000} s`);
}

// LaunchServices' notifications, read as they come: only those that speak of this copy of
// shroom are kept — from the one that announces its launch to its end. The announcement
// names the bundle by its folder on disk (its path is spelled one way before the app has
// checked in and another after: /tmp, /private/tmp). `t` is milliseconds since then.
function launchLog() {
  const paths = [...new Set([app, realpathSync(app)])];
  const folder = `"LSBundlePathINode"=${statSync(app).ino},`;
  let clock = 0;
  let asn = null;
  let t0 = 0;
  const events = [];
  // The name LaunchServices gives each app that comes to the front: said of the one that
  // takes the front from shroom, and of no other.
  const fronts = new Map();
  // Every coming to the front, of any app, from the launch's announcement to shroom's end.
  const moved = [];
  let over = false;
  return {
    take(line) {
      const m = /^Notification: (\S+) time=(\S+)/.exec(line);
      if (!m) return;
      // The first carries the date; each one after it, the seconds since the one before.
      if (m[2].startsWith("+")) clock += (Number(m[2].slice(1).replace(/s$/, "")) || 0) * 1000;
      const cut = line.lastIndexOf("} affectedASN=");
      const data = cut < 0 ? line : line.slice(0, cut);
      const affected = cut < 0 ? "" : line.slice(cut);
      const about = /"LSASN"=(ASN:[0-9a-fx-]+:)/.exec(data)?.[1] ?? null;
      if (!asn && about && (data.includes(folder) || paths.some((p) => data.includes(`"LSBundlePath"="${p}"`)))) { asn = about; t0 = clock; }
      const name = m[1].replace(/^kLSNotif(y|ication)/, "");
      if (name === "BecameFrontmost" && about) {
        fronts.set(about, /affectedASN="([^"]*)"/.exec(affected)?.[1] ?? null);
        if (asn && !over) moved.push(`${Math.round(clock - t0)} ms: ${about === asn ? "shroom" : (fronts.get(about) ?? "another app")}`);
      }
      if (asn && about === asn && name === "ApplicationDeath") over = true;
      if (!asn || (about !== asn && !affected.includes(asn))) return;
      events.push({ t: Math.round(clock - t0), name, ours: about === asn, kind: about === asn ? (/"ApplicationType"="(\w+)"/.exec(data)?.[1] ?? null) : null, by: about === asn ? null : (fronts.get(about) ?? null) });
    },
    // The time in the Dock (the kind `Foreground`), in front, and owning the menu bar.
    done() {
      const sum = { dock_ms: 0, front_ms: 0, menu_ms: 0 };
      const since = { dock_ms: null, front_ms: null, menu_ms: null };
      const turn = (k, on, t) => {
        if (on && since[k] === null) since[k] = t;
        if (!on && since[k] !== null) { sum[k] += Math.max(1, t - since[k]); since[k] = null; }
      };
      const said = [];
      const older = [];
      let kind = null;
      // As LaunchServices stamped them: a line can arrive after one stamped later.
      events.sort((a, b) => a.t - b.t);
      for (const e of events) {
        // `Foreground` is believed from any line that says it. Out of the Dock again, only
        // on LaunchServices' own word that the kind changed: every other line carries a
        // copy of the app's entry, and the copy can be older than the change before it (a
        // first run on a macOS 15 clone: `Foreground` at 431 ms, a line 30 ms later still
        // saying `UIElement`, `Foreground` again at the app's end and no change between;
        // the sampler saw the icon up throughout). Those are kept apart, not taken — so
        // the time in the Dock is never read shorter than every line taken at its word.
        const copy = kind === "Foreground" && e.name !== "ApplicationTypeChanged";
        if (e.kind && e.kind !== kind && copy) older.push(`${e.t} ms: ${e.name} said ${e.kind}`);
        if (e.kind && e.kind !== kind && !copy) { kind = e.kind; said.push(`${e.t} ms: ${kind}`); turn("dock_ms", kind === "Foreground", e.t); }
        if (e.name === "BecameFrontmost" && e.ours) { said.push(`${e.t} ms: in front`); turn("front_ms", true, e.t); }
        if (e.name === "LostFrontmost" && !e.ours) { said.push(`${e.t} ms: behind${e.by ? ` ${e.by}` : ""}`); turn("front_ms", false, e.t); }
        if (e.name === "MenuBarAcquired" && e.ours) turn("menu_ms", true, e.t);
        if (e.name === "MenuBarLost" && !e.ours) turn("menu_ms", false, e.t);
        if (e.name === "ApplicationDeath") { said.push(`${e.t} ms: ended`); for (const k of Object.keys(sum)) turn(k, false, e.t); }
      }
      const end = events.length ? events[events.length - 1].t : 0;
      for (const k of Object.keys(sum)) turn(k, false, end);
      return { read: asn !== null, born: events.find((e) => e.kind)?.kind ?? null, ...sum, said, older_copies: older, front_moved: moved };
    },
  };
}

// The app in front, as macOS names it (its bundle id) — read from outside, before and after.
function inFront() {
  try {
    const asn = execFileSync("lsappinfo", ["front"], { encoding: "utf8" }).trim();
    return /="([^"]*)"/.exec(execFileSync("lsappinfo", ["info", "-only", "bundleid", asn], { encoding: "utf8" }))?.[1] ?? null;
  } catch { return null; }
}

// The two witnesses, started and listening before the launch (see the header).
async function witness() {
  const sampler = spawn("osascript", ["-l", "JavaScript", join(here, "launch-watch.js"), bundleId, app], { stdio: ["pipe", "pipe", "inherit"] });
  let said = "";
  sampler.stdout.on("data", (d) => { said += d; });
  sampler.stdin.on("error", () => {});
  for (let i = 0; i < 500 && !said.includes("ready") && sampler.exitCode === null; i++) await sleep(20);
  const tick = setInterval(() => sampler.stdin.write("\n"), 4);
  const listen = spawn("lsappinfo", ["listen", "+all", "forever"], { stdio: ["ignore", "pipe", "ignore"] });
  const log = launchLog();
  createInterface({ input: listen.stdout }).on("line", log.take);
  await sleep(1000);
  return {
    async end() {
      clearInterval(tick);
      sampler.stdin.end();
      const gone = exited(sampler);
      await Promise.race([gone, sleep(10_000)]);
      if (sampler.exitCode === null) sampler.kill();
      listen.kill();
      await sleep(200);
      let looked = null;
      try { looked = JSON.parse(said.split("\n").find((l) => l.startsWith("{"))); } catch { /* the sampler said nothing */ }
      return { looked, launchservices: log.done() };
    },
  };
}

// What the sampler saw of the launched process: the time in the Dock, the time in front or
// with the menu bar, and the time it was all three at once (a first run's resting state).
function sampled(looked, pid) {
  const sum = { dock_ms: 0, front_ms: 0, shown_ms: 0 };
  const rows = looked?.changes ?? [];
  rows.forEach((row, i) => {
    const a = row.apps.find((x) => x.pid === pid);
    if (!a) return;
    const ms = Math.max(1, (rows[i + 1]?.t ?? looked.ran_ms) - row.t);
    if (a.policy === "regular") sum.dock_ms += ms;
    if (a.front || a.active || a.menu) sum.front_ms += ms;
    if (a.policy === "regular" && a.front && a.active && a.menu) sum.shown_ms += ms;
  });
  return sum;
}

// What the two witnesses hold against a launch, and what they couldn't look at.
function judge(c, r, { looked, launchservices: ls }) {
  const problems = [];
  const unseen = [];
  const sm = sampled(looked, r.pid);
  const close = looked && looked.looks >= 100 && looked.every_ms <= 15;
  if (!close) unseen.push(`the Dock and the front, sampled (${looked ? `${looked.looks} looks, one every ${looked.every_ms} ms — 15 at most` : "the sampler said nothing"})`);
  if (!ls.read) unseen.push("LaunchServices' own account of the launch (its notifications named no such launch)");
  if (c.quiet) {
    if (ls.read && (ls.born === "Foreground" || ls.dock_ms > 0)) problems.push(`in the Dock for ${ls.dock_ms} ms, as LaunchServices has it (${ls.said.join(" → ")})`);
    if (ls.read && (ls.front_ms > 0 || ls.menu_ms > 0)) problems.push(`in front for ${ls.front_ms} ms (the menu bar its own for ${ls.menu_ms} ms), as LaunchServices has it (${ls.said.join(" → ")})`);
    if (sm.dock_ms > 0) problems.push(`in the Dock for ${sm.dock_ms} ms, as the sampler saw it`);
    if (sm.front_ms > 0) problems.push(`in front for ${sm.front_ms} ms, as the sampler saw it`);
    // And the app the person is in keeps the front: nothing comes to it while shroom starts.
    if (ls.read && ls.front_moved.length) problems.push(`the app in front changed while shroom started, as LaunchServices has it (${ls.front_moved.join(" → ")})`);
  }
  if (c.shows) {
    // A first run rests in the Dock, with the welcome in front and the keyboard in it.
    if (sm.dock_ms < 1000 && close) problems.push(`the welcome's Dock icon was up for ${sm.dock_ms} ms of the launch, as the sampler saw it (want it up and staying)`);
    if (ls.read && ls.dock_ms < 1000) problems.push(`the welcome's Dock icon was up for ${ls.dock_ms} ms of the launch, as LaunchServices has it (${ls.said.join(" → ")})`);
    if (r.no_keyboard) unseen.push(`the welcome in front with the keyboard — ${r.no_keyboard}`);
    else {
      const w = r.window ?? {};
      if (!(w.frontmost && w.active && w.key && w.shown && w.on_top)) problems.push(`the welcome isn't in front with the keyboard: ${JSON.stringify(r.window)}`);
      if (sm.shown_ms < 1000 && close) problems.push(`shroom was in front with the menu bar its own for ${sm.shown_ms} ms of the launch, as the sampler saw it (want it there and staying)`);
      if (ls.read && (ls.front_ms < 1000 || ls.menu_ms < 1000)) problems.push(`in front for ${ls.front_ms} ms, the menu bar its own for ${ls.menu_ms} ms, as LaunchServices has it (${ls.said.join(" → ")})`);
    }
  }
  if (c.keyboard) {
    // The popover a launch you did opens has the keyboard — and your app stays in front.
    if (r.no_keyboard) unseen.push(`the launch's popover with the keyboard — ${r.no_keyboard}`);
    else if (!(r.popover_key === true && r.keys_ours !== false)) problems.push(`the launch's popover hasn't the keyboard: its window has it=${r.popover_key}, keys go to shroom=${r.keys_ours}`);
  }
  const seen = { looks: looked?.looks ?? 0, every_ms: looked?.every_ms ?? null, longest_ms: looked?.longest_ms ?? null, sampler: sm, launchservices: ls };
  return { problems, unseen, seen };
}

async function drive(name) {
  const c = CASES[name];
  const work = mkdtempSync(join(tmpdir(), "shroom-launch-"));
  const out = join(work, "report.json");
  const f = c.provisioned ? await startFake() : null;
  const args = ["--home", join(work, "home"), ...(f ? ["--cloud-fake", f.ep.api, f.ep.r2] : []), "--drive", "launch", out, ...(c.provisioned ? ["provisioned"] : []), ...(c.late ? ["late"] : []), ...c.flags];
  const before = inFront();
  const w = await witness();
  let r = null;
  let saw = null;
  try {
    // Never a call that waits here: the witnesses need this process's turns while shroom starts.
    if (c.via === "open") {
      spawn("open", ["-W", "-n", app, "--args", ...args], { stdio: "ignore" });
    } else if (c.via === "installer") {
      // installer/install.sh's own line: `open "$APP" --args --relaunched=install-update`.
      const o = spawn("open", [app, "--args", ...args], { stdio: "ignore" });
      if ((await exited(o)) !== 0) throw new Error("open, as the install script runs it, failed");
    } else {
      const script = join(work, "launch.js");
      writeFileSync(script, JXA);
      const o = spawn("osascript", ["-l", "JavaScript", script, app, ...args], { stdio: ["ignore", "pipe", "pipe"] });
      let said = "";
      o.stdout.on("data", (d) => { said += d; });
      o.stderr.on("data", (d) => { said += d; });
      if ((await exited(o)) !== 0 || said.trim() === "failed") throw new Error(`the login-event launch failed: ${said.trim()}`);
    }
    r = await waitFor(out, 20_000);
    // The process ends right after its report: the witnesses watch it go.
    for (let i = 0; i < 80; i++) {
      try { process.kill(r.pid, 0); } catch { break; }
      await sleep(100);
    }
    await sleep(500);
    saw = await w.end();
  } finally {
    // Whatever happened, nothing this launch started is left listening.
    if (!saw) await w.end();
    if (f) f.fake.stdin.end();
  }
  const { problems, unseen, seen } = judge(c, r, saw);
  seen.in_front = { before, after: inFront() };
  if (c.anchored) {
    // Behind a lock nobody can see the mushroom, and the popover takes the corner.
    const hidden = r.behind_the_lock && !(r.mushroom && r.mushroom[1]);
    if (hidden) unseen.push(`the launch's popover under the mushroom — ${r.behind_the_lock}`);
    problems.push(...(hidden ? cornerProblems(r) : anchorProblems(r)));
  }
  for (const [k, v] of Object.entries(c.want)) {
    const got = k === "dock" ? (r.dock ?? "").split(/[ (]/)[0].toLowerCase() : r[k];
    if (got !== v) problems.push(`${k}: ${JSON.stringify(r[k])} (want ${JSON.stringify(v)})`);
  }
  for (const f of c.faces ?? []) if (!r.faces.includes(f)) problems.push(`faces ${JSON.stringify(r.faces)} (want ${f})`);
  for (const f of c.noFaces ?? []) if (r.faces.includes(f)) problems.push(`faces ${JSON.stringify(r.faces)} (not ${f})`);
  if (process.env.SHROOM_HANDS === "1") problems.push(...unseen.map((u) => `not looked at: ${u}`));
  rmSync(work, { recursive: true, force: true });
  return { launch: name, ok: problems.length === 0, report: r, seen, ...(unseen.length ? { unseen } : {}), problems };
}

let asked = process.argv.slice(2);
if (asked.length === 0) asked = Object.keys(CASES);
const results = [];
for (const name of asked) results.push(await drive(name));
console.log(JSON.stringify(results, null, 2));
process.exit(results.every((r) => r.ok) ? 0 : 1);
