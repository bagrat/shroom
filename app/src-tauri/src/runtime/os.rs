//! The shell's macOS calls for the onboarding effects: public pages in the browser, the
//! permission prompts, the login item, and quitting to reopen. Every one of them is only
//! ever the answer to a click (law 11); in quiet runs (the test harness) each one logs
//! what it would do instead.

use objc2::msg_send;
use objc2::rc::Retained;
use objc2::runtime::{AnyClass, AnyObject};
use objc2_foundation::NSError;
use shroom_core::effect::{Perm, PublicUrl};
use shroom_core::event::{Event, OsNews};
use shroom_core::world::RelaunchWhy;

use super::Runtime;

pub const SIGNUP: &str = "https://dash.cloudflare.com/sign-up";
/// The pane the screen step opens (POC-LEARNINGS §4: the grant is a toggle there, and
/// takes effect on the next launch).
pub const SCREEN_PANE: &str = "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture";

/// Cloudflare's pre-filled, account-owned token page: name `shroom`, Workers Scripts Edit
/// and Workers R2 Storage Edit. UNVERIFIED: the template parameters follow Cloudflare's
/// template-URL docs (RESEARCH R7, account tokens via `?to=/:account/api-tokens`); the
/// exact keys still need a live look at Bagrat's checkpoint (DECISIONS notes it).
pub fn token_page() -> String {
    let groups = r#"[{"key":"workers_scripts","type":"edit"},{"key":"workers_r2","type":"edit"}]"#;
    format!("https://dash.cloudflare.com/?to=/:account/api-tokens&name=shroom&permissionGroupKeys={}", percent(groups))
}

pub fn url(page: &PublicUrl) -> String {
    match page {
        PublicUrl::Signup => SIGNUP.to_string(),
        PublicUrl::TokenPage => token_page(),
        PublicUrl::Billing { account_id } => format!("https://dash.cloudflare.com/{}/r2/overview", percent(account_id)),
        // shroom.am's page for that release; one with no notes shows the newest (B152).
        PublicUrl::ReleaseNotes { version } => format!("https://shroom.am/whats-new/{}", percent(version)),
        PublicUrl::NewIssue { version } => {
            format!("https://github.com/bagrat/shroom/issues/new?body={}", percent(&format!("shroom {version}")))
        }
        // A send that didn't go through (B180): a draft with the words and the version — short,
        // since a mail link that's too long opens nothing.
        PublicUrl::MailReport { version, words } => {
            let words: String = words.chars().take(1_500).collect();
            format!("mailto:hi@shroom.am?subject={}&body={}", percent(&format!("shroom {version} — a problem")), percent(&format!("{words}\n\nshroom {version}")))
        }
        PublicUrl::AgentGuide => "https://shroom.am/cli".to_string(),
    }
}

/// RFC 3986 percent-encoding of everything but the unreserved set.
pub fn percent(s: &str) -> String {
    s.bytes()
        .map(|b| if b.is_ascii_alphanumeric() || b"-._~".contains(&b) { (b as char).to_string() } else { format!("%{b:02X}") })
        .collect()
}

fn quiet(rt: &Runtime) -> bool {
    rt.quiet.load(std::sync::atomic::Ordering::SeqCst)
}

pub fn open_url(rt: &Runtime, page: &PublicUrl) {
    let u = url(page);
    if quiet(rt) {
        rt.log(&format!("shell would open {u}"));
        return;
    }
    rt.log(&format!("shell opens {}", match page {
        PublicUrl::Signup => "signup",
        PublicUrl::TokenPage => "token-page",
        PublicUrl::Billing { .. } => "billing",
        PublicUrl::ReleaseNotes { .. } => "release-notes",
        PublicUrl::NewIssue { .. } => "new-issue",
        PublicUrl::MailReport { .. } => "mail-report",
        PublicUrl::AgentGuide => "agent-guide",
    }));
    open_detached(u);
}

/// `open <target>` off the core thread: LaunchServices' hand-off (a cold browser, System
/// Settings) never holds the core loop.
pub fn open_detached(target: String) {
    std::thread::spawn(move || {
        let _ = std::process::Command::new("open").arg(target).status();
    });
}

/// The grants as macOS reports them now (callable off the main thread).
pub fn grants() -> OsNews {
    OsNews::Grants {
        screen: shroom_capture::has_permission(),
        mic: super::probe::access(shroom_capture::mic_access()),
        camera: super::probe::access(shroom_capture::camera_access()),
    }
}

/// Puts one permission to macOS from this process (so the prompt is shroom's), then
/// re-probes the grants — the microphone's answer is what chains the screen step.
pub fn request_permission(rt: &Runtime, perm: Perm) {
    if quiet(rt) {
        rt.log(&format!("shell would ask macOS for {perm:?}"));
        if perm == Perm::Screen {
            rt.post(Event::Os(OsNews::ScreenAsked { alert: false }));
        }
        rt.post(Event::Os(grants()));
        return;
    }
    let rt = rt.clone();
    std::thread::spawn(move || {
        match perm {
            // AVCaptureDevice requestAccessForMediaType:audio — blocks until answered.
            Perm::Mic => {
                shroom_capture::request_mic();
            }
            Perm::Camera => {
                shroom_capture::request_camera();
            }
            // One surface at a time (N43, DECISIONS B146). CGRequestScreenCaptureAccess is
            // what lists shroom in the pane, and macOS raises its own alert on the first ask
            // only, whose Open System Settings opens the pane: so the first ask on this Mac
            // opens nothing of ours, and every later one opens the pane. The window check
            // backs the first ask: no alert after all (a Mac that asked before this home
            // existed), and the pane opens.
            Perm::Screen => {
                let first = rt.home.read_json::<serde_json::Value>(SCREEN_ASKED).is_none();
                shroom_capture::request_permission();
                if first {
                    let _ = rt.home.write_json(SCREEN_ASKED, &serde_json::json!({ "v": 1 }));
                }
                let alert = first && screen_alert_shows();
                if !alert {
                    let _ = std::process::Command::new("open").arg(SCREEN_PANE).status();
                }
                rt.log(&format!("shell asked for screen recording: {}", if alert { "macOS's alert leads" } else { "opened the pane" }));
                rt.post(Event::Os(OsNews::ScreenAsked { alert }));
            }
        }
        rt.post(Event::Os(grants()));
    });
}

/// macOS was asked for screen recording on this Mac before (B146): an uninstall clears it
/// with macOS's grants.
const SCREEN_ASKED: &str = "screen-asked.json";

/// The process that draws macOS's screen-recording alert (macOS 26, read in the probe of
/// DECISIONS B146). A prefix: a process's short name stops at 16 characters.
const SCREEN_ALERT: &str = "universalAccess";

/// Whether macOS's screen-recording alert comes up after the ask: the request returns at
/// once and the alert follows, so the window list is watched for up to two seconds.
fn screen_alert_shows() -> bool {
    let until = std::time::Instant::now() + std::time::Duration::from_secs(2);
    loop {
        if window_owners().iter().any(|o| o.starts_with(SCREEN_ALERT)) {
            return true;
        }
        if std::time::Instant::now() >= until {
            return false;
        }
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGWindowListCopyWindowInfo(option: u32, relative_to: u32) -> *mut std::ffi::c_void;
}

/// The owners of the windows on screen, by name — from the window server's list, which any
/// app may read (no window's own name or picture is asked for): the owner's name, else its
/// process's.
fn window_owners() -> Vec<String> {
    use objc2_foundation::{NSArray, NSDictionary, NSString};
    const ON_SCREEN_ONLY: u32 = 1;
    // SAFETY: a +1 array of dictionaries, or null.
    let raw = unsafe { CGWindowListCopyWindowInfo(ON_SCREEN_ONLY, 0) } as *mut NSArray<NSDictionary<NSString, AnyObject>>;
    let Some(list) = (unsafe { Retained::from_raw(raw) }) else { return vec![] };
    let (owner, owner_pid) = (NSString::from_str("kCGWindowOwnerName"), NSString::from_str("kCGWindowOwnerPID"));
    list.iter()
        .filter_map(|w| {
            if let Some(name) = w.objectForKey(&owner) {
                // SAFETY: the owner's name is an NSString.
                return Some(unsafe { &*(Retained::as_ptr(&name) as *const NSString) }.to_string());
            }
            // SAFETY: the owner's pid is an NSNumber.
            let pid: i32 = w.objectForKey(&owner_pid).map(|n| unsafe { msg_send![&*n, intValue] })?;
            let mut name = [0u8; 256];
            // SAFETY: the buffer outlives the call and its size is given.
            let len = unsafe { libc::proc_name(pid, name.as_mut_ptr().cast(), name.len() as u32) };
            (len > 0).then(|| String::from_utf8_lossy(&name[..len as usize]).into_owned())
        })
        .collect()
}

#[link(name = "ServiceManagement", kind = "framework")]
unsafe extern "C" {}

fn main_app_service() -> Option<Retained<AnyObject>> {
    let cls = AnyClass::get(c"SMAppService")?;
    unsafe { msg_send![cls, mainAppService] }
}

/// Whether shroom starts at login, as macOS says now (read only — `status` never registers
/// or unregisters anything).
pub fn login_item() -> Option<bool> {
    let svc = main_app_service()?;
    let status: isize = unsafe { msg_send![&svc, status] };
    Some(starts_at_login(status))
}

/// `SMAppServiceStatus` as the login box reads it: only `enabled` (1) starts shroom at
/// login. `requiresApproval` (2) — registered, but switched off in System Settings — does
/// not, so the box reads off and its click registers again (N36: the box reads the
/// system's truth). `notRegistered` (0) and `notFound` (3) are off.
pub fn starts_at_login(status: isize) -> bool {
    status == 1
}

/// The login item off, here and now (an uninstall's first step, B135): synchronous, so
/// the caller is off the core thread.
pub fn unregister_login_item(rt: &Runtime) {
    let Some(svc) = main_app_service() else { return };
    let result: Result<(), Retained<NSError>> = unsafe { msg_send![&svc, unregisterAndReturnError: _] };
    if let Err(e) = result {
        // notRegistered (a login item never switched on) is the usual answer here.
        rt.log(&format!("uninstall: login item unregister: code {}", e.code()));
    }
}

pub fn set_login_item(rt: &Runtime, on: bool) {
    if quiet(rt) {
        rt.log(&format!("shell would {} the login item", if on { "register" } else { "unregister" }));
        return;
    }
    // A synchronous call to the service-management daemon: off the core thread.
    let rt = rt.clone();
    std::thread::spawn(move || {
        let Some(svc) = main_app_service() else {
            rt.log("shell: no SMAppService");
            return;
        };
        let result: Result<(), Retained<NSError>> =
            unsafe { if on { msg_send![&svc, registerAndReturnError: _] } else { msg_send![&svc, unregisterAndReturnError: _] } };
        if let Err(e) = result {
            rt.log(&format!("shell: login item {}: code {}", if on { "register" } else { "unregister" }, e.code()));
        }
        if let Some(now) = login_item() {
            rt.post(Event::Os(OsNews::LoginItem(now)));
        }
    });
}

/// The `.app` this process runs from.
pub fn bundle() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let app = exe.parent()?.parent()?.parent()?.to_path_buf();
    app.extension().is_some_and(|e| e == "app").then_some(app)
}

/// Quit and reopen: a detached shell (its own session, so it outlives this process) waits
/// a second and opens the bundle again with the relaunch's provenance; then this process
/// exits cleanly.
pub fn relaunch(rt: &Runtime, app: &tauri::AppHandle, why: RelaunchWhy) {
    let flag = match why {
        RelaunchWhy::Permission => "permission",
        RelaunchWhy::Update => "update",
    };
    let Some(bundle) = bundle() else {
        rt.log("shell: not running from an .app — no relaunch");
        return;
    };
    if quiet(rt) {
        rt.log(&format!("shell would relaunch ({flag})"));
        return;
    }
    use std::os::unix::process::CommandExt;
    let spawned = unsafe {
        std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg(r#"sleep 1; exec open "$0" --args "--relaunched=$1""#)
            .arg(&bundle)
            .arg(flag)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .pre_exec(|| {
                libc::setsid();
                Ok(())
            })
            .spawn()
    };
    match spawned {
        Ok(_) => {
            rt.log(&format!("shell relaunches ({flag})"));
            rt.exit(app);
        }
        Err(e) => rt.log(&format!("shell: relaunch failed: {e}")),
    }
}

/// The Mac account's name in macOS's own short, locale-aware form — “Bagrat” for
/// “Bagrat Aznauryan” (N29): the inviter's name on the join page, the owner's name in the
/// roster and `space.json`. Falls back to the full name, then the login.
pub fn short_user_name() -> String {
    use objc2_foundation::{NSPersonNameComponentsFormatter, NSPersonNameComponentsFormatterStyle, NSString};
    let full = full_user_name();
    let formatter = NSPersonNameComponentsFormatter::new();
    formatter.setStyle(NSPersonNameComponentsFormatterStyle::Short);
    let short = formatter
        .personNameComponentsFromString(&NSString::from_str(&full))
        .map(|c| formatter.stringFromPersonNameComponents(&c).to_string())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty());
    short.unwrap_or(full)
}

/// The hardware's own id (`IOPlatformUUID`), read from `ioreg` — only ever hashed into a
/// space's machine stamp (B144), never sent. `None` if it can't be read.
pub fn hardware_uuid() -> Option<String> {
    let out = std::process::Command::new("/usr/sbin/ioreg").args(["-rd1", "-c", "IOPlatformExpertDevice"]).stdin(std::process::Stdio::null()).output().ok()?;
    platform_uuid(&String::from_utf8_lossy(&out.stdout))
}

/// `"IOPlatformUUID" = "…"` out of `ioreg`'s answer.
pub fn platform_uuid(ioreg: &str) -> Option<String> {
    let line = ioreg.lines().find(|l| l.contains("\"IOPlatformUUID\""))?;
    let v = line.rsplit('=').next()?.trim().trim_matches('"');
    (v.len() == 36 && v.chars().all(|c| c.is_ascii_hexdigit() || c == '-')).then(|| v.to_string())
}

/// This Mac's sharing name (“MacBook Air”, System Settings › General › Sharing) — what an
/// own-Mac join names the device (§6.4). `None` if macOS has none.
pub fn sharing_name() -> Option<String> {
    use std::ffi::c_void;
    #[link(name = "SystemConfiguration", kind = "framework")]
    unsafe extern "C" {
        fn SCDynamicStoreCopyComputerName(store: *const c_void, encoding: *mut u32) -> *const c_void;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    unsafe extern "C" {
        fn CFStringGetCString(s: *const c_void, buf: *mut std::ffi::c_char, size: isize, encoding: u32) -> u8;
        fn CFRelease(p: *const c_void);
    }
    const UTF8: u32 = 0x0800_0100;
    // SAFETY: a copied CFString (released here) read into a buffer we own.
    unsafe {
        let s = SCDynamicStoreCopyComputerName(std::ptr::null(), std::ptr::null_mut());
        if s.is_null() {
            return None;
        }
        let mut buf = vec![0 as std::ffi::c_char; 512];
        let ok = CFStringGetCString(s, buf.as_mut_ptr(), buf.len() as isize, UTF8) != 0;
        CFRelease(s);
        ok.then(|| std::ffi::CStr::from_ptr(buf.as_ptr()).to_string_lossy().trim().to_string()).filter(|n| !n.is_empty())
    }
}

/// The Mac account's full name, else its login.
pub fn full_user_name() -> String {
    unsafe {
        let pw = libc::getpwuid(libc::getuid());
        if pw.is_null() {
            return "You".into();
        }
        let field = |p: *const libc::c_char| (!p.is_null()).then(|| std::ffi::CStr::from_ptr(p).to_string_lossy().into_owned());
        let gecos = field((*pw).pw_gecos).map(|g| g.split(',').next().unwrap_or_default().trim().to_string()).filter(|g| !g.is_empty());
        gecos.or_else(|| field((*pw).pw_name)).unwrap_or_else(|| "You".into())
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn the_platform_uuid_is_read_out_of_ioreg_and_nothing_else() {
        let ioreg = "+-o MacBookPro18,3  <class IOPlatformExpertDevice>\n    {\n      \"IOPlatformSerialNumber\" = \"C02XXXXXXX\"\n      \"IOPlatformUUID\" = \"1A2B3C4D-5E6F-7A8B-9C0D-1E2F3A4B5C6D\"\n    }\n";
        assert_eq!(platform_uuid(ioreg).as_deref(), Some("1A2B3C4D-5E6F-7A8B-9C0D-1E2F3A4B5C6D"));
        assert_eq!(platform_uuid("\"IOPlatformUUID\" = \"not-a-uuid\""), None);
        assert_eq!(platform_uuid(""), None);
    }

    #[test]
    fn only_an_enabled_login_item_reads_as_on() {
        assert!(super::starts_at_login(1), "enabled");
        assert!(!super::starts_at_login(2), "requires approval: switched off in System Settings");
        assert!(!super::starts_at_login(0) && !super::starts_at_login(3));
    }

    use super::*;

    #[test]
    fn the_token_page_carries_its_name_and_two_powers_encoded() {
        let u = token_page();
        assert!(u.starts_with("https://dash.cloudflare.com/?to=/:account/api-tokens&name=shroom&permissionGroupKeys="));
        assert!(u.ends_with("%5B%7B%22key%22%3A%22workers_scripts%22%2C%22type%22%3A%22edit%22%7D%2C%7B%22key%22%3A%22workers_r2%22%2C%22type%22%3A%22edit%22%7D%5D"));
    }

    #[test]
    fn pages() {
        assert_eq!(url(&PublicUrl::Signup), "https://dash.cloudflare.com/sign-up");
        assert_eq!(url(&PublicUrl::Billing { account_id: "abc123".into() }), "https://dash.cloudflare.com/abc123/r2/overview");
        assert_eq!(url(&PublicUrl::ReleaseNotes { version: "0.1.12".into() }), "https://shroom.am/whats-new/0.1.12");
    }
}

/// The pasteboard a harness run copies to: shroom's own, by name — a drive checks what
/// landed there and the person's clipboard is never touched.
pub const HARNESS_PASTEBOARD: &str = "am.shroom.harness";

/// Copy link (law 10: only ever from the click): the space's clean link — never a keyed
/// one — onto the general pasteboard. The copy itself is harmless, so quiet builds do it
/// too; a harness run (a drive) copies to its own named pasteboard instead.
pub fn copy_link(rt: &Runtime, rec: &shroom_core::world::RecId) {
    let Some(space) = rt.home.read_json::<shroom_core::world::SpaceIdentity>("space.json") else {
        rt.log("shell: copy with no space");
        return;
    };
    let link = format!("https://{}/{}", space.host, rec.0);
    rt.log(&format!("shell: copied the link to {}{}", super::short(&rec.0), if rt.harness { " (the harness's pasteboard)" } else { "" }));
    put_on_pasteboard(rt, link);
}

/// Copy thumbnail (N46; law 10: only ever from the click): the video's picture wrapped in its
/// clean link as HTML, and the link as text, so a paste where pictures go shows the picture
/// and anywhere else gets the link. The picture is the newest the page made for the title
/// (the page's status names it), else the video's first frame; never an image file on the
/// pasteboard (an editor would take the file and drop the link). Asked off the main thread.
pub fn copy_thumb(rt: &Runtime, rec: &shroom_core::world::RecId, title: &str) {
    let Some(space) = rt.home.read_json::<shroom_core::world::SpaceIdentity>("space.json") else {
        rt.log("shell: copy with no space");
        return;
    };
    let (host, id, title) = (space.host.clone(), rec.0.clone(), title.to_string());
    let status = super::posters::reachable(rt, &host, &format!("/_shroom/status/{id}"));
    let harness = rt.harness;
    rt.log(&format!("shell: copied the thumbnail of {}{}", super::short(&id), if harness { " (the harness's pasteboard)" } else { "" }));
    std::thread::spawn(move || {
        let newest = status
            .and_then(|url| super::posters::get(&url))
            .and_then(|body| serde_json::from_slice::<serde_json::Value>(&body).ok())
            .and_then(|v| v.get("thumb").and_then(|t| t.as_str()).map(String::from))
            .filter(|p| p.starts_with(&format!("/m/{id}/thumb-")) && !p.contains(['"', '<', '>', '&']));
        let link = format!("https://{host}/{id}");
        let html = thumb_html(&link, &format!("https://{host}{}", newest.unwrap_or_else(|| format!("/m/{id}/poster.jpg"))), &title);
        put_html_on_pasteboard(harness, html, link);
    });
}

/// The pasted picture: a link around one image, 600 wide, named by the title.
fn thumb_html(link: &str, src: &str, title: &str) -> String {
    let esc = |s: &str| s.replace('&', "&amp;").replace('"', "&quot;").replace('<', "&lt;").replace('>', "&gt;");
    format!(r#"<a href="{}"><img src="{}" width="600" alt="{}" style="max-width:100%"></a>"#, esc(link), esc(src), esc(title))
}

fn put_html_on_pasteboard(harness: bool, html: String, link: String) {
    super::observe::dispatch_main(move || {
        use objc2_app_kit::{NSPasteboard, NSPasteboardTypeHTML, NSPasteboardTypeString};
        let pb = if harness { NSPasteboard::pasteboardWithName(&objc2_foundation::NSString::from_str(HARNESS_PASTEBOARD)) } else { NSPasteboard::generalPasteboard() };
        pb.clearContents();
        // SAFETY: both are constants AppKit exports.
        pb.setString_forType(&objc2_foundation::NSString::from_str(&html), unsafe { NSPasteboardTypeHTML });
        pb.setString_forType(&objc2_foundation::NSString::from_str(&link), unsafe { NSPasteboardTypeString });
    });
}

/// A join link onto the pasteboard (law 10: only ever from its Copy link click), its text
/// straight from the stash (law 13) — never logged.
pub fn copy_invite(rt: &Runtime, id: shroom_core::world::SecretId) {
    let Some(link) = rt.stash.lock().unwrap().link(id).map(String::from) else {
        rt.log(&format!("shell: no join link {} to copy", id.0));
        return;
    };
    rt.log(&format!("shell: copied join link {}{}", id.0, if rt.harness { " (the harness's pasteboard)" } else { "" }));
    put_on_pasteboard(rt, link);
}

/// The general pasteboard — or, in a harness run (a drive), shroom's own named one.
pub fn put_on_pasteboard(rt: &Runtime, link: String) {
    let harness = rt.harness;
    super::observe::dispatch_main(move || {
        use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
        let pb = if harness { NSPasteboard::pasteboardWithName(&objc2_foundation::NSString::from_str(HARNESS_PASTEBOARD)) } else { NSPasteboard::generalPasteboard() };
        pb.clearContents();
        // SAFETY: NSPasteboardTypeString is a constant AppKit exports.
        pb.setString_forType(&objc2_foundation::NSString::from_str(&link), unsafe { NSPasteboardTypeString });
    });
}

/// Shows a take's folder in Finder (a saved take's “Show in Finder”).
pub fn reveal(rt: &Runtime, dir: &shroom_core::world::TakeDir) {
    let path = rt.home.takes().join(&dir.0);
    if quiet(rt) {
        rt.log(&format!("shell would reveal {}", path.display()));
        return;
    }
    let _ = std::process::Command::new("open").arg("-R").arg(path).stdin(std::process::Stdio::null()).spawn();
}

#[link(name = "IOKit", kind = "framework")]
unsafe extern "C" {
    fn IOPMAssertionCreateWithName(kind: *const std::ffi::c_void, level: u32, name: *const std::ffi::c_void, id: *mut u32) -> i32;
    fn IOPMAssertionRelease(id: u32) -> i32;
}

static POWER: std::sync::Mutex<Option<u32>> = std::sync::Mutex::new(None);

/// Holds off idle sleep while uploads drain — never lid sleep (N41): an assertion of type
/// `PreventUserIdleSystemSleep`, named so `pmset -g assertions` says whose it is.
pub fn power_assertion(rt: &Runtime, on: bool) {
    let mut held = POWER.lock().unwrap();
    match (on, *held) {
        (true, None) => {
            let kind = objc2_foundation::NSString::from_str("PreventUserIdleSystemSleep");
            let name = objc2_foundation::NSString::from_str("shroom is finishing an upload");
            let mut id = 0u32;
            // SAFETY: NSString is toll-free bridged to CFString; IOKit copies both strings.
            let rc = unsafe {
                IOPMAssertionCreateWithName(
                    objc2::rc::Retained::as_ptr(&kind) as *const _,
                    255, // kIOPMAssertionLevelOn
                    objc2::rc::Retained::as_ptr(&name) as *const _,
                    &mut id,
                )
            };
            if rc == 0 {
                *held = Some(id);
                rt.log("power: holding off idle sleep while uploads drain");
            } else {
                rt.log(&format!("power: the assertion was refused ({rc})"));
            }
        }
        (false, Some(id)) => {
            // SAFETY: id came from IOPMAssertionCreateWithName and is released once.
            unsafe { IOPMAssertionRelease(id) };
            *held = None;
            rt.log("power: idle sleep allowed again");
        }
        _ => {}
    }
}

/// This Mac's offset from UTC now, in minutes (the Library's dates are its calendar's).
/// `tzset` first, so a time zone changed while shroom runs is read afresh.
pub fn utc_offset_min() -> i32 {
    unsafe extern "C" {
        fn tzset();
    }
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0) as libc::time_t;
    let mut tm: libc::tm = unsafe { std::mem::zeroed() };
    // SAFETY: tzset re-reads the zone; localtime_r fills the caller's struct.
    unsafe {
        tzset();
        libc::localtime_r(&now, &mut tm);
    }
    (tm.tm_gmtoff / 60) as i32
}

/// The running macOS's major version (`kern.osproductversion`, e.g. "15.3" → 15).
pub fn macos_major() -> u32 {
    let mut buf = [0u8; 32];
    let mut len = buf.len();
    // SAFETY: the name is a C string; buf/len describe a writable buffer.
    let rc = unsafe { libc::sysctlbyname(c"kern.osproductversion".as_ptr(), buf.as_mut_ptr() as *mut _, &mut len, std::ptr::null_mut(), 0) };
    if rc != 0 {
        return 0;
    }
    let text = String::from_utf8_lossy(&buf[..len.saturating_sub(1)]).into_owned();
    text.split('.').next().and_then(|m| m.trim().parse().ok()).unwrap_or(0)
}
