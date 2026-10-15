// Another app, for the surfaces drive (app/test/surfaces-drive.mjs --fullscreen): one
// ordinary window that can go full screen — a Space of its own, the way a browser or an
// editor takes one. Run with `osascript -l JavaScript fullscreen-helper.js <dir>`; only
// ever inside the L5 VM (tests/l5/surfaces-vm.sh), never on a desk somebody is using.
//
// It says what it is in <dir>/state.json and does what the files dropped beside it ask:
//   cmd-full   its window goes full screen        cmd-desk   back to a desktop window
//   cmd-front  it comes forward                   cmd-quit   it leaves
// It cleans up after itself: out of full screen before it leaves, and gone after five
// minutes — or as soon as <dir> is — whatever happens to the run that started it.
ObjC.import("Cocoa");

function run(argv) {
  const dir = argv[0];
  const fm = $.NSFileManager.defaultManager;
  const app = $.NSApplication.sharedApplication;
  app.setActivationPolicy($.NSApplicationActivationPolicyRegular);
  // Titled, closable, miniaturizable, resizable.
  const win = $.NSWindow.alloc.initWithContentRectStyleMaskBackingDefer($.NSMakeRect(160, 160, 720, 460), 1 | 2 | 4 | 8, $.NSBackingStoreBuffered, false);
  win.title = "shroom test: another app";
  win.collectionBehavior = 1 << 7; // FullScreenPrimary
  win.releasedWhenClosed = false;
  app.finishLaunching;
  win.makeKeyAndOrderFront(null);
  app.activateIgnoringOtherApps(true);

  const FULL = 1 << 14;
  const full = () => (Number(win.styleMask) & FULL) !== 0;
  let seq = 0;
  const say = () => {
    const s = JSON.stringify({
      pid: Number($.NSProcessInfo.processInfo.processIdentifier),
      active: Boolean(app.isActive),
      fullscreen: full(),
      on_active_space: Boolean(win.isOnActiveSpace),
      key: Boolean(win.isKeyWindow),
      visible: Boolean(win.isVisible),
      seq,
    });
    $(s).writeToFileAtomicallyEncodingError(dir + "/state.json", true, $.NSUTF8StringEncoding, null);
  };
  const asked = (name) => {
    const p = dir + "/cmd-" + name;
    if (!fm.fileExistsAtPath(p)) return false;
    fm.removeItemAtPathError(p, null);
    seq += 1;
    return true;
  };
  const pump = (seconds) => {
    const until = Date.now() + seconds * 1000;
    do {
      const ev = app.nextEventMatchingMaskUntilDateInModeDequeue(0xffffffff, $.NSDate.dateWithTimeIntervalSinceNow(0.05), $.NSDefaultRunLoopMode, true);
      if (!ev.isNil()) app.sendEvent(ev);
    } while (Date.now() < until);
  };

  const deadline = Date.now() + 5 * 60 * 1000;
  for (;;) {
    pump(0.1);
    if (asked("full") && !full()) win.toggleFullScreen(null);
    if (asked("desk") && full()) win.toggleFullScreen(null);
    if (asked("front")) { app.activateIgnoringOtherApps(true); win.makeKeyAndOrderFront(null); }
    const leaving = asked("quit") || Date.now() > deadline || !fm.fileExistsAtPath(dir);
    if (leaving) {
      if (full()) { win.toggleFullScreen(null); pump(2.5); }
      win.orderOut(null);
      if (fm.fileExistsAtPath(dir)) say();
      return "left";
    }
    say();
  }
}
