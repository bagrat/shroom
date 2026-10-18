//! What a drive reads and does at the keyboard's level (dev builds, the harness only): who
//! macOS sends keys to, which of shroom's windows holds them, and keys typed the way AppKit
//! delivers them — posted to the app's own event queue, addressed to whichever window holds
//! the keyboard at that moment, never to a named one. A set-up shroom with no window open
//! has no Dock icon (law 6), so what an ordinary app gets for free is proven here
//! (DECISIONS B105).
//!
//! Nothing here sends an event to another app, needs a permission (no Accessibility, no
//! Input Monitoring, no CGEvent), or reads or writes the person's clipboard. Every
//! function runs on the main thread, but `open_again` and `caret`, which hop there
//! themselves.

use objc2::rc::Retained;
use objc2::runtime::{AnyObject, Sel};
use objc2::{class, msg_send};
use objc2_app_kit::{NSApplication, NSApplicationActivationOptions, NSEvent, NSEventModifierFlags, NSEventType, NSRunningApplication, NSWindow, NSWorkspace};
use objc2_foundation::{MainThreadMarker, NSPoint, NSString};
use serde::Serialize;
use tauri::{AppHandle, Manager};

/// Where one of shroom's windows stands.
#[derive(Debug, Clone, Default, Serialize)]
pub struct Front {
    /// The activation policy: `regular (Dock icon)` or `accessory (no Dock icon)`.
    pub policy: String,
    /// macOS sends the keyboard to shroom (NSWorkspace's frontmost application).
    pub frontmost: bool,
    /// AppKit says shroom is the active app.
    pub active: bool,
    /// The window is the one that holds the keyboard.
    pub key: bool,
    /// It is on screen: shown, not minimized, shroom not hidden.
    pub shown: bool,
    /// Nothing covers its middle — no window of any app.
    pub on_top: bool,
}

impl Front {
    /// In front of everything, and what the person types goes to it.
    pub fn has_keyboard(&self) -> bool {
        self.frontmost && self.active && self.key && self.shown && self.on_top
    }

    pub fn says(&self) -> String {
        serde_json::to_string(self).unwrap_or_default()
    }
}

fn mtm() -> MainThreadMarker {
    MainThreadMarker::new().expect("main thread")
}

fn pid() -> i32 {
    std::process::id() as i32
}

fn ns_window(app: &AppHandle, label: &str) -> Option<Retained<NSWindow>> {
    let ptr = app.get_webview_window(label)?.ns_window().ok()?;
    // SAFETY: Tauri hands back its live NSWindow; retained for as long as it's read here.
    unsafe { Retained::retain(ptr as *mut NSWindow) }
}

/// Where `label`'s window stands right now.
pub fn front(app: &AppHandle, label: &str) -> Front {
    let mtm = mtm();
    let ns_app = NSApplication::sharedApplication(mtm);
    let mut f = Front { policy: crate::harness::policy(), frontmost: ours_in_front(), active: ns_app.isActive(), ..Front::default() };
    let Some(w) = ns_window(app, label) else { return f };
    f.key = w.isKeyWindow();
    f.shown = w.isVisible() && !w.isMiniaturized() && !ns_app.isHidden();
    let r = w.frame();
    let middle = NSPoint::new(r.origin.x + r.size.width / 2.0, r.origin.y + r.size.height / 2.0);
    f.on_top = f.shown && NSWindow::windowNumberAtPoint_belowWindowWithWindowNumber(middle, 0, mtm) == w.windowNumber();
    f
}

/// macOS sends the keyboard to shroom.
pub fn ours_in_front() -> bool {
    NSWorkspace::sharedWorkspace().frontmostApplication().is_some_and(|a| a.processIdentifier() == pid())
}

/// The app macOS sends the keyboard to, by bundle id.
pub fn frontmost_id() -> Option<String> {
    NSWorkspace::sharedWorkspace().frontmostApplication().and_then(|a| a.bundleIdentifier()).map(|s| s.to_string())
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    /// The login session as the window server has it (a CFDictionary, owned by the caller).
    fn CGSessionCopyCurrentDictionary() -> *mut AnyObject;
}

/// Why nothing of any app's can be seen right now, if nothing can: the screen is locked (or
/// there is no login session to show a window in). The lock screen is over everything, the
/// menu bar too — and macOS says so of every menu-bar item: behind a lock none counts as
/// seen, whichever app's it is. Measured 2026-10-13 with a bare status item of another
/// process's, on this Mac behind its lock (parked above the top of the screen, on no
/// screen, not seen) and in one macOS 26.6.2 clone before and after its screen was locked
/// (at rest in the menu bar and seen; at rest in the menu bar and not seen). So behind a
/// lock the mushroom reads as hidden, and a popover takes the corner (N41) — as it should:
/// nobody can see either.
pub fn behind_the_lock() -> Option<&'static str> {
    // SAFETY: the dictionary comes back owned (the Copy rule) and is toll-free bridged to
    // NSDictionary; it's read and released here.
    unsafe {
        let session = CGSessionCopyCurrentDictionary();
        let Some(session) = Retained::from_raw(session) else { return Some("there is no login session on screen") };
        let locked: *mut AnyObject = msg_send![&*session, objectForKey: &*NSString::from_str("CGSSessionScreenIsLocked")];
        if !locked.is_null() && msg_send![locked, boolValue] {
            return Some("the screen is locked");
        }
    }
    None
}

/// Why nothing can take the keyboard right now, if nothing can: the screen is locked (or
/// there is no login session to show a window in), or the login window is in front. Every
/// app is behind the lock then — shroom included — so a drive says what it couldn't look
/// at instead of failing on it.
pub fn no_keyboard() -> Option<&'static str> {
    behind_the_lock().or_else(|| (frontmost_id().as_deref() == Some("com.apple.loginwindow")).then_some("the login window is in front"))
}

/// The person turns to another app: the one whose menus the menu bar shows (a menu-bar
/// app never owns the menu bar, so that is the app they were in), else Finder. Its
/// bundle id, once asked — the caller waits for macOS to send the keyboard there.
pub fn turn_to_another_app() -> Option<String> {
    let ns_app = NSApplication::sharedApplication(mtm());
    let ws = NSWorkspace::sharedWorkspace();
    let other = ws.menuBarOwningApplication().filter(|a| a.processIdentifier() != pid()).or_else(|| {
        NSRunningApplication::runningApplicationsWithBundleIdentifier(&NSString::from_str("com.apple.finder")).iter().next()
    })?;
    // macOS 14's cooperative activation: shroom gives the keyboard up to it, by name.
    ns_app.yieldActivationToApplication(&other);
    let me = NSRunningApplication::currentApplication();
    if !other.activateFromApplication_options(&me, NSApplicationActivationOptions(0)) {
        other.activateWithOptions(NSApplicationActivationOptions(0));
    }
    other.bundleIdentifier().map(|s| s.to_string())
}

/// US-ANSI key codes for the keys a drive types (the characters carry the meaning; the
/// codes keep the events shaped like a keyboard's).
fn key_code(c: char) -> u16 {
    match c.to_ascii_lowercase() {
        'a' => 0,
        's' => 1,
        'd' => 2,
        'f' => 3,
        'h' => 4,
        'g' => 5,
        'z' => 6,
        'x' => 7,
        'c' => 8,
        'v' => 9,
        'b' => 11,
        'q' => 12,
        'w' => 13,
        'e' => 14,
        'r' => 15,
        'y' => 16,
        't' => 17,
        '1' => 18,
        '2' => 19,
        '3' => 20,
        '4' => 21,
        '6' => 22,
        '5' => 23,
        '9' => 25,
        '7' => 26,
        '8' => 28,
        '0' => 29,
        'o' => 31,
        'u' => 32,
        'i' => 34,
        'p' => 35,
        'l' => 37,
        'j' => 38,
        'k' => 40,
        'n' => 45,
        'm' => 46,
        ' ' => 49,
        '\u{7f}' => 51,
        _ => 0,
    }
}

fn post(kind: NSEventType, c: char, flags: NSEventModifierFlags) -> Result<(), String> {
    let ns_app = NSApplication::sharedApplication(mtm());
    // A keyboard's keys go to the window that holds the keyboard — whichever that is.
    let Some(key) = ns_app.keyWindow() else { return Err("no window of shroom's holds the keyboard".into()) };
    let chars = NSString::from_str(&c.to_string());
    // SAFETY: NSProcessInfo's shared instance and its plain getter.
    let uptime: f64 = unsafe {
        let info: *mut AnyObject = msg_send![class!(NSProcessInfo), processInfo];
        msg_send![info, systemUptime]
    };
    let nil: *const AnyObject = std::ptr::null();
    // SAFETY: NSEvent's own constructor with its documented argument types; the context
    // argument is documented as "pass nil".
    let ev: Option<Retained<NSEvent>> = unsafe {
        msg_send![
            class!(NSEvent),
            keyEventWithType: kind,
            location: NSPoint::new(0.0, 0.0),
            modifierFlags: flags,
            timestamp: uptime,
            windowNumber: key.windowNumber(),
            context: nil,
            characters: &*chars,
            charactersIgnoringModifiers: &*chars,
            isARepeat: false,
            keyCode: key_code(c)
        ]
    };
    let ev = ev.ok_or("AppKit made no key event")?;
    ns_app.postEvent_atStart(&ev, false);
    Ok(())
}

/// One key, down and up, into shroom's own event queue.
pub fn press(c: char) -> Result<(), String> {
    post(NSEventType::KeyDown, c, NSEventModifierFlags(0))?;
    post(NSEventType::KeyUp, c, NSEventModifierFlags(0))
}

/// ⌘ and a key. (AppKit never delivers the key-up of a ⌘ chord.)
pub fn command(c: char) -> Result<(), String> {
    post(NSEventType::KeyDown, c, NSEventModifierFlags::Command)
}

/// The app menu's Quit — `terminate:` from no window, which is what ⌘Q sends when none of
/// shroom's windows holds the keyboard, so no key event can be addressed (B145: shroom quits,
/// through the core). Main thread.
pub fn terminate() {
    let ns_app = NSApplication::sharedApplication(mtm());
    let nil: *const AnyObject = std::ptr::null();
    // SAFETY: NSApplication's own action, with the nil sender a menu item may send.
    let _: () = unsafe { msg_send![&*ns_app, terminate: nil] };
}

/// A standard key of the app's menu, as AppKit would answer it now.
#[derive(Debug, Clone, Serialize)]
pub struct MenuKey {
    pub title: String,
    /// The action the key sends (`copy:`, `performClose:`, `terminate:`, …).
    pub action: String,
    /// Something in front answers it now (the field with the caret, the window, the app).
    pub answered: bool,
}

/// The menu item behind ⌘`key` in the app's menu — which an app with no Dock icon never
/// shows, and which still answers its keys.
pub fn menu_key(key: &str) -> Option<MenuKey> {
    let ns_app = NSApplication::sharedApplication(mtm());
    // SAFETY: plain AppKit getters; every object is checked for nil before it's messaged.
    unsafe {
        let menu: *mut AnyObject = msg_send![&*ns_app, mainMenu];
        find(&ns_app, menu, key)
    }
}

unsafe fn find(ns_app: &NSApplication, menu: *mut AnyObject, key: &str) -> Option<MenuKey> {
    if menu.is_null() {
        return None;
    }
    unsafe {
        let n: isize = msg_send![menu, numberOfItems];
        for i in 0..n {
            let item: *mut AnyObject = msg_send![menu, itemAtIndex: i];
            if item.is_null() {
                continue;
            }
            let sub: *mut AnyObject = msg_send![item, submenu];
            if !sub.is_null() {
                if let Some(found) = find(ns_app, sub, key) {
                    return Some(found);
                }
                continue;
            }
            let ke: *mut NSString = msg_send![item, keyEquivalent];
            let mask: NSEventModifierFlags = msg_send![item, keyEquivalentModifierMask];
            if ke.is_null() || (*ke).to_string() != key || mask != NSEventModifierFlags::Command {
                continue;
            }
            let action: Option<Sel> = msg_send![item, action];
            let Some(action) = action else { continue };
            let title: *mut NSString = msg_send![item, title];
            let target: *mut AnyObject = msg_send![item, target];
            let to: *mut AnyObject = msg_send![ns_app, targetForAction: action, to: target, from: item];
            return Some(MenuKey { title: if title.is_null() { String::new() } else { (*title).to_string() }, action: action.name().to_string_lossy().into_owned(), answered: !to.is_null() });
        }
    }
    None
}

/// shroom is opened again while it runs — Finder, Spotlight, `open`: macOS sends the
/// running app the reopen event. Sent here to this very process through the Apple-event
/// manager, so nothing can start a second shroom.
///
/// Called off the main thread. An event an app sends itself is delivered at once, inside
/// the send — so it goes out from the main queue, between the event loop's turns (inside
/// one, the reopen it delivers would wait on that very turn for ever).
pub fn open_again() -> Result<(), String> {
    let (tx, rx) = std::sync::mpsc::channel();
    crate::runtime::observe::dispatch_main(move || {
        let _ = tx.send(send_reopen());
    });
    rx.recv_timeout(std::time::Duration::from_secs(5)).map_err(|_| "the main queue didn't answer".to_string())?
}

fn send_reopen() -> Result<(), String> {
    const AEVT: u32 = u32::from_be_bytes(*b"aevt");
    const RAPP: u32 = u32::from_be_bytes(*b"rapp");
    // `kAutoGenerateReturnID`, `kAnyTransactionID`, `NSAppleEventSendNoReply`.
    const ANY_RETURN: i16 = -1;
    const ANY_TRANSACTION: i32 = 0;
    const NO_REPLY: usize = 1;
    // SAFETY: Foundation's own constructors and send, with their documented types; each
    // object is checked for nil before it's messaged.
    unsafe {
        let me: *mut AnyObject = msg_send![class!(NSAppleEventDescriptor), descriptorWithProcessIdentifier: pid()];
        if me.is_null() {
            return Err("no descriptor for this process".into());
        }
        let ev: *mut AnyObject = msg_send![class!(NSAppleEventDescriptor), appleEventWithEventClass: AEVT, eventID: RAPP, targetDescriptor: me, returnID: ANY_RETURN, transactionID: ANY_TRANSACTION];
        if ev.is_null() {
            return Err("no reopen event".into());
        }
        let mut err: *mut AnyObject = std::ptr::null_mut();
        let _reply: *mut AnyObject = msg_send![ev, sendEventWithOptions: NO_REPLY, timeout: 5.0f64, error: &mut err];
        if err.is_null() {
            return Ok(());
        }
        let said: *mut NSString = msg_send![err, localizedDescription];
        Err(if said.is_null() { "the reopen event wasn't sent".into() } else { (*said).to_string() })
    }
}

/// Puts the caret in a field of `label`'s window, as a click in it would (a drive can't
/// click a webview without a permission it must not ask for) — and answers once the field
/// really holds it: the page is drawn, the field is there, and the page has the window's
/// keyboard. Called off the main thread.
pub fn caret(app: &AppHandle, label: &str, selector: &str) -> Result<(), String> {
    let selector = serde_json::to_string(selector).map_err(|e| e.to_string())?;
    let js = format!("(() => {{ const f = document.querySelector({selector}); if (!f) return 'the field is not there yet'; f.focus(); return document.hasFocus() && document.activeElement === f ? 'ok' : 'the field has no caret yet'; }})()");
    let until = std::time::Instant::now() + std::time::Duration::from_secs(6);
    loop {
        let said = ask(app, label, &js)?;
        if said == "ok" {
            return Ok(());
        }
        if std::time::Instant::now() > until {
            return Err(said);
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// Waits, for up to `secs`, until `label`'s page is drawn with the field `selector` in it —
/// without touching the caret — and answers which field has the caret then (its
/// `data-field`, or what has it instead). Called off the main thread.
pub fn drawn_with(app: &AppHandle, label: &str, selector: &str, secs: u64) -> Result<String, String> {
    let selector = serde_json::to_string(selector).map_err(|e| e.to_string())?;
    let js = format!("(() => {{ if (!document.querySelector({selector})) return ''; const a = document.activeElement; return 'caret: ' + (a ? (a.dataset?.field ?? a.tagName.toLowerCase()) : 'none'); }})()");
    let until = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    loop {
        let said = ask(app, label, &js)?;
        if !said.is_empty() {
            return Ok(said);
        }
        if std::time::Instant::now() > until {
            return Err(format!("the page had no {selector} after {secs} s"));
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

/// Waits, for up to `secs`, until the field with the caret in `label`'s page has all of a
/// non-empty value selected — what ⌘A leaves — and answers what it last saw. Called off the
/// main thread.
pub fn all_selected(app: &AppHandle, label: &str, secs: u64) -> Result<(), String> {
    let js = "(() => { const f = document.activeElement; if (!f || f.selectionStart == null) return 'no field has the caret'; return f.value.length > 0 && f.selectionStart === 0 && f.selectionEnd === f.value.length ? 'ok' : `selected ${f.selectionStart}..${f.selectionEnd} of ${JSON.stringify(f.value)}`; })()";
    let until = std::time::Instant::now() + std::time::Duration::from_secs(secs);
    loop {
        let said = ask(app, label, js)?;
        if said == "ok" {
            return Ok(());
        }
        if std::time::Instant::now() > until {
            return Err(said);
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
}

/// Runs a line of script in `label`'s page and brings back what it said. Called off the
/// main thread (the page answers on it).
fn ask(app: &AppHandle, label: &str, js: &str) -> Result<String, String> {
    let w = app.get_webview_window(label).ok_or("no such window")?;
    let (tx, rx) = std::sync::mpsc::channel();
    let js = js.to_string();
    w.with_webview(move |webview| {
        let page = webview.inner() as *mut AnyObject;
        let done = block2::RcBlock::new(move |said: *mut AnyObject, _error: *mut AnyObject| {
            // SAFETY: WebKit hands the script's value back as an object (or nil); its
            // description is read while the handler runs.
            let said = if said.is_null() {
                String::new()
            } else {
                unsafe {
                    let d: *mut NSString = msg_send![said, description];
                    if d.is_null() { String::new() } else { (*d).to_string() }
                }
            };
            let _ = tx.send(said);
        });
        // SAFETY: Tauri's live WKWebView, on the main thread, with WebKit's own
        // evaluate-and-answer call.
        unsafe {
            let _: () = msg_send![page, evaluateJavaScript: &*NSString::from_str(&js), completionHandler: &*done];
        }
    })
    .map_err(|e| e.to_string())?;
    rx.recv_timeout(std::time::Duration::from_secs(5)).map_err(|_| "the page didn't answer".to_string())
}

/// The yellow button: `label`'s window goes down to the Dock's right end.
pub fn minimize(app: &AppHandle, label: &str) {
    if let Some(w) = ns_window(app, label) {
        w.miniaturize(None);
    }
}

pub fn minimized(app: &AppHandle, label: &str) -> bool {
    ns_window(app, label).is_some_and(|w| w.isMiniaturized())
}

pub fn hidden() -> bool {
    NSApplication::sharedApplication(mtm()).isHidden()
}
