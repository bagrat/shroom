// Where macOS keeps a status item while it changes (scenarios/tray-moves.sh): read every 10 ms, a
// line each time the reading changes. Run with osascript -l JavaScript.
ObjC.import('AppKit');
ObjC.import('CoreGraphics');
const sh = Application.currentApplication();
sh.includeStandardAdditions = true;
const app = $.NSApplication.sharedApplication;
app.setActivationPolicy(1); // accessory: a menu-bar app, as shroom is when idle
const t0 = $.NSDate.date;
const ms = () => Math.round(-t0.timeIntervalSinceNow * 1000);
const pump = (s) => $.NSRunLoop.currentRunLoop.runUntilDate($.NSDate.dateWithTimeIntervalSinceNow(s));
const item = $.NSStatusBar.systemStatusBar.statusItemWithLength(-1);
const img = (name, template) => { const i = $.NSImage.imageNamed(name).copy; i.setSize({ width: 18, height: 18 }); i.setTemplate(template); return i; };
item.button.setImage(img('NSActionTemplate', true));
const f = (r) => `${Math.round(r.origin.x)},${Math.round(r.origin.y)} ${Math.round(r.size.width)}x${Math.round(r.size.height)}`;
const screens = () => { const s = $.NSScreen.screens; const out = []; for (let i = 0; i < s.count; i++) out.push(f(s.objectAtIndex(i).frame)); return out.join(' + '); };
const read = () => {
  const w = item.button.window;
  if (!w || w.isNil()) return 'no window';
  const s = w.screen;
  return `item ${f(w.frame)} screen ${s && !s.isNil() ? f(s.frame) : 'none'} occlusion ${(w.occlusionState & 2) ? 'visible' : 'not visible'} onscreen ${w.isOnActiveSpace}`;
};
let last = '';
const watch = (label, seconds) => {
  console.log(`--- ${label} (t ${ms()} ms; screens ${screens()})`);
  last = '';
  const end = ms() + seconds * 1000;
  while (ms() < end) {
    const r = read();
    if (r !== last) { console.log(`${String(ms()).padStart(6)} ${r}`); last = r; }
    pump(0.01);
  }
};
const mouse = (x, y) => $.CGWarpMouseCursorPosition({ x, y });
const top = $.NSScreen.screens.objectAtIndex(0).frame.size.height;

watch('made', 2);
item.button.setImage(img('NSStatusAvailable', false)); // a template face to a drawn one…
watch('face: template → bitmap', 1);
item.button.setImage(img('NSActionTemplate', true)); // …and back, as Red → pause bars
watch('face: bitmap → template', 1);
item.button.setImage(img('NSStatusAvailable', false));
item.button.setImage(img('NSActionTemplate', true));
watch('face: twice in one turn', 1);

mouse(300, 400);
sh.doShellScript('defaults write NSGlobalDomain _HIHideMenuBar -bool true');
$.NSDistributedNotificationCenter.defaultCenter.postNotificationNameObjectUserInfoDeliverImmediately('AppleInterfaceMenuBarHidingChangedNotification', $(), $(), true);
watch('menu bar set to hide, pointer away', 3);
const w = item.button.window;
mouse(w.frame.origin.x + 10, 2);
watch('pointer at the top', 2);
mouse(300, 400);
watch('pointer away again', 3);
sh.doShellScript('defaults delete NSGlobalDomain _HIHideMenuBar');
$.NSDistributedNotificationCenter.defaultCenter.postNotificationNameObjectUserInfoDeliverImmediately('AppleInterfaceMenuBarHidingChangedNotification', $(), $(), true);
watch('menu bar back', 2);

// A full-screen app hides the menu bar on its Space.
sh.doShellScript('open -a TextEdit');
pump(3);
try {
  sh.doShellScript(`osascript -e 'tell application "System Events" to tell process "TextEdit" to set value of attribute "AXFullScreen" of window 1 to true'`);
} catch (e) { console.log(`full screen: ${e}`); }
watch('TextEdit full screen, pointer away', 4);
mouse(w.frame.origin.x + 10, 2);
watch('full screen, pointer at the top', 2);
mouse(300, 400);
watch('full screen, pointer away again', 2);
sh.doShellScript('killall TextEdit || true');
watch('TextEdit gone', 2);
