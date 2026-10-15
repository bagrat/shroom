// The launch drive's sampler: what macOS says of one app, looked at again and again while
// that app starts — is it in the Dock, is it the app in front, is the menu bar its own.
// The running-applications list any app may read; nothing is asked of macOS for it, and
// nothing is said of any other app (only whether the one in front is this one).
//
//   osascript -l JavaScript launch-watch.js <bundle id> <the .app's path>
//
// One look per line on stdin (the runner sets the pace — a process that sleeps by itself
// in a VM wakes every 16 ms at best); when stdin closes, the report on stdout: how many
// looks, how far apart, and every change seen.

ObjC.import("AppKit");

function run(argv) {
  const [bundle, path] = argv;
  const workspace = $.NSWorkspace.sharedWorkspace;
  const info = $.NSProcessInfo.processInfo;
  const loop = $.NSRunLoop.currentRunLoop;
  const past = $.NSDate.distantPast;
  const stdin = $.NSFileHandle.fileHandleWithStandardInput;
  const stdout = $.NSFileHandle.fileHandleWithStandardOutput;
  const say = (s) => stdout.writeData($(s + "\n").dataUsingEncoding($.NSUTF8StringEncoding));
  const real = (p) => $(p).stringByResolvingSymlinksInPath.js;
  const pidOf = (a) => (a.isNil() ? -1 : a.processIdentifier);
  const want = real(path);
  const t0 = info.systemUptime;
  const changes = [];
  const gaps = [];
  let last = "";
  let prev = t0;
  say("ready");
  // A read at the end of the input comes back empty (and its length as a string).
  while (Number(stdin.availableData.length) !== 0) {
    // What macOS has announced since the last look lands here.
    loop.runModeBeforeDate($.NSDefaultRunLoopMode, past);
    const now = info.systemUptime;
    gaps.push(now - prev);
    prev = now;
    const front = pidOf(workspace.frontmostApplication);
    const menu = pidOf(workspace.menuBarOwningApplication);
    const running = $.NSRunningApplication.runningApplicationsWithBundleIdentifier(bundle);
    const mine = [];
    for (let i = 0; i < running.count; i++) {
      const a = running.objectAtIndex(i);
      // Only the copy under test: another shroom with the same name is not this launch.
      if (a.bundleURL.isNil() || real(a.bundleURL.path.js) !== want) continue;
      const pid = a.processIdentifier;
      mine.push({ pid, policy: ["regular", "accessory", "prohibited"][a.activationPolicy] || "unknown", active: a.active, front: pid === front, menu: pid === menu });
    }
    const s = JSON.stringify(mine);
    if (s !== last) {
      last = s;
      changes.push({ t: Math.round((now - t0) * 1000), apps: mine });
    }
  }
  gaps.sort((a, b) => a - b);
  const ms = (s) => Math.round(s * 10000) / 10;
  say(
    JSON.stringify({
      looks: gaps.length,
      every_ms: ms(gaps[gaps.length >> 1] || 0),
      over_15_ms: gaps.filter((g) => g > 0.015).length,
      longest_ms: ms(gaps[gaps.length - 1] || 0),
      ran_ms: Math.round((prev - t0) * 1000),
      changes,
    }),
  );
  return "";
}
