//! Region 5's drives (dev builds only, `--home <tmp>` + `--cloud-fake <api> <r2>`): a
//! provisioned Mac — seeded into the throwaway home and the L1 fake's `found` space — walked
//! through the Library, Settings with the repair mood, and an update, like a person: the
//! same events the webviews and the tray send, against the real core, lanes and presenter.
//! Each writes a JSON report with the observed state-id sequence and its checks, then
//! exits. Nothing here opens a browser or System Settings (the shell is quiet), and the
//! update drive swaps only its `--update-target` throwaway.

use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use shroom_core::event::{Event, Field, Key, Surface};
use shroom_core::scene::{Action, Face, Scene, SettingsPane, Style, Window};
use shroom_core::world::{RecId, Role, SettingsTab, SpaceIdentity, WindowKind};
use shroom_storage::s3::Client;
use tauri::AppHandle;

use crate::front::{self, Front};
use crate::harness::{app_active, click, files, leaks, policy, snapshot, write_json};
use crate::runtime::presenter::{LIBRARY, SETTINGS};
use crate::runtime::{Runtime, home};

/// The fake's `found` space (cloudfake's `--state found`): its account, name and bucket.
const ACCOUNT: &str = "acc0000000000000000000000000001";
const HOST: &str = "v.myshroom.workers.dev";

/// How long what a drive reads has to hold still once the keyboard has left shroom — the
/// person turned to another app, or hid shroom with ⌘H — before the drive goes back to the
/// mushroom. macOS says who has the keyboard before it has finished handing it over: the
/// other app's window comes forward a moment later, and a reading taken before that would
/// say nothing covers shroom's window.
///
/// Measured 2026-10-13 in clean clones (macOS 15.7.7 and 26.6.2; TextEdit in front, and an
/// app in a full-screen Space; quiet, and with eight spinning processes beside the drive),
/// 216 hand-overs: what is read lands, and has stopped changing, within 48 ms of the
/// keyboard leaving (once 451 ms, in a clone still coming down from load) — and in the 52
/// that were then watched for three seconds, nothing changed again. Half a second is ten
/// times that, and still no slower than a quick hand: nobody is back at the mushroom sooner.
///
/// It was three seconds at first, for two failures seen that day and not caught again: a
/// popover dismissed 3 ms after a click opened it (once in thirteen runs), and — another
/// app in a full-screen Space — the door failing 150 ms after ⌘H where 3 s after worked.
/// Both were seen with the popover as it then was: its page closed it on losing the
/// keyboard and said so late (nothing else could have sent that dismissal), and showing it
/// un-hid all of shroom. Neither is so now (DEVIATIONS #39, DECISIONS B112), and neither
/// failure came back with no hold at all: back at the mushroom within 80 ms of the keyboard
/// leaving, the Library and Settings drives passed 66 runs of 66 in those clones — 24 of
/// them over the full-screen app.
const HAND: Duration = Duration::from_millis(500);

/// What a drive can read once the keyboard has left shroom.
#[derive(Debug, Clone, Default)]
struct Away {
    /// Where the window stands.
    front: Front,
    /// shroom is hidden (⌘H).
    hidden: bool,
    /// The app macOS sends the keyboard to.
    keyboard: Option<String>,
}

impl Away {
    fn says(&self) -> String {
        format!("{} has the keyboard; hidden {}; {}", self.keyboard.as_deref().unwrap_or("nothing"), self.hidden, self.front.says())
    }
}

/// A reading that has to hold still: landed, and in the same words, for as long as asked.
#[derive(Default)]
struct Still(Option<(String, Instant)>);

impl Still {
    /// Takes the reading made at `at`; whether it has now landed and stayed the same for
    /// `hand`. A reading that hasn't landed, or that changed, starts the wait again.
    fn held(&mut self, landed: bool, says: &str, at: Instant, hand: Duration) -> bool {
        if !landed {
            self.0 = None;
            return false;
        }
        match &self.0 {
            Some((was, since)) if was == says => at.duration_since(*since) >= hand,
            _ => {
                self.0 = Some((says.to_string(), at));
                false
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region5 {
    Library,
    Settings,
    Update,
    /// The update drive's second half, in the copy the swap reopened.
    UpdateRelaunched,
    /// Uninstall shroom… from a throwaway copy (B135): its runner checks what's left.
    Uninstall,
    /// The editor (N63): `--drive editor <out> <dir>`, the runner's folder holding the test
    /// video it made (`preview.mp4`, `poster.jpg`) and taking what it's asked for there.
    Editor,
    /// The popover's AI banner, turned on (N66), paced for its video.
    Ai,
}

impl Region5 {
    pub fn parse(s: &str) -> Option<Region5> {
        Some(match s {
            "library" => Region5::Library,
            "settings" => Region5::Settings,
            "update" => Region5::Update,
            "update-relaunched" => Region5::UpdateRelaunched,
            "uninstall" => Region5::Uninstall,
            "editor" => Region5::Editor,
            "ai" => Region5::Ai,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Region5::Library => "library",
            Region5::Settings => "settings",
            Region5::Update => "update",
            Region5::UpdateRelaunched => "update-relaunched",
            Region5::Uninstall => "uninstall",
            Region5::Editor => "editor",
            Region5::Ai => "ai",
        }
    }

    /// What the trace reads from each Scene: the window's ids, or (updates) every id.
    pub fn probe(self) -> fn(&Scene) -> String {
        match self {
            Region5::Library => |s| window_ids(s.library.as_ref(), s.library.as_ref().and_then(|l| l.body.sheet.as_ref())),
            Region5::Settings | Region5::Uninstall => |s| window_ids(s.settings.as_ref(), s.settings.as_ref().and_then(|w| w.body.sheet.as_ref())),
            Region5::Update | Region5::UpdateRelaunched | Region5::Ai => |s| s.state_ids().join(" | "),
            Region5::Editor => |s| format!("editor {} · library {}", window_ids(s.editor.as_ref(), None), window_ids(s.library.as_ref(), None)),
        }
    }
}

fn window_ids<B>(w: Option<&Window<B>>, sheet: Option<&shroom_core::scene::Sheet>) -> String {
    let Some(w) = w else { return "closed".into() };
    let mut ids = vec![w.id];
    ids.extend(&w.faces);
    if let Some(s) = sheet {
        ids.push(s.id);
        ids.extend(&s.faces);
    }
    ids.join(" | ")
}

/// A provisioned token-holding Mac in the throwaway home, matching the fake's `found`
/// space: `space.json`, a token (random, token-shaped — the fake takes any), and this Mac's
/// permissions chapter done. Idempotent: a home that has a space keeps it.
pub fn seed_provisioned(flags: &crate::runtime::DevFlags) -> Result<(), String> {
    let root = flags.home.clone().ok_or("no --home")?;
    let h = home::Home::at(root);
    h.ensure().map_err(|e| e.to_string())?;
    if h.path("space.json").exists() {
        return Ok(());
    }
    let mut raw = [0u8; 30];
    getrandom::fill(&mut raw).map_err(|e| e.to_string())?;
    let alphabet = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let token: String = "cfat_".chars().chain(raw.iter().map(|b| alphabet[*b as usize % alphabet.len()] as char)).collect();
    let space = SpaceIdentity {
        account_id: ACCOUNT.into(),
        label: "v".into(),
        subdomain: "myshroom".into(),
        host: HOST.into(),
        bucket: shroom_cloud::wire::BUCKET.into(),
        role: Role::TokenHolder,
        member_id: "m1".into(),
        owner_name: "Drive".into(),
        account_name: None,
        foreign_v: false,
    };
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
    h.write_json("token.json", &serde_json::json!({ "v": 1, "token": token, "account_id": ACCOUNT, "several": false })).map_err(|e| e.to_string())?;
    h.write_json("space.json", &space).map_err(|e| e.to_string())?;
    h.write_json("primed.json", &crate::runtime::home_files::Primed { v: 1, at: now }).map_err(|e| e.to_string())?;
    Ok(())
}

#[derive(Serialize, Default)]
struct Report {
    drive: String,
    /// Every state id sequence the trace saw, in order (a repeat isn't recorded).
    states: Vec<String>,
    /// What the drive waited for, and when (ms since start).
    steps: Vec<(String, u128)>,
    /// Each check: (what, passed, what was seen).
    checks: Vec<(String, bool, String)>,
    /// Effects the quiet shell logged instead of doing.
    would: Vec<String>,
    files: Vec<String>,
    /// Files under the home but token.json holding the token's text — must be empty.
    token_leaks: Vec<String>,
    /// What a drive noted along the way (ids the runner reads back; never a secret).
    #[serde(skip_serializing_if = "serde_json::Map::is_empty")]
    facts: serde_json::Map<String, serde_json::Value>,
    /// What this run couldn't look at, and why (a locked screen keeps every app from the
    /// keyboard): not failures — but not proven by this run either.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    unseen: Vec<String>,
    ok: bool,
    error: Option<String>,
}

struct Drive {
    app: AppHandle,
    rt: Runtime,
    t0: Instant,
    report: Arc<Mutex<Report>>,
    /// Secrets the drive saw the app make (a born link's code): checked like the seed's.
    seen: Arc<Mutex<Vec<String>>>,
}

impl Drive {
    fn step(&self, what: &str) {
        self.report.lock().unwrap().steps.push((what.to_string(), self.t0.elapsed().as_millis()));
    }

    fn check(&self, what: &str, ok: bool, seen: impl Into<String>) {
        self.report.lock().unwrap().checks.push((what.to_string(), ok, seen.into()));
    }

    fn fact_like(&self, key: &str, value: impl Serialize) {
        self.report.lock().unwrap().facts.insert(key.into(), serde_json::to_value(value).unwrap_or_default());
    }

    fn wait(&self, what: &str, secs: u64, pred: impl Fn(&Scene) -> bool) -> Result<Scene, String> {
        let until = Instant::now() + Duration::from_secs(secs);
        while Instant::now() < until {
            if let Some(s) = snapshot(&self.rt) {
                if pred(&s) {
                    self.step(what);
                    return Ok(s);
                }
            }
            std::thread::sleep(Duration::from_millis(25));
        }
        Err(format!("timed out waiting for {what}; at {:?}", snapshot(&self.rt).map(|s| (self.rt.trace.lock().unwrap().as_ref().map(|t| (t.probe)(&s))))))
    }

    fn act(&self, surface: Surface, action: Action) {
        self.rt.post(Event::Act { surface, action, rev: 0 });
    }

    fn key(&self, surface: Surface, key: Key) {
        self.rt.post(Event::Key { surface, key });
    }

    /// The window came to the front again (what the presenter posts on focus).
    fn focus(&self, kind: WindowKind) {
        let rt = self.rt.clone();
        let _ = self.app.run_on_main_thread(move || {
            if let Some(mtm) = objc2_foundation::MainThreadMarker::new() {
                let (quick, _) = crate::runtime::probe::quick(mtm, &rt.home.root);
                rt.post(Event::WindowFocused(kind, quick));
            }
        });
    }

    fn on_main<T: Send + 'static>(&self, f: impl FnOnce() -> T + Send + 'static) -> Option<T> {
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = self.app.run_on_main_thread(move || {
            let _ = tx.send(f());
        });
        rx.recv_timeout(Duration::from_secs(5)).ok()
    }

    fn dock(&self) -> String {
        self.on_main(policy).unwrap_or_default()
    }

    /// Where one of shroom's windows stands: in front or not, with the keyboard or not.
    fn front(&self, label: &'static str) -> Front {
        let app = self.app.clone();
        self.on_main(move || front::front(&app, label)).unwrap_or_default()
    }

    /// Waits a few seconds for `ok` to hold of the window's standing; what it saw last.
    fn front_when(&self, label: &'static str, ok: impl Fn(&Front) -> bool) -> Front {
        let until = Instant::now() + Duration::from_secs(4);
        loop {
            let f = self.front(label);
            if ok(&f) || Instant::now() > until {
                return f;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    /// The keyboard has left shroom, and the drive is about to go back to the mushroom — at
    /// a person's pace ([`HAND`]): it waits until `landed` holds of what it can read (who
    /// macOS sends the keyboard to, where `label`'s window stands, whether shroom is hidden)
    /// and none of it has changed for that long. Whether that came (within fifteen
    /// seconds), and what it read last.
    fn hand_over(&self, label: &'static str, landed: impl Fn(&Away) -> bool) -> (bool, Away) {
        let t0 = Instant::now();
        let until = t0 + HAND + Duration::from_secs(12);
        let mut still = Still::default();
        // When it landed, and when what was read last changed (ms since the keyboard left).
        let (mut landed_at, mut changed_at, mut was) = (None, 0, String::new());
        loop {
            let app = self.app.clone();
            let away = self.on_main(move || Away { front: front::front(&app, label), hidden: front::hidden(), keyboard: front::frontmost_id() }).unwrap_or_default();
            let (there, says, ms) = (landed(&away), away.says(), t0.elapsed().as_millis() as u64);
            if there && landed_at.is_none() {
                landed_at = Some(ms);
            }
            if says != was {
                (changed_at, was) = (ms, says.clone());
            }
            let held = still.held(there, &says, Instant::now(), HAND);
            if held || Instant::now() > until {
                // Kept beside the checks: a hand-over that was still moving shows here.
                let mut r = self.report.lock().unwrap();
                if let Some(list) = r.facts.entry("hand_overs").or_insert_with(|| serde_json::json!([])).as_array_mut() {
                    list.push(serde_json::json!({ "window": label, "held": held, "landed_ms": landed_at, "last_change_ms": changed_at, "went_on_ms": ms }));
                }
                return (held, away);
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }

    /// Whether anything can take the keyboard on this Mac right now. Behind a locked screen
    /// nothing can — the drive notes what it couldn't look at, and looks at the rest.
    fn hands(&self, what: &str) -> bool {
        match self.on_main(front::no_keyboard).flatten() {
            None => true,
            Some(why) => {
                self.report.lock().unwrap().unseen.push(format!("{what} — {why}"));
                false
            }
        }
    }

    /// The app macOS sends the keyboard to.
    fn frontmost(&self) -> Option<String> {
        self.on_main(front::frontmost_id).flatten()
    }

    /// The person turns to another app: macOS sends the keyboard there, not to shroom.
    /// Which app it was.
    fn elsewhere(&self) -> Result<String, String> {
        let other = self.on_main(front::turn_to_another_app).flatten().ok_or("no other app to turn to")?;
        let there = || poll(3, || self.frontmost().filter(|f| *f == other));
        if there().is_none() {
            // Asked through LaunchServices instead, as a click on the app would.
            let _ = std::process::Command::new("open").args(["-b", &other]).status();
            there().ok_or(format!("{other} never came to the front"))?;
        }
        self.step(&format!("another app in front: {other}"));
        Ok(other)
    }

    /// One key, typed the way a keyboard's arrives: at whichever window holds the keyboard.
    fn press(&self, c: char) -> Result<(), String> {
        self.on_main(move || front::press(c)).ok_or("the main thread didn't answer")?
    }

    /// ⌘ and a key.
    fn command(&self, c: char) -> Result<(), String> {
        self.on_main(move || front::command(c)).ok_or("the main thread didn't answer")?
    }

    /// Settles a moment, then reads the Scene (for “nothing changed” checks).
    fn after(&self, ms: u64) -> Scene {
        std::thread::sleep(Duration::from_millis(ms));
        snapshot(&self.rt).expect("a Scene")
    }

    /// How many probe passes the wizard lane has answered (the repair mood's re-probes).
    fn sweeps(&self) -> usize {
        std::fs::read_to_string(self.rt.home.root.join("logs/run.log")).unwrap_or_default().lines().filter(|l| l.contains("wizard sweep:")).count()
    }

    /// A dashboard knob on the fake (`POST /_fake/<command>`).
    fn fake(&self, command: &str) -> Result<(), String> {
        let (api, _) = self.rt.cloud_fake.clone().ok_or("no fake")?;
        let root = api.trim_end_matches("/client/v4");
        let url = format!("{root}/_fake/{}", command.replace(' ', "%20"));
        ureq::post(&url).send_empty().map_err(|e| format!("fake {command}: {e}"))?;
        self.step(&format!("the fake: {command}"));
        Ok(())
    }

    /// The space's bucket, through the token's own S3 credentials (as the app reaches it).
    fn bucket(&self) -> Result<Client, String> {
        let token = self.rt.home.token().ok_or("no token")?;
        let cloud = self.rt.cloud(&token);
        let token_id = cloud.verify(ACCOUNT).map_err(|e| e.to_string())?.id;
        Ok(shroom_cloud::wire::bucket_client(&cloud, ACCOUNT, shroom_cloud::wire::BUCKET, &token_id))
    }

    /// A real drag with the pointer, by the drive's runner: a click on `focus` (the window
    /// takes the pointer), then `from` → `to`, held down all the way.
    fn runner_drag(&self, runner: &Runner, focus: (f64, f64), from: (f64, f64), to: (f64, f64)) -> Result<(), String> {
        runner.ask(self, serde_json::json!({ "do": "drag", "focus": [focus.0, focus.1], "from": [from.0, from.1], "to": [to.0, to.1] }))
    }

    /// Opens a window the way a person does: the mushroom, then the popover's footer.
    fn open(&self, door: Action) -> Result<(), String> {
        click(&self.app, &self.rt);
        self.wait("config", 10, |s| s.popover.as_ref().is_some_and(|p| p.id == "config"))?;
        self.act(Surface::Popover, door);
        Ok(())
    }
}

pub fn drive_region5(app: AppHandle, rt: Runtime, which: Region5, out: PathBuf) {
    let d = Drive { app: app.clone(), rt: rt.clone(), t0: Instant::now(), report: Arc::new(Mutex::new(Report { drive: which.name().into(), ..Report::default() })), seen: Arc::default() };
    // The update's quit ends this process mid-drive (so does the uninstall's): the report is
    // written on the way out.
    if which == Region5::Uninstall {
        let (rt2, report, out2) = (rt.clone(), d.report.clone(), out.clone());
        *rt.on_exit.lock().unwrap() = Some(Box::new(move || {
            let mut r = report.lock().unwrap();
            // The log goes with the home: what the uninstall said is kept here, on the way out.
            let said: Vec<String> = std::fs::read_to_string(rt2.home.root.join("logs/run.log")).unwrap_or_default().lines().filter(|l| l.contains("uninstall")).map(String::from).collect();
            let handed = said.iter().any(|l| l.contains("uninstall: the helper waits"));
            r.checks.push(("the helper took over, and shroom quits".into(), handed, said.join(" / ")));
            finish(&rt2, &mut r);
            let _ = write_json(&out2, &*r);
        }));
    }
    if which == Region5::Update {
        let (rt2, report, out2) = (rt.clone(), d.report.clone(), out.clone());
        *rt.on_exit.lock().unwrap() = Some(Box::new(move || {
            let mut r = report.lock().unwrap();
            finish(&rt2, &mut r);
            let _ = write_json(&out2, &*r);
        }));
        let second = PathBuf::from(format!("{}.relaunched.json", out.display()));
        *rt.relaunch_extra.lock().unwrap() = vec!["--drive".into(), "update-relaunched".into(), second.display().to_string()];
    }
    // The Settings drive ends on ⌘Q with no window open (one with Settings in front closes
    // it, B145): the core quits shroom, and the report is written on the way out — with
    // whether the quit was the one the key asked for.
    if which == Region5::Settings {
        let (rt2, report, out2) = (rt.clone(), d.report.clone(), out.clone());
        *rt.on_exit.lock().unwrap() = Some(Box::new(move || {
            let mut r = report.lock().unwrap();
            let pressed = r.steps.iter().any(|(s, _)| s == CMD_Q);
            let asked = std::fs::read_to_string(rt2.home.root.join("logs/run.log")).unwrap_or_default().contains("quit asked: CmdQ");
            r.checks.push(("⌘Q asks the core, and with nothing recording or uploading shroom quits".into(), pressed && asked, format!("pressed {pressed}, the core was asked {asked}")));
            finish(&rt2, &mut r);
            let _ = write_json(&out2, &*r);
        }));
    }
    let result = match which {
        Region5::Library => library(&d),
        Region5::Settings => settings(&d),
        Region5::Update => update(&d),
        Region5::UpdateRelaunched => update_relaunched(&d),
        Region5::Uninstall => uninstall(&d),
        Region5::Editor => editor(&d),
        Region5::Ai => ai(&d),
    };
    if matches!(which, Region5::Update | Region5::Uninstall) && result.is_ok() {
        // The quit for the swap (or the uninstall) is under way: the exit hook writes the report.
        std::thread::sleep(Duration::from_secs(30));
        d.report.lock().unwrap().error = Some(format!("the app didn't quit for the {} within 30 s", if which == Region5::Update { "swap" } else { "uninstall" }));
    }
    if let Err(e) = result {
        d.report.lock().unwrap().error = Some(e);
    }
    let _ = rt.on_exit.lock().unwrap().take();
    let code = {
        let mut r = d.report.lock().unwrap();
        finish(&rt, &mut r);
        write_json(&out, &*r)
    };
    rt.ending_ours.store(true, std::sync::atomic::Ordering::SeqCst);
    app.exit(code);
}

fn finish(rt: &Runtime, r: &mut Report) {
    r.states = rt.trace.lock().unwrap().as_ref().map(|t| t.ids.clone()).unwrap_or_default();
    r.would = std::fs::read_to_string(rt.home.root.join("logs/run.log"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once("would ").map(|(_, w)| w.to_string()))
        .collect();
    let mut f: Vec<String> = files(&rt.home.root).into_iter().map(|p| p.strip_prefix(&rt.home.root).unwrap_or(&p).display().to_string()).collect();
    f.sort();
    r.files = f;
    r.token_leaks = match home::read_json::<serde_json::Value>(&rt.home.path("token.json")).and_then(|v| v["token"].as_str().map(String::from)) {
        Some(t) => leaks(&rt.home.root, &t),
        None => vec![],
    };
    r.ok = r.error.is_none() && r.token_leaks.is_empty() && r.checks.iter().all(|(_, ok, _)| *ok);
}

// ── the Library ──

/// Three videos in the space, as Macs leave them: a live record and its bytes.
const SEEDED: [(&str, &str, &str, f64); 3] = [
    ("Ldrive0000000502", "Where the 502s come from", "2026-08-21T12:00:00Z", 272.0),
    ("Ldrive0000000019", "Untitled — Aug 19", "2026-08-19T09:30:00Z", 48.0),
    ("Ldrive0000000003", "Onboarding flow, take 2", "2026-08-03T16:10:00Z", 730.0),
];

fn seed_videos(s3: &Client) -> Result<(), String> {
    use shroom_upload::{keys, meta};
    for (id, title, at, secs) in SEEDED {
        let mut m = meta::Meta::uploading(id, title, at);
        m.state = meta::State::Live;
        m.published_at = Some(at.into());
        m.duration_sec = Some(secs);
        m.owner = Some("m1".into());
        m.media = Some(meta::Media { hls: Some(keys::master(id)), mp4: None, poster: None, teaser: None, width: 1920, height: 1080 });
        s3.put(&keys::master(id), b"#EXTM3U\n", "application/vnd.apple.mpegurl").map_err(|e| e.to_string())?;
        s3.put(&format!("{}v0/seg0.m4s", keys::prefix(id)), &[0u8; 512], "video/iso.segment").map_err(|e| e.to_string())?;
        s3.put(&keys::meta(id), &serde_json::to_vec(&m).map_err(|e| e.to_string())?, "application/json").map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn titles(s: &Scene) -> Vec<String> {
    s.library.as_ref().map(|l| l.body.rows.iter().filter_map(|r| r.title.clone()).collect()).unwrap_or_default()
}

fn rec_of(s: &Scene, title: &str) -> Option<RecId> {
    s.library.as_ref()?.body.rows.iter().find(|r| r.title.as_deref() == Some(title)).map(|r| RecId(r.key.clone()))
}

/// What a harness run's Copy link put on its own pasteboard (never the person's clipboard).
fn pasteboard() -> Option<String> {
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
    let pb = NSPasteboard::pasteboardWithName(&objc2_foundation::NSString::from_str(crate::runtime::os::HARNESS_PASTEBOARD));
    // SAFETY: NSPasteboardTypeString is a constant AppKit exports.
    pb.stringForType(unsafe { NSPasteboardTypeString }).map(|s| s.to_string())
}

/// Empties shroom's own harness pasteboard (never the person's), so a drive reads only
/// what this run copied.
fn clear_harness_pasteboard() {
    let pb = objc2_app_kit::NSPasteboard::pasteboardWithName(&objc2_foundation::NSString::from_str(crate::runtime::os::HARNESS_PASTEBOARD));
    pb.clearContents();
}

/// The person's own clipboard's change count — read without reading what's on it (no
/// paste prompt); a drive proves it never moved.
fn clipboard_count() -> isize {
    objc2_app_kit::NSPasteboard::generalPasteboard().changeCount()
}

fn library(d: &Drive) -> Result<(), String> {
    d.wait("ready", 20, |s| s.tray.click == shroom_core::scene::Click::OpenPopover)?;
    let s3 = d.bucket()?;
    seed_videos(&s3)?;
    d.step("seeded three videos into the fake's bucket");

    // The person is working in another app when they ask for the Library.
    let hands = d.hands("the Library over another app: in front with the keyboard, typing, ⌘A ⌘Z ⌘W, back through the popover's door, the keyboard handed back");
    let other = if hands { Some(d.elsewhere()?) } else { None };
    // Open: the cache at once (empty the first time), then the refresh slides them in.
    d.open(Action::OpenLibrary)?;
    let s = d.wait("the space's videos", 20, |s| titles(s).len() == 3)?;
    // The empty cache never claimed the space had no videos before the bucket answered.
    let seen = d.rt.trace.lock().unwrap().as_ref().map(|t| t.ids.clone()).unwrap_or_default();
    d.check("an empty cache never says “No videos yet”", !seen.iter().any(|id| id.contains("library-empty")), seen.join(" → "));
    let want: Vec<String> = SEEDED.iter().map(|(_, t, _, _)| t.to_string()).collect();
    d.check("rows newest first", titles(&s) == want, format!("{:?}", titles(&s)));
    let lib = s.library.as_ref().expect("the library");
    d.check("the footer counts them", lib.body.footer.as_deref() == Some("3 videos"), format!("{:?}", lib.body.footer));
    d.check("asked: it takes the keyboard", lib.activates, lib.activates.to_string());
    let row = &lib.body.rows[0];
    d.check("a row reads date · length", row.meta == "Aug 21 · 4:32", row.meta.clone());
    d.check("silence means live", row.state.is_none(), format!("{:?}", row.state));
    // The Library is a window, and brings the Dock icon while it is open (law 6, SM:133).
    std::thread::sleep(Duration::from_millis(400));
    let dock = d.dock();
    d.check("law 6: the Dock icon with the Library open", dock.starts_with("regular"), dock);
    // It opens like any window: in front of the app the person was in, with the keyboard.
    if let Some(other) = &other {
        let f = d.front_when(LIBRARY, Front::has_keyboard);
        d.check("asked for over another app, the Library is in front with the keyboard", f.has_keyboard(), format!("{other} was in front; {}", f.says()));
        library_keys(d)?;
    }

    // Search: local and instant, over titles.
    d.rt.field(Field::LibrarySearch, "502".into());
    d.wait("search 502", 5, |s| titles(s) == ["Where the 502s come from"])?;
    d.rt.field(Field::LibrarySearch, "zzz".into());
    let s = d.wait("no match", 5, |s| s.library.as_ref().is_some_and(|l| l.faces.contains(&"library-no-match")))?;
    d.check("no match says so", s.library.as_ref().and_then(|l| l.body.empty.clone()).is_some_and(|e| e.contains("zzz")), format!("{:?}", s.library.as_ref().and_then(|l| l.body.empty.clone())));
    d.rt.field(Field::LibrarySearch, String::new());
    d.wait("search cleared", 5, |s| titles(s).len() == 3)?;

    // Rename in place: click the title, type, Return (the title field's own Return).
    let s = snapshot(&d.rt).ok_or("no scene")?;
    let r19 = rec_of(&s, "Untitled — Aug 19").ok_or("no Aug 19 row")?;
    d.act(Surface::Library, Action::StartRename(r19.clone()));
    d.wait("renaming", 5, |s| s.library.as_ref().is_some_and(|l| l.body.rows.iter().any(|r| r.rename.is_some())))?;
    d.rt.field(Field::Title, "Standup  notes ".into());
    d.after(150);
    d.key(Surface::Library, Key::Return);
    d.wait("renamed", 15, |s| rec_of(s, "Standup notes").as_ref() == Some(&r19))?;
    let landed = poll(10, || {
        let m = s3.get(&shroom_upload::keys::meta(&r19.0)).ok().flatten()?;
        let v: serde_json::Value = serde_json::from_slice(&m).ok()?;
        (v["title"] == "Standup notes").then_some(())
    });
    d.check("the record's title changed in the bucket", landed.is_some(), "meta title");

    // Copy link: the clean link, on the harness's own pasteboard — the person's clipboard
    // is never touched by a drive.
    let r502 = rec_of(&s, "Where the 502s come from").ok_or("no 502 row")?;
    let clip_before = d.on_main(clipboard_count);
    d.act(Surface::Library, Action::CopyRow(r502.clone()));
    let s = d.wait("Copied", 5, |s| {
        s.library.as_ref().is_some_and(|l| l.body.rows.iter().any(|r| r.key == r502.0 && r.buttons.iter().any(|b| b.label.contains("Copied"))))
    })?;
    let _ = s;
    let link = poll(3, || d.on_main(pasteboard).flatten().filter(|t| t.contains(&r502.0)));
    let want_link = format!("https://{HOST}/{}", r502.0);
    d.check("law 10: the clean link was copied", link.as_deref() == Some(want_link.as_str()), format!("{link:?}"));
    let clip_after = d.on_main(clipboard_count);
    d.check("the person's clipboard is untouched", clip_before.is_some() && clip_before == clip_after, format!("change count {clip_before:?} → {clip_after:?}"));
    d.wait("Copied reverts", 5, |s| {
        s.library.as_ref().is_some_and(|l| l.body.rows.iter().any(|r| r.key == r502.0 && !r.buttons.iter().any(|b| b.label.contains("Copied"))))
    })?;

    // Delete — the sheet (law 9): no Return default, Esc is Cancel, the red-outlined
    // answer rightmost.
    d.act(Surface::Library, Action::DeleteRow(r502.clone()));
    let s = d.wait("the delete sheet", 5, |s| s.library.as_ref().is_some_and(|l| l.body.sheet.is_some()))?;
    let sheet = s.library.as_ref().and_then(|l| l.body.sheet.clone()).expect("a sheet");
    d.check("law 9: no Return default", s.ret(Surface::Library).is_none(), format!("{:?}", s.ret(Surface::Library)));
    d.check("law 9: Esc is Cancel", s.esc(Surface::Library) == Some(Action::Back), format!("{:?}", s.esc(Surface::Library)));
    let last = sheet.buttons.last();
    d.check(
        "law 9: the answer is red-outlined and rightmost",
        last.is_some_and(|b| b.action == Action::ConfirmDelete && b.style == Style::RedOutline),
        format!("{:?}", sheet.buttons.iter().map(|b| (&b.label, b.style)).collect::<Vec<_>>()),
    );
    d.check("the sheet asks about this video", sheet.title.contains("Where the 502s come from"), sheet.title.clone());
    d.key(Surface::Library, Key::Return);
    let s = d.after(300);
    d.check("Return does nothing on the sheet", s.library.as_ref().is_some_and(|l| l.body.sheet.is_some()), "sheet still up");
    d.key(Surface::Library, Key::Esc);
    d.wait("Esc backs out", 5, |s| s.library.as_ref().is_some_and(|l| l.body.sheet.is_none()))?;
    d.act(Surface::Library, Action::DeleteRow(r502.clone()));
    d.wait("the delete sheet again", 5, |s| s.library.as_ref().is_some_and(|l| l.body.sheet.is_some()))?;
    d.act(Surface::Library, Action::ConfirmDelete);
    d.wait("the row is gone", 15, |s| rec_of(s, "Where the 502s come from").is_none() && s.library.as_ref().is_some_and(|l| l.body.sheet.is_none()))?;
    let tomb = poll(10, || {
        let m = s3.get(&shroom_upload::keys::meta(&r502.0)).ok().flatten()?;
        let v: serde_json::Value = serde_json::from_slice(&m).ok()?;
        (v["state"] == "deleted").then_some(())
    });
    d.check("the tombstone is in the bucket", tomb.is_some(), "meta state deleted");
    // The bytes go after the tombstone (`shroom_upload::library::delete`), so they're waited
    // for the same way: a loaded Mac can still be removing them when the tombstone shows.
    let left = || s3.list(&shroom_upload::keys::prefix(&r502.0)).map(|v| v.len()).unwrap_or(usize::MAX);
    let bytes = if poll(10, || (left() == 0).then_some(())).is_some() { 0 } else { left() };
    d.check("its bytes are gone", bytes == 0, format!("{bytes} objects under rec/"));
    let others = s3.list(&shroom_upload::keys::prefix(SEEDED[2].0)).map(|v| v.len()).unwrap_or(0);
    d.check("another video's bytes stay", others == 2, format!("{others} objects"));

    // The cache: mode 600, without the deleted video. The row left the Scene the moment the
    // delete was asked for; the cache is rewritten when the delete has finished — after the
    // tombstone and the bytes, which is all the drive has waited for so far. On a busy disk
    // that write lands a moment later (a clean clone's first minutes: 155 ms against 30), so
    // the drive waits for it as it waits for the tombstone.
    use std::os::unix::fs::PermissionsExt;
    let forgot = poll(10, || {
        let cache = std::fs::read_to_string(d.rt.home.path("library.json")).ok()?;
        (!cache.contains(&r502.0) && cache.contains("Standup notes")).then_some(())
    });
    d.check("the cache forgot the deleted video", forgot.is_some(), "library.json");
    let mode = std::fs::metadata(d.rt.home.path("library.json")).map(|m| format!("{:o}", m.permissions().mode() & 0o777)).unwrap_or_default();
    d.check("library.json is mode 600", mode == "600", mode);

    // Opened again from Finder or Spotlight with the Library open: a click on the mushroom
    // (N41) — config, the Dock icon still there, the Library where it was.
    launched_again(d, "the Library", |s| s.library.is_some())?;

    if hands {
        // Beside the Dock and ⌘-Tab, the popover's door is a way back to it.
        let other = back_through_the_door(d, LIBRARY, Action::OpenLibrary, "the Library")?;
        // ⌘W closes it, as ✕ does — the core answers the close — and with nothing of
        // shroom's left on screen, the keyboard goes back to the app the person was in.
        closes_with_cmd_w(d, LIBRARY, "the Library", &other, |s| s.library.is_none())?;
    } else {
        d.rt.post(Event::WindowClosed(WindowKind::Library));
        d.wait("closed", 5, |s| s.library.is_none())?;
    }
    // The last window closed: the Dock icon goes with it (law 6).
    std::thread::sleep(Duration::from_millis(400));
    let dock = d.dock();
    d.check("law 6: no Dock icon after close", dock.starts_with("accessory"), dock);
    Ok(())
}

/// shroom is opened again while it runs — Finder, Spotlight, `open` — with a window of its
/// own already open: a launch you did is a click on the mushroom (N41). The reopen event
/// macOS sends then is sent to this process itself (`front::open_again`).
fn launched_again(d: &Drive, name: &str, still_open: impl Fn(&Scene) -> bool) -> Result<(), String> {
    let opened = front::open_again();
    let s = d.wait("opened again: config", 8, |s| s.popover.as_ref().is_some_and(|p| p.id == "config"));
    d.check(&format!("opened again with {name} open, shroom shows config (N41)"), opened.is_ok() && s.as_ref().is_ok_and(|s| still_open(s) && s.popover.as_ref().is_some_and(|p| p.activates)), format!("sent {opened:?}; {:?}", s.as_ref().map(|s| s.state_ids())));
    let dock = d.dock();
    d.check(&format!("law 6: opened again with {name} open, the Dock icon stays"), dock.starts_with("regular"), dock);
    d.rt.post(Event::PopoverDismissed);
    d.wait("config dismissed", 5, |s| s.popover.is_none())?;
    Ok(())
}

/// ⌘W with the window in front: the app's own menu — never shown, with no Dock icon — asks
/// the window to close, and the core answers as it answers ✕. Nothing of shroom's is left
/// on screen then, so the keyboard goes back to the app the person was in.
fn closes_with_cmd_w(d: &Drive, label: &'static str, name: &str, other: &str, closed: impl Fn(&Scene) -> bool) -> Result<(), String> {
    d.command('w')?;
    let gone = d.wait("⌘W closed it", 5, closed).is_ok();
    d.check(&format!("⌘W closes {name}, through the core"), gone, d.front(label).says());
    let back = poll(3, || d.frontmost().filter(|f| f == other));
    d.check(&format!("{name} closed, the keyboard goes back to the app the person was in"), back.is_some(), format!("{:?} has it (want {other})", d.frontmost()));
    Ok(())
}

/// What a window of shroom's must do with the keyboard (DECISIONS B105): what's typed lands in its field, and the standard
/// keys work. The drive puts the caret in the Library's search field (it can't click),
/// then types the way a keyboard's keys arrive: at whichever window holds the keyboard.
fn library_keys(d: &Drive) -> Result<(), String> {
    let search = |s: &Scene| s.library.as_ref().map(|l| l.body.search.value.clone()).unwrap_or_default();
    let now = || snapshot(&d.rt).map(|s| search(&s)).unwrap_or_default();
    front::caret(&d.app, LIBRARY, "[data-field=\"library-search\"]")?;
    // A key at a time, each one seen by the core before the next.
    let mut typed = String::new();
    for c in "502".chars() {
        d.press(c)?;
        typed.push(c);
        let want = typed.clone();
        if d.wait(&format!("typed {want}"), 3, |s| search(s) == want).is_err() {
            break;
        }
    }
    let s = snapshot(&d.rt).ok_or("no scene")?;
    d.check("what's typed lands in the Library's search field", search(&s) == "502" && titles(&s) == ["Where the 502s come from"], format!("search {:?}, rows {:?}", search(&s), titles(&s)));

    // ⌘A selects it all — the next key replaces it — and ⌘Z takes that back. `z` waits
    // until the page has the selection: on a busy Mac a key reaches the page later than a
    // fixed pause allowed for (issue #3).
    d.command('a')?;
    let selected = front::all_selected(&d.app, LIBRARY, 3);
    d.press('z')?;
    let all = d.wait("⌘A, then z", 3, |s| search(s) == "z").is_ok();
    d.check("⌘A selects all: the next key replaces it", all, match &selected {
        Ok(()) => now(),
        Err(saw) => format!("{} — after ⌘A, 3 s on: {saw}", now()),
    });
    d.command('z')?;
    let undone = d.wait("⌘Z", 3, |s| search(s) == "502").is_ok();
    d.check("⌘Z takes it back", undone, now());

    // ⌘C, ⌘X and ⌘V are never pressed here — they would read or write the person's own
    // clipboard. They are the same menu's keys, answered the same way: each must have its
    // item, and the field with the caret must answer it.
    for (key, action) in [("a", "selectAll:"), ("z", "undo:"), ("c", "copy:"), ("x", "cut:"), ("v", "paste:")] {
        let m = d.on_main(move || front::menu_key(key)).flatten();
        d.check(&format!("⌘{} is the field's to answer", key.to_uppercase()), m.as_ref().is_some_and(|m| m.action == action && m.answered), format!("{m:?}"));
    }
    for (key, action) in [("w", "performClose:"), ("q", "terminate:")] {
        let m = d.on_main(move || front::menu_key(key)).flatten();
        d.check(&format!("⌘{} is answered", key.to_uppercase()), m.as_ref().is_some_and(|m| m.action == action && m.answered), format!("{m:?}"));
    }
    d.rt.field(Field::LibrarySearch, String::new());
    d.wait("search cleared", 5, |s| titles(s).len() == 3)?;
    Ok(())
}

/// A window that went behind another app — the Dock and ⌘-Tab reach it, and so does the
/// popover's door, which brings it forward with the keyboard — from behind another
/// app, from the Dock's minimized end, and from hidden (⌘H).
fn back_through_the_door(d: &Drive, label: &'static str, door: Action, name: &str) -> Result<String, String> {
    let other = d.elsewhere()?;
    // AppKit tells shroom it lost the keyboard a moment after macOS moved it, and the other
    // app's windows come forward a moment after that: the hand-over lands, then the door.
    let (lost, away) = d.hand_over(label, |a| !a.front.frontmost && !a.front.active && !a.front.key && a.keyboard.as_deref() == Some(other.as_str()));
    d.check(&format!("{name} lost the keyboard to another app"), lost, format!("{other}; {}", away.front.says()));
    // Which app that was, and whether a window of its own covered this one's middle: a lane
    // that puts another app's window on screen first holds the drive to both (on a Mac with
    // only Finder in front, nothing ever covers the window, and there is nowhere but Finder
    // for the keyboard to go back to).
    d.fact_like("behind", serde_json::json!({ "app": other, "covered": !away.front.on_top }));
    d.open(door.clone())?;
    let f = d.front_when(label, Front::has_keyboard);
    d.check(&format!("behind another app, {name} comes back through the popover's door"), f.has_keyboard(), f.says());

    let app = d.app.clone();
    d.on_main(move || front::minimize(&app, label));
    let app = d.app.clone();
    let down = poll(5, || d.on_main({
        let app = app.clone();
        move || front::minimized(&app, label)
    }).filter(|m| *m)).is_some();
    d.open(door.clone())?;
    let f = d.front_when(label, Front::has_keyboard);
    d.check(&format!("minimized, {name} comes back through the popover's door"), down && f.has_keyboard(), format!("minimized {down}; {}", f.says()));

    d.command('h')?;
    // shroom says it is hidden at once; the keyboard's move to another app lands after.
    let (hid, away) = d.hand_over(label, |a| a.hidden && !a.front.frontmost && !a.front.active);
    d.open(door)?;
    let f = d.front_when(label, Front::has_keyboard);
    d.check(&format!("hidden with ⌘H, {name} comes back through the popover's door"), hid && f.has_keyboard(), format!("hidden {hid}, {} had the keyboard; {}", away.keyboard.as_deref().unwrap_or("nothing"), f.says()));
    Ok(other)
}

pub(crate) fn poll<T>(secs: u64, f: impl Fn() -> Option<T>) -> Option<T> {
    let until = Instant::now() + Duration::from_secs(secs);
    loop {
        if let Some(v) = f() {
            return Some(v);
        }
        if Instant::now() > until {
            return None;
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

// ── Settings ──

fn faces(s: &Scene) -> Vec<&'static str> {
    s.settings.as_ref().map(|w| w.faces.clone()).unwrap_or_default()
}

fn has(face: &'static str) -> impl Fn(&Scene) -> bool {
    move |s| faces(s).contains(&face)
}

fn settings(d: &Drive) -> Result<(), String> {
    d.wait("ready", 20, |s| s.tray.click == shroom_core::scene::Click::OpenPopover)?;
    // The person is working in another app when they ask for Settings.
    let hands = d.hands("Settings over another app: in front with the keyboard, back through the popover's door, ⌘W, the keyboard handed back, ⌘Q");
    let other = if hands { Some(d.elsewhere()?) } else { None };
    d.open(Action::OpenSettings)?;
    let s = d.wait("settings-general", 10, has("settings-general"))?;
    let general = match s.settings.as_ref().map(|w| &w.body.pane) {
        Some(SettingsPane::General(g)) => Some(g.clone()),
        _ => None,
    };
    d.check("General first", general.is_some(), format!("{:?}", faces(&s)));
    agent_switch(d, general.as_ref())?;
    d.check("the title is shroom", d.on_main({
        let app = d.app.clone();
        move || {
            use tauri::Manager;
            app.get_webview_window(crate::runtime::presenter::SETTINGS).and_then(|w| w.title().ok())
        }
    }).flatten().as_deref() == Some("shroom"), "window title");
    // Settings is a window, and brings the Dock icon while it is open (law 6, SM:133).
    std::thread::sleep(Duration::from_millis(300));
    let dock = d.dock();
    d.check("law 6: the Dock icon with Settings open", dock.starts_with("regular"), dock);
    if let Some(other) = &other {
        let f = d.front_when(SETTINGS, Front::has_keyboard);
        d.check("asked for over another app, Settings is in front with the keyboard", f.has_keyboard(), format!("{other} was in front; {}", f.says()));
    }

    // Space: the inspection mood once the entry's re-probe answers.
    let n = d.sweeps();
    d.act(Surface::Settings, Action::SettingsTab(SettingsTab::Space));
    d.wait("the space re-probed", 30, |_| d.sweeps() > n)?;
    d.wait("settings-space-inspection", 10, has("settings-space-inspection"))?;

    // The card gate goes up in the dashboard: the next entry routes to gate-card.
    d.fake("card-gate")?;
    let n = d.sweeps();
    d.focus(WindowKind::Settings);
    d.wait("re-probed (card gate)", 30, |_| d.sweeps() > n)?;
    d.wait("gate-card", 10, has("gate-card"))?;
    d.check("the repair mood", faces(&snapshot(&d.rt).unwrap()).contains(&"repair-mood"), format!("{:?}", faces(&snapshot(&d.rt).unwrap())));
    // The card is added: the gate's own poll sees it and the space is whole again.
    d.fake("clear-gates")?;
    d.wait("back to inspection (gate cleared)", 40, has("settings-space-inspection"))?;

    // The bucket is deleted in the dashboard: space-storage-gone on the next entry.
    d.fake(&format!("drop-bucket {}", shroom_cloud::wire::BUCKET))?;
    let n = d.sweeps();
    d.focus(WindowKind::Settings);
    d.wait("re-probed (bucket gone)", 30, |_| d.sweeps() > n)?;
    d.wait("space-storage-gone", 10, has("space-storage-gone"))?;
    // Put back (the person restores it): the next entry finds the space whole.
    d.fake(&format!("restore-bucket {}", shroom_cloud::wire::BUCKET))?;
    let n = d.sweeps();
    d.focus(WindowKind::Settings);
    d.wait("re-probed (bucket back)", 30, |_| d.sweeps() > n)?;
    d.wait("back to inspection (bucket back)", 10, has("settings-space-inspection"))?;

    if !hands {
        d.rt.post(Event::WindowClosed(WindowKind::Settings));
        d.wait("closed", 5, |s| s.settings.is_none())?;
        std::thread::sleep(Duration::from_millis(400));
        let dock = d.dock();
        d.check("law 6: no Dock icon after close", dock.starts_with("accessory"), dock);
        return Ok(());
    }
    let other = back_through_the_door(d, SETTINGS, Action::OpenSettings, "Settings")?;
    closes_with_cmd_w(d, SETTINGS, "Settings", &other, |s| s.settings.is_none())?;
    std::thread::sleep(Duration::from_millis(400));
    let dock = d.dock();
    d.check("law 6: no Dock icon after close", dock.starts_with("accessory"), dock);

    // ⌘Q with Settings in front closes it, as its ✕ does, and shroom stays in the menu bar
    // (B145). ⌘Q again, no window open — no window to address a key to, so the menu's Quit
    // it sends (`terminate:`): nothing is recording and nothing is uploading, so the core
    // quits — the drive's report is written on the way out (`drive_region5`).
    d.open(Action::OpenSettings)?;
    d.wait("settings again", 10, |s| s.settings.is_some())?;
    let f = d.front_when(SETTINGS, Front::has_keyboard);
    d.check("Settings again, in front with the keyboard", f.has_keyboard(), f.says());
    d.command('q')?;
    let closed = d.wait("⌘Q closes Settings (B145)", 5, |s| s.settings.is_none());
    d.check("⌘Q with Settings in front closes it, and shroom stays in the menu bar (B145)", closed.is_ok(), closed.err().unwrap_or_default());
    d.step(CMD_Q);
    d.on_main(front::terminate).ok_or("the main thread didn't answer")?;
    std::thread::sleep(Duration::from_secs(10));
    Err("⌘Q with no window open didn't quit shroom within 10 s".into())
}

/// General's “Let it start and stop recordings” (spec N62, DECISIONS B216), in the real app:
/// off by default; turned on, the app keeps it (`prefs.json`) and the control socket lets the
/// command's ask through to the core (a stop with nothing recording: `no_take`); off again,
/// refused again (`recording_off`). Then the refusal's link, through the router every opened
/// link takes: the switch's row lit, then not, and the switch still off.
fn agent_switch(d: &Drive, general: Option<&shroom_core::scene::GeneralBody>) -> Result<(), String> {
    let on = |s: &Scene| matches!(s.settings.as_ref().map(|w| &w.body.pane), Some(SettingsPane::General(g)) if g.agent.recording.on);
    d.check("the coding agent's switch is off by default", general.is_some_and(|g| !g.agent.recording.on), format!("{:?}", general.map(|g| &g.agent.recording)));
    let answer = ask_socket(d, "record.stop");
    d.check("the command's ask is refused while it's off", answer.contains("\"recording_off\""), answer);
    d.act(Surface::Settings, Action::SetAgentRecording(true));
    d.wait("the switch on", 5, on)?;
    let kept = || std::fs::read_to_string(d.rt.home.path("prefs.json")).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()).map(|p| p["agent_recording"].clone());
    let saved = poll(5, || (kept() == Some(serde_json::Value::Bool(true))).then_some(()));
    d.check("turned on, the app keeps it for the next launch (prefs.json)", saved.is_some(), format!("{:?}", kept()));
    let answer = ask_socket(d, "record.stop");
    d.check("turned on, the command's ask reaches the core (nothing recording: no_take)", answer.contains("\"no_take\""), answer);
    d.act(Surface::Settings, Action::SetAgentRecording(false));
    d.wait("the switch off", 5, |s| !on(s))?;
    let answer = ask_socket(d, "record.stop");
    d.check("off again, refused again", answer.contains("\"recording_off\""), answer);
    let lit = |s: &Scene| matches!(s.settings.as_ref().map(|w| &w.body.pane), Some(SettingsPane::General(g)) if g.agent.lit);
    crate::runtime::opened(&d.rt, shroom_core::event::SettingsSpot::AgentRecording.link());
    let shown = d.wait("the settings link lights the switch's row", 5, lit);
    d.check("the settings link opens General with the switch's row lit", shown.is_ok(), shown.as_ref().err().cloned().unwrap_or_default());
    d.check("the link flips nothing", shown.as_ref().is_ok_and(|s| !on(s)), "the switch read on");
    let out = d.wait("the light goes out", 10, |s| s.settings.is_some() && !lit(s));
    d.check("the link's light goes out by itself, Settings stays", out.is_ok(), out.err().unwrap_or_default());
    let answer = ask_socket(d, "record.stop");
    d.check("after the link, still refused", answer.contains("\"recording_off\""), answer);
    Ok(())
}

/// One ask over the app's control socket, as the `shroom` command makes it; its answer line.
fn ask_socket(d: &Drive, op: &str) -> String {
    use std::io::{BufRead, BufReader, Write};
    let Ok(mut s) = std::os::unix::net::UnixStream::connect(d.rt.home.path("control.sock")) else { return "no socket".into() };
    let _ = s.set_read_timeout(Some(Duration::from_secs(5)));
    let _ = s.write_all(format!("{{\"v\":1,\"op\":\"{op}\"}}\n").as_bytes());
    let mut line = String::new();
    let _ = BufReader::new(s).read_line(&mut line);
    line.trim().to_string()
}

/// The step a drive notes just before it presses ⌘Q.
const CMD_Q: &str = "⌘Q pressed";

// ── the update ──

/// The version the update drive's release stand-in serves: this very build, packaged under
/// its own version (`app/test/update-drive.sh` reads the same `tauri.conf.json`).
fn release_version() -> &'static str {
    static V: std::sync::OnceLock<String> = std::sync::OnceLock::new();
    V.get_or_init(|| {
        let conf: serde_json::Value = serde_json::from_str(include_str!("../tauri.conf.json")).expect("tauri.conf.json");
        conf["version"].as_str().expect("a version").to_string()
    })
}

fn update(d: &Drive) -> Result<(), String> {
    d.check("the running version is the drive's", d.rt.version == "0.0.1", d.rt.version.clone());
    // Checked at launch, downloaded and verified in the background: the Frost dot.
    let s = d.wait("tray-frost-dot", 90, |s| s.tray.face == Face::Frost)?;
    let staged = std::fs::read_dir(d.rt.home.updates()).into_iter().flatten().flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect::<Vec<_>>();
    d.check("staged in the home's updates/", staged.len() == 1 && d.rt.home.updates().join(&staged[0]).join("shroom.app").is_dir(), format!("{staged:?}"));
    let _ = s;
    // Settings · General mirrors the banner; it's the window in front at Update.
    d.open(Action::OpenSettings)?;
    let s = d.wait("settings-general", 10, has("settings-general"))?;
    let offered = match s.settings.as_ref().map(|w| &w.body.pane) {
        Some(SettingsPane::General(g)) => format!("{g:?}"),
        _ => String::new(),
    };
    d.check("General offers the update", offered.contains("Update"), "General pane");
    click(&d.app, &d.rt);
    let s = d.wait("config with the update banner", 10, |s| s.popover.as_ref().is_some_and(|p| p.banner.as_ref().is_some_and(|b| b.id == "update-ready")))?;
    d.check("the banner says the version", s.popover.as_ref().and_then(|p| p.banner.as_ref()).is_some_and(|b| b.line.contains(release_version())), "banner line");
    // Update: the banner says so, the app quits for the swap (the exit hook reports; the
    // second half proves the window in front came back).
    d.act(Surface::Popover, Action::Update);
    d.step("Update clicked");
    Ok(())
}

/// Uninstall shroom… as a person walks it (B135): General's quiet link, the sheet's words,
/// Uninstall — then shroom quits; `app/test/uninstall-drive.sh` checks what's left.
fn uninstall(d: &Drive) -> Result<(), String> {
    d.wait("ready", 20, |s| s.tray.click == shroom_core::scene::Click::OpenPopover)?;
    d.open(Action::OpenSettings)?;
    let s = d.wait("settings-general", 10, has("settings-general"))?;
    let link = match s.settings.as_ref().map(|w| &w.body.pane) {
        Some(SettingsPane::General(g)) => Some(g.uninstall.label.clone()),
        _ => None,
    };
    d.check("General carries Uninstall shroom…", link.as_deref() == Some("Uninstall shroom…"), format!("{link:?}"));
    d.act(Surface::Settings, Action::Uninstall);
    let s = d.wait("uninstall-confirm", 10, |s| s.settings.as_ref().and_then(|w| w.body.sheet.as_ref()).is_some_and(|sh| sh.id == "uninstall-confirm"))?;
    let sheet = s.settings.as_ref().and_then(|w| w.body.sheet.clone()).expect("the sheet");
    d.check("the sheet says the space stays", sheet.lines.first().is_some_and(|l| l.starts_with("Your space, every video in it")), format!("{:?}", sheet.lines));
    d.act(Surface::Settings, Action::ConfirmUninstall);
    d.wait("Uninstalling…", 10, |s| s.settings.as_ref().and_then(|w| w.body.sheet.as_ref()).is_some_and(|sh| sh.buttons.iter().any(|b| b.label == "Uninstalling…")))?;
    Ok(())
}

/// N66, walked as a person does on the Mac that holds the setup key, paced so its video reads:
/// the popover offers AI (the launch's page check says it's off), Turn on… asks with the cost,
/// Back keeps the banner, Turn on re-wires the page with Cloudflare's AI, the status line says
/// so once, and Settings · Space shows the switch on. Against the fake: the page it re-wires
/// is the fake's, and its ping then says the AI is there.
fn ai(d: &Drive) -> Result<(), String> {
    use shroom_core::scene::PopoverBody;
    let pause = || std::thread::sleep(Duration::from_millis(1800));
    let banner = |s: &Scene| s.popover.as_ref().and_then(|p| p.banner.as_ref()).map(|b| b.id);
    d.wait("ready", 20, |s| s.tray.click == shroom_core::scene::Click::OpenPopover)?;
    click(&d.app, &d.rt);
    let s = d.wait("the AI banner", 60, |s| banner(s) == Some("ai-offer"))?;
    let b = s.popover.as_ref().and_then(|p| p.banner.clone()).expect("the banner");
    d.check("the banner says what shroom can do", b.line == shroom_core::copy::ai_banner(), b.line.clone());
    d.check("Turn on… and Not now", b.buttons.iter().map(|x| x.label.as_str()).collect::<Vec<_>>() == ["Turn on…", "Not now"], format!("{:?}", b.buttons.iter().map(|x| &x.label).collect::<Vec<_>>()));
    pause();

    d.act(Surface::Popover, Action::AiAsk);
    let s = d.wait("ai-confirm", 10, |s| s.popover.as_ref().is_some_and(|p| p.id == "ai-confirm"))?;
    let line = match s.popover.as_ref().map(|p| &p.body) {
        Some(PopoverBody::Question(q)) => q.line.clone(),
        _ => String::new(),
    };
    d.check("the confirm says what it costs, per video hour", line == shroom_core::copy::ai_cost() && line.contains("per video hour"), line);
    d.check("no Return default (law 9)", s.popover.as_ref().is_some_and(|p| p.ret.is_none()), "ret");
    pause();
    d.act(Surface::Popover, Action::Back);
    d.wait("back: the banner stays", 10, |s| banner(s) == Some("ai-offer"))?;
    pause();

    d.act(Surface::Popover, Action::AiAsk);
    d.wait("ai-confirm again", 10, |s| s.popover.as_ref().is_some_and(|p| p.id == "ai-confirm"))?;
    pause();
    d.act(Surface::Popover, Action::AiOn);
    d.wait("turning on, or on", 10, |s| matches!(banner(s), Some("ai-turning-on")) || s.popover.as_ref().is_some_and(|p| p.status_line.is_some()))?;
    let s = d.wait("AI is on", 60, |s| s.popover.as_ref().and_then(|p| p.status_line.as_deref()) == Some(shroom_core::copy::ai_on_said().as_str()))?;
    d.check("the banner is gone once it's on", banner(&s).is_none(), format!("{:?}", banner(&s)));
    let host = d.rt.home.read_json::<SpaceIdentity>("space.json").map(|s| s.host).unwrap_or_default();
    let ping = d.rt.page_at(&host).ping();
    d.check("the space's page now carries the AI", matches!(ping, shroom_cloud::member::Ping::Shroom { ai: true, .. }), format!("{ping:?}"));
    pause();

    d.act(Surface::Popover, Action::OpenSettings);
    d.wait("settings", 10, |s| s.settings.is_some())?;
    d.act(Surface::Settings, Action::SettingsTab(SettingsTab::Space));
    let s = d.wait("the switch on in Settings · Space", 30, |s| match s.settings.as_ref().map(|w| &w.body.pane) {
        Some(SettingsPane::Space(sp)) => sp.ai.as_ref().is_some_and(|t| t.on),
        _ => false,
    })?;
    let _ = s;
    pause();
    d.rt.post(Event::WindowClosed(WindowKind::Settings));
    d.wait("closed", 5, |s| s.settings.is_none())?;
    Ok(())
}

fn update_relaunched(d: &Drive) -> Result<(), String> {
    d.check("an update relaunch", d.rt.flags.home.is_some(), "carried its throwaway home");
    d.check("the running version is the new one", d.rt.version == release_version(), d.rt.version.clone());
    // The window that was in front comes back — without the keyboard (law 5).
    let s = d.wait("settings restored", 20, |s| s.settings.is_some())?;
    let w = s.settings.as_ref().expect("settings");
    d.check("law 5: restored unasked", !w.activates, w.activates.to_string());
    std::thread::sleep(Duration::from_millis(400));
    let active = d.on_main(app_active).unwrap_or(true);
    d.check("law 5: shroom didn't take the keyboard", !active, active.to_string());
    let dock = d.dock();
    d.check("law 6: the restored window brings the Dock icon", dock.starts_with("regular"), dock);
    if d.hands("the restored Settings window in front of the app the person is in") {
        let f = d.front(SETTINGS);
        d.check("restored in front, without the keyboard", f.shown && f.on_top && !f.key, f.says());
    }
    // The popover opens by itself and says it once — unasked, under the mushroom, the
    // keyboard left where it was; a click away puts it away (B155, amending N41).
    let s = d.wait("the popover says Updated", 10, |s| s.popover.as_ref().is_some_and(|p| p.id == "config"))?;
    let p = s.popover.as_ref().expect("config");
    d.check("B155: the popover opens by itself after the relaunch", !p.activates && p.away_closes, format!("activates {} away_closes {}", p.activates, p.away_closes));
    d.check("“Updated to <this build's version> ✓”", p.faces.contains(&"updated") && p.status_line.as_deref().is_some_and(|l| l.contains(release_version())), format!("{:?} {:?}", p.faces, p.status_line));
    std::thread::sleep(Duration::from_millis(400));
    let active = d.on_main(app_active).unwrap_or(true);
    d.check("law 5: the popover didn't take the keyboard either", !active, active.to_string());
    let placed = d.on_main({
        let app = d.app.clone();
        move || crate::runtime::presenter::placement(&app).0
    });
    d.check("it hangs under the mushroom", placed.flatten().is_some_and(|p| p.under), format!("{placed:?}"));
    d.rt.post(Event::PopoverDismissed);
    d.wait("popover closed", 5, |s| s.popover.is_none())?;
    click(&d.app, &d.rt);
    let s = d.wait("config again", 10, |s| s.popover.as_ref().is_some_and(|p| p.id == "config"))?;
    let p = s.popover.as_ref().expect("config");
    d.check("said once", !p.faces.contains(&"updated"), format!("{:?}", p.faces));
    d.rt.post(Event::PopoverDismissed);
    d.wait("popover closed again", 5, |s| s.popover.is_none())?;
    let s = d.after(2_000);
    d.check("nothing newer: no Frost dot", s.tray.face != Face::Frost, format!("{:?}", s.tray.face));
    d.rt.post(Event::WindowClosed(WindowKind::Settings));
    d.wait("closed", 5, |s| s.settings.is_none())?;
    Ok(())
}

// ── the editor (N63) ──

/// The editor's two videos: one recorded on this Mac (its take folder holds the single-file
/// copy) and one only in the space (the editor fetches it). Their sound and picture are the
/// runner's test video.
const HERE: (&str, &str) = ("Edrive00000here0", "Setting up shroom in two minutes");
const AWAY: (&str, &str) = ("Edrive00000away0", "Onboarding flow, take 2");
const THIRD: (&str, &str) = ("Edrive0000third0", "Where the 502s come from");
/// The markers dropped while the first was recorded (N64).
const MARKERS: [f64; 2] = [6.0, 31.0];

/// The runner's folder: `--drive editor <out> <dir>`.
fn drive_dir() -> Result<PathBuf, String> {
    let args: Vec<String> = std::env::args().collect();
    let at = args.iter().position(|a| a == "--drive").ok_or("no --drive")?;
    args.get(at + 3).map(PathBuf::from).filter(|d| d.is_dir()).ok_or_else(|| "--drive editor <out> <dir>: no folder from the runner".into())
}

/// What the drive asks of its runner, which can do what the app shouldn't do for itself: a
/// screenshot of one window (`screencapture`), the pointer (`cliclick`, with the runner's
/// own Accessibility). Each ask is `ask-<n>.json` in the runner's folder; its answer
/// `done-<n>` (`ok`, or what went wrong).
struct Runner {
    dir: PathBuf,
    n: std::cell::Cell<u32>,
}

impl Runner {
    fn ask(&self, d: &Drive, what: serde_json::Value) -> Result<(), String> {
        let n = self.n.get() + 1;
        self.n.set(n);
        let (tmp, ask) = (self.dir.join(format!("ask-{n}.tmp")), self.dir.join(format!("ask-{n}.json")));
        std::fs::write(&tmp, what.to_string()).and_then(|()| std::fs::rename(&tmp, &ask)).map_err(|e| format!("ask {n}: {e}"))?;
        let done = self.dir.join(format!("done-{n}"));
        // An empty answer is one still being written: it's read again.
        let said = poll(60, || std::fs::read_to_string(&done).ok().filter(|s| !s.trim().is_empty())).ok_or(format!("the runner never answered {what}"))?;
        d.step(&format!("the runner: {what} — {}", said.trim()));
        if said.trim() == "ok" { Ok(()) } else { Err(format!("the runner couldn't {what}: {said}")) }
    }

    /// One of shroom's windows, as it is on screen now, into `<name>.png` beside the asks.
    fn shot(&self, d: &Drive, label: &'static str, name: &str) -> Result<(), String> {
        use tauri::Manager;
        let app = d.app.clone();
        let id = d.on_main(move || {
            let w = app.get_webview_window(label)?;
            let ns = w.ns_window().ok()?;
            // SAFETY: Tauri's own NSWindow for this webview window, read on the main thread.
            Some(unsafe { &*(ns as *const objc2_app_kit::NSWindow) }.windowNumber())
        });
        let id = id.flatten().ok_or(format!("no {label} window to take"))?;
        self.ask(d, serde_json::json!({ "do": "shot", "window": id, "name": name }))
    }
}

/// Where the editor's parts are in its page (CSS pixels from the page's top-left), as its
/// page answers now: `.track`, the chip's button, the video.
fn editor_layout(d: &Drive) -> Result<serde_json::Value, String> {
    use tauri::Manager;
    // Also, from the first look on: where the pointer last moved over the page, and the
    // pointer's presses there (what a press landed on), for the drive's report.
    const JS: &str = r#"(() => {
      if (!window.__drive) {
        window.__drive = { at: null, presses: [] };
        addEventListener("pointermove", (e) => (window.__drive.at = [e.clientX, e.clientY]), true);
        for (const t of ["pointerdown", "pointerup", "lostpointercapture", "click"]) addEventListener(t, (e) => window.__drive.presses.push(`${t} ${e.target.className?.baseVal ?? e.target.className} ${Math.round(e.clientX)},${Math.round(e.clientY)}`), true);
      }
      const r = (q) => { const e = document.querySelector(q); if (!e) return null; const b = e.getBoundingClientRect(); return [b.left, b.top, b.width, b.height]; };
      const v = document.querySelector("video");
      return JSON.stringify({ track: r(".track"), chip: r(".chip button"), screen: r(".screen"), delete: r(".ask .b.filled"), discard: r(".ask .b.red-outline"), ask_title: document.querySelector(".ask h2")?.textContent ?? null, ask: [...document.querySelectorAll(".ask button")].map((b) => b.textContent.trim()), play: r(".play"), label: document.querySelector(".play")?.getAttribute("aria-label") ?? null, video: v ? [v.duration, v.readyState] : null, paused: v ? v.paused : null, pointer: window.__drive.at, presses: window.__drive.presses.slice(-12) });
    })()"#;
    let w = d.app.get_webview_window(crate::runtime::presenter::EDITOR).ok_or("no editor window")?;
    let (tx, rx) = std::sync::mpsc::channel();
    w.eval_with_callback(JS, move |out| {
        let _ = tx.send(out);
    })
    .map_err(|e| e.to_string())?;
    let out = rx.recv_timeout(Duration::from_secs(5)).map_err(|_| String::from("the editor's page didn't answer"))?;
    // The page's answer comes back as JSON — a string holding JSON, here.
    let v: serde_json::Value = serde_json::from_str(&out).map_err(|e| format!("{e}: {out}"))?;
    match v {
        serde_json::Value::String(inner) => serde_json::from_str(&inner).map_err(|e| format!("{e}: {inner}")),
        v => Ok(v),
    }
}

/// Where AppKit says the editor's content starts on screen, in points from the top-left of
/// the main display (what `cliclick` takes). On the hosted Mac it reads the title bar's top,
/// as Tauri's inner position does — the page sits 32 pt lower — so the drive takes it as a
/// first guess and checks it against where the page sees the pointer land.
fn editor_origin(d: &Drive) -> Result<(f64, f64), String> {
    use tauri::Manager;
    let app = d.app.clone();
    d.on_main(move || {
        let mtm = objc2_foundation::MainThreadMarker::new()?;
        let w = app.get_webview_window(crate::runtime::presenter::EDITOR)?;
        let ns = w.ns_window().ok()?;
        // SAFETY: Tauri's own NSWindow for this webview window, read on the main thread.
        let ns: &objc2_app_kit::NSWindow = unsafe { &*(ns as *const objc2_app_kit::NSWindow) };
        let content = ns.contentRectForFrameRect(ns.frame());
        // AppKit's y runs up from the bottom of the main display (the first screen).
        let screens = objc2_app_kit::NSScreen::screens(mtm);
        let main_h = (screens.count() > 0).then(|| screens.objectAtIndex(0).frame().size.height)?;
        Some((content.origin.x, main_h - (content.origin.y + content.size.height)))
    })
    .flatten()
    .ok_or_else(|| "the editor's window isn't on screen".into())
}

/// A rectangle from the page, its centre on screen.
/// The editor just opened: its video paused, and its button reading Play.
fn starts_paused(d: &Drive, what: &str) -> Result<(), String> {
    let fresh = poll(5, || editor_layout(d).ok().filter(|l| l["video"].get(1).and_then(serde_json::Value::as_u64).is_some_and(|ready| ready >= 1)));
    let l = fresh.ok_or("the editor's video never loaded")?;
    d.check(what, l["paused"] == true && l["label"] == shroom_core::copy::editor_play().as_str(), format!("paused {}, the button reads {}", l["paused"], l["label"]));
    Ok(())
}

/// Play, pressed with the pointer: a real click on the editor's play button.
/// The editor window's own close, on the main thread, as a click on its ✕ makes it: the shell
/// hears CloseRequested, keeps the window, and the core decides (N70).
fn close_by_hand(d: &Drive) {
    use tauri::Manager;
    let app = d.app.clone();
    d.on_main(move || app.get_webview_window(crate::runtime::presenter::EDITOR).map(|w| w.close()));
}

fn play_by_hand(d: &Drive, runner: &Runner, origin: (f64, f64)) -> Result<(), String> {
    let at = editor_layout(d)?.get("play").and_then(|r| centre(origin, r)).ok_or("no play button on the page")?;
    runner.ask(d, serde_json::json!({ "do": "click", "at": [at.0, at.1] }))
}

fn centre(origin: (f64, f64), r: &serde_json::Value) -> Option<(f64, f64)> {
    let n = |i: usize| r.get(i).and_then(serde_json::Value::as_f64);
    Some((origin.0 + n(0)? + n(2)? / 2.0, origin.1 + n(1)? + n(3)? / 2.0))
}

/// Seeds a live record of the test video, and its bytes: the stream's stand-in, the single
/// file the editor fetches, and the poster.
fn seed_edit_video(s3: &Client, (id, title): (&str, &str), video: &std::path::Path, poster: &std::path::Path, length: f64, markers: &[f64]) -> Result<(), String> {
    use shroom_upload::{keys, meta};
    let at = "2026-10-12T09:30:00Z";
    let mut m = meta::Meta::uploading(id, title, at);
    m.state = meta::State::Live;
    m.published_at = Some(at.into());
    m.duration_sec = Some(length);
    m.owner = Some("m1".into());
    m.markers = (!markers.is_empty()).then(|| markers.to_vec());
    m.media = Some(meta::Media { hls: Some(keys::master(id)), mp4: Some(keys::mp4(id)), poster: Some(keys::poster(id)), teaser: None, width: 1280, height: 720 });
    let e = |e: shroom_storage::s3::S3Error| e.to_string();
    s3.put(&keys::master(id), b"#EXTM3U\n", "application/vnd.apple.mpegurl").map_err(e)?;
    s3.put(&format!("{}v0/seg0.m4s", keys::prefix(id)), &[0u8; 512], "video/iso.segment").map_err(e)?;
    s3.put(&keys::teaser(id), &[0u8; 512], "video/mp4").map_err(e)?;
    s3.put_file(&keys::mp4(id), video, "video/mp4", shroom_upload::upload::SINGLE_PUT_MAX, shroom_upload::upload::PART_SIZE).map_err(e)?;
    s3.put_file(&keys::poster(id), poster, "image/jpeg", shroom_upload::upload::SINGLE_PUT_MAX, shroom_upload::upload::PART_SIZE).map_err(e)?;
    s3.put(&keys::meta(id), &serde_json::to_vec(&m).map_err(|e| e.to_string())?, "application/json").map_err(e)
}

/// The take this Mac recorded the first video as, long since up: its folder, holding the
/// single-file copy and its poster.
fn seed_edit_take(rt: &Runtime, (id, title): (&str, &str), video: &std::path::Path, poster: &std::path::Path) -> Result<PathBuf, String> {
    use crate::runtime::takes::{PublishJson, TakeJson, new_dir_name};
    let wall = 1_791_797_400_000;
    let dir = rt.home.takes().join(new_dir_name(wall));
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let e = |e: std::io::Error| e.to_string();
    home::write_json(&dir.join("take.json"), &TakeJson { v: 1, rec: id.into(), created: wall }).map_err(e)?;
    home::write_json(&dir.join("publish.json"), &PublishJson { v: 1, rec: id.into(), title: title.into(), stopped: wall + 151_000 }).map_err(e)?;
    std::fs::copy(video, dir.join(shroom_encode::files::PREVIEW)).map_err(e)?;
    std::fs::copy(poster, dir.join(shroom_encode::files::POSTER)).map_err(e)?;
    std::fs::write(dir.join("uploaded"), b"").map_err(e)?;
    Ok(dir)
}

fn editor_body(s: &Scene) -> Option<&shroom_core::scene::EditorBody> {
    s.editor.as_ref().map(|e| &e.body)
}

fn editor_ready(s: &Scene) -> bool {
    editor_body(s).is_some_and(|b| matches!(b.media, shroom_core::scene::EditorMediaView::Ready { .. }) && b.list.is_some())
}

fn cuts_of(s: &Scene) -> Vec<(f64, f64)> {
    editor_body(s).and_then(|b| b.list.as_ref()).map(|l| l.cuts.iter().filter(|c| c.applied).map(|c| (c.start, c.end)).collect()).unwrap_or_default()
}

/// The record in the fake's bucket, as JSON.
fn record_json(s3: &Client, id: &str) -> Option<serde_json::Value> {
    serde_json::from_slice(&s3.get(&shroom_upload::keys::meta(id)).ok()??).ok()
}

fn near(a: f64, b: f64, tol: f64) -> bool {
    (a - b).abs() <= tol
}

fn editor(d: &Drive) -> Result<(), String> {
    use shroom_core::event::EditStep;
    use tauri::Manager;
    use shroom_edit::Op;
    let runner = Runner { dir: drive_dir()?, n: std::cell::Cell::new(0) };
    let (video, poster) = (runner.dir.join("preview.mp4"), runner.dir.join("poster.jpg"));
    let length = shroom_encode::cut::picture_length(&video).ok_or("the runner's test video has no length")?;
    d.fact_like("video_seconds", length);
    d.wait("ready", 20, |s| s.tray.click == shroom_core::scene::Click::OpenPopover)?;
    let s3 = d.bucket()?;
    seed_edit_video(&s3, HERE, &video, &poster, length, &MARKERS)?;
    seed_edit_video(&s3, AWAY, &video, &poster, length, &[])?;
    seed_edit_video(&s3, THIRD, &video, &poster, length, &[])?;
    let take = seed_edit_take(&d.rt, HERE, &video, &poster)?;
    d.step("seeded three videos into the fake's bucket, and the first one's take on this Mac");

    // Edit… on a live row of the Library.
    d.open(Action::OpenLibrary)?;
    let s = d.wait("the three videos", 20, |s| titles(s).len() == 3)?;
    let here = rec_of(&s, HERE.1).ok_or("no row for the first video")?;
    d.check("a live row offers Edit… (N63)", s.offers(Surface::Library, &Action::EditRow(here.clone())), format!("{:?}", s.library.as_ref().and_then(|l| l.body.rows.iter().find(|r| r.key == here.0)).map(|r| r.menu.iter().map(|b| b.label.clone()).collect::<Vec<_>>())));
    d.act(Surface::Library, Action::EditRow(here.clone()));
    let s = d.wait("the editor, ready", 30, editor_ready)?;
    let b = editor_body(&s).ok_or("no editor")?;
    d.check("the editor's title is its video's", b.title == format!("Edit — {}", HERE.1), b.title.clone());
    let native = || {
        let app = d.app.clone();
        d.on_main(move || app.get_webview_window(crate::runtime::presenter::EDITOR).and_then(|w| w.title().ok())).flatten()
    };
    let titled = poll(3, || native().filter(|t| *t == b.title));
    d.check("… and so is its window's", titled.is_some(), format!("{:?}", native()));
    d.check("the record's markers are its flags (N64)", b.markers == MARKERS, format!("{:?}", b.markers));
    d.check("the recording's length is the record's", near(b.source_duration, length, 0.01), format!("{} vs {length}", b.source_duration));
    let l = b.list.as_ref().expect("a list");
    d.check("nothing cut yet: the whole recording", l.in_ == 0.0 && near(l.out, length, 0.01) && l.cuts.is_empty(), format!("{} – {}, {} cuts", l.in_, l.out, l.cuts.len()));
    d.check("Done is live, Revert to original greyed", b.done.enabled && b.revert.as_ref().is_some_and(|r| !r.enabled), format!("{} {:?}", b.done.enabled, b.revert.as_ref().map(|r| r.enabled)));
    let edits = d.rt.home.edits().join(&here.0);
    d.check("found on this Mac, from its take: nothing fetched", !edits.join(crate::runtime::editor::SOURCE).exists() && take.join(shroom_encode::files::PREVIEW).is_file(), edits.display().to_string());
    let peaks = std::fs::metadata(edits.join(crate::runtime::editor::PEAKS)).map(|m| m.len()).unwrap_or(0);
    let want = shroom_encode::peaks::buckets(length) as u64;
    d.check("its sound read into the edits folder, one byte per 20 ms", peaks.abs_diff(want) <= 2, format!("{peaks} bytes for {length:.2} s (want {want})"));
    use std::os::unix::fs::PermissionsExt;
    let mode = |p: &std::path::Path| std::fs::metadata(p).map(|m| format!("{:o}", m.permissions().mode() & 0o777)).unwrap_or_default();
    d.check("the edits folder is private", mode(&d.rt.home.edits()) == "700" && mode(&edits) == "700" && mode(&edits.join(crate::runtime::editor::PEAKS)) == "600", format!("{} {} {}", mode(&d.rt.home.edits()), mode(&edits), mode(&edits.join(crate::runtime::editor::PEAKS))));
    std::thread::sleep(Duration::from_millis(400));
    let dock = d.dock();
    d.check("law 6: the Dock icon with the editor open", dock.starts_with("regular"), dock);
    // A window's first paint takes a few seconds on the runner.
    std::thread::sleep(Duration::from_secs(4));
    let layout = editor_layout(d)?;
    d.fact_like("editor_layout", &layout);
    d.check("the page draws its timeline", layout.get("track").is_some_and(|t| !t.is_null()), layout.to_string());
    runner.shot(d, crate::runtime::presenter::EDITOR, "editor-open")?;

    // Steps, as the webview sends them: a trim, a cut, Undo and Redo.
    d.rt.post(Event::Edit(EditStep::Op(Op::Trim { in_: 2.0, out: 25.0 })));
    d.wait("trimmed 0:02 – 0:25", 5, |s| editor_body(s).and_then(|b| b.list.as_ref()).is_some_and(|l| l.in_ == 2.0 && l.out == 25.0))?;
    d.rt.post(Event::Edit(EditStep::Op(Op::Cut { start: 8.0, end: 12.0 })));
    let s = d.wait("cut 0:08 – 0:12", 5, |s| cuts_of(s) == [(8.0, 12.0)])?;
    let b = editor_body(&s).expect("the editor");
    d.check("what plays: 0:02 – 0:08, then 0:12 – 0:25", b.keep == [[2.0, 8.0], [12.0, 25.0]] && near(b.edited_duration, 19.0, 1e-9), format!("{:?} = {}", b.keep, b.edited_duration));
    d.rt.post(Event::Edit(EditStep::Undo));
    d.wait("undone", 5, |s| cuts_of(s).is_empty())?;
    d.rt.post(Event::Edit(EditStep::Redo));
    let s = d.wait("redone", 5, |s| cuts_of(s) == [(8.0, 12.0)])?;
    d.check("Undo takes the cut back, Redo makes it again", editor_body(&s).is_some_and(|b| b.can_undo && !b.can_redo && b.revert.as_ref().is_some_and(|r| r.enabled)), "");
    std::thread::sleep(Duration::from_millis(800));
    runner.shot(d, crate::runtime::presenter::EDITOR, "editor-edited")?;

    // A drag across the sound with the pointer — the page's own selection, its Cut chip,
    // and the chip's cut through the edit command into the core. Where the page is on
    // screen is AppKit's word, checked against where the page sees the pointer land.
    let guess = editor_origin(d)?;
    let layout = editor_layout(d)?;
    let probe = layout.get("screen").and_then(|r| centre(guess, r)).ok_or("no video on the page")?;
    runner.ask(d, serde_json::json!({ "do": "move", "at": [probe.0, probe.1] }))?;
    let seen = poll(3, || editor_layout(d).ok().and_then(|l| l.get("pointer").and_then(|p| Some((p.get(0)?.as_f64()?, p.get(1)?.as_f64()?)))));
    let origin = seen.map_or(guess, |(cx, cy)| (probe.0 - cx, probe.1 - cy));
    d.fact_like("origin", serde_json::json!({ "appkit": [guess.0, guess.1], "pointer": seen.map(|p| [p.0, p.1]), "taken": [origin.0, origin.1] }));
    d.check("the page sees the pointer where it was put", seen.is_some(), format!("put at {probe:?}, the page saw {seen:?}"));
    let track = layout.get("track").cloned().unwrap_or_default();
    let (x0, y0, w0, h0) = (track[0].as_f64().unwrap_or(0.0), track[1].as_f64().unwrap_or(0.0), track[2].as_f64().unwrap_or(0.0), track[3].as_f64().unwrap_or(0.0));
    let x = |t: f64| origin.0 + x0 + t / length * w0;
    let y = origin.1 + y0 + h0 / 2.0;
    let video_at = layout.get("screen").and_then(|r| centre(origin, r)).ok_or("no video on the page")?;
    d.fact_like("drag", serde_json::json!({ "origin": [origin.0, origin.1], "from": [x(14.0), y], "to": [x(18.0), y] }));
    d.runner_drag(&runner, video_at, (x(14.0), y), (x(18.0), y))?;
    let chip = poll(5, || editor_layout(d).ok().and_then(|l| l.get("chip").filter(|c| !c.is_null()).cloned()));
    d.check("a drag across the sound selects a part, and the chip offers to cut it", chip.is_some(), layout.to_string());
    runner.shot(d, crate::runtime::presenter::EDITOR, "editor-drag")?;
    if let Some(at) = chip.as_ref().and_then(|c| centre(origin, c)) {
        runner.ask(d, serde_json::json!({ "do": "click", "at": [at.0, at.1] }))?;
        let s = d.wait("the dragged part, cut", 5, |s| cuts_of(s).len() == 2);
        let cuts = snapshot(&d.rt).map(|s| cuts_of(&s)).unwrap_or_default();
        let presses = editor_layout(d).ok().and_then(|l| l.get("presses").cloned()).unwrap_or_default();
        d.fact_like("presses", &presses);
        d.check("the chip's Cut lands in the core, where it was dragged", s.is_ok() && near(cuts[1].0, 14.0, 0.6) && near(cuts[1].1, 18.0, 0.6), format!("{cuts:?}; the page's presses: {presses}"));
        std::thread::sleep(Duration::from_millis(600));
        runner.shot(d, crate::runtime::presenter::EDITOR, "editor-dragged-cut")?;

        // That cut's end, dragged two seconds on with the pointer (B195 (g)): one step, which
        // Undo takes back — the rest of the drive goes on with the cut as it was.
        if let Some(&(_, end)) = cuts.get(1) {
            let dragged = d.runner_drag(&runner, video_at, (x(end), y), (x(end + 2.0), y));
            let s = d.wait("the cut's edge, moved", 5, |s| cuts_of(s).get(1).is_some_and(|c| near(c.1, end + 2.0, 0.6)));
            let moved = snapshot(&d.rt).map(|s| cuts_of(&s)).unwrap_or_default();
            d.check("a cut's edge drags: the cut ends where it was let go (B195 (g))", dragged.is_ok() && s.is_ok(), format!("{moved:?} (its end was {end:.2})"));
            std::thread::sleep(Duration::from_millis(500));
            runner.shot(d, crate::runtime::presenter::EDITOR, "editor-edge-dragged")?;
            d.rt.post(Event::Edit(EditStep::Undo));
            let back = d.wait("the edge, back", 5, |s| cuts_of(s).get(1).is_some_and(|c| c.1 == end));
            d.check("… one step: Undo puts the edge back", back.is_ok(), format!("{:?}", snapshot(&d.rt).map(|s| cuts_of(&s))));
        }
    }

    // ✕ with changes not saved asks first (N70): the card in the footer's place, the preview
    // stopped and nothing saved. Esc is Keep editing: back to the edit, every change there.
    play_by_hand(d, &runner, origin)?;
    let held = snapshot(&d.rt).map(|s| cuts_of(&s)).unwrap_or_default();
    close_by_hand(d);
    let asked = d.wait("✕ asks first", 10, |s| s.editor.as_ref().is_some_and(|e| e.id == "editor-closing"));
    let card = poll(5, || editor_layout(d).ok().filter(|l| l["ask_title"].is_string()));
    let words = serde_json::json!([shroom_core::copy::discard(), shroom_core::copy::keep_editing(), shroom_core::copy::save()]);
    d.check(
        "✕ with changes not saved asks: “Save your edit?” in the footer's place, then Discard, Keep editing and Save (N70)",
        asked.is_ok() && card.as_ref().is_some_and(|l| l["ask_title"] == shroom_core::copy::close_unsaved_title().as_str() && l["ask"] == words),
        format!("{:?}", card.map(|l| (l["ask_title"].clone(), l["ask"].clone()))),
    );
    let stopped = poll(3, || editor_layout(d).ok().filter(|l| l["paused"] == true));
    let unsaved = record_json(&s3, &here.0).is_some_and(|r| r["edits"].is_null());
    d.check("… the preview stops while it asks, and nothing is saved yet", stopped.is_some() && unsaved, format!("paused {}, saved nothing {unsaved}", stopped.is_some()));
    std::thread::sleep(Duration::from_millis(500));
    runner.shot(d, crate::runtime::presenter::EDITOR, "editor-closing")?;
    d.key(Surface::Editor, Key::Esc);
    let back = d.wait("Keep editing", 5, |s| s.editor.as_ref().is_some_and(|e| e.id == "editor"));
    let kept = snapshot(&d.rt).map(|s| cuts_of(&s)).unwrap_or_default();
    d.check("Esc is Keep editing: back to the edit, every change still there", back.is_ok() && kept == held, format!("{held:?} → {kept:?}"));

    // Playing when Done is pressed: the preview stops with it, and nothing plays on behind
    // the faces that follow (the audio went on after Done, 2026-10-18).
    play_by_hand(d, &runner, origin)?;
    let playing = poll(3, || editor_layout(d).ok().filter(|l| l["paused"] == false));
    d.check("Play plays the edit (a real click on it)", playing.is_some(), format!("{:?}", editor_layout(d).map(|l| l["paused"].clone())));

    // Done: the edit goes into the record, and the window makes the final video (B197 (a)).
    let want = snapshot(&d.rt).as_ref().and_then(|s| editor_body(s).map(|b| (cuts_of(s), b.edited_duration))).ok_or("no editor")?;
    d.act(Surface::Editor, Action::EditorDone);
    let s = d.wait("the final video, being made", 30, |s| s.editor.as_ref().is_some_and(|e| e.id == "editor-making" || e.id == "editor-keep-original"))?;
    let stopped = poll(3, || editor_layout(d).ok().filter(|l| l["paused"] == true));
    d.check("Done stops the preview: nothing plays on while the final video is made", stopped.is_some(), format!("{:?}", editor_layout(d).map(|l| l["paused"].clone())));
    let making = s.editor.as_ref().is_some_and(|e| e.id == "editor-making");
    runner.shot(d, crate::runtime::presenter::EDITOR, "editor-making")?;
    let shown = snapshot(&d.rt).and_then(|s| s.editor.map(|e| e.id));
    d.fact_like("making_shot_face", shown);
    let seen = d.rt.trace.lock().unwrap().as_ref().map(|t| t.ids.clone()).unwrap_or_default();
    d.check("Done saves, then the window makes the final video (B197)", seen.iter().any(|id| id.starts_with("editor editor-saving")) && seen.iter().any(|id| id.starts_with("editor editor-making")), seen.join(" → "));
    d.fact_like("making_seen_first", making);
    let rec = record_json(&s3, &here.0).ok_or("no record")?;
    let ed = &rec["edits"];
    let cuts: Vec<(f64, f64)> = ed["cuts"].as_array().map(|a| a.iter().filter(|c| c["applied"] == true).filter_map(|c| Some((c["start"].as_f64()?, c["end"].as_f64()?))).collect()).unwrap_or_default();
    d.check("the record holds the edit: revision 1, the trim and the cuts", ed["rev"] == 1 && ed["in"] == 2.0 && ed["out"] == 25.0 && cuts == want.0, ed.to_string());
    d.check("… and its title and markers are as they were", rec["title"] == HERE.1 && rec["markers"] == serde_json::json!(MARKERS), format!("{} {}", rec["title"], rec["markers"]));
    let len = shroom_core::copy::length((want.1 * 1000.0).round() as u64);
    let row = |s: &Scene| s.library.as_ref().and_then(|l| l.body.rows.iter().find(|r| r.key == here.0)).map(|r| r.meta.clone()).unwrap_or_default();
    d.check("the Library row reads the edited length", row(&s).ends_with(&format!("· {len}")), format!("{} (want {len})", row(&s)));

    // The copy is up: the window asks about the original (B197 (b)). Meanwhile its bar
    // fills as the copy is cut and sent.
    let shares = Mutex::new(Vec::<u8>::new());
    let s = d.wait("“Delete the original?”", 240, |s| {
        if let Some(shroom_core::scene::EditorAfterView::Making { pct, .. }) = s.editor.as_ref().and_then(|e| e.body.after.as_ref()) {
            let mut v = shares.lock().unwrap();
            if v.last() != Some(pct) {
                v.push(*pct);
            }
        }
        s.editor.as_ref().is_some_and(|e| e.id == "editor-keep-original")
    })?;
    let shares = shares.into_inner().unwrap();
    d.check("the bar fills as the final video is made, forward only", shares.iter().any(|p| (1..100).contains(p)) && shares.windows(2).all(|w| w[0] < w[1]), format!("{shares:?}"));
    std::thread::sleep(Duration::from_millis(600));
    runner.shot(d, crate::runtime::presenter::EDITOR, "editor-asking")?;
    let asked = s.editor.as_ref().and_then(|e| match &e.body.after {
        Some(shroom_core::scene::EditorAfterView::Asking { title, keep, delete, .. }) => Some((title.clone(), keep.label.clone(), delete.label.clone(), delete.style, e.ret.clone())),
        _ => None,
    });
    d.check("it asks “Delete the original?”: Keep for 2 days, Yes, delete — filled, never on Return", asked.as_ref().is_some_and(|(t, k, del, style, ret)| t == "Delete the original?" && k == "Keep for 2 days" && del == "Yes, delete" && *style == Style::Filled && ret.is_none()), format!("{asked:?}"));
    let rec = record_json(&s3, &here.0).ok_or("no record")?;
    let copy = rec["edits"]["copy"].clone();
    d.fact_like("copy", &copy);
    d.check("the copy is named for revision 1", copy["rev"] == 1 && copy["file"] == "edit-1.mp4" && copy["poster"] == "edit-1-poster.jpg" && copy["teaser"] == "edit-1-teaser.mp4", copy.to_string());
    d.check("the copy is as long as the edit", copy["duration"].as_f64().is_some_and(|c| near(c, want.1, 0.6)), format!("{} vs {}", copy["duration"], want.1));
    let two_days = |r: &serde_json::Value| {
        let at = r["edits"]["keep_until"].as_str().and_then(shroom_upload::library::iso_ms);
        let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
        (at.is_some_and(|t| (t - now - 2 * 86_400_000).abs() < 15 * 60_000), r["edits"]["keep_until"].clone())
    };
    let (kept, until) = two_days(&rec);
    d.check("its landing set when the original goes: two days on", kept, until.to_string());
    let files: Vec<String> = s3.list(&shroom_upload::keys::prefix(&here.0)).map(|v| v.into_iter().map(|o| o.key).collect()).unwrap_or_default();
    let has = |files: &[String], name: &str| files.iter().any(|k| k.ends_with(&format!("/{name}")));
    d.check("its three files are beside the recording, the original's too", ["edit-1.mp4", "edit-1-poster.jpg", "edit-1-teaser.mp4", "video.mp4", "poster.jpg"].iter().all(|f| has(&files, f)), files.join(" "));
    let temp: Vec<String> = std::fs::read_dir(&edits).map(|r| r.filter_map(|e| e.ok()).map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
    d.check("the copy's working files are gone from this Mac", temp.iter().all(|f| f == crate::runtime::editor::PEAKS), temp.join(" "));
    let got = s3.get(&format!("{}edit-1.mp4", shroom_upload::keys::prefix(&here.0))).ok().flatten().unwrap_or_default();
    let copy_file = runner.dir.join("edit-1.mp4");
    let _ = std::fs::write(&copy_file, &got);
    let read = shroom_encode::cut::picture_length(&copy_file);
    d.check("the copy is a faststart MP4 whose picture is the edit's length", read.is_some_and(|r| near(r, want.1, 0.6)), format!("{read:?}"));
    // The Library comes forward and reads the space again: its cached row carries the
    // edit's length, so the row keeps it through every refresh.
    d.focus(WindowKind::Library);
    let edited = |c: &str| serde_json::from_str::<serde_json::Value>(c).ok()?.pointer(&format!("/cache/rows/{}/1/edited_sec", here.0))?.as_f64();
    let cached = poll(15, || std::fs::read_to_string(d.rt.home.path("library.json")).ok().and_then(|c| edited(&c)).filter(|e| near(*e, want.1, 0.01)));
    let s = d.after(300);
    d.check("after a refresh the row still reads the edit's length (the cache holds it)", cached.is_some() && row(&s).ends_with(&format!("· {len}")), format!("{} (want {len}); cached {cached:?}", row(&s)));
    // Return isn't Yes, delete (law 9; DEVIATIONS #55).
    d.key(Surface::Editor, Key::Return);
    let s = d.after(800);
    d.check("Return does nothing there", s.editor.as_ref().is_some_and(|e| e.id == "editor-keep-original") && record_json(&s3, &here.0).is_some_and(|r| !r["edits"].is_null()), format!("{:?}", s.editor.as_ref().map(|e| e.id)));

    // Yes, delete — a click on it: the edit becomes the video, the original goes (B197 (c)).
    let layout = editor_layout(d)?;
    let yes = layout.get("delete").and_then(|r| centre(origin, r)).ok_or("no Yes, delete on the page")?;
    runner.ask(d, serde_json::json!({ "do": "click", "at": [yes.0, yes.1] }))?;
    d.wait("Yes, delete closed it", 10, |s| s.editor.is_none())?;
    let baked = poll(30, || record_json(&s3, &here.0).filter(|r| r["edits"].is_null()));
    let rec = baked.ok_or("the bake never landed in the record (30 s)")?;
    d.fact_like("baked_record", serde_json::json!({ "media": rec["media"], "durationSec": rec["durationSec"], "markers": rec["markers"], "bakedFrom": rec["bakedFrom"] }));
    let key = |name: &str| format!("{}{name}", shroom_upload::keys::prefix(&here.0));
    let m = &rec["media"];
    d.check(
        "the record plays the copy: its file, its poster and teaser, no stream",
        m["mp4"] == key("edit-1.mp4") && m["poster"] == key("edit-1-poster.jpg") && m["teaser"] == key("edit-1-teaser.mp4") && m["hls"].is_null(),
        m.to_string(),
    );
    d.check("… its length the copy's, its edit gone, one bake remembered", rec["durationSec"] == copy["duration"] && rec["bakedFrom"].as_array().is_some_and(|b| b.len() == 1), format!("{} {}", rec["durationSec"], rec["bakedFrom"]));
    d.check("… a marker inside what's kept moved onto the edit's clock, one past the trim gone", rec["markers"] == serde_json::json!([4.0]), rec["markers"].to_string());
    let gone = poll(20, || {
        let files: Vec<String> = s3.list(&shroom_upload::keys::prefix(&here.0)).ok()?.into_iter().map(|o| o.key).collect();
        (!["master.m3u8", "v0/seg0.m4s", "video.mp4", "poster.jpg", "teaser.mp4"].iter().any(|f| has(&files, f))).then_some(files)
    });
    d.check("the original's files are gone; the copy's stay", gone.as_ref().is_some_and(|f| ["edit-1.mp4", "edit-1-poster.jpg", "edit-1-teaser.mp4"].iter().all(|n| has(f, n))), format!("{gone:?}"));
    d.check("… and this Mac's copy of the original with them", !edits.exists(), edits.display().to_string());
    let s = d.after(500);
    d.check("baked, the row reads the copy's length", row(&s).ends_with(&format!("· {len}")), format!("{} (want {len})", row(&s)));
    std::thread::sleep(Duration::from_secs(3));
    runner.shot(d, LIBRARY, "library-edited")?;

    // Edit… again: the baked video is an ordinary one-file recording — no original kept,
    // nothing to revert — opened on its own file, fetched (no take holds it).
    d.act(Surface::Library, Action::EditRow(here.clone()));
    let s = d.wait("the baked video, ready", 60, editor_ready)?;
    let b = editor_body(&s).ok_or("no editor")?;
    let l = b.list.as_ref().expect("a list");
    d.check(
        "the baked video opens as itself: its length, no edit, no countdown, nothing to revert",
        near(b.source_duration, want.1, 0.6) && l.in_ == 0.0 && l.cuts.is_empty() && b.original_kept.is_none() && b.revert.as_ref().is_some_and(|r| !r.enabled) && b.markers == [4.0],
        format!("{} s, {} cuts, kept {:?}, revert {:?}, markers {:?}", b.source_duration, l.cuts.len(), b.original_kept, b.revert.as_ref().map(|r| r.enabled), b.markers),
    );
    let fetched_key = std::fs::read_to_string(edits.join(crate::runtime::editor::SOURCE_KEY)).unwrap_or_default();
    d.check("… from its own file, fetched — never the take's original", fetched_key.trim() == key("edit-1.mp4"), fetched_key);
    // The window held the original under the same name: its page reads the new file.
    let played = poll(10, || editor_layout(d).ok().and_then(|l| l.get("video")?.get(0)?.as_f64()).filter(|t| near(*t, want.1, 0.6)));
    d.check("… and the window plays that file, not the one it held before", played.is_some(), format!("{:?}", editor_layout(d).ok().and_then(|l| l.get("video").cloned())));
    // Opened again after playing through Done: it starts paused, reading Play (early.1 opened
    // every editor stuck on Pause, the button and Space doing nothing, 2026-10-18).
    starts_paused(d, "reopened after playing through Done, it starts paused, reading Play")?;
    std::thread::sleep(Duration::from_secs(3));
    runner.shot(d, crate::runtime::presenter::EDITOR, "editor-baked")?;
    // Closed while playing: the window is only hidden, so its page lets go of the video, and
    // nothing plays on behind it (the audio went on after the window closed, 2026-10-18).
    play_by_hand(d, &runner, origin)?;
    let playing = poll(3, || editor_layout(d).ok().filter(|l| l["paused"] == false));
    d.check("… and Play plays it", playing.is_some(), format!("{:?}", editor_layout(d).map(|l| l["paused"].clone())));
    d.rt.post(Event::WindowClosed(WindowKind::Editor));
    d.wait("✕ closed it", 10, |s| s.editor.is_none())?;
    let gone = poll(3, || editor_layout(d).ok().filter(|l| l["video"].is_null()));
    d.check("closed while playing, the window lets go of its video: nothing plays behind it", playing.is_some() && gone.is_some(), format!("playing {}, then {:?}", playing.is_some(), editor_layout(d).map(|l| l["video"].clone())));

    // The second video isn't on this Mac: the editor fetches it; an edit, Done, and Esc at
    // the question is Keep for 2 days — the original kept, nothing baked.
    let s = snapshot(&d.rt).ok_or("no scene")?;
    let away = rec_of(&s, AWAY.1).ok_or("no row for the second video")?;
    d.act(Surface::Library, Action::EditRow(away.clone()));
    let s = d.wait("the fetched video, ready", 60, editor_ready)?;
    let fetched = d.rt.home.edits().join(&away.0).join(crate::runtime::editor::SOURCE);
    let size = std::fs::metadata(&fetched).map(|m| m.len()).unwrap_or(0);
    let bytes = std::fs::metadata(&video).map(|m| m.len()).unwrap_or(1);
    d.check("not on this Mac: fetched from the space, whole", size == bytes && mode(&fetched) == "600", format!("{size} of {bytes} bytes, mode {}", mode(&fetched)));
    d.check("… its flags none, its length the record's", editor_body(&s).is_some_and(|b| b.markers.is_empty() && near(b.source_duration, length, 0.01)), "");
    // The window was closed while playing just before: this one starts paused, and Play plays.
    starts_paused(d, "opened after one closed while playing, it starts paused, reading Play")?;
    play_by_hand(d, &runner, origin)?;
    let playing = poll(3, || editor_layout(d).ok().filter(|l| l["paused"] == false && l["label"] == shroom_core::copy::editor_pause().as_str()));
    d.check("… and Play plays it, reading Pause", playing.is_some(), format!("{:?}", editor_layout(d).map(|l| (l["paused"].clone(), l["label"].clone()))));
    play_by_hand(d, &runner, origin)?;
    std::thread::sleep(Duration::from_secs(3));
    runner.shot(d, crate::runtime::presenter::EDITOR, "editor-fetched")?;
    // Discard (N70): a cut, ✕, and Discard — a real click — closes the window and writes
    // nothing; opened again, the video is as it was.
    d.rt.post(Event::Edit(EditStep::Op(Op::Cut { start: 3.0, end: 6.0 })));
    d.wait("a cut to discard", 5, |s| cuts_of(s).len() == 1)?;
    close_by_hand(d);
    d.wait("✕ asks", 10, |s| s.editor.as_ref().is_some_and(|e| e.id == "editor-closing"))?;
    let at = poll(5, || editor_layout(d).ok().and_then(|l| centre(origin, l.get("discard")?))).ok_or("no Discard on the page")?;
    runner.ask(d, serde_json::json!({ "do": "click", "at": [at.0, at.1] }))?;
    let gone = d.wait("Discard closed it", 10, |s| s.editor.is_none());
    let untouched = record_json(&s3, &away.0).is_some_and(|r| r["edits"].is_null());
    d.check("Discard, a real click, closes the window and saves nothing (N70)", gone.is_ok() && untouched, format!("closed {}, the record untouched {untouched}", gone.is_ok()));
    d.act(Surface::Library, Action::EditRow(away.clone()));
    let s = d.wait("the fetched video, opened again", 60, editor_ready)?;
    d.check("… opened again, it's as it was: no cut", cuts_of(&s).is_empty(), format!("{:?}", cuts_of(&s)));
    d.rt.post(Event::Edit(EditStep::Op(Op::Trim { in_: 1.0, out: 30.0 })));
    d.wait("trimmed", 5, |s| editor_body(s).and_then(|b| b.list.as_ref()).is_some_and(|l| l.in_ == 1.0))?;
    d.act(Surface::Editor, Action::EditorDone);
    d.wait("“Delete the original?” again", 240, |s| s.editor.as_ref().is_some_and(|e| e.id == "editor-keep-original"))?;
    d.key(Surface::Editor, Key::Esc);
    d.wait("Esc closed it", 10, |s| s.editor.is_none())?;
    std::thread::sleep(Duration::from_secs(2));
    let rec = record_json(&s3, &away.0).ok_or("no record")?;
    let (kept, until) = two_days(&rec);
    d.check("Esc is Keep for 2 days: the copy named, the original kept two days, nothing baked", kept && rec["edits"]["copy"]["rev"] == 1 && rec["media"]["mp4"] == format!("{}video.mp4", shroom_upload::keys::prefix(&away.0)), format!("{until} {}", rec["media"]));

    // The third: the window closed while the final video is made — the copy still lands,
    // and its two days are set.
    let s = snapshot(&d.rt).ok_or("no scene")?;
    let third = rec_of(&s, THIRD.1).ok_or("no row for the third video")?;
    d.act(Surface::Library, Action::EditRow(third.clone()));
    d.wait("the third video, ready", 60, editor_ready)?;
    d.rt.post(Event::Edit(EditStep::Op(Op::Cut { start: 5.0, end: 15.0 })));
    d.wait("cut", 5, |s| cuts_of(s).len() == 1)?;
    d.act(Surface::Editor, Action::EditorDone);
    d.wait("making", 30, |s| s.editor.as_ref().is_some_and(|e| e.id == "editor-making"))?;
    d.act(Surface::Editor, Action::EditorClose);
    d.wait("Close closed it mid-way", 10, |s| s.editor.is_none())?;
    let landed = poll(240, || record_json(&s3, &third.0).filter(|r| r["edits"]["copy"]["rev"] == 1));
    let (kept, until) = landed.as_ref().map(two_days).unwrap_or((false, serde_json::Value::Null));
    d.check("closed while it was made, the copy still lands, its two days set", kept, format!("{until}"));

    // The upkeep (B197 (d)), as the lane runs it at launch and every six hours: the second
    // video's two days are up (put back by hand), so it's baked; the third holds a newer
    // revision with no copy (as a copy that failed leaves it), and its video is on this Mac
    // (fetched when it opened), so the copy is made.
    // The pass reads the space itself, whatever the Library's cache has heard.
    let rewrite = |id: &str, change: &dyn Fn(&mut serde_json::Value)| -> Result<(), String> {
        let mut v = record_json(&s3, id).ok_or("no record")?;
        change(&mut v);
        s3.put(&shroom_upload::keys::meta(id), &serde_json::to_vec(&v).map_err(|e| e.to_string())?, "application/json").map_err(|e| e.to_string())
    };
    rewrite(&away.0, &|v| v["edits"]["keep_until"] = serde_json::json!("2026-01-01T00:00:00Z"))?;
    rewrite(&third.0, &|v| v["edits"]["rev"] = serde_json::json!(2))?;
    let pass = crate::runtime::editor::upkeep_pass(&d.rt);
    d.check("the upkeep bakes one and makes one copy", pass == Ok((1, 1)), format!("{pass:?}"));
    let away_rec = record_json(&s3, &away.0).unwrap_or_default();
    d.check("… the second, past its two days, is the video now", away_rec["edits"].is_null() && away_rec["media"]["mp4"] == format!("{}edit-1.mp4", shroom_upload::keys::prefix(&away.0)), away_rec["media"].to_string());
    let third_rec = record_json(&s3, &third.0).unwrap_or_default();
    let (kept, until) = two_days(&third_rec);
    d.check("… the third has the copy its revision lacked, its two days set", third_rec["edits"]["copy"]["rev"] == 2 && third_rec["edits"]["copy"]["file"] == "edit-2.mp4" && kept, format!("{} {until}", third_rec["edits"]["copy"]));
    d.check("… and what this Mac kept of the baked one's original is gone", !d.rt.home.edits().join(&away.0).exists(), "");

    // The owner page's Edit in shroom (N69, B221): the link a browser hands over when shroom
    // opens it — handed in here as macOS would, since a dev build never claims the scheme.
    let space = read_file(d, "space.json").ok_or("no space.json")?;
    let host = space["host"].as_str().unwrap_or_default().to_string();
    crate::runtime::opened(&d.rt, &format!("shroom://edit/{host}/{}", here.0));
    let s = d.wait("the page's Edit in shroom opens the editor", 30, |s| editor_body(s).is_some_and(|b| b.rec == here.0))?;
    d.check("Edit in shroom: the editor, on the page's video (N69)", editor_body(&s).is_some_and(|b| b.title == format!("Edit — {}", HERE.1)), editor_body(&s).map(|b| b.title.clone()).unwrap_or_default());
    // Pressed again for another video while it holds this one: forward as it is, saying why.
    crate::runtime::opened(&d.rt, &format!("shroom://edit/{host}/{}", away.0));
    let s = d.wait("pressed for another video", 10, |s| editor_body(s).is_some_and(|b| b.footer == shroom_core::copy::editing_another()))?;
    d.check("… pressed for another video: it stays on this one, its footer saying why", editor_body(&s).is_some_and(|b| b.rec == here.0), "");
    std::thread::sleep(Duration::from_millis(600));
    runner.shot(d, crate::runtime::presenter::EDITOR, "editor-editing-another")?;
    // Edit… in the Library for that video says the same (it said nothing there, 2026-10-19):
    // the next step puts the page's line away, then Edit… brings it back.
    d.rt.post(Event::Edit(EditStep::Undo));
    d.wait("the line, put away by the next step", 5, |s| editor_body(s).is_some_and(|b| b.footer != shroom_core::copy::editing_another()))?;
    d.act(Surface::Library, Action::EditRow(away.clone()));
    let said = d.wait("Edit… in the Library for another video", 10, |s| editor_body(s).is_some_and(|b| b.footer == shroom_core::copy::editing_another()));
    d.check(
        "… Edit… in the Library for another video: the same, it stays on this one and says why",
        said.as_ref().is_ok_and(|s| editor_body(s).is_some_and(|b| b.rec == here.0)),
        format!("{:?}", snapshot(&d.rt).and_then(|s| editor_body(&s).map(|b| (b.rec.clone(), b.footer.clone())))),
    );
    d.rt.post(Event::WindowClosed(WindowKind::Editor));
    d.wait("closed", 10, |s| s.editor.is_none())?;
    // Another space's video: the popover opens by itself and says so, once.
    crate::runtime::opened(&d.rt, &format!("shroom://edit/v.someoneelse.example.com/{}", here.0));
    let s = d.wait("another space's video", 10, |s| s.popover.as_ref().is_some_and(|p| p.status_line.as_deref() == Some(shroom_core::copy::edit_other_space().as_str())))?;
    d.check("… another space's video: the popover says why, without the keyboard (N69)", s.popover.as_ref().is_some_and(|p| !p.activates && p.away_closes), "");
    // Its panel is left out of every capture, so no picture of it can be taken: it's read as
    // macOS has it instead — on screen, and not holding the keyboard (law 5).
    let panel = || {
        use crate::runtime::presenter::{Overlay, overlay_window};
        let app = d.app.clone();
        d.on_main(move || overlay_window(&app, Overlay::Popover).map(|w| (w.isVisible(), w.isKeyWindow()))).flatten()
    };
    let shown = poll(5, || panel().filter(|(visible, _)| *visible));
    d.check("… on screen, the keyboard left where it was", shown == Some((true, false)), format!("{:?}", panel()));
    d.rt.post(Event::PopoverDismissed);
    let s = d.wait("put away", 10, |s| s.popover.is_none())?;
    d.check("… said once: put away, it's gone", s.popover.is_none(), "");

    // What a report would carry of all that (DECISIONS B199 (t)): a line a step, in numbers.
    let log = std::fs::read_to_string(d.rt.home.root.join("logs/run.log")).unwrap_or_default();
    let (h, a) = (crate::runtime::short(&here.0), crate::runtime::short(&away.0));
    for (what, needle) in [
        ("the open", format!("editor: {h} opened: ")),
        ("the save, its ranges and seconds", format!("editor: {h} saved as revision 1 in ")),
        ("the final video's cut: encoder, parts, size, time", format!("editor: {h}'s final video of revision 1 cut with ")),
        ("… and its naming", format!("editor: {h}'s final video of revision 1 is up and named")),
        ("the edge dragged", "core edit-resize".to_string()),
        ("the page's Edit in shroom, never its link", "link: the page's Edit in shroom".to_string()),
        ("… the editor opened by it", "core edit-link-open".to_string()),
        ("… brought forward, another video asked", "core edit-link-forward".to_string()),
        ("… another space's, said", "core edit-link-no".to_string()),
        ("Yes, delete", "core original-deleted".to_string()),
        ("the bake, the files it deleted", format!("editor: {h}'s edit is the video now (revision 1): ")),
        ("the upkeep's bake", format!("editor upkeep: {a}'s edit is the video now")),
        ("the upkeep's counts", "due, 1 baked (".to_string()),
    ] {
        let line = log.lines().find(|l| l.contains(&needle));
        d.check(&format!("run.log: {what}"), line.is_some(), line.unwrap_or(&needle).to_string());
    }
    d.check("run.log: never the page's link, its space or its video (B221 (e))", !log.contains("shroom://edit/") && !log.contains("someoneelse"), "");

    d.rt.post(Event::WindowClosed(WindowKind::Library));
    d.wait("the Library closed", 5, |s| s.library.is_none())?;
    Ok(())
}

// ── Region 6: joining, Members, switching (`--space-base <local Worker>`) ──
//
// Against `worker/dev/serve.mjs` — the real `worker.js` on 127.0.0.1, one in-memory bucket per
// space — with `--space-base` (the member client's only route in a dev build) and the L1 fake
// for anything a token would touch. The drive seeds the local Worker through its `/_dev/`
// knobs (only hashes and verifiers, as the app itself sends) and the throwaway home, before
// the core boots. Each report carries the state sequence and its checks, the law-13 ones
// included: no join code, device secret or minted credential anywhere but the files §5
// allows them in.

/// The two spaces the local Worker serves for the drives.
const SPACE_A: &str = "v.alpha.workers.dev";
const SPACE_B: &str = "v.beta.workers.dev";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Region6 {
    /// A member's first launch from the installer: the invite file, member-welcome, Join,
    /// done · member, all-set · member, a recording minted through `/mint`, uploaded.
    Join,
    /// An owner's Members tab: invite Ani, copy the link, the row ages, cancel it; Add
    /// another Mac.
    Members,
    /// A member of space A opens a link for space B: join-switch-confirm, Join, the old
    /// space's files gone and the new ones in place.
    Switch,
    /// The owner's token Mac whose wiring enroll didn't land: Members enrolls it (the 401 is
    /// never a removal), the owner page's key, a recording minted locally with its own
    /// ledger entry, and the Library over the token's S3 pair.
    Token,
    /// L6 (TESTING-PLAN §5; DECISIONS B65): the owner's token Mac, one of two app instances
    /// on one local Worker — invites Mher, removes him, invites him again. Steps on the
    /// runner's signals (`tests/l6/two-macs.mjs`).
    L6Owner,
    /// L6: Mher's Mac, the other instance — joins from the installer's hand-off, records
    /// (the pattern, the beep), is removed with a take still waiting and saves it to Movies,
    /// joins again from the page's Open shroom, and the waiting take drains.
    L6Member,
    /// The token Mac whose space's `R2_PARENT` binding is another token's — stale (DECISIONS
    /// B90; ARCHITECTURE §16 #46, B42's open note): the launch's enroll meets `stale-parent`,
    /// the app re-places the binding on its own (`wire::place_parent`, through the L1 fake,
    /// which hands it to the local Worker with `--relay-parent`), the enroll lands, and the
    /// next credential minted from the token and the Library's listing go through. Nothing
    /// else of the space is rewritten. No recording.
    Stale,
    /// The heartbeat's member leg (DECISIONS B68): a second instance of the installed dev
    /// app on a throwaway home joins the heartbeat's fenced live space from the owner's
    /// invite, records the pattern into it, and — once the owner removed it — reads
    /// `member-revoked` from the live page's `/me`. Live: no fake, no local Worker.
    HbMember,
    /// N68 on the owner's token Mac: a space with videos nobody has watched in months, one
    /// played today and one too small to matter. The Library's line, Suggested to clean up,
    /// each row's cost past the free 10 GB, Keep, a one-tap Delete and its Undo, a Delete
    /// that goes, and Settings · General's Your videos with Count plays switched off and on.
    Cleanup,
}

impl Region6 {
    pub fn parse(s: &str) -> Option<Region6> {
        Some(match s {
            "join" => Region6::Join,
            "members" => Region6::Members,
            "switch" => Region6::Switch,
            "token" => Region6::Token,
            "stale" => Region6::Stale,
            "l6-owner" => Region6::L6Owner,
            "l6-member" => Region6::L6Member,
            "hb-member" => Region6::HbMember,
            "cleanup" => Region6::Cleanup,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            Region6::Join => "join",
            Region6::Members => "members",
            Region6::Switch => "switch",
            Region6::Token => "token",
            Region6::Stale => "stale",
            Region6::L6Owner => "l6-owner",
            Region6::L6Member => "l6-member",
            Region6::HbMember => "hb-member",
            Region6::Cleanup => "cleanup",
        }
    }

    pub fn probe(self) -> fn(&Scene) -> String {
        match self {
            Region6::Join | Region6::Switch => |s| window_ids(s.onboarding.as_ref(), None),
            Region6::Members | Region6::Token => |s| window_ids(s.settings.as_ref(), s.settings.as_ref().and_then(|w| w.body.sheet.as_ref())),
            Region6::Stale | Region6::L6Owner | Region6::L6Member | Region6::HbMember | Region6::Cleanup => |s| s.state_ids().join(" | "),
        }
    }
}

/// What the seed made that the drive needs — secrets held only in this process's memory
/// (never in the report, the log or an argv).
pub struct Region6Seed {
    /// The page's Open shroom for space B (the switch drive).
    link: Option<String>,
    /// Every join code the seed made: each must appear nowhere but `invite` / `join.json`.
    codes: Vec<String>,
    /// The seeded Mac's device secret (members, switch).
    seeded_secret: Option<String>,
    /// The seeded member's id (switch).
    member: Option<String>,
    /// The token's id on the fake (token): the parent's key id, a local mint's issuer.
    token_id: Option<String>,
    /// The token drive's precondition, read before the app started: the space had never
    /// seen the device key on disk (its wiring enroll lost).
    unknown_at_seed: Option<bool>,
    /// The stale drive's preconditions, read before the app started: the binding's key id
    /// (another token's), this token's own key id (not secret), whether a credential minted
    /// from this token was refused, and the space's objects but the roster (key, size).
    stale: Option<StaleSeed>,
}

/// What the stale drive saw before the app started.
pub struct StaleSeed {
    binding_key: String,
    token_key: String,
    refused_before: Result<(), String>,
    objects: Vec<(String, u64)>,
    roster: serde_json::Value,
}

fn knob(base: &str, host: &str, what: &str, body: serde_json::Value) -> Result<(), String> {
    let url = format!("{}/_dev/space/{host}/{what}", base.trim_end_matches('/'));
    ureq::post(&url).header("content-type", "application/json").send(body.to_string()).map_err(|e| format!("the local Worker's {what} knob: {e}"))?;
    Ok(())
}

fn knob_get(base: &str, host: &str, what: &str) -> Result<serde_json::Value, String> {
    let url = format!("{}/_dev/space/{host}/{what}", base.trim_end_matches('/'));
    let mut res = ureq::get(&url).call().map_err(|e| format!("the local Worker's {what}: {e}"))?;
    let text = res.body_mut().read_to_string().map_err(|e| e.to_string())?;
    serde_json::from_str(&text).map_err(|e| e.to_string())
}

/// A pending member invite on `host`, made the way the inviting Mac makes one: the code
/// stays here, only its hash goes to the space.
fn seed_invite(base: &str, host: &str, label: &str, inviter: &str) -> Result<String, String> {
    use shroom_cloud::device;
    let code = device::new_join_code();
    knob(base, host, "invite", serde_json::json!({ "id": device::new_invite_id(), "codeHash": device::hash(&code), "kind": "member", "label": label, "inviter": inviter }))?;
    Ok(code)
}

/// The throwaway home and the local Worker, as each drive starts.
pub fn seed_region6(which: Region6, flags: &crate::runtime::DevFlags) -> Result<Region6Seed, String> {
    use shroom_cloud::device::{self, DeviceKey};
    let root = flags.home.clone().ok_or("no --home")?;
    let h = home::Home::at(root);
    h.ensure().map_err(|e| e.to_string())?;
    let mut seed = Region6Seed { link: None, codes: vec![], seeded_secret: None, member: None, token_id: None, unknown_at_seed: None, stale: None };
    // The heartbeat's member: live, so nothing to seed but the installer's hand-off the
    // heartbeat moved in (its code a secret to keep track of, never to log).
    if which == Region6::HbMember {
        let invite: serde_json::Value = h.read_json("invite").ok_or("no invite handed over")?;
        let link = invite["link"].as_str().ok_or("the hand-off has no link")?;
        let (host, code) = shroom_cloud::member::parse_link(link).ok_or("the hand-off's link doesn't parse")?;
        let e2e = flags.e2e && crate::runtime::e2e_host(&host);
        if !host.starts_with(crate::runtime::TEST_PREFIX) && !e2e {
            return Err("the heartbeat's member joins only a fenced shroom-test- space, or the e2e run's".into());
        }
        seed.codes.push(code);
        return Ok(seed);
    }
    let base = flags.space_base.clone().ok_or("no --space-base")?;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
    match which {
        Region6::HbMember => unreachable!("seeded above"),
        Region6::Join => {
            // The installer's hand-off (§6.12): the link by file, mode 600, its shape.
            let code = seed_invite(&base, SPACE_A, "Mher", "Bagrat")?;
            let installer = serde_json::json!({ "v": 1, "link": format!("{SPACE_A}/join/{code}"), "arrived": shroom_cloud::registry::iso((now / 1000) as u64) });
            h.write_json("invite", &installer).map_err(|e| e.to_string())?;
            seed.codes.push(code);
        }
        Region6::Members | Region6::Cleanup => {
            // The owner's token Mac, provisioned and enrolled (member #1, the token flag).
            seed_provisioned(flags)?;
            let mut space: SpaceIdentity = h.read_json("space.json").ok_or("no space.json")?;
            space.host = SPACE_A.into();
            space.subdomain = "alpha".into();
            space.owner_name = "Bagrat".into();
            let key = DeviceKey::generate(SPACE_A);
            h.write_json("device.key", &key).map_err(|e| e.to_string())?;
            h.write_json("space.json", &space).map_err(|e| e.to_string())?;
            knob(&base, SPACE_A, "device", serde_json::json!({ "id": key.device, "verifier": key.verifier(), "member": "m1", "name": "MacBook Pro", "token": true }))?;
            seed.seeded_secret = Some(key.secret);
        }
        Region6::Switch => {
            // A member of space A (Mher, on Bagrat's space)…
            let member = format!("m_{}", device::random_b64(12));
            let key = DeviceKey::generate(SPACE_A);
            let space = SpaceIdentity {
                account_id: "a".repeat(32),
                label: "v".into(),
                subdomain: "alpha".into(),
                host: SPACE_A.into(),
                bucket: shroom_cloud::wire::BUCKET.into(),
                role: Role::Member,
                member_id: member.clone(),
                owner_name: "Bagrat".into(),
                account_name: None,
                foreign_v: false,
            };
            h.write_json("device.key", &key).map_err(|e| e.to_string())?;
            h.write_json("space.json", &space).map_err(|e| e.to_string())?;
            h.write_json("primed.json", &crate::runtime::home_files::Primed { v: 1, at: now }).map_err(|e| e.to_string())?;
            knob(&base, SPACE_A, "device", serde_json::json!({ "id": key.device, "verifier": key.verifier(), "member": member, "label": "Mher" }))?;
            // …and an invite to Ani's space B, opened from its page.
            let code = seed_invite(&base, SPACE_B, "Mher", "Ani")?;
            seed.link = Some(format!("shroom://join/{SPACE_B}/{code}"));
            seed.codes.push(code);
            seed.seeded_secret = Some(key.secret);
            seed.member = Some(member);
        }
        Region6::L6Owner => {
            // The owner's token Mac on space A, enrolled (member #1, the token flag), the
            // space's parent its token's own pair — as wiring leaves a real space.
            seed_provisioned(flags)?;
            let mut space: SpaceIdentity = h.read_json("space.json").ok_or("no space.json")?;
            space.host = SPACE_A.into();
            space.subdomain = "alpha".into();
            space.owner_name = "Bagrat".into();
            let key = DeviceKey::generate(SPACE_A);
            h.write_json("device.key", &key).map_err(|e| e.to_string())?;
            h.write_json("space.json", &space).map_err(|e| e.to_string())?;
            let token = h.saved_token().ok_or("no token")?.token;
            let (api, r2) = flags.cloud_fake.clone().ok_or("no --cloud-fake")?;
            let cloud = shroom_cloud::Cloud::new(&token, shroom_cloud::Policy::Unfenced).with_endpoints(&api, &r2);
            let token_id = cloud.verify(ACCOUNT).map_err(|e| e.to_string())?.id;
            let pair = cloud.s3_credentials(&token_id);
            knob(&base, SPACE_A, "parent", serde_json::json!({ "account_id": ACCOUNT, "bucket": shroom_cloud::wire::BUCKET, "key_id": pair.access_key_id, "secret": pair.secret_access_key }))?;
            knob(&base, SPACE_A, "device", serde_json::json!({ "id": key.device, "verifier": key.verifier(), "member": "m1", "name": "MacBook Pro", "token": true }))?;
            seed.codes.push(pair.secret_access_key);
            seed.seeded_secret = Some(key.secret);
            seed.token_id = Some(token_id);
        }
        Region6::L6Member => {
            // An empty Mac but for the installer's hand-off, which the runner moved in from
            // the owner's drive: its code is a secret to keep track of, never to log.
            let invite: serde_json::Value = h.read_json("invite").ok_or("no invite handed over")?;
            let link = invite["link"].as_str().ok_or("the hand-off has no link")?;
            let (_, code) = shroom_cloud::member::parse_link(link).ok_or("the hand-off's link doesn't parse")?;
            seed.codes.push(code);
        }
        Region6::Stale => {
            // The owner's token Mac, provisioned, its device key not yet enrolled — and its
            // space's binding left as the local Worker made it: the space's own derived pair,
            // another token's. Never this token's: so it's stale for this Mac (§3.4).
            seed_provisioned(flags)?;
            let mut space: SpaceIdentity = h.read_json("space.json").ok_or("no space.json")?;
            space.owner_name = "Bagrat".into();
            h.write_json("space.json", &space).map_err(|e| e.to_string())?;
            let key = DeviceKey::generate(HOST);
            h.write_json("device.key", &key).map_err(|e| e.to_string())?;
            let token = h.saved_token().ok_or("no token")?.token;
            let (api, r2) = flags.cloud_fake.clone().ok_or("no --cloud-fake")?;
            let cloud = shroom_cloud::Cloud::new(&token, shroom_cloud::Policy::Unfenced).with_endpoints(&api, &r2);
            let token_id = cloud.verify(ACCOUNT).map_err(|e| e.to_string())?.id;
            let pair = cloud.s3_credentials(&token_id);
            let binding = knob_get(&base, HOST, "parent")?;
            let objects = objects(&base, HOST)?.iter().filter(|o| o["key"] != ".shroom/registry/roster.json").map(|o| (o["key"].as_str().unwrap_or_default().to_string(), o["size"].as_u64().unwrap_or(0))).collect();
            seed.stale = Some(StaleSeed {
                binding_key: binding["key_id"].as_str().unwrap_or_default().into(),
                token_key: pair.access_key_id.clone(),
                // The stale binding bites: the token Mac's own mint (B32) is refused by the space.
                refused_before: minted_put(&base, &cloud, &token_id, "StaleProbeBefore"),
                objects,
                roster: knob_get(&base, HOST, "roster")?,
            });
            seed.codes.push(pair.secret_access_key);
            seed.seeded_secret = Some(key.secret);
            seed.token_id = Some(token_id);
        }
        Region6::Token => {
            // The owner's token Mac, provisioned, its wiring enroll lost: a device key on
            // disk the space has never seen.
            seed_provisioned(flags)?;
            let mut space: SpaceIdentity = h.read_json("space.json").ok_or("no space.json")?;
            space.owner_name = "Bagrat".into();
            h.write_json("space.json", &space).map_err(|e| e.to_string())?;
            let key = DeviceKey::generate(HOST);
            h.write_json("device.key", &key).map_err(|e| e.to_string())?;
            // The space's parent is this token's derived pair, as wiring placed it.
            let token = h.saved_token().ok_or("no token")?.token;
            let (api, r2) = flags.cloud_fake.clone().ok_or("no --cloud-fake")?;
            let cloud = shroom_cloud::Cloud::new(&token, shroom_cloud::Policy::Unfenced).with_endpoints(&api, &r2);
            let token_id = cloud.verify(ACCOUNT).map_err(|e| e.to_string())?.id;
            let pair = cloud.s3_credentials(&token_id);
            knob(&base, HOST, "parent", serde_json::json!({ "account_id": ACCOUNT, "bucket": shroom_cloud::wire::BUCKET, "key_id": pair.access_key_id, "secret": pair.secret_access_key }))?;
            // The parent secret is the token's own S3 key: allowed nowhere on disk but token.json's source.
            seed.codes.push(pair.secret_access_key);
            // Read before the app starts: B51's launch self-check enrolls the key once the
            // app is up, so this is the only moment the lost enroll can be seen.
            seed.unknown_at_seed = Some(knob_get(&base, HOST, "roster")?["devices"][&key.device].is_null());
            seed.seeded_secret = Some(key.secret);
            seed.token_id = Some(token_id);
        }
    }
    Ok(seed)
}

pub fn drive_region6(app: AppHandle, rt: Runtime, which: Region6, out: PathBuf, seed: Region6Seed) {
    let d = Drive { app: app.clone(), rt: rt.clone(), t0: Instant::now(), report: Arc::new(Mutex::new(Report { drive: which.name().into(), ..Report::default() })), seen: Arc::default() };
    let result = match which {
        Region6::Join => join_drive(&d, &seed),
        Region6::Members => members_drive(&d, &seed),
        Region6::Switch => switch_drive(&d, &seed),
        Region6::Token => token_drive(&d, &seed),
        Region6::Stale => stale_drive(&d, &seed),
        Region6::L6Owner => l6_owner(&d, out.parent().unwrap_or(std::path::Path::new("."))),
        Region6::L6Member => l6_member(&d, out.parent().unwrap_or(std::path::Path::new("."))),
        Region6::HbMember => hb_member(&d, out.parent().unwrap_or(std::path::Path::new("."))),
        Region6::Cleanup => cleanup_drive(&d),
    };
    if let Err(e) = result {
        d.report.lock().unwrap().error = Some(e);
    }
    // Law 13, wherever the drive ended: the secrets live only where §5 allows them.
    secrets_stay_put(&d, &seed);
    let code = {
        let mut r = d.report.lock().unwrap();
        finish(&rt, &mut r);
        write_json(&out, &*r)
    };
    rt.ending_ours.store(true, std::sync::atomic::Ordering::SeqCst);
    app.exit(code);
}

/// The onboarding window shows this state, by its id or one of its faces (`done · member`).
fn is_onb(id: &'static str) -> impl Fn(&Scene) -> bool {
    move |s| s.onboarding.as_ref().is_some_and(|o| o.id == id || o.faces.contains(&id))
}

/// Every secret this run made or was handed, with the files §5 allows each in.
fn secrets_stay_put(d: &Drive, seed: &Region6Seed) {
    use shroom_cloud::device::{CredFile, DeviceKey, JoinFile};
    let root = d.rt.home.root.clone();
    let mut secrets: Vec<(&'static str, String)> = seed.codes.iter().chain(d.seen.lock().unwrap().iter()).map(|c| ("a join code or device secret", c.clone())).collect();
    if let Some(s) = &seed.seeded_secret {
        secrets.push(("the seeded device secret", s.clone()));
    }
    if let Some(k) = home::read_json::<DeviceKey>(&root.join("device.key")) {
        secrets.push(("this Mac's device secret", k.secret));
    }
    if let Some(j) = home::read_json::<JoinFile>(&root.join("join.json")) {
        secrets.push(("join.json's device secret", j.secret));
    }
    for dir in crate::runtime::takes::dirs(&d.rt.home.takes()) {
        if let Some(c) = home::read_json::<CredFile>(&dir.join("cred.json")) {
            secrets.push(("a minted secret key", c.secret_access_key));
            secrets.push(("a minted session token", c.session_token));
        }
    }
    let allowed = |p: &std::path::Path| {
        let rel = p.strip_prefix(&root).unwrap_or(p).display().to_string();
        matches!(rel.as_str(), "invite" | "join.json" | "device.key") || (rel.starts_with("takes/") && rel.ends_with("/cred.json"))
    };
    let mut leaks = Vec::new();
    // Regular files only: a take holds pipes (its audio's FIFO) a read would block on.
    for p in files(&root).into_iter().filter(|p| !allowed(p) && std::fs::symlink_metadata(p).is_ok_and(|m| m.is_file())) {
        let Ok(bytes) = std::fs::read(&p) else { continue };
        for (what, s) in &secrets {
            if bytes.windows(s.len()).any(|w| w == s.as_bytes()) {
                leaks.push(format!("{what} in {}", p.strip_prefix(&root).unwrap_or(&p).display()));
            }
        }
    }
    d.check("law 13: no secret in run.log or any file §5 doesn't allow it in", leaks.is_empty(), format!("{leaks:?}"));
    let argv: Vec<String> = std::env::args().collect();
    let in_argv = secrets.iter().any(|(_, s)| argv.iter().any(|a| a.contains(s.as_str())));
    d.check("law 13: no secret in this app's argv", !in_argv, format!("{} args", argv.len()));
    if let Some(s) = snapshot(&d.rt) {
        let scene = serde_json::to_string(&(&s.onboarding, &s.settings, &s.popover, &s.library)).unwrap_or_default();
        let in_scene = secrets.iter().any(|(_, x)| scene.contains(x.as_str()));
        d.check("law 13: no secret in the Scene the webviews read", !in_scene, "the last Scene's slices");
    }
}

fn read_file(d: &Drive, name: &str) -> Option<serde_json::Value> {
    home::read_json::<serde_json::Value>(&d.rt.home.path(name))
}

fn join_drive(d: &Drive, _seed: &Region6Seed) -> Result<(), String> {
    use shroom_cloud::device::DeviceKey;
    let base = d.rt.space_base.clone().ok_or("no --space-base")?;
    // The installer opened shroom with `--invite`: the file's arrival peeks and opens the
    // welcome — asked (the person just ran it).
    let s = d.wait("member-welcome", 40, is_onb("member-welcome"))?;
    let w = s.onboarding.as_ref().expect("the window");
    d.check("the person opened it: the window takes the keyboard", w.activates, w.activates.to_string());
    d.check("Return is Join", s.ret(Surface::Onboarding) == Some(Action::Join), format!("{:?}", s.ret(Surface::Onboarding)));
    let invite = std::fs::read_to_string(d.rt.home.path("invite")).unwrap_or_default();
    d.check("the installer's file was rewritten in the app's shape", !invite.contains("\"link\"") && invite.contains(SPACE_A) && invite.contains("\"inviter\""), "invite");
    d.check("no join.json before Join", !d.rt.home.path("join.json").exists(), "join.json");

    d.act(Surface::Onboarding, Action::Join);
    d.wait("member-done", 40, is_onb("member-done"))?;
    let key = home::read_json::<DeviceKey>(&d.rt.home.path("device.key"));
    let space = read_file(d, "space.json");
    d.check("device.key is for the space joined", key.as_ref().is_some_and(|k| k.host == SPACE_A), format!("{:?}", key.as_ref().map(|k| &k.host)));
    let role = space.as_ref().and_then(|v| v["role"].as_str().map(String::from));
    d.check("space.json: a member, its member and device ids", role.as_deref() == Some("Member") && space.as_ref().is_some_and(|v| v["member_id"].as_str().is_some_and(|m| m.starts_with("m_")) && key.as_ref().is_some_and(|k| v["device_id"] == k.device.as_str())), format!("{space:?}"));
    d.check("space.json names the space's owner", space.as_ref().is_some_and(|v| v["owner_name"] == "Bagrat"), "owner_name");
    d.check("the invite and join.json are gone", !d.rt.home.path("invite").exists() && !d.rt.home.path("join.json").exists(), "invite, join.json");
    let roster = knob_get(&base, SPACE_A, "roster")?;
    let member = space.as_ref().and_then(|v| v["member_id"].as_str().map(String::from)).unwrap_or_default();
    let device = key.as_ref().map(|k| k.device.clone()).unwrap_or_default();
    d.check("the space has this Mac, active, under its new member", roster["devices"][&device]["state"] == "active" && roster["devices"][&device]["member"] == member.as_str() && roster["members"][&member]["label"] == "Mher", "the roster");
    d.check("the device key's verifier is what the space holds", key.as_ref().is_some_and(|k| roster["devices"][&device]["verifier"] == k.verifier().as_str()), "verifier");

    // Region 3's chapter: this Mac's grants are given — the warm-up, then all-set.
    d.act(Surface::Onboarding, Action::AllowRecording);
    let s = d.wait("member-all-set", 40, is_onb("member-all-set"))?;
    d.check("two doors: members don't invite", !s.offers(Surface::Onboarding, &Action::InviteSomeone), "member-all-set");

    // The first video: Record now → config → Return → recording → Stop.
    let (dir, rec) = record_one(d, || d.act(Surface::Onboarding, Action::RecordNow))?;

    // The mint went through the space's Worker: its ledger names this member, and the bytes
    // and the record are in the space's bucket — put there with the minted credential.
    let objs = knob_get(&base, SPACE_A, "objects")?;
    let keys: Vec<String> = objs.as_array().map(|a| a.iter().filter_map(|o| o["key"].as_str().map(String::from)).collect()).unwrap_or_default();
    let ledger = objs.as_array().and_then(|a| a.iter().find(|o| o["key"] == format!(".shroom/registry/recs/{rec}").as_str()).cloned());
    d.check("the Worker wrote the ledger at /mint, naming this member", ledger.as_ref().is_some_and(|l| l["customMetadata"]["m"] == member.as_str()), format!("{ledger:?}"));
    d.check("the record landed in the space's bucket", keys.contains(&format!("meta/{rec}.json")), format!("{} keys", keys.len()));
    d.check("the member's /mint spent its device's upload budget (H4)", keys.contains(&format!(".shroom/registry/rate/{device}")), "rate counter");
    d.check("the video's bytes landed under its prefix", keys.iter().any(|k| k == &format!("rec/{rec}/master.m3u8")) && keys.iter().filter(|k| k.starts_with(&format!("rec/{rec}/"))).count() >= 3, "rec/<id>/");
    let cred = cred_claims(&dir);
    d.check(
        "cred.json holds the upload credential for this recording alone",
        cred.as_ref().is_some_and(|(c, claims)| c.purpose == "upload" && claims["paths"]["prefixPaths"][0] == format!("rec/{rec}/").as_str()),
        format!("{:?}", cred.as_ref().map(|(_, c)| &c["paths"])),
    );
    use std::os::unix::fs::PermissionsExt;
    let mode = |p: &std::path::Path| std::fs::metadata(p).map(|m| format!("{:o}", m.permissions().mode() & 0o777)).unwrap_or_default();
    d.check("device.key, space.json and cred.json are mode 600", [d.rt.home.path("device.key"), d.rt.home.path("space.json"), dir.join("cred.json")].iter().all(|p| mode(p) == "600"), "modes");
    let log = std::fs::read_to_string(d.rt.home.root.join("logs/run.log")).unwrap_or_default();
    d.check("no Cloudflare call: a member has no token", !log.contains("wizard") && d.rt.home.token().is_none(), "run.log");
    d.check("the stash forgot the invite once joined", d.rt.stash.lock().unwrap().is_empty(), "stash");

    // The member's Library (§6.8): the space's Worker lists this member's videos, and a
    // rename and a delete run under a `manage` credential minted for that one video.
    d.open(Action::OpenLibrary)?;
    let s = d.wait("the Library shows the member's video", 30, |s| lib_row(s, &rec))?;
    let log = std::fs::read_to_string(d.rt.home.root.join("logs/run.log")).unwrap_or_default();
    d.check("refreshed through the space, scoped to this member", log.contains("library refreshed through the space (Mine)"), "run.log");
    d.check("the row is this member's own (no name tag)", s.library.as_ref().is_some_and(|l| l.body.rows.iter().any(|r| r.key == rec)), "row");
    d.act(Surface::Library, Action::StartRename(RecId(rec.clone())));
    d.wait("renaming", 5, |s| s.library.as_ref().is_some_and(|l| l.body.rows.iter().any(|r| r.rename.is_some())))?;
    d.rt.field(Field::Title, "Mher's first".into());
    d.after(150);
    d.key(Surface::Library, Key::Return);
    d.wait("renamed", 20, |s| rec_of(s, "Mher's first").is_some_and(|r| r.0 == rec))?;
    let titled = poll(10, || object_json(&objects(&base, SPACE_A).ok()?, &format!("meta/{rec}.json")).filter(|m| m["title"] == "Mher's first"));
    d.check("the record's title changed in the space's bucket (a manage mint)", titled.is_some(), "meta title");
    d.act(Surface::Library, Action::DeleteRow(RecId(rec.clone())));
    d.wait("the delete sheet", 5, |s| s.library.as_ref().is_some_and(|l| l.body.sheet.is_some()))?;
    d.act(Surface::Library, Action::ConfirmDelete);
    d.wait("deleted", 20, |s| !lib_row(s, &rec) && s.library.as_ref().is_some_and(|l| l.body.sheet.is_none()))?;
    let objs = objects(&base, SPACE_A)?;
    let tomb = object_json(&objs, &format!("meta/{rec}.json")).is_some_and(|m| m["state"] == "deleted");
    let left = objs.iter().filter(|o| o["key"].as_str().is_some_and(|k| k.starts_with(&format!("rec/{rec}/")))).count();
    d.check("the tombstone is in the bucket and the bytes are gone", tomb && left == 0, format!("tombstone {tomb}, {left} objects under rec/"));
    d.rt.post(Event::WindowClosed(WindowKind::Library));
    d.wait("closed", 5, |s| s.library.is_none())?;
    Ok(())
}

/// One short recording, like a person: `start` opens the config popover, Return records,
/// four seconds of the test pattern, the mushroom pauses it, Stop — then the take is
/// published and uploaded. Its folder and recording id.
fn record_one(d: &Drive, start: impl Fn()) -> Result<(PathBuf, String), String> {
    record_take(d, start, true)
}

/// [`record_one`], waiting for the upload only when `upload` says so.
fn record_take(d: &Drive, start: impl Fn(), upload: bool) -> Result<(PathBuf, String), String> {
    #[repr(C)]
    struct CGPoint {
        x: f64,
        y: f64,
    }
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGWarpMouseCursorPosition(p: CGPoint) -> i32;
    }
    let pattern = crate::harness::cover_with_pattern(&d.app)?;
    unsafe { CGWarpMouseCursorPosition(CGPoint { x: 4000.0, y: 3000.0 }) };
    let recorded = (|| {
        start();
        let s = d.wait("config", 10, |s| s.popover.as_ref().is_some_and(|p| p.id == "config"))?;
        // Only the pattern is measured: the camera picker starts on a camera (N43), so its bubble goes.
        if s.bubble.is_some() {
            d.act(Surface::Popover, Action::SetCamera(None));
            d.wait("the bubble off", 5, |s| s.bubble.is_none())?;
        }
        // Only the display the pattern covers is ever recorded (b4 #16): chosen, then
        // checked on the Scene the Return starts from.
        let covered = shroom_capture::main_display_id();
        d.act(Surface::Popover, Action::SetSource(shroom_core::world::SourceId::Display(covered)));
        let s = d.wait("the pattern's display chosen", 5, |s| crate::runtime::records(s) == Some(shroom_core::world::SourceId::Display(covered)))?;
        crate::runtime::guard_source(&s, covered)?;
        d.key(Surface::Popover, Key::Return);
        d.wait("recording", 15, |s| s.tray.state == Some("recording") && s.countdown.is_none())?;
        std::thread::sleep(Duration::from_secs(4));
        click(&d.app, &d.rt);
        d.wait("paused", 10, |s| s.popover.as_ref().is_some_and(|p| p.id == "paused"))?;
        d.act(Surface::Popover, Action::Stop);
        Ok::<(), String>(())
    })();
    let _ = pattern.close();
    recorded?;
    let takes = d.rt.home.takes();
    let dir = poll(15, || crate::runtime::takes::dirs(&takes).into_iter().last().filter(|t| t.join("publish.json").exists())).ok_or("no publish.json after Stop")?;
    let rec = home::read_json::<crate::runtime::takes::PublishJson>(&dir.join("publish.json")).map(|p| p.rec).ok_or("no rec")?;
    d.step("published");
    if upload {
        poll(120, || dir.join("uploaded").exists().then_some(())).ok_or("the upload didn't finish in 2 minutes")?;
        d.step("uploaded");
    }
    Ok((dir, rec))
}

/// The local Worker's bucket for `host`: keys, with metadata and the small JSON texts.
fn objects(base: &str, host: &str) -> Result<Vec<serde_json::Value>, String> {
    Ok(knob_get(base, host, "objects")?.as_array().cloned().unwrap_or_default())
}

fn object_json(objs: &[serde_json::Value], key: &str) -> Option<serde_json::Value> {
    objs.iter().find(|o| o["key"] == key).and_then(|o| serde_json::from_str(o["text"].as_str()?).ok())
}

/// The claims of a take's `cred.json` credential.
fn cred_claims(dir: &std::path::Path) -> Option<(shroom_cloud::device::CredFile, serde_json::Value)> {
    let c = home::read_json::<shroom_cloud::device::CredFile>(&dir.join("cred.json"))?;
    let m = shroom_cloud::member::Minted {
        access_key_id: c.access_key_id.clone(),
        secret_access_key: c.secret_access_key.clone(),
        session_token: c.session_token.clone(),
        expires_at: c.expires_at.clone(),
        bucket: c.bucket.clone(),
        endpoint: c.endpoint.clone(),
        owner: String::new(),
    };
    let claims = shroom_cloud::mint::claims(&m.jwt()?)?;
    Some((c, claims))
}

/// The Library window's rows by recording id.
fn lib_row(s: &Scene, rec: &str) -> bool {
    s.library.as_ref().is_some_and(|l| l.body.rows.iter().any(|r| r.key == rec))
}

fn members_rows(s: &Scene) -> Option<shroom_core::scene::MembersBody> {
    match s.settings.as_ref().map(|w| &w.body.pane) {
        Some(SettingsPane::Members(m)) => Some(m.clone()),
        _ => None,
    }
}

fn members_drive(d: &Drive, _seed: &Region6Seed) -> Result<(), String> {
    use shroom_core::scene::RowRef;
    let base = d.rt.space_base.clone().ok_or("no --space-base")?;
    d.wait("ready", 20, |s| s.tray.click == shroom_core::scene::Click::OpenPopover)?;
    d.on_main(clear_harness_pasteboard);
    d.open(Action::OpenSettings)?;
    d.wait("settings", 10, |s| s.settings.is_some())?;
    d.act(Surface::Settings, Action::SettingsTab(SettingsTab::Members));
    let s = d.wait("settings · Members with the space's rows", 20, |s| faces(s).contains(&"settings-members") && members_rows(s).is_some_and(|m| m.invite_line.contains("link")))?;
    d.check("Members shows on the token's Mac", faces(&s).contains(&"settings-members"), format!("{:?}", faces(&s)));

    // Invite Ani: typed, then Return in the field.
    d.rt.field(Field::InviteName, "Ani".into());
    d.after(150);
    d.key(Surface::Settings, Key::Return);
    let s = d.wait("invite-born", 20, |s| members_rows(s).is_some_and(|m| m.rows.iter().any(|r| r.born.is_some())))?;
    let m = members_rows(&s).expect("members");
    let row = m.rows.iter().find(|r| r.born.is_some()).cloned().expect("the born row");
    let pill = row.born.clone().expect("the pill");
    d.check("the born row reads “invited just now”", row.line == "Ani — invited just now", row.line.clone());
    d.check("the pill shows the address, never the code", pill.prefix == format!("{SPACE_A}/join/"), pill.prefix.clone());
    d.check("nothing was auto-copied (law 10)", pasteboard().is_none_or(|t| !t.contains("/join/")), "harness pasteboard");
    let clip_before = d.on_main(clipboard_count);
    d.act(Surface::Settings, Action::CopyInvite(pill.link));
    let copied = poll(5, || d.on_main(pasteboard).flatten().filter(|t| t.contains("/join/")));
    let code = copied.as_deref().and_then(shroom_cloud::member::parse_link).map(|(_, c)| c);
    d.seen.lock().unwrap().extend(code.clone());
    d.check("Copy link put the whole link on shroom's pasteboard", copied.as_deref().is_some_and(|t| t.starts_with(&format!("https://{SPACE_A}/join/"))) && code.is_some(), "a join link");
    let clip_after = d.on_main(clipboard_count);
    d.check("the person's clipboard is untouched", clip_before.is_some() && clip_before == clip_after, format!("change count {clip_before:?} → {clip_after:?}"));
    let roster = knob_get(&base, SPACE_A, "roster")?;
    let iid = row.key.clone();
    let hash = code.as_deref().map(shroom_cloud::device::hash).unwrap_or_default();
    d.check("the space holds only the code's hash, under the row's id", roster["invites"][&iid]["codeHash"] == hash.as_str() && roster["invites"][&iid]["label"] == "Ani" && roster["invites"][&iid]["inviter"].as_str().is_some_and(|n| !n.is_empty() && !n.contains(' ')), format!("{}", roster["invites"][&iid]["inviter"]));
    if let Some(c) = &code {
        let scene = serde_json::to_string(&snapshot(&d.rt).and_then(|s| s.settings)).unwrap_or_default();
        d.check("law 13: the code isn't in the Settings slice", !scene.contains(c.as_str()), "settings slice");
    }

    // The row ages: closed and opened again, the link isn't re-shown and the row reads its date.
    d.rt.post(Event::WindowClosed(WindowKind::Settings));
    d.wait("closed", 5, |s| s.settings.is_none())?;
    d.open(Action::OpenSettings)?;
    let s = d.wait("Members again, aged", 20, |s| members_rows(s).is_some_and(|m| m.rows.iter().any(|r| r.key == iid && r.line.contains("days left"))))?;
    let m = members_rows(&s).expect("members");
    let aged = m.rows.iter().find(|r| r.key == iid).map(|r| r.line.clone()).unwrap_or_default();
    d.check("the pending row reads “invited <date> · 7 days left”", aged.starts_with("Ani — invited ") && aged.ends_with(" · 7 days left"), aged.clone());
    d.check("the link isn't re-shown", m.rows.iter().all(|r| r.born.is_none()), "no pill");

    // Cancel it: Remove… → the sheet → the red answer.
    d.act(Surface::Settings, Action::Remove(RowRef::Invite(iid.clone())));
    let s = d.wait("member-remove-confirm", 5, |s| s.settings.as_ref().is_some_and(|w| w.body.sheet.is_some()))?;
    d.check("law 9: no Return default on the sheet", s.ret(Surface::Settings).is_none(), format!("{:?}", s.ret(Surface::Settings)));
    d.act(Surface::Settings, Action::ConfirmRemove);
    d.wait("the row is gone", 20, |s| members_rows(s).is_some_and(|m| m.rows.iter().all(|r| r.key != iid)) && s.settings.as_ref().is_some_and(|w| w.body.sheet.is_none()))?;
    let roster = knob_get(&base, SPACE_A, "roster")?;
    d.check("the space cancelled it", roster["invites"][&iid]["state"] == "cancelled", format!("{}", roster["invites"][&iid]["state"]));

    // Add another Mac: an own-Mac link, one hour, under You.
    d.act(Surface::Settings, Action::AddMac);
    let s = d.wait("the own-Mac link", 20, |s| members_rows(s).is_some_and(|m| m.mac_link.is_some()))?;
    let pill = members_rows(&s).and_then(|m| m.mac_link).expect("the nest");
    d.check("“good for one hour”", pill.line.contains("one hour"), pill.line.clone());
    d.act(Surface::Settings, Action::CopyInvite(pill.link));
    let mac = poll(5, || d.on_main(pasteboard).flatten().filter(|t| t.contains("/join/") && Some(t.as_str()) != copied.as_deref()));
    let mac_code = mac.as_deref().and_then(shroom_cloud::member::parse_link).map(|(_, c)| c);
    d.seen.lock().unwrap().extend(mac_code.clone());
    let roster = knob_get(&base, SPACE_A, "roster")?;
    let own = roster["invites"].as_object().map(|o| o.values().any(|i| i["kind"] == "own_mac" && i["state"] == "pending" && mac_code.as_deref().is_some_and(|c| i["codeHash"] == shroom_cloud::device::hash(c).as_str()))).unwrap_or(false);
    d.check("the space holds an own-Mac invite for the copied link", own, "own_mac");
    d.rt.post(Event::WindowClosed(WindowKind::Settings));
    d.wait("closed", 5, |s| s.settings.is_none())?;
    Ok(())
}

// ── N68: suggested to clean up ──

/// The cleanup drive's space: (id, title, days old, bytes, plays, days since the last play,
/// tenth most plays reached). Plays `None`: never played. 10.6 GB in all, just past the free
/// 10 GB, so the rows say their cost and deleting all three brings the space back inside.
const CLEANUP_SEEDED: [(&str, &str, i64, u64, Option<(u32, i64, usize)>); 5] = [
    ("Cdrive000billing", "Code walkthrough: billing service", 380, 3_200_000_000, Some((4, 300, 3))),
    ("Cdrive00000recap", "Customer call recap (Acme)", 120, 1_800_000_000, None),
    ("Cdrive00migrated", "Migration dry run", 140, 1_400_000_000, Some((2, 120, 1))),
    ("Cdrive000000t502", "Where the 502s come from", 60, 4_200_000_000, None),
    ("Cdrive000standup", "Standup notes", 50, 40_000_000, Some((3, 30, 9))),
];
const DAY_SECS: i64 = 86_400;

/// One of the cleanup drive's videos, as a Mac's drained upload leaves its record (`bytes`).
fn cleanup_meta(id: &str, title: &str, days: i64, bytes: u64, now: i64) -> shroom_upload::meta::Meta {
    use shroom_upload::{keys, meta};
    let at = shroom_cloud::registry::iso((now - days * DAY_SECS) as u64);
    let mut m = meta::Meta::uploading(id, title, &at);
    m.state = meta::State::Live;
    m.published_at = Some(at);
    m.duration_sec = Some(600.0);
    m.owner = Some("m1".into());
    m.media = Some(meta::Media { hls: Some(keys::master(id)), mp4: None, poster: None, teaser: None, width: 1920, height: 1080 });
    m.extra.insert("bytes".into(), serde_json::json!(bytes));
    m
}

/// The space the drive meets, in the local Worker's bucket: the one the Library reads over
/// the token's pair (the space's parent, as wiring places it) and the one that counts plays
/// and answers `/cleanup`.
fn seed_cleanup(d: &Drive, base: &str) -> Result<(), String> {
    use shroom_upload::keys;
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|t| t.as_secs() as i64).unwrap_or(0);
    let token = d.rt.home.token().ok_or("no token")?;
    let cloud = d.rt.cloud(&token);
    let token_id = cloud.verify(ACCOUNT).map_err(|e| e.to_string())?.id;
    let pair = cloud.s3_credentials(&token_id);
    d.seen.lock().unwrap().push(pair.secret_access_key.clone());
    knob(base, SPACE_A, "parent", serde_json::json!({ "account_id": ACCOUNT, "bucket": shroom_cloud::wire::BUCKET, "key_id": pair.access_key_id, "secret": pair.secret_access_key }))?;
    knob(base, SPACE_A, "put", serde_json::json!({ "key": ".shroom/views/since.json", "json": { "since": shroom_cloud::registry::iso((now - 200 * DAY_SECS) as u64) } }))?;
    for (id, title, days, bytes, plays) in CLEANUP_SEEDED {
        let m = cleanup_meta(id, title, days, bytes, now);
        knob(base, SPACE_A, "put", serde_json::json!({ "key": keys::meta(id), "json": m }))?;
        if let Some((n, quiet, tenth)) = plays {
            let last = now - quiet * DAY_SECS;
            let day = |t: i64| shroom_cloud::registry::iso(t as u64)[..10].to_string();
            let mut watch = [0u32; 10];
            watch[tenth] = n;
            knob(base, SPACE_A, "put", serde_json::json!({ "key": format!(".shroom/views/{id}.json"), "json": {
                "v": 1, "plays": n, "viewers": n, "ends": n, "watch": watch, "watchedSec": 0,
                "firstAt": shroom_cloud::registry::iso(last as u64), "lastAt": shroom_cloud::registry::iso(last as u64),
                "days": serde_json::Map::from_iter([(day(last), serde_json::json!(n))]), "today": { "d": day(last), "h": [], "t": {} },
            } }))?;
        }
    }
    Ok(())
}

/// A viewer plays a video on its page today: the page's beacon, as a browser sends it.
fn play_today(base: &str, id: &str) -> Result<(), String> {
    let url = format!("{}/{SPACE_A}/_shroom/view/{id}", base.trim_end_matches('/'));
    for body in [serde_json::json!({ "e": "play", "t": "drivePlay0001" }), serde_json::json!({ "e": "end", "t": "drivePlay0001", "f": 0.9 })] {
        ureq::post(&url).header("origin", format!("https://{SPACE_A}")).header("content-type", "text/plain").send(body.to_string()).map_err(|e| format!("the page's beacon: {e}"))?;
    }
    Ok(())
}

fn cleanup_view(s: &Scene) -> Option<&shroom_core::scene::CleanupBody> {
    s.library.as_ref()?.body.cleanup.as_ref()
}

fn cleanup_titles(s: &Scene) -> Vec<String> {
    cleanup_view(s).map(|v| v.rows.iter().filter_map(|r| r.title.clone()).collect()).unwrap_or_default()
}

fn general_videos(s: &Scene) -> Option<shroom_core::scene::VideosGroup> {
    match s.settings.as_ref().map(|w| &w.body.pane) {
        Some(SettingsPane::General(g)) => g.videos.clone(),
        _ => None,
    }
}

fn worker_object(base: &str, key: &str) -> Option<serde_json::Value> {
    objects(base, SPACE_A).ok()?.into_iter().find(|o| o["key"] == key).and_then(|o| serde_json::from_str(o["text"].as_str()?).ok())
}

fn cleanup_drive(d: &Drive) -> Result<(), String> {
    use shroom_upload::keys;
    let base = d.rt.space_base.clone().ok_or("no --space-base")?;
    d.wait("ready", 20, |s| s.tray.click == shroom_core::scene::Click::OpenPopover)?;
    seed_cleanup(d, &base)?;
    play_today(&base, "Cdrive000000t502")?;
    let rec = worker_object(&base, ".shroom/views/Cdrive000000t502.json");
    d.check("the page's beacon is counted in the space", rec.as_ref().is_some_and(|r| r["plays"] == 1 && r["viewers"] == 1 && r["watch"][9] == 1), format!("{rec:?}"));
    d.step("seeded five videos, their plays, and a play today");

    // The Library: its rows, then the line once the space has said what's worth deleting.
    d.open(Action::OpenLibrary)?;
    let s = d.wait("the line above the rows", 30, |s| titles(s).len() == 5 && s.library.as_ref().is_some_and(|l| l.body.cleanup_line.is_some()))?;
    let line = s.library.as_ref().and_then(|l| l.body.cleanup_line.clone()).expect("the line");
    d.check("the line counts them and their size", line.line == "Three videos might be worth deleting — they take 6.4 GB.", line.line.clone());
    d.check("Review… and Not now", line.review.label == "Review…" && line.dismiss.label == "Not now", format!("{} / {}", line.review.label, line.dismiss.label));
    d.after(2500);

    d.act(Surface::Library, line.review.action.clone());
    let s = d.wait("library-cleanup", 10, |s| cleanup_view(s).is_some_and(|v| v.rows.len() == 3))?;
    let v = cleanup_view(&s).expect("the view");
    d.check("the space's order: the costliest per remaining play first", cleanup_titles(&s) == ["Code walkthrough: billing service", "Customer call recap (Acme)", "Migration dry run"], format!("{:?}", cleanup_titles(&s)));
    d.check("the one played today and the tiny one aren't suggested", !cleanup_titles(&s).iter().any(|t| t.contains("502") || t.contains("Standup")), format!("{:?}", cleanup_titles(&s)));
    d.check("the intro: deleting all three brings the space back inside the free 10 GB", v.intro.as_deref() == Some("These might be worth deleting: rarely watched lately, and taking real room. Deleting all three frees 6.4 GB and brings your space back inside Cloudflare's free 10 GB."), format!("{:?}", v.intro));
    let metas: Vec<String> = v.rows.iter().map(|r| r.meta.clone()).collect();
    d.check("each row: its size, its cost a month, and why", metas[0].ends_with("3.2 GB · 5¢ a month · Watched 4 times, the last 10 months ago") && metas[1].ends_with("1.8 GB · 3¢ a month · Nobody has watched it") && metas[2].contains("1.4 GB · 2¢ a month · Watched twice, stopped early"), metas.join(" / "));
    d.check("Delete is red text, Delete all three… too (law 9)", v.rows[0].buttons.iter().any(|b| b.label == "Delete" && b.style == Style::RedText) && v.delete_all.as_ref().is_some_and(|b| b.label == "Delete all three…" && b.style == Style::RedText), "styles");
    d.after(3000);

    // Keep: off the list for good, and the record says so.
    let migrated = RecId("Cdrive00migrated".into());
    d.act(Surface::Library, Action::KeepRow(migrated.clone()));
    let s = d.wait("kept", 15, |s| cleanup_view(s).is_some_and(|v| v.rows.len() == 2))?;
    d.check("Keep takes it off the list", !cleanup_titles(&s).iter().any(|t| t == "Migration dry run"), format!("{:?}", cleanup_titles(&s)));
    let kept = poll(10, || worker_object(&base, &keys::meta(&migrated.0)).filter(|m| m["keep"] == true));
    d.check("the record says keep", kept.is_some(), "meta keep");
    d.after(2000);

    // Delete, then Undo: nothing is sent.
    let recap = RecId("Cdrive00000recap".into());
    d.act(Surface::Library, Action::DeleteSuggested(recap.clone()));
    let s = d.wait("the Undo line", 5, |s| cleanup_view(s).is_some_and(|v| v.undo.is_some()))?;
    let undo = cleanup_view(&s).and_then(|v| v.undo.clone()).expect("undo");
    d.check("the Undo line names it", undo.line == "Deleted “Customer call recap (Acme)”." && undo.undo.label == "Undo", undo.line.clone());
    d.after(1500);
    d.act(Surface::Library, Action::UndoDelete);
    d.wait("back on the list", 5, |s| cleanup_titles(s).iter().any(|t| t == "Customer call recap (Acme)") && cleanup_view(s).is_some_and(|v| v.undo.is_none()))?;
    std::thread::sleep(Duration::from_secs(6));
    let still = worker_object(&base, &keys::meta(&recap.0));
    d.check("Undo sent nothing: the video is still live", still.as_ref().is_some_and(|m| m["state"] == "live"), format!("{:?}", still.as_ref().map(|m| m["state"].clone())));

    // Delete again, and let it go.
    d.act(Surface::Library, Action::DeleteSuggested(recap.clone()));
    d.wait("the Undo line again", 5, |s| cleanup_view(s).is_some_and(|v| v.undo.is_some()))?;
    let s = d.wait("the delete went", 20, |s| cleanup_view(s).is_some_and(|v| v.undo.is_none() && v.rows.len() == 1) && !titles(s).iter().any(|t| t == "Customer call recap (Acme)"))?;
    let gone = poll(15, || worker_object(&base, &keys::meta(&recap.0)).filter(|m| m["state"] == "deleted"));
    d.check("the record is a tombstone", gone.is_some(), "meta deleted");
    d.check("one left, and no Delete all… for one", cleanup_titles(&s) == ["Code walkthrough: billing service"] && cleanup_view(&s).is_some_and(|v| v.delete_all.is_none()), format!("{:?}", cleanup_titles(&s)));
    d.after(2500);

    // Back to the Library.
    d.act(Surface::Library, Action::Back);
    d.wait("library", 5, |s| s.library.as_ref().is_some_and(|l| l.body.cleanup.is_none()))?;
    d.after(1500);
    d.rt.post(Event::WindowClosed(WindowKind::Library));
    d.wait("closed", 5, |s| s.library.is_none())?;

    // Settings · General: Your videos, the space's size, and Count plays off and on again.
    // The read Settings asks for meets a busy space once: the failure is in the run log.
    knob(&base, SPACE_A, "fault", serde_json::json!({ "prefix": "/_shroom/api/cleanup", "status": 500, "count": 1 }))?;
    d.open(Action::OpenSettings)?;
    let s = d.wait("Your videos", 30, |s| general_videos(s).is_some_and(|v| v.used.is_some()))?;
    let v = general_videos(&s).expect("videos");
    d.check("Your videos: both switches on", v.heading == "Your videos" && v.count.on && v.count.enabled && v.suggest.on, format!("count {} suggest {}", v.count.on, v.suggest.on));
    d.check("Space used: inside the free 10 GB now", v.used.as_deref().is_some_and(|u| u.ends_with("inside Cloudflare's free 10 GB")), format!("{:?}", v.used));
    d.after(2500);
    d.act(Surface::Settings, v.count.action.clone());
    // The switch moves at once and stays greyed until the space has taken it.
    d.wait("Count plays off, taken", 20, |s| general_videos(s).is_some_and(|v| !v.count.on && v.count.enabled))?;
    let sw = worker_object(&base, ".shroom/plays.json");
    d.check("the space's switch is off", sw.as_ref().is_some_and(|o| o["on"] == false), format!("{sw:?}"));
    play_today(&base, "Cdrive000billing")?;
    let rec = worker_object(&base, ".shroom/views/Cdrive000billing.json");
    d.check("with it off, a play isn't counted", rec.as_ref().is_some_and(|r| r["plays"] == 4), format!("{:?}", rec.as_ref().map(|r| r["plays"].clone())));
    d.after(2000);
    let s = snapshot(&d.rt).ok_or("no scene")?;
    let v = general_videos(&s).ok_or("no Your videos")?;
    d.act(Surface::Settings, v.count.action.clone());
    d.wait("Count plays on again, taken", 20, |s| general_videos(s).is_some_and(|v| v.count.on && v.count.enabled))?;
    let sw = worker_object(&base, ".shroom/plays.json");
    d.check("the space's switch is on again", sw.as_ref().is_some_and(|o| o["on"] == true), format!("{sw:?}"));
    d.after(2000);
    d.rt.post(Event::WindowClosed(WindowKind::Settings));
    d.wait("closed", 5, |s| s.settings.is_none())?;

    // What a problem report would carry of all this: the run log's lines, as a report
    // cleans them (no title, no link, no whole id).
    let log = poll(10, || {
        let text = std::fs::read_to_string(d.rt.home.root.join("logs/run.log")).ok()?;
        text.contains("library Suggest: failed").then_some(text)
    })
    .unwrap_or_default();
    let cleaned = shroom_core::report::clean_log(&log);
    let has = |want: &str| cleaned.lines().any(|l| l.contains(want));
    d.check("the log: the read, its size and how counting fares", has("library suggestions: 3 of 5 videos, 6.4 GB of 10.6 GB, counting on; plays read 4, unreadable 0, newest play 0h ago"), "library suggestions: …");
    d.check("the log: each candidate by a short id, why, its score and facts", has("library suggested: Cdri… not-lately s0.") && has(" never-played s0.") && has(" stopped-early s0."), "library suggested: …");
    d.check("the log: the line showing and going, Keep, the delete and its Undo", ["library: the cleanup line shows", "core cleanup-opened", "library kept Cdri…", "core delete-undone", "library deleted Cdri…"].iter().all(|w| has(w)), "the steps");
    d.check("the log: the read that met a busy space", has("library Suggest: failed (Transport)"), "the failure");
    d.check("the log names no video", !CLEANUP_SEEDED.iter().any(|(id, title, ..)| log.contains(title) || log.contains(id)), "no titles, no whole ids");
    Ok(())
}

fn switch_drive(d: &Drive, seed: &Region6Seed) -> Result<(), String> {
    use shroom_cloud::device::DeviceKey;
    let base = d.rt.space_base.clone().ok_or("no --space-base")?;
    d.wait("ready", 20, |s| s.tray.click == shroom_core::scene::Click::OpenPopover)?;
    let old = home::read_json::<DeviceKey>(&d.rt.home.path("device.key")).ok_or("no seeded device.key")?;
    // The page's Open shroom for Ani's space: the deep link, through the shell's own door.
    crate::runtime::member::deep_link(&d.rt, seed.link.as_deref().ok_or("no link")?);
    let s = d.wait("join-switch-confirm", 40, is_onb("join-switch-confirm"))?;
    d.check("the switch asks first", s.onboarding.as_ref().is_some_and(|o| o.activates), "asked");
    let still = home::read_json::<DeviceKey>(&d.rt.home.path("device.key"));
    d.check("the old space's files are untouched until the new join's 200", still.as_ref().is_some_and(|k| k.device == old.device) && read_file(d, "space.json").is_some_and(|v| v["host"] == SPACE_A), "device.key, space.json");
    d.act(Surface::Onboarding, Action::Join);
    // This Mac's permissions chapter is behind it (primed — row six is per-Mac): joined,
    // it's all set at once, never resting at done · member's Allow recording.
    d.wait("joined: all-set · member", 40, is_onb("member-all-set"))?;
    let seen = d.rt.trace.lock().unwrap().as_ref().map(|t| t.ids.clone()).unwrap_or_default();
    d.check("a primed Mac skips done · member (row six already ticked)", !seen.iter().any(|id| id.contains("member-done")), seen.join(" → "));
    let key = home::read_json::<DeviceKey>(&d.rt.home.path("device.key"));
    let space = read_file(d, "space.json");
    d.check("device.key is a new one, for space B", key.as_ref().is_some_and(|k| k.host == SPACE_B && k.device != old.device), format!("{:?}", key.as_ref().map(|k| &k.host)));
    d.check("space.json is space B's, a member's, named for Ani", space.as_ref().is_some_and(|v| v["host"] == SPACE_B && v["role"] == "Member" && v["owner_name"] == "Ani"), format!("{space:?}"));
    d.check("nothing of space A is left", ["token.json", "library.json", "invite", "join.json"].iter().all(|f| !d.rt.home.path(f).exists()), "files");
    let a = knob_get(&base, SPACE_A, "roster")?;
    d.check("space A was told this Mac left (best-effort deregistration)", a["devices"][&old.device]["state"] != "active", format!("{}", a["devices"][&old.device]["state"]));
    d.check("space A keeps the member's row (only the Mac left)", seed.member.as_ref().is_some_and(|m| a["members"][m].is_object()), "member row");
    let b = knob_get(&base, SPACE_B, "roster")?;
    let device = key.as_ref().map(|k| k.device.clone()).unwrap_or_default();
    // Leave below takes device.key with it: its secret is checked like the seed's.
    d.seen.lock().unwrap().extend(key.as_ref().map(|k| k.secret.clone()));
    d.check("space B has this Mac, active", b["devices"][&device]["state"] == "active", format!("{}", b["devices"][&device]["state"]));
    let files_now = files(&d.rt.home.root).into_iter().map(|p| p.strip_prefix(&d.rt.home.root).unwrap_or(&p).display().to_string()).collect::<Vec<_>>();
    d.check("no old secret left on disk", files_now.iter().all(|f| f != "token.json"), format!("{files_now:?}"));
    d.act(Surface::Onboarding, Action::CloseWindow);
    d.wait("the window closed", 10, |s| s.onboarding.is_none())?;

    // Leave (§5's recipe): Settings · Space → Leave this space… → the sheet → Leave. The
    // space hears this Mac go (DELETE devices/self, best-effort); its files go here.
    d.open(Action::OpenSettings)?;
    d.wait("settings", 10, |s| s.settings.is_some())?;
    d.act(Surface::Settings, Action::SettingsTab(SettingsTab::Space));
    d.wait("settings-space", 10, |s| s.settings.as_ref().is_some_and(|w| w.faces.iter().any(|f| f.starts_with("settings-space"))))?;
    d.act(Surface::Settings, Action::Disconnect);
    let s = d.wait("the Leave sheet", 10, |s| s.settings.as_ref().is_some_and(|w| w.body.sheet.is_some()))?;
    let sheet = s.settings.as_ref().and_then(|w| w.body.sheet.clone()).expect("a sheet");
    d.check("the sheet is Leave's, a member's words", sheet.buttons.iter().any(|b| b.action == Action::ConfirmDisconnect), format!("{} {:?}", sheet.id, sheet.buttons.iter().map(|b| &b.label).collect::<Vec<_>>()));
    d.act(Surface::Settings, Action::ConfirmDisconnect);
    d.wait("left", 20, |_| !d.rt.home.path("space.json").exists())?;
    d.check("Leave took the space's files", ["space.json", "device.key", "invite", "join.json", "token.json"].iter().all(|f| !d.rt.home.path(f).exists()), "files");
    d.check("…and kept this Mac's permissions chapter", d.rt.home.path("primed.json").exists(), "primed.json");
    let left = poll(10, || {
        let b = knob_get(&base, SPACE_B, "roster").ok()?;
        (b["devices"][&device]["state"] != "active").then(|| b["devices"][&device]["state"].to_string())
    });
    d.check("space B was told this Mac left", left.is_some(), format!("{left:?}"));
    Ok(())
}

/// The owner's token Mac whose wiring enroll didn't land (best-effort, §6.11): its device
/// key is on disk and unknown to the space. Members enrolls it on the way (the 401 is never
/// a removal); the owner page's key goes through `/key`; a recording mints locally (B32)
/// with its own ledger entry first (`x-amz-meta-m`); the Library lists over the token's
/// S3 pair — all against the local Worker, whose parent is this token's derived pair.
fn token_drive(d: &Drive, seed: &Region6Seed) -> Result<(), String> {
    use shroom_cloud::device::{self, DeviceKey};
    let base = d.rt.space_base.clone().ok_or("no --space-base")?;
    let token_id = seed.token_id.clone().ok_or("no token id")?;
    let key = home::read_json::<DeviceKey>(&d.rt.home.path("device.key")).ok_or("no seeded device.key")?;
    d.check("before launch, the space didn't know this Mac's device key", seed.unknown_at_seed == Some(true), format!("{:?}", seed.unknown_at_seed));
    d.wait("ready", 20, |s| s.tray.click == shroom_core::scene::Click::OpenPopover)?;
    let run_log = || std::fs::read_to_string(d.rt.home.root.join("logs/run.log")).unwrap_or_default();

    // B51: the launch's self-check proves the token over its own page and enrolls this Mac
    // on the way (§6.11's every entry) — the lost enroll heals before anyone opens anything
    // (the drive read to this reality, DECISIONS B67).
    poll(60, || run_log().contains("enroll on entry:").then_some(())).ok_or("the launch's self-check never enrolled")?;
    let roster = knob_get(&base, HOST, "roster")?;
    let dev = &roster["devices"][&key.device];
    d.check(
        "the launch self-check enrolled this key under member #1, with the token's proof",
        dev["state"] == "active" && dev["member"] == "m1" && dev["tokenKey"] == token_id.as_str() && run_log().contains(&format!("enroll on entry: {} as m1", key.device)),
        format!("{dev}"),
    );
    d.check("the self-check's enroll is no removal", !run_log().contains("standing revoked"), "run.log");

    // The space loses the key again (an enroll that never landed, a restored bucket): the
    // Members lane's 401 must still be an enroll, never a removal.
    knob(&base, HOST, "forget", serde_json::json!({ "id": key.device }))?;
    d.check("the space no longer knows the key", knob_get(&base, HOST, "roster")?["devices"][&key.device].is_null(), "roster");

    // Members, straight away (no Settings · Space entry first): GET /members answers 401,
    // the lane enrolls with the token's proof and asks again.
    d.open(Action::OpenSettings)?;
    d.wait("settings", 10, |s| s.settings.is_some())?;
    d.act(Surface::Settings, Action::SettingsTab(SettingsTab::Members));
    d.wait("settings-members", 20, |s| faces(s).contains(&"settings-members"))?;
    // The roster's answer lands after the enroll and the second ask.
    poll(60, || run_log().contains("member: roster —").then_some(())).ok_or("the roster never answered")?;
    d.step("the roster answered");
    let s = snapshot(&d.rt).ok_or("no scene")?;
    let roster = knob_get(&base, HOST, "roster")?;
    let dev = &roster["devices"][&key.device];
    d.check("enrolled under member #1, with the token's proof", dev["state"] == "active" && dev["member"] == "m1" && dev["tokenKey"] == token_id.as_str(), format!("{dev}"));
    let log = std::fs::read_to_string(d.rt.home.root.join("logs/run.log")).unwrap_or_default();
    d.check("the 401 was an enroll, never a removal", log.contains(&format!("enrolled this token Mac ({} as m1)", key.device)) && !log.contains("standing revoked"), "run.log");
    let ids = d.rt.trace.lock().unwrap().as_ref().map(|t| t.ids.join(" → ")).unwrap_or_default();
    d.check("no removed-Mac face", !ids.contains("removed") && !s.state_ids().iter().any(|i| i.contains("removed") || i.contains("revoked")), ids);
    d.rt.post(Event::WindowClosed(WindowKind::Settings));
    d.wait("closed", 5, |s| s.settings.is_none())?;

    // The owner page's one-time key (B36): made here, only its hash in the space.
    let space: SpaceIdentity = home::read_json(&d.rt.home.path("space.json")).ok_or("no space.json")?;
    let k = crate::runtime::cloud::owner_key(&d.rt, &space, &mut None);
    d.seen.lock().unwrap().extend(k.clone());
    let objs = objects(&base, HOST)?;
    let grant = k.as_deref().map(|k| format!(".shroom/keys/{}", device::hash(k)));
    d.check("the owner page's key went through /key (its hash in the space)", grant.as_ref().is_some_and(|g| objs.iter().any(|o| o["key"] == g.as_str())), format!("{}", grant.is_some()));

    // A recording from the mushroom: minted locally, its ledger entry written first.
    let (dir, rec) = record_one(d, || click(&d.app, &d.rt))?;
    let objs = objects(&base, HOST)?;
    let lkey = format!(".shroom/registry/recs/{rec}");
    let entry = objs.iter().find(|o| o["key"] == lkey.as_str());
    let body = object_json(&objs, &lkey);
    d.check(
        "the token Mac wrote its own ledger entry, x-amz-meta-m naming member #1",
        entry.is_some_and(|e| e["customMetadata"]["m"] == "m1") && body.as_ref().is_some_and(|b| b["member"] == "m1" && b["device"] == key.device.as_str()),
        format!("{entry:?}"),
    );
    let keys: Vec<&str> = objs.iter().filter_map(|o| o["key"].as_str()).collect();
    d.check("the record and the bytes landed in the space's bucket", keys.contains(&format!("meta/{rec}.json").as_str()) && keys.iter().filter(|k| k.starts_with(&format!("rec/{rec}/"))).count() >= 3, format!("{} keys", keys.len()));
    let cred = cred_claims(&dir);
    d.check(
        "minted locally under the token's pair, for this recording alone",
        cred.as_ref().is_some_and(|(c, claims)| c.purpose == "upload" && claims["iss"] == token_id.as_str() && claims["paths"]["prefixPaths"][0] == format!("rec/{rec}/").as_str()),
        format!("{:?}", cred.as_ref().map(|(_, c)| (&c["iss"], &c["paths"]))),
    );
    // `/mint` spends the device's upload budget (H4): no counter means no `/mint` call.
    let rate = keys.iter().filter(|k| k.starts_with(".shroom/registry/rate/")).count();
    d.check("no /mint call: the token Mac mints locally (B32)", rate == 0, format!("{rate} budget counter(s)"));

    // The Library: the token's own S3 list of the space's records.
    d.open(Action::OpenLibrary)?;
    d.wait("the Library shows the video", 30, |s| lib_row(s, &rec))?;
    let log = std::fs::read_to_string(d.rt.home.root.join("logs/run.log")).unwrap_or_default();
    d.check("listed over the token's S3 pair", log.contains("library refreshed:") && !log.contains("library refreshed through the space"), "run.log");
    d.rt.post(Event::WindowClosed(WindowKind::Library));
    d.wait("closed", 5, |s| s.library.is_none())?;
    Ok(())
}

/// A credential minted from the token's derived pair for one recording id, as the token
/// Mac's uploader mints it (B32: HS256 under the pair, `rec/<id>/` and its record only) —
/// and one small object put with it into the space's bucket on the local Worker.
fn minted_put(base: &str, cloud: &shroom_cloud::Cloud, token_id: &str, rec: &str) -> Result<(), String> {
    use shroom_cloud::mint::{self, Grant, Paths};
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
    let grant = Grant {
        account_id: ACCOUNT,
        bucket: shroom_cloud::wire::BUCKET,
        scope: "object-read-write",
        paths: Paths { prefixes: vec![shroom_upload::keys::prefix(rec)], objects: vec![shroom_upload::keys::meta(rec)] },
        issued_at: now,
        ttl_seconds: 900,
    };
    let creds = mint::mint(&cloud.s3_credentials(token_id), &grant);
    let client = Client::new(shroom_storage::s3::Bucket { endpoint: format!("{}/_s3/{HOST}", base.trim_end_matches('/')), region: "auto".into(), name: shroom_cloud::wire::BUCKET.into(), credentials: creds });
    client.put(&format!("{}probe", shroom_upload::keys::prefix(rec)), b"ok", "text/plain").map_err(|e| e.to_string().chars().take(160).collect())
}

/// The L1 fake's calls so far (`GET /_fake/calls`: "METHOD /path" and the relay's
/// deliveries, no bodies).
fn fake_calls(rt: &Runtime) -> Result<Vec<String>, String> {
    let (api, _) = rt.cloud_fake.clone().ok_or("no fake")?;
    let url = format!("{}/_fake/calls", api.trim_end_matches("/client/v4"));
    let mut res = ureq::get(&url).call().map_err(|e| format!("the fake's calls: {e}"))?;
    let v: serde_json::Value = serde_json::from_str(&res.body_mut().read_to_string().map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    Ok(v["result"]["calls"].as_array().map(|a| a.iter().filter_map(|c| c.as_str().map(String::from)).collect()).unwrap_or_default())
}

/// DECISIONS B90 — the stale-parent re-place, end to end: the space's `R2_PARENT` is another
/// token's when the app starts; the launch's enroll meets `stale-parent`, re-places the
/// binding through the secrets endpoint (once — no redeploy, no other write), the enroll
/// lands, and the next mint from the token and the Library's listing go through.
fn stale_drive(d: &Drive, seed: &Region6Seed) -> Result<(), String> {
    use shroom_cloud::device::DeviceKey;
    let base = d.rt.space_base.clone().ok_or("no --space-base")?;
    let token_id = seed.token_id.clone().ok_or("no token id")?;
    let st = seed.stale.as_ref().ok_or("no stale seed")?;
    let key = home::read_json::<DeviceKey>(&d.rt.home.path("device.key")).ok_or("no seeded device.key")?;
    d.check("before launch, the space's R2_PARENT was another token's pair", !st.binding_key.is_empty() && st.binding_key != st.token_key, format!("binding {}… token {}…", &st.binding_key[..6.min(st.binding_key.len())], &st.token_key[..6.min(st.token_key.len())]));
    d.check("before launch, a credential minted from this Mac's token was refused by the space", st.refused_before.is_err(), format!("{:?}", st.refused_before));
    d.wait("ready", 20, |s| s.tray.click == shroom_core::scene::Click::OpenPopover)?;
    let run_log = || std::fs::read_to_string(d.rt.home.root.join("logs/run.log")).unwrap_or_default();

    // B51: the launch's self-check enrolls; the enroll meets the stale binding and re-places it.
    poll(90, || run_log().contains("enroll on entry:").then_some(())).ok_or("the launch's self-check never enrolled")?;
    d.step("the launch's enroll answered");
    let log = run_log();
    let replaced = log.find("enroll: the page held another parent — re-placed");
    let enrolled = log.find(&format!("enroll on entry: {} as m1", key.device));
    d.check("the enroll met stale-parent and the app re-placed the binding on its own", replaced.is_some(), "run.log");
    d.check("…then the enroll landed: re-place before enroll", matches!((replaced, enrolled), (Some(r), Some(e)) if r < e), format!("{replaced:?} < {enrolled:?}"));
    d.check("the re-place happened once", log.matches("the page held another parent").count() == 1, format!("{}", log.matches("the page held another parent").count()));

    // The fake: one secrets PUT, handed to the Worker, and nothing else of the space written.
    let calls = fake_calls(&d.rt)?;
    let secrets = format!("PUT /client/v4/accounts/{ACCOUNT}/workers/scripts/v/secrets");
    let put_at: Vec<usize> = calls.iter().enumerate().filter(|(_, c)| **c == secrets).map(|(i, _)| i).collect();
    let relay_at: Vec<usize> = calls.iter().enumerate().filter(|(_, c)| c.starts_with("RELAY ")).map(|(i, _)| i).collect();
    d.check("exactly one R2_PARENT placement through the secrets endpoint", put_at.len() == 1, format!("{put_at:?}"));
    d.check("the placement reached the space's Worker binding, in the same call", relay_at.len() == 1 && put_at.first().map(|p| p + 1) == relay_at.first().copied() && calls[relay_at[0]] == format!("RELAY R2_PARENT {HOST} 200"), format!("{:?}", relay_at.first().map(|i| &calls[*i])));
    // Every write the run made on the account but the placement: only the self-check's
    // harmless powers probe (a throwaway `shroom-probe` Worker, put and deleted — §6.11's
    // token proof) may appear. No redeploy of the page, no bucket, route or name, nothing of
    // the space's deleted, its marker, cookie key or roster untouched.
    let acct = format!("/client/v4/accounts/{ACCOUNT}");
    let probe = [format!("PUT {acct}/workers/scripts/shroom-probe"), format!("DELETE {acct}/workers/scripts/shroom-probe")];
    let written: Vec<&String> = calls.iter().filter(|c| !c.starts_with("GET ") && !c.starts_with("HEAD ") && !c.starts_with("RELAY ") && **c != secrets).collect();
    d.fact_like("account_writes", &written);
    let other: Vec<&&String> = written.iter().filter(|c| !probe.contains(c)).collect();
    d.check("nothing else written on the account: no redeploy, bucket, route or name, nothing deleted, the space's files untouched", other.is_empty(), format!("{other:?}"));
    // The local Worker: the binding is this token's pair now, placed once.
    let binding = knob_get(&base, HOST, "parent")?;
    d.check(
        "the space's binding is this token's derived pair, for the space's account, placed once",
        binding["key_id"] == st.token_key.as_str() && binding["account_id"] == ACCOUNT && binding["bucket"] == shroom_cloud::wire::BUCKET && binding["placed"] == 1,
        format!("account {} placed {}", binding["account_id"], binding["placed"]),
    );
    // The roster: this Mac enrolled under member #1 with the token's proof; nothing else changed.
    let roster = knob_get(&base, HOST, "roster")?;
    let dev = &roster["devices"][&key.device];
    d.check("enrolled under member #1, with the token's proof", dev["state"] == "active" && dev["member"] == "m1" && dev["tokenKey"] == token_id.as_str(), format!("{dev}"));
    let mut others = roster["devices"].as_object().cloned().unwrap_or_default();
    others.remove(&key.device);
    d.check(
        "the roster's members, invites and other devices are as they were",
        roster["members"] == st.roster["members"] && roster["invites"] == st.roster["invites"] && serde_json::Value::Object(others) == st.roster["devices"],
        "roster",
    );
    let now: Vec<(String, u64)> = objects(&base, HOST)?.iter().filter(|o| o["key"] != ".shroom/registry/roster.json").map(|o| (o["key"].as_str().unwrap_or_default().to_string(), o["size"].as_u64().unwrap_or(0))).collect();
    d.check("every other object of the space is as it was", now == st.objects, format!("{} before, {} now", st.objects.len(), now.len()));

    // The next mint goes through: a credential from the token's pair, and one object with it.
    let token = d.rt.home.token().ok_or("no token")?;
    let after = minted_put(&base, &d.rt.cloud(&token), &token_id, "StaleProbeAfter1");
    d.check("the next credential minted from the token is taken by the space", after.is_ok(), format!("{after:?}"));
    let keys: Vec<String> = objects(&base, HOST)?.iter().filter_map(|o| o["key"].as_str().map(String::from)).collect();
    d.check("its object landed (and the refused one never did)", keys.iter().any(|k| k == "rec/StaleProbeAfter1/probe") && !keys.iter().any(|k| k.starts_with("rec/StaleProbeBefore/")), format!("{} keys", keys.len()));

    // The Library: the app's own listing over the token's pair, now the space's parent.
    d.open(Action::OpenLibrary)?;
    d.wait("the Library", 10, |s| s.library.is_some())?;
    poll(30, || run_log().contains("library refreshed:").then_some(())).ok_or("the Library never refreshed")?;
    d.check("the Library listed over the token's S3 pair", !run_log().contains("library refreshed through the space"), "run.log");
    d.rt.post(Event::WindowClosed(WindowKind::Library));
    d.wait("closed", 5, |s| s.library.is_none())?;
    let ids = d.rt.trace.lock().unwrap().as_ref().map(|t| t.ids.join(" → ")).unwrap_or_default();
    d.check("the stale binding never surfaced as a face: no space-degraded, no amber", !ids.contains("space-degraded") && !ids.contains("tray-amber-dot"), ids.chars().take(400).collect::<String>());
    Ok(())
}

// ── L6: two instances, one space (TESTING-PLAN §5; DECISIONS B65) ──

/// The runner's step protocol: empty files in the run's work folder.
fn signal(work: &std::path::Path, name: &str) {
    let _ = std::fs::write(work.join(name), b"");
}

fn await_signal(d: &Drive, work: &std::path::Path, name: &str, secs: u64) -> Result<(), String> {
    poll(secs, || work.join(name).exists().then_some(())).ok_or(format!("the runner never said {name}"))?;
    d.step(&format!("runner: {name}"));
    Ok(())
}

/// The owner makes an invite for Mher in Members and hands its link over the way the
/// installer would (a file the runner moves into the other Mac's home, mode 600) — never in
/// a log, a report or an argv.
fn l6_invite(d: &Drive, work: &std::path::Path, handoff: &str) -> Result<String, String> {
    invite_by_members(d, work, handoff, "Mher", 20)
}

/// Members → a name → Return → the born row's Copy link → the hand-off file (mode 600).
/// The born row's key.
fn invite_by_members(d: &Drive, work: &std::path::Path, handoff: &str, name: &str, secs: u64) -> Result<String, String> {
    d.on_main(clear_harness_pasteboard);
    d.open(Action::OpenSettings)?;
    d.wait("settings", 10, |s| s.settings.is_some())?;
    d.act(Surface::Settings, Action::SettingsTab(SettingsTab::Members));
    d.wait("Members with the space's rows", secs, |s| faces(s).contains(&"settings-members") && members_rows(s).is_some_and(|m| m.invite_line.contains("link")))?;
    d.rt.field(Field::InviteName, name.into());
    d.after(150);
    d.key(Surface::Settings, Key::Return);
    let s = d.wait("invite-born", secs, |s| members_rows(s).is_some_and(|m| m.rows.iter().any(|r| r.born.is_some())))?;
    let row = members_rows(&s).and_then(|m| m.rows.into_iter().find(|r| r.born.is_some())).ok_or("no born row")?;
    let pill = row.born.clone().ok_or("no pill")?;
    d.act(Surface::Settings, Action::CopyInvite(pill.link));
    let copied = poll(5, || d.on_main(pasteboard).flatten().filter(|t| t.contains("/join/"))).ok_or("Copy link put nothing on shroom's pasteboard")?;
    let (host, code) = shroom_cloud::member::parse_link(&copied).ok_or("the copied link doesn't parse")?;
    d.seen.lock().unwrap().push(code.clone());
    let now = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|t| t.as_secs()).unwrap_or(0);
    let file = serde_json::json!({ "v": 1, "link": format!("{host}/join/{code}"), "arrived": shroom_cloud::registry::iso(now) });
    home::write_json(&work.join(handoff), &file).map_err(|e| e.to_string())?;
    d.rt.post(Event::WindowClosed(WindowKind::Settings));
    d.wait("closed", 5, |s| s.settings.is_none())?;
    Ok(row.key)
}

fn l6_owner(d: &Drive, work: &std::path::Path) -> Result<(), String> {
    use shroom_core::scene::RowRef;
    let base = d.rt.space_base.clone().ok_or("no --space-base")?;
    d.wait("ready", 30, |s| s.tray.click == shroom_core::scene::Click::OpenPopover)?;
    l6_invite(d, work, "invite.handoff")?;
    signal(work, "owner-invited");

    // Mher joined and recorded on the other Mac; a second take waits there. Remove him.
    await_signal(d, work, "go-remove", 600)?;
    d.open(Action::OpenSettings)?;
    d.wait("settings", 10, |s| s.settings.is_some())?;
    d.act(Surface::Settings, Action::SettingsTab(SettingsTab::Members));
    let s = d.wait("Mher's row", 30, |s| members_rows(s).is_some_and(|m| m.rows.iter().any(|r| r.line.starts_with("Mher") && r.buttons.iter().any(|b| matches!(b.action, Action::Remove(RowRef::Member(_)))))))?;
    let row = members_rows(&s).and_then(|m| m.rows.into_iter().find(|r| r.line.starts_with("Mher"))).ok_or("no row")?;
    let remove = row.buttons.iter().find(|b| matches!(b.action, Action::Remove(RowRef::Member(_)))).map(|b| b.action.clone()).ok_or("no Remove…")?;
    let Action::Remove(RowRef::Member(mid)) = remove.clone() else { return Err("not a member's Remove".into()) };
    d.fact_like("mher", &mid);
    d.act(Surface::Settings, remove);
    let s = d.wait("member-remove-confirm", 10, |s| s.settings.as_ref().is_some_and(|w| w.body.sheet.is_some()))?;
    d.check("law 9: removing has no Return default", s.ret(Surface::Settings).is_none(), format!("{:?}", s.ret(Surface::Settings)));
    d.act(Surface::Settings, Action::ConfirmRemove);
    d.wait("Mher's row gone", 30, |s| members_rows(s).is_some_and(|m| m.rows.iter().all(|r| !r.line.starts_with("Mher"))) && s.settings.as_ref().is_some_and(|w| w.body.sheet.is_none()))?;
    let roster = knob_get(&base, SPACE_A, "roster")?;
    d.check("the space removed Mher", roster["members"][&mid]["state"] == "removed", format!("{}", roster["members"][&mid]["state"]));
    let devices_left = roster["devices"].as_object().map(|o| o.values().filter(|v| v["member"] == mid.as_str() && v["state"] == "active").count()).unwrap_or(0);
    d.check("…and every Mac of his", devices_left == 0, format!("{devices_left} active"));
    d.rt.post(Event::WindowClosed(WindowKind::Settings));
    d.wait("closed", 5, |s| s.settings.is_none())?;
    signal(work, "owner-removed");

    // Invited again — a fresh link for the same person.
    await_signal(d, work, "go-reinvite", 600)?;
    l6_invite(d, work, "rejoin.handoff")?;
    signal(work, "owner-reinvited");

    await_signal(d, work, "go-finish", 600)?;
    let roster = knob_get(&base, SPACE_A, "roster")?;
    d.check("Mher is back, the same member revived (§4.5's previous)", roster["members"][&mid]["state"] == "active", format!("{}", roster["members"][&mid]["state"]));
    d.open(Action::OpenSettings)?;
    d.wait("settings", 10, |s| s.settings.is_some())?;
    d.act(Surface::Settings, Action::SettingsTab(SettingsTab::Members));
    d.wait("Mher's row again", 30, |s| members_rows(s).is_some_and(|m| m.rows.iter().any(|r| r.line.starts_with("Mher") && r.born.is_none() && !r.line.contains("invited"))))?;
    d.rt.post(Event::WindowClosed(WindowKind::Settings));
    d.wait("closed", 5, |s| s.settings.is_none())?;
    Ok(())
}

fn l6_member(d: &Drive, work: &std::path::Path) -> Result<(), String> {
    use shroom_cloud::device::DeviceKey;
    let base = d.rt.space_base.clone().ok_or("no --space-base")?;
    // 1. The installer's hand-off: member-welcome → Join → this Mac's chapter → all set.
    d.wait("member-welcome", 60, is_onb("member-welcome"))?;
    d.act(Surface::Onboarding, Action::Join);
    d.wait("member-done", 60, is_onb("member-done"))?;
    d.act(Surface::Onboarding, Action::AllowRecording);
    d.wait("member-all-set", 60, is_onb("member-all-set"))?;
    let key = home::read_json::<DeviceKey>(&d.rt.home.path("device.key")).ok_or("no device.key")?;
    let member = read_file(d, "space.json").and_then(|v| v["member_id"].as_str().map(String::from)).ok_or("no member id")?;
    let roster = knob_get(&base, SPACE_A, "roster")?;
    d.check("joined: this Mac is in the space under its own member", roster["devices"][&key.device]["state"] == "active" && roster["members"][&member]["label"] == "Mher", "roster");
    signal(work, "member-joined");

    // 2. A first video: the pattern, the beep — minted, uploaded, in the space's bucket.
    let (_, rec1) = record_one(d, || d.act(Surface::Onboarding, Action::RecordNow))?;
    let keys1: Vec<String> = objects(&base, SPACE_A)?.iter().filter_map(|o| o["key"].as_str().map(String::from)).collect();
    d.check("the first video is in the space", keys1.contains(&format!("meta/{rec1}.json")) && keys1.contains(&format!("rec/{rec1}/master.m3u8")), format!("{} keys", keys1.len()));

    // 3. A second take while the space is too busy to mint (the Worker's 503 busy): it
    //    waits here, published, holding no credential.
    knob(&base, SPACE_A, "fault", serde_json::json!({ "prefix": "/_shroom/api/mint", "status": 503 }))?;
    d.rt.post(Event::PopoverDismissed);
    let (dir2, rec2) = record_take(d, || click(&d.app, &d.rt), false)?;
    d.check("the second take waits, holding no credential", !dir2.join("uploaded").exists() && !dir2.join("cred.json").exists(), "");
    signal(work, "member-take2-waiting");

    // 4. The owner removed this Mac; the space mints again. The next look learns it.
    await_signal(d, work, "go-learn", 600)?;
    knob(&base, SPACE_A, "fault", serde_json::json!({ "clear": true }))?;
    d.rt.post(Event::PopoverDismissed);
    d.after(300);
    click(&d.app, &d.rt);
    let s = d.wait("member-revoked", 120, |s| s.popover.as_ref().is_some_and(|p| p.banner.as_ref().is_some_and(|b| b.id == "member-revoked")))?;
    let p = s.popover.as_ref().expect("the popover");
    let banner = p.banner.as_ref().expect("the banner");
    d.check("the revoked banner says so in the spec's words", banner.line == shroom_core::copy::member_revoked("Bagrat"), banner.line.clone());
    let save = banner.buttons.iter().find(|b| b.action == Action::SaveStrandedToMovies && b.enabled).cloned();
    d.check("…with Save to Movies for the waiting take", save.as_ref().is_some_and(|b| b.label == shroom_core::copy::save_n_to_movies(1)), format!("{:?}", banner.buttons.iter().map(|b| &b.label).collect::<Vec<_>>()));
    d.check("Start greys for a removed member", match &p.body { shroom_core::scene::PopoverBody::Config(c) => !c.start.enabled, _ => false }, "config");
    d.check("the take never uploaded (no credential, a removed member)", !dir2.join("uploaded").exists(), "");
    d.act(Surface::Popover, Action::SaveStrandedToMovies);
    let movies = d.rt.home.root.join("Movies");
    let file = poll(60, || std::fs::read_dir(&movies).ok()?.flatten().map(|e| e.path()).find(|p| p.extension().is_some_and(|x| x == "mp4"))).ok_or("nothing landed in the throwaway Movies")?;
    let ffmpeg = crate::runtime::locate_ffmpeg().ok_or("no ffmpeg")?;
    d.check("Save to Movies made a playable file in this Mac's (throwaway) Movies", shroom_encode::take::playable(&ffmpeg, &file), file.display().to_string());
    d.check("…and never ~/Movies", file.starts_with(&d.rt.home.root), file.display().to_string());
    d.wait("Saved to Movies ✓", 30, |s| s.popover.as_ref().and_then(|p| p.banner.as_ref()).is_some_and(|b| b.buttons.iter().any(|x| x.label == shroom_core::copy::saved_to_movies())))?;
    d.rt.post(Event::PopoverDismissed);
    signal(work, "member-saved");

    // 5. Invited again: the page's Open shroom (the deep link), Join — the same member back.
    await_signal(d, work, "go-rejoin", 600)?;
    let again: serde_json::Value = home::read_json(&work.join("rejoin.handoff")).ok_or("no second hand-off")?;
    let link = again["link"].as_str().ok_or("no link")?.to_string();
    let (host, code) = shroom_cloud::member::parse_link(&link).ok_or("unparsable")?;
    d.seen.lock().unwrap().push(code.clone());
    let _ = std::fs::remove_file(work.join("rejoin.handoff"));
    crate::runtime::member::deep_link(&d.rt, &format!("shroom://join/{host}/{code}"));
    let s = d.wait("the invite's window offers Join", 60, |s| s.offers(Surface::Onboarding, &Action::Join))?;
    d.fact_like("rejoin_face", s.onboarding.as_ref().map(|o| o.id).unwrap_or_default());
    d.act(Surface::Onboarding, Action::Join);
    d.wait("joined again", 60, |s| s.onboarding.as_ref().is_some_and(|o| o.id.starts_with("all-set") || o.id.starts_with("done")))?;
    let roster = knob_get(&base, SPACE_A, "roster")?;
    let space = read_file(d, "space.json");
    d.check("the same member, revived", space.as_ref().is_some_and(|v| v["member_id"] == member.as_str()) && roster["members"][&member]["state"] == "active", format!("{}", roster["members"][&member]["state"]));
    if s.onboarding.is_some() {
        d.act(Surface::Onboarding, Action::CloseWindow);
    }
    signal(work, "member-rejoined");

    // 6. The waiting take drains into the space.
    poll(180, || dir2.join("uploaded").exists().then_some(())).ok_or("the waiting take never drained")?;
    d.step("drained");
    let keys2: Vec<String> = objects(&base, SPACE_A)?.iter().filter_map(|o| o["key"].as_str().map(String::from)).collect();
    d.check("the take that waited is in the space now", keys2.contains(&format!("meta/{rec2}.json")) && keys2.contains(&format!("rec/{rec2}/master.m3u8")), format!("{} keys", keys2.len()));
    d.check("the first video is still there", keys2.contains(&format!("meta/{rec1}.json")), "");
    signal(work, "member-drained");
    Ok(())
}

// ── the heartbeat's member leg, live (TESTING-PLAN §5; DECISIONS B68) ──

/// The heartbeat's member: joins the fenced live space from the owner's invite (the
/// installer's hand-off), records the pattern into it — minted by the live page, uploaded
/// with that credential — then, once the owner removed it, its next look reads
/// `member-revoked` from the live page's `/me`. The heartbeat steps it through files in
/// its work folder, and reads the video back from the space itself.
fn hb_member(d: &Drive, work: &std::path::Path) -> Result<(), String> {
    use shroom_cloud::device::DeviceKey;
    d.wait("member-welcome", 90, is_onb("member-welcome"))?;
    d.act(Surface::Onboarding, Action::Join);
    d.wait("member-done", 90, is_onb("member-done"))?;
    d.act(Surface::Onboarding, Action::AllowRecording);
    d.wait("member-all-set", 60, is_onb("member-all-set"))?;
    let key = home::read_json::<DeviceKey>(&d.rt.home.path("device.key")).ok_or("no device.key")?;
    let space = read_file(d, "space.json").ok_or("no space.json")?;
    let host = space["host"].as_str().unwrap_or_default().to_string();
    d.check(
        "joined the live test space as a member, with its own device key",
        space["role"] == "Member" && space["member_id"].as_str().is_some_and(|m| m.starts_with("m_")) && (host.starts_with(crate::runtime::TEST_PREFIX) || (d.rt.e2e() && crate::runtime::e2e_host(&host))) && key.host == host,
        format!("{} · {}", space["role"], host),
    );
    d.check("no token on a member's Mac", d.rt.home.token().is_none(), "token.json");

    // A video, minted by the live page and uploaded with its credential.
    let (dir, rec) = record_one(d, || d.act(Surface::Onboarding, Action::RecordNow))?;
    let cred = home::read_json::<shroom_cloud::device::CredFile>(&dir.join("cred.json"));
    d.check("the take carried an upload credential the space minted for it", cred.as_ref().is_some_and(|c| c.purpose == "upload"), format!("{:?}", cred.as_ref().map(|c| &c.purpose)));
    d.fact_like("rec", &rec);
    d.fact_like("host", &host);
    let note = serde_json::json!({ "rec": rec, "host": host });
    std::fs::write(work.join("member-recorded"), note.to_string()).map_err(|e| e.to_string())?;

    // The owner removed this Mac: the next look learns it from the live page.
    await_signal(d, work, "go-learn", 600)?;
    d.rt.post(Event::PopoverDismissed);
    d.after(300);
    click(&d.app, &d.rt);
    let s = d.wait("member-revoked", 90, |s| s.popover.as_ref().is_some_and(|p| p.banner.as_ref().is_some_and(|b| b.id == "member-revoked")))?;
    let p = s.popover.as_ref().expect("the popover");
    let banner = p.banner.as_ref().expect("the banner");
    let owner = space["owner_name"].as_str().unwrap_or_default();
    d.check("the revoked banner says so in the spec's words", banner.line == shroom_core::copy::member_revoked(owner), banner.line.clone());
    d.check("…with Leave space… (nothing waits to be saved)", banner.buttons.iter().any(|b| b.label == shroom_core::copy::leave_space_ellipsis()), format!("{:?}", banner.buttons.iter().map(|b| &b.label).collect::<Vec<_>>()));
    d.check("Start greys for a removed member", match &p.body { shroom_core::scene::PopoverBody::Config(c) => !c.start.enabled, _ => false }, "config");
    d.rt.post(Event::PopoverDismissed);
    signal(work, "member-learned");
    Ok(())
}

/// The heartbeat's owner half of the member leg (DECISIONS B68), run by the heartbeat's
/// own drive on the dev home: `Invite` makes an invite in Members and hands its link over
/// as the installer would (a mode-600 file in `work`); `Remove` removes that member in
/// Members (the sheet has no Return default) and reads Members again from the live space.
/// What it did, as JSON for the drive's report — never a secret.
pub enum HbOwner {
    Invite,
    Remove,
}

pub fn heartbeat_owner(app: &AppHandle, rt: &Runtime, work: &std::path::Path, step: HbOwner) -> serde_json::Value {
    use shroom_core::scene::RowRef;
    let d = Drive { app: app.clone(), rt: rt.clone(), t0: Instant::now(), report: Arc::new(Mutex::new(Report { drive: "heartbeat member leg".into(), ..Report::default() })), seen: Arc::default() };
    let result = (|| -> Result<(), String> {
        match step {
            HbOwner::Invite => {
                let row = invite_by_members(&d, work, "invite.handoff", "Heartbeat", 60)?;
                d.fact_like("born_row", row);
            }
            HbOwner::Remove => {
                d.open(Action::OpenSettings)?;
                d.wait("settings", 10, |s| s.settings.is_some())?;
                d.act(Surface::Settings, Action::SettingsTab(SettingsTab::Members));
                let joined = |s: &Scene| members_rows(s).and_then(|m| m.rows.into_iter().find(|r| r.line.starts_with("Heartbeat") && r.born.is_none() && r.buttons.iter().any(|b| matches!(b.action, Action::Remove(RowRef::Member(_))))));
                let s = d.wait("the member's row, joined", 60, |s| joined(s).is_some())?;
                let row = joined(&s).ok_or("no row")?;
                let remove = row.buttons.iter().find(|b| matches!(b.action, Action::Remove(RowRef::Member(_)))).map(|b| b.action.clone()).ok_or("no Remove…")?;
                d.act(Surface::Settings, remove);
                let s = d.wait("member-remove-confirm", 10, |s| s.settings.as_ref().is_some_and(|w| w.body.sheet.is_some()))?;
                d.check("law 9: removing has no Return default", s.ret(Surface::Settings).is_none(), format!("{:?}", s.ret(Surface::Settings)));
                d.act(Surface::Settings, Action::ConfirmRemove);
                let gone = |s: &Scene| members_rows(s).is_some_and(|m| m.rows.iter().all(|r| !r.line.starts_with("Heartbeat"))) && s.settings.as_ref().is_some_and(|w| w.body.sheet.is_none());
                d.wait("the member's row gone", 60, gone)?;
                d.rt.post(Event::WindowClosed(WindowKind::Settings));
                d.wait("closed", 5, |s| s.settings.is_none())?;
                // Law 1: Members reads the roster again on entry — from the live space.
                d.open(Action::OpenSettings)?;
                d.wait("settings again", 10, |s| s.settings.is_some())?;
                d.act(Surface::Settings, Action::SettingsTab(SettingsTab::Members));
                let s = d.wait("Members read again", 60, |s| faces(s).contains(&"settings-members") && members_rows(s).is_some_and(|m| m.invite_line.contains("link")))?;
                d.after(2000);
                let s = snapshot(&d.rt).unwrap_or(s);
                d.check("the live roster no longer lists the member", gone(&s), format!("{:?}", members_rows(&s).map(|m| m.rows.iter().map(|r| r.line.clone()).collect::<Vec<_>>())));
                d.rt.post(Event::WindowClosed(WindowKind::Settings));
                d.wait("closed", 5, |s| s.settings.is_none())?;
            }
        }
        Ok(())
    })();
    // Law 13 on the dev home's own files: the born link's code lives only in the hand-off.
    let code = d.seen.lock().unwrap().clone();
    let root = rt.home.root.clone();
    let mut leaked = Vec::new();
    for dir in [root.clone(), root.join("logs")] {
        for p in std::fs::read_dir(&dir).into_iter().flatten().flatten().map(|e| e.path()).filter(|p| p.is_file()) {
            let Ok(bytes) = std::fs::read(&p) else { continue };
            if code.iter().any(|c| bytes.windows(c.len()).any(|w| w == c.as_bytes())) {
                leaked.push(p.strip_prefix(&root).unwrap_or(&p).display().to_string());
            }
        }
    }
    d.check("law 13: the invite's code is in no file of this home", leaked.is_empty(), format!("{leaked:?}"));
    // What Members showed last, for reading a failure (labels and buttons — never a link).
    if result.is_err() {
        let rows = snapshot(rt).and_then(|s| members_rows(&s)).map(|m| m.rows.iter().map(|r| format!("{} · born {} · {:?}", r.line, r.born.is_some(), r.buttons.iter().map(|b| &b.label).collect::<Vec<_>>())).collect::<Vec<_>>());
        d.fact_like("members_rows", rows);
    }
    let mut r = d.report.lock().unwrap();
    if let Err(e) = result {
        r.error = Some(e);
    }
    let ok = r.error.is_none() && r.checks.iter().all(|(_, ok, _)| *ok);
    serde_json::json!({ "ok": ok, "error": r.error, "steps": r.steps, "checks": r.checks, "facts": r.facts })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The drive's pace back to the mushroom: what it reads must have landed and stayed the
    /// same for the whole wait — the first reading that says the keyboard is elsewhere
    /// isn't the end of the hand-over (the other app's window comes forward after it).
    #[test]
    fn a_hand_over_has_to_land_and_hold_still() {
        let t0 = Instant::now();
        let ms = |n: u64| t0 + Duration::from_millis(n);
        let hand = Duration::from_millis(1000);
        let mut still = Still::default();
        assert!(!still.held(false, "shroom has the keyboard", ms(0), hand), "not landed");
        assert!(!still.held(true, "another app has it; shroom's window on top", ms(50), hand), "landed this instant is not held");
        assert!(!still.held(true, "another app has it; shroom's window on top", ms(900), hand), "not yet a whole wait");
        // Its window comes forward: the reading changed, so the wait starts again.
        assert!(!still.held(true, "another app has it; its window on top", ms(950), hand));
        assert!(!still.held(true, "another app has it; its window on top", ms(1900), hand), "a whole wait from the change, not from the start");
        assert!(still.held(true, "another app has it; its window on top", ms(1950), hand));
        // The keyboard comes back for a moment: nothing held counts any more.
        assert!(!still.held(false, "shroom has the keyboard", ms(2000), hand));
        assert!(!still.held(true, "another app has it; its window on top", ms(2050), hand));
        assert!(still.held(true, "another app has it; its window on top", ms(3050), hand));
    }
}
