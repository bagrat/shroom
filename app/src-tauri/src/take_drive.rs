//! Region 3's take drives (dev builds only, `--home <tmp>` + `--cloud-fake <api> <r2>`): a
//! provisioned Mac recording the test pattern through the real core, lanes and presenter,
//! interrupted the ways a take really is (audit b2-take). Each writes a JSON report with
//! the observed state sequence and its checks, then exits.
//!
//! - `--drive take <out> disk`: the home sits on a small disk image the runner (the M5
//!   step in `app/test/take-drive.mjs`) filled to just above the floor. A take records
//!   until the recorder's watchdog finds the floor: it must end as `disk-full-stop`, whole
//!   and unpublished, without ever meeting ENOSPC (law 2).
//! - `--drive take <out> signals`: the bubble (a native panel that never takes the
//!   keyboard, on the core's camera, dragged and remembered), every window excluded from
//!   capture at creation, Camera as the source (its preview, the count on its display,
//!   the preview staying through the take while the take still gets its frames,
//!   `camera-lost` for the recorded camera — then Discard, so the room never outlives the
//!   drive), then display takes interrupted by: the microphone leaving (`mic-lost`, Resume
//!   without sound), the microphone switched off (`permission-lost (microphone)`), the
//!   encoder killed (`kill -9`: capture dying → a saved take), and screen recording
//!   switched off (`permission-lost`).
//! - `--drive take <out> movies`: the space's storage deleted on the fake, a take stopped
//!   that the space can't take (the uploader never builds its single-file copy), then Save
//!   to Movies from the Library: the file lands in the throwaway home's own `Movies/` under
//!   the take's title, playable, and a second save never overwrites it (b4 #1, #2, #10).
//!
//! - `--drive take <out> upload-kill` / `upload-resume` (L3, DECISIONS B64): the runner
//!   (`tests/e2e/kill-relaunch.mjs upload`) slows the fake's bucket with the fault seam; a
//!   take of the pattern is stopped, and once some of it is up the runner `kill -9`s this
//!   process (the first half writes its report first, then waits to be killed). The second
//!   half is the relaunch on the same home, the seam now dropping, resetting and 503-ing a
//!   few requests: the take must still be the same link, never read as interrupted or
//!   saved, and its upload must finish — the runner then holds the bucket to law 2 (every
//!   file once, byte for byte, one record).
//!
//! - `--drive take <out> soak:<minutes>` (L3's soak, `tests/e2e/soak.mjs`): one long take of
//!   the pattern, paused every ~45 s for ~8 s, uploading to the fake (a `sink` bucket, so
//!   hours of video don't sit in its memory); the app's process tree's CPU and memory
//!   sampled every 5 s. Then Stop: the take must be whole, as long as the time recorded,
//!   decodable, uploaded — and memory must not have grown past a bound after warm-up.
//!
//! - `--drive take <out> surfaces` (DEVIATIONS #39, #40; `take_drive/surfaces.rs`): no take
//!   at all — the popover, the countdown and the bubble as AppKit and the window server
//!   have them: who has the keyboard, who is in front, which Space they sit on, whether a
//!   first click acts. `app/test/surfaces-drive.mjs` runs it; `tests/l5/surfaces-vm.sh`
//!   runs it in a clean VM beside another app that goes full screen.
//!
//! - `--drive take <out> left-out` (DECISIONS B114): the popover's and the count's panels
//!   are made as shroom starts and are on the list of windows a take leaves out — and a take
//!   refuses to start when ScreenCaptureKit can't find one of them. So: as shroom starts,
//!   before either has ever shown, both are already found; then a take through the real
//!   flow, both panels lifted above the pattern for the run so that whatever isn't left out
//!   would be in the picture — the popover at `config`, the count counting, each put up
//!   again in the middle of the take, the popover the pause opens. A capture that leaves
//!   nothing out sees them every time (the proof they were there), and
//!   `app/test/take-drive.mjs` reads every frame of the take back: nothing but the pattern.
//!   A locked screen records nothing: the drive says so after its first reading, and stops.
//! - `--drive take <out> not-left-out`: the reader's own proof (as `--guard-probe` is the
//!   pattern guard's). The same drive with the count taken off the list — and only the
//!   count: a digit and a line of copy, nothing of the Mac's — so the take has it in its
//!   picture, and the reader must be seen to find it there.
//!
//! - `--drive take <out> camera-take` (DECISIONS B121): Camera as the source through the
//!   real flow — `config`, Return, the count, five seconds, the pause, Stop — and the take
//!   kept: `camera-take.mp4` and a frame from its middle, `camera-take.png`, beside the
//!   report. Only when the camera the core picked is OBS's virtual camera, which
//!   `dev/mac/camera.sh` sets playing the test pattern on the hosted Mac; the frame must show
//!   the pattern's colour bars, or the take is deleted.
//!
//! - `--drive take <out> show` (`--e2e` only; DECISIONS B127): the take a person makes, paced
//!   for the screen's recording of the pre-merge run — never measured. The mushroom opens
//!   `config`, Show yourself turns the bubble on, Return, the whole count, a few seconds of
//!   the desktop, the mushroom again, Stop, the link — each surface held until it has
//!   painted (a hosted runner paints a new window two to four seconds late). On the e2e
//!   run's own space, from the home its wizard set up; only with OBS's virtual camera
//!   playing the test pattern as the bubble, and the microphone off: nothing of a room.
//!
//! Where a signal can't be produced safely on this Mac — unplugging a device, revoking a
//! TCC grant — it enters at the lowest real layer: the device lists as the observer posts
//! them, the grants through the observer's own `grants_if_changed`. The encoder's death
//! and the disk filling are real.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use serde::Serialize;
use shroom_core::event::{Event, Key, OsNews, Surface};
use shroom_core::scene::{Action, Click, PopoverBody, Scene};
use shroom_core::world::{Access, SourceId, WindowKind};
use tauri::{AppHandle, Manager};

use crate::harness::{click, cover_with_pattern, snapshot, write_json};
use crate::runtime::{Runtime, camera, observe, presenter, probe};

mod surfaces;
pub(crate) use surfaces::key_focus_pid;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TakeDrive {
    Disk,
    Signals,
    Movies,
    UploadKill,
    UploadResume,
    /// L3 at a random point in capture and encode (DECISIONS B69): a take recording (or
    /// paused, on the runner's word) when the runner `kill -9`s this process.
    CaptureKill,
    /// The relaunch after it: a login launch, silent, the amber dot; the recovered take's
    /// banner, Get link, the upload.
    CaptureResume,
    /// A pause-heavy take this many minutes long.
    Soak(u32),
    /// No take: the overlays as macOS has them (`take_drive/surfaces.rs`).
    Surfaces,
    /// A take from a cold start, and what shroom leaves out of its picture.
    LeftOut,
    /// The same with the count taken off the list: the take must have it in its picture.
    NotLeftOut,
    /// Camera as the source, a take kept — the hosted Mac's pattern camera only (B121).
    CameraTake,
    /// Camera only with its preview dragged first: the place kept through the count and the
    /// take, the preview never reopened, and the take alive to the Library (Bagrat's 0.1.10
    /// checkpoint: the rectangle snapped back at Start and the take died unsaid).
    CameraDrag,
    /// The take a person makes, paced for the pre-merge run's video (B127).
    Show,
}

impl TakeDrive {
    pub fn parse(s: &str) -> Option<TakeDrive> {
        Some(match s {
            "disk" => TakeDrive::Disk,
            "signals" => TakeDrive::Signals,
            "movies" => TakeDrive::Movies,
            "upload-kill" => TakeDrive::UploadKill,
            "upload-resume" => TakeDrive::UploadResume,
            "capture-kill" => TakeDrive::CaptureKill,
            "capture-resume" => TakeDrive::CaptureResume,
            "surfaces" => TakeDrive::Surfaces,
            "left-out" => TakeDrive::LeftOut,
            "not-left-out" => TakeDrive::NotLeftOut,
            "camera-take" => TakeDrive::CameraTake,
            "camera-drag" => TakeDrive::CameraDrag,
            "show" => TakeDrive::Show,
            _ => match s.strip_prefix("soak:").and_then(|m| m.parse::<u32>().ok()) {
                Some(m) if (1..=240).contains(&m) => TakeDrive::Soak(m),
                _ => return None,
            },
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            TakeDrive::Disk => "take disk",
            TakeDrive::Signals => "take signals",
            TakeDrive::Movies => "take movies",
            TakeDrive::UploadKill => "take upload-kill",
            TakeDrive::UploadResume => "take upload-resume",
            TakeDrive::CaptureKill => "take capture-kill",
            TakeDrive::CaptureResume => "take capture-resume",
            TakeDrive::Soak(_) => "take soak",
            TakeDrive::Surfaces => "take surfaces",
            TakeDrive::LeftOut => "take left-out",
            TakeDrive::NotLeftOut => "take not-left-out",
            TakeDrive::CameraTake => "take camera-take",
            TakeDrive::CameraDrag => "take camera-drag",
            TakeDrive::Show => "take show",
        }
    }

    /// Every state id of each Scene — and, for the drive that reads what has shown so far,
    /// whether the Scene has the popover or the count up.
    pub fn probe(self) -> fn(&Scene) -> String {
        match self {
            TakeDrive::LeftOut | TakeDrive::NotLeftOut => |s| format!("{}{}", s.state_ids().join(" | "), if s.popover.is_some() || s.countdown.is_some() { OVERLAY_UP } else { "" }),
            _ => |s| s.state_ids().join(" | "),
        }
    }
}

/// How the left-out drive's trace marks a Scene that has the popover or the count up.
const OVERLAY_UP: &str = " (an overlay up)";

#[derive(Serialize, Default)]
struct Report {
    drive: String,
    states: Vec<String>,
    steps: Vec<(String, u128)>,
    checks: Vec<(String, bool, String)>,
    facts: serde_json::Map<String, serde_json::Value>,
    ok: bool,
    error: Option<String>,
}

struct D {
    app: AppHandle,
    rt: Runtime,
    t0: Instant,
    report: Arc<Mutex<Report>>,
    out: PathBuf,
}

impl D {
    fn step(&self, what: &str) {
        self.report.lock().unwrap().steps.push((what.to_string(), self.t0.elapsed().as_millis()));
    }
    fn check(&self, what: &str, ok: bool, seen: impl Into<String>) {
        self.report.lock().unwrap().checks.push((what.to_string(), ok, seen.into()));
    }
    fn fact(&self, key: &str, value: impl Serialize) {
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
        Err(format!("timed out waiting for {what}; at {:?}", snapshot(&self.rt).map(|s| s.state_ids())))
    }
    fn act(&self, action: Action) {
        self.rt.post(Event::Act { surface: Surface::Popover, action, rev: 0 });
    }
    fn on_main<T: Send + 'static>(&self, f: impl FnOnce() -> T + Send + 'static) -> Option<T> {
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = self.app.run_on_main_thread(move || {
            let _ = tx.send(f());
        });
        rx.recv_timeout(Duration::from_secs(5)).ok()
    }
    fn after(&self, ms: u64) -> Scene {
        std::thread::sleep(Duration::from_millis(ms));
        snapshot(&self.rt).expect("a Scene")
    }
    /// The devices as the observer reads them now (main thread).
    fn devices(&self) -> (Vec<shroom_core::world::Display>, Vec<shroom_core::world::Device>, Vec<shroom_core::world::Device>) {
        self.on_main(|| {
            let mtm = objc2_foundation::MainThreadMarker::new().expect("main");
            (probe::displays(mtm), probe::mics(), probe::cameras())
        })
        .unwrap_or_default()
    }
    /// The mushroom, clicked on a closed popover (a click on an open one closes it), then
    /// `config` as the Scene shows it.
    fn config(&self) -> Result<Scene, String> {
        if snapshot(&self.rt).is_some_and(|s| s.popover.is_some()) {
            self.rt.post(Event::PopoverDismissed);
            self.wait("the popover closed", 5, |s| s.popover.is_none())?;
        }
        click(&self.app, &self.rt);
        let s = self.wait("config", 10, |s| s.popover.as_ref().is_some_and(|p| p.id == "config"))?;
        // Show yourself starts on (N43): every drive begins with the bubble off, and the
        // ones about it switch it on themselves.
        if s.bubble.is_none() {
            return Ok(s);
        }
        self.act(Action::SetCamera(None));
        self.wait("the bubble off", 5, |s| s.bubble.is_none())
    }
    /// Return at `config`, through the count, to the red dot — on the display the pattern
    /// covers, chosen and then checked on the Scene the Return starts from (b4 #16).
    fn record(&self) -> Result<(), String> {
        let covered = shroom_capture::main_display_id();
        self.act(Action::SetSource(SourceId::Display(covered)));
        let s = self.wait("the pattern's display chosen", 5, |s| crate::runtime::records(s) == Some(SourceId::Display(covered)))?;
        crate::runtime::guard_source(&s, covered)?;
        self.rt.post(Event::Key { surface: Surface::Popover, key: Key::Return });
        self.wait("countdown", 5, |s| s.countdown.is_some())?;
        self.wait("recording", 15, |s| s.tray.state == Some("recording") && s.countdown.is_none())?;
        Ok(())
    }
    fn popover_id(s: &Scene) -> Option<&'static str> {
        s.popover.as_ref().map(|p| p.id)
    }
    fn newest_take(&self) -> Option<PathBuf> {
        crate::runtime::takes::dirs(&self.rt.home.takes()).into_iter().last()
    }
}

pub fn drive(app: AppHandle, rt: Runtime, which: TakeDrive, out: PathBuf) {
    let d = D { app: app.clone(), rt: rt.clone(), t0: Instant::now(), report: Arc::new(Mutex::new(Report { drive: which.name().into(), ..Report::default() })), out: out.clone() };
    // The pointer moves off the counter strip; the pattern covers what's recorded. (The
    // surfaces drive records nothing, and leaves the pointer where the person has it.)
    // (The left-out drive moves it only once it knows it will record.)
    // The show pass records the desktop as a person would see it, the pointer where it is.
    if !matches!(which, TakeDrive::Surfaces | TakeDrive::LeftOut | TakeDrive::NotLeftOut | TakeDrive::Show) {
        warp_pointer_away();
    }
    let result = match which {
        TakeDrive::Disk => disk(&d),
        TakeDrive::Signals => signals(&d),
        TakeDrive::Movies => movies(&d),
        TakeDrive::UploadKill => upload_kill(&d),
        TakeDrive::UploadResume => upload_resume(&d),
        TakeDrive::CaptureKill => capture_kill(&d),
        TakeDrive::CaptureResume => capture_resume(&d),
        TakeDrive::Soak(minutes) => soak(&d, minutes),
        TakeDrive::Surfaces => surfaces::surfaces(&d),
        TakeDrive::LeftOut => left_out(&d, false),
        TakeDrive::NotLeftOut => left_out(&d, true),
        TakeDrive::CameraTake => camera_take(&d),
        TakeDrive::CameraDrag => camera_drag(&d),
        TakeDrive::Show => show(&d),
    };
    if let Err(e) = result {
        d.report.lock().unwrap().error = Some(e);
    }
    let code = {
        let mut r = d.report.lock().unwrap();
        r.states = rt.trace.lock().unwrap().as_ref().map(|t| t.ids.clone()).unwrap_or_default();
        r.ok = r.error.is_none() && r.checks.iter().all(|(_, ok, _)| *ok);
        write_json(&out, &*r)
    };
    rt.ending_ours.store(true, std::sync::atomic::Ordering::SeqCst);
    app.exit(code);
}

fn warp_pointer_away() {
    #[repr(C)]
    struct CGPoint {
        x: f64,
        y: f64,
    }
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGWarpMouseCursorPosition(p: CGPoint) -> i32;
    }
    unsafe { CGWarpMouseCursorPosition(CGPoint { x: 4000.0, y: 3000.0 }) };
}

/// A take's folder, as the disk has it: its reason, its files, and whether it decodes.
fn take_files(dir: &Path) -> serde_json::Value {
    let finalized: Option<serde_json::Value> = crate::runtime::home::read_json(&dir.join("finalized.json"));
    let playlist = std::fs::read_to_string(dir.join(shroom_encode::files::PLAYLIST)).unwrap_or_default();
    let segments: Vec<&str> = playlist.lines().filter(|l| !l.starts_with('#') && !l.is_empty()).collect();
    let missing: Vec<&str> = segments.iter().copied().filter(|s| !dir.join(s).exists()).collect();
    let decode = crate::runtime::locate_ffmpeg().map(|ff| {
        std::process::Command::new(ff)
            .args(["-v", "error", "-i"])
            .arg(dir.join(shroom_encode::files::PLAYLIST))
            .args(["-f", "null", "-"])
            .output()
            .map(|o| (o.status.success(), String::from_utf8_lossy(&o.stderr).trim().to_string()))
            .unwrap_or((false, "ffmpeg didn't run".into()))
    });
    serde_json::json!({
        "dir": dir.file_name().map(|n| n.to_string_lossy().into_owned()),
        "why": finalized.as_ref().and_then(|f| f["why"].as_str().map(String::from)),
        "length_ms": finalized.as_ref().and_then(|f| f["length_ms"].as_u64()),
        "published": dir.join("publish.json").exists(),
        "segments": segments.len(),
        "missing": missing,
        "decodes": decode.as_ref().map(|d| d.0),
        "decode_errors": decode.map(|d| d.1),
    })
}

// ── disk-full-stop on a small disk ──

fn disk(d: &D) -> Result<(), String> {
    let home = d.rt.home.root.clone();
    d.fact("free_at_start", probe::disk_free(&home));
    d.fact("floor_at_start", shroom_core::derive::take_floor_bytes(shroom_core::world::Quality::Normal, 0));
    let pattern = cover_with_pattern(&d.app)?;
    let result = (|| {
        d.wait("ready", 20, |s| s.tray.click == Click::OpenPopover)?;
        let s = d.config()?;
        if let Some(PopoverBody::Config(c)) = s.popover.as_ref().map(|p| &p.body) {
            d.check("Start is live with room above the floor", c.start.enabled, format!("{:?}", c.room_line));
        }
        d.record()?;
        let started = Instant::now();
        let s = d.wait("disk-full-stop", 300, |s| D::popover_id(s) == Some("disk-full-stop"))?;
        d.fact("recorded_seconds", started.elapsed().as_secs_f64());
        d.fact("free_at_stop", probe::disk_free(&home));
        let p = s.popover.as_ref().unwrap();
        d.check("the stop opens by itself without the keyboard (law 5)", !p.activates, "");
        let line = match &p.body {
            PopoverBody::Stopped(b) => b.line.clone(),
            _ => String::new(),
        };
        d.check("it says the Mac ran out of room", line.starts_with("This Mac ran out of room, so the recording stopped at "), line);
        d.check("the tray isn't red", s.tray.state != Some("recording"), format!("{:?}", s.tray.state));
        let dir = d.newest_take().ok_or("no take")?;
        let files = take_files(&dir);
        d.check("the take was finalized for the disk", files["why"] == "disk-full", files["why"].to_string());
        d.check("never auto-published (N25)", files["published"] == false, "");
        d.check("every segment is on disk (law 2)", files["missing"].as_array().is_some_and(|m| m.is_empty()) && files["segments"].as_u64().unwrap_or(0) > 0, files["missing"].to_string());
        d.check("the whole take decodes", files["decodes"] == true, files["decode_errors"].to_string());
        d.fact("take", files);
        let log = std::fs::read_to_string(home.join("logs/run.log")).unwrap_or_default();
        let enospc: Vec<&str> = log.lines().filter(|l| l.contains("No space left") || l.contains("ENOSPC") || l.contains("os error 28")).collect();
        d.check("nothing ever met ENOSPC", enospc.is_empty(), enospc.join(" / "));
        d.fact("watchdog", log.lines().filter(|l| l.contains("recorder watchdog")).map(String::from).collect::<Vec<_>>());
        Ok(())
    })();
    let _ = pattern.close();
    result
}

// ── Save to Movies on a take the space can't take ──

fn movies(d: &D) -> Result<(), String> {
    d.wait("ready", 20, |s| s.tray.click == Click::OpenPopover)?;
    let movies = d.rt.home.root.join("Movies");
    d.check("Movies is the throwaway home's own (never ~/Movies)", crate::runtime::takes::movies_dir(&d.rt).as_deref() == Some(movies.as_path()), movies.display().to_string());
    // The space's storage goes, as when the bucket is deleted in the dashboard.
    let (api, _) = d.rt.cloud_fake.clone().ok_or("no fake")?;
    let url = format!("{}/_fake/drop-bucket%20{}", api.trim_end_matches("/client/v4"), shroom_cloud::wire::BUCKET);
    ureq::post(&url).send_empty().map_err(|e| format!("the fake's drop-bucket: {e}"))?;
    d.step("the space's storage deleted");

    let pattern = cover_with_pattern(&d.app)?;
    let recorded = (|| {
        d.config()?;
        d.record()?;
        std::thread::sleep(Duration::from_secs(3));
        click(&d.app, &d.rt);
        d.wait("paused", 10, |s| D::popover_id(s) == Some("paused"))?;
        d.act(Action::Stop);
        Ok::<(), String>(())
    })();
    let _ = pattern.close();
    recorded?;
    let takes = d.rt.home.takes();
    let until = Instant::now() + Duration::from_secs(15);
    let dir = loop {
        if let Some(t) = crate::runtime::takes::dirs(&takes).into_iter().last().filter(|t| t.join("publish.json").exists()) {
            break t;
        }
        if Instant::now() > until {
            return Err("no publish.json after Stop".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let title = crate::runtime::home::read_json::<crate::runtime::takes::PublishJson>(&dir.join("publish.json")).map(|p| p.title).ok_or("no title")?;
    let key = dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default();

    // The space refuses it: the Library's row waits for the space, Save to Movies beside it.
    d.rt.post(Event::PopoverDismissed);
    d.config()?;
    d.act(Action::OpenLibrary);
    let row_save = Action::SaveRowToMovies(shroom_core::world::TakeDir(key));
    let s = d.wait("the row waits for the space", 60, |s| {
        s.library.as_ref().is_some_and(|l| l.body.rows.iter().any(|r| r.state.as_deref() == Some("Waiting for your space") && r.buttons.iter().any(|b| b.action == row_save && b.enabled)))
    })?;
    d.fact("library_faces", s.library.as_ref().map(|l| l.faces.clone()));
    d.check("the uploader never made its single-file copy", !dir.join(shroom_encode::files::PREVIEW).exists(), "no preview.mp4 before the save");

    let save = |what: &str, want: PathBuf| -> Result<(), String> {
        d.rt.post(Event::Act { surface: Surface::Library, action: row_save.clone(), rev: 0 });
        let until = Instant::now() + Duration::from_secs(60);
        while !want.is_file() && Instant::now() < until {
            std::thread::sleep(Duration::from_millis(100));
        }
        let ffmpeg = crate::runtime::locate_ffmpeg().ok_or("no ffmpeg")?;
        d.check(&format!("{what}: {} is in Movies and plays", want.file_name().unwrap_or_default().to_string_lossy()), shroom_encode::take::playable(&ffmpeg, &want), want.display().to_string());
        d.wait(&format!("{what}: the button is Save to Movies again"), 10, |s| {
            s.library.as_ref().is_some_and(|l| l.body.rows.iter().any(|r| r.buttons.iter().any(|b| b.action == row_save && b.enabled && b.label == "Save to Movies")))
        })?;
        Ok(())
    };
    let stem = crate::runtime::takes::file_stem(&title);
    save("saved under its title", movies.join(format!("{stem}.mp4")))?;
    let first = std::fs::read(movies.join(format!("{stem}.mp4"))).map(|b| b.len()).unwrap_or(0);
    save("saved again, beside it", movies.join(format!("{stem} 2.mp4")))?;
    d.check("the first copy was never overwritten", std::fs::read(movies.join(format!("{stem}.mp4"))).map(|b| b.len()).unwrap_or(0) == first && first > 0, format!("{first} bytes"));
    d.check("the take is still on this Mac, still waiting (N34)", dir.join("publish.json").exists() && !dir.join("uploaded").exists(), "");
    let names: Vec<String> = std::fs::read_dir(&movies).map(|it| it.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect()).unwrap_or_default();
    d.check("nothing else in Movies (no temporary left)", names.len() == 2, format!("{names:?}"));
    d.fact("movies", names);
    let log = std::fs::read_to_string(d.rt.home.root.join("logs/run.log")).unwrap_or_default();
    d.check("the shell would reveal it in Finder (quiet run)", log.contains("shell would reveal"), "run.log");
    d.rt.post(Event::WindowClosed(WindowKind::Library));
    Ok(())
}

// ── killed mid-upload, relaunched (L3) ──

/// The newest take that Stop published, waited for.
fn published_take(d: &D, secs: u64) -> Result<(PathBuf, crate::runtime::takes::PublishJson), String> {
    let until = Instant::now() + Duration::from_secs(secs);
    loop {
        if let Some(dir) = d.newest_take().filter(|t| t.join("publish.json").exists()) {
            if let Some(p) = crate::runtime::home::read_json::<crate::runtime::takes::PublishJson>(&dir.join("publish.json")) {
                return Ok((dir, p));
            }
        }
        if Instant::now() > until {
            return Err("no publish.json after Stop".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn upload_kill(d: &D) -> Result<(), String> {
    d.wait("ready", 20, |s| s.tray.click == Click::OpenPopover)?;
    let pattern = cover_with_pattern(&d.app)?;
    let recorded = (|| {
        d.config()?;
        d.record()?;
        std::thread::sleep(Duration::from_secs(8));
        click(&d.app, &d.rt);
        d.wait("paused", 10, |s| D::popover_id(s) == Some("paused"))?;
        d.act(Action::Stop);
        Ok::<(), String>(())
    })();
    let _ = pattern.close();
    recorded?;
    let (dir, publish) = published_take(d, 20)?;
    d.check("Stop made the link", !publish.rec.is_empty(), publish.rec.clone());
    d.check("not uploaded yet (the seam is slow)", !dir.join("uploaded").exists(), "");
    d.fact("rec", &publish.rec);
    d.fact("dir", dir.file_name().map(|n| n.to_string_lossy().into_owned()));
    d.fact("pid", std::process::id());
    // The report goes out now: this process is about to be killed, mid-upload.
    {
        let mut r = d.report.lock().unwrap();
        r.states = d.rt.trace.lock().unwrap().as_ref().map(|t| t.ids.clone()).unwrap_or_default();
        r.ok = r.checks.iter().all(|(_, ok, _)| *ok);
        write_json(&d.out, &*r);
    }
    d.step("waiting to be killed");
    std::thread::sleep(Duration::from_secs(180));
    Err("never killed".into())
}

fn upload_resume(d: &D) -> Result<(), String> {
    let s = d.wait("ready", 30, |s| s.tray.click == Click::OpenPopover)?;
    let (dir, publish) = published_take(d, 5)?;
    d.fact("rec", &publish.rec);
    d.fact("boot_states", s.state_ids());
    d.check("the relaunch finds the take as published, still to finish", !dir.join("uploaded").exists(), "");
    let finalized: Option<serde_json::Value> = crate::runtime::home::read_json(&dir.join("finalized.json"));
    d.check("the take was finalized by Stop before the kill", finalized.as_ref().is_some_and(|f| f["why"] == "stop"), format!("{finalized:?}"));
    // The upload finishes by itself — through the seam's drops, resets and 503s.
    let until = Instant::now() + Duration::from_secs(240);
    while !dir.join("uploaded").exists() {
        if Instant::now() > until {
            return Err("the upload never finished after the relaunch".into());
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    d.step("uploaded");
    let (dir2, publish2) = published_take(d, 1)?;
    d.check("the same take, the same link", dir2 == dir && publish2.rec == publish.rec, publish2.rec.clone());
    let takes = crate::runtime::takes::dirs(&d.rt.home.takes());
    d.check("no second take appeared", takes.len() == 1, format!("{} takes", takes.len()));
    let files = take_files(&dir);
    d.check("every segment still on this Mac (law 2)", files["missing"].as_array().is_some_and(|m| m.is_empty()), files["missing"].to_string());
    d.fact("take", files);
    let ids = d.rt.trace.lock().unwrap().as_ref().map(|t| t.ids.join(" → ")).unwrap_or_default();
    d.check("never read as an interrupted, recovered or stranded take", !["interrupted", "recovered", "stranded", "saved"].iter().any(|w| ids.contains(w)), ids);
    Ok(())
}

// ── killed mid-take, relaunched (L3, DECISIONS B69) ──

/// Wall-clock ms (the runner's clock too: it kills on its own timer).
fn wall_ms() -> u128 {
    std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis()).unwrap_or(0)
}

fn capture_kill(d: &D) -> Result<(), String> {
    d.wait("ready", 20, |s| s.tray.click == Click::OpenPopover)?;
    // The pattern stays up until the kill takes it (and this process) down.
    let _pattern = cover_with_pattern(&d.app)?;
    d.config()?;
    d.record()?;
    let started = wall_ms();
    let dir = d.newest_take().ok_or("no take folder once recording")?;
    d.fact("dir", dir.file_name().map(|n| n.to_string_lossy().into_owned()));
    d.fact("pid", std::process::id());
    d.fact("recording_since_ms", started);
    {
        let mut r = d.report.lock().unwrap();
        r.states = d.rt.trace.lock().unwrap().as_ref().map(|t| t.ids.clone()).unwrap_or_default();
        r.ok = r.checks.iter().all(|(_, ok, _)| *ok);
        write_json(&d.out, &*r);
    }
    // The runner may ask for a pause first (the encoder's segment closed, the take held):
    // the kill then lands while paused. The moment the pause took is noted beside the report.
    let work = d.out.parent().unwrap_or(Path::new(".")).to_path_buf();
    let until = Instant::now() + Duration::from_secs(180);
    while Instant::now() < until {
        if work.join("pause-now").exists() && !work.join("paused-at").exists() {
            click(&d.app, &d.rt);
            if d.wait("paused", 10, |s| D::popover_id(s) == Some("paused")).is_ok() {
                let _ = std::fs::write(work.join("paused-at"), wall_ms().to_string());
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Err("never killed".into())
}

fn capture_resume(d: &D) -> Result<(), String> {
    // A login launch after the crash: silent — no popover, no window — and the amber dot
    // once the scan found the saved take (N25).
    let s = d.wait("the saved take found: the amber dot", 60, |s| s.tray.face == shroom_core::scene::Face::Amber)?;
    d.check("the launch stays silent", s.popover.is_none() && s.onboarding.is_none() && s.library.is_none(), s.state_ids().join(" | "));
    let dir = d.newest_take().ok_or("no take on disk after the relaunch")?;
    d.check("one take on this Mac, the one killed", crate::runtime::takes::dirs(&d.rt.home.takes()).len() == 1, "");
    d.check("never published by itself (you didn't choose to stop)", !dir.join("publish.json").exists(), "");
    let files = take_files(&dir);
    d.check("rebuilt whole: every segment of its playlist on this Mac", files["missing"].as_array().is_some_and(|m| m.is_empty()) && files["segments"].as_u64().is_some_and(|n| n > 0), files.to_string());
    d.check("…and it decodes", files["decodes"] == true, files["decode_errors"].to_string());
    d.fact("take", &files);
    // The next click: config, the banner on top.
    let s = d.config()?;
    let banner = s.popover.as_ref().and_then(|p| p.banner.clone()).ok_or("config without the recovered take's banner")?;
    let length_ms = files["length_ms"].as_u64().unwrap_or(0);
    d.check("the banner in the spec's words, with the saved length", banner.id == "recovered-take" && banner.line == shroom_core::copy::recovered(&shroom_core::copy::length(length_ms)), banner.line.clone());
    d.check("Get link is its first door", banner.buttons.first().is_some_and(|b| b.action == Action::GetLink && b.enabled), format!("{:?}", banner.buttons.iter().map(|b| &b.label).collect::<Vec<_>>()));
    d.check("config stays usable beneath it", match s.popover.as_ref().map(|p| &p.body) { Some(PopoverBody::Config(c)) => c.start.enabled, _ => false }, "");
    // Get link: the normal stop sequence from the saved file.
    d.act(Action::GetLink);
    let (dir2, publish) = published_take(d, 30)?;
    d.check("Get link published the take that was saved", dir2 == dir && !publish.rec.is_empty(), publish.rec.clone());
    d.fact("rec", &publish.rec);
    d.fact("dir", dir.file_name().map(|n| n.to_string_lossy().into_owned()));
    let until = Instant::now() + Duration::from_secs(180);
    while !dir.join("uploaded").exists() {
        if Instant::now() > until {
            return Err("the recovered take never finished uploading".into());
        }
        std::thread::sleep(Duration::from_millis(200));
    }
    d.step("uploaded");
    d.wait("the amber dot cleared", 30, |s| s.tray.face != shroom_core::scene::Face::Amber)?;
    Ok(())
}

// ── the soak (L3) ──

/// The time a run log's take spent recording: each `started` or `resumed` to the next
/// `click-pause`, in the log's monotonic ms.
fn recording_intervals_ms(log: &str) -> u64 {
    let mut total = 0;
    let mut since: Option<u64> = None;
    for line in log.lines() {
        let mut words = line.split_whitespace();
        let (Some(mono), Some("core"), Some(what)) = (words.next().and_then(|m| m.parse::<u64>().ok()), words.next(), words.next()) else { continue };
        match what {
            "started" | "resumed" => since = Some(mono),
            "click-pause" => {
                if let Some(t) = since.take() {
                    total += mono.saturating_sub(t);
                }
            }
            _ => {}
        }
    }
    total
}

/// This process and every process under it: (CPU %, resident KiB), summed, as `ps` reads them.
fn tree_usage() -> (f64, u64) {
    let me = std::process::id();
    let out = std::process::Command::new("ps").args(["-A", "-o", "pid=,ppid=,%cpu=,rss="]).output().map(|o| String::from_utf8_lossy(&o.stdout).into_owned()).unwrap_or_default();
    let rows: Vec<(u32, u32, f64, u64)> = out
        .lines()
        .filter_map(|l| {
            let f: Vec<&str> = l.split_whitespace().collect();
            Some((f.first()?.parse().ok()?, f.get(1)?.parse().ok()?, f.get(2)?.parse().ok()?, f.get(3)?.parse().ok()?))
        })
        .collect();
    let mut tree = vec![me];
    let mut i = 0;
    while i < tree.len() {
        let p = tree[i];
        tree.extend(rows.iter().filter(|r| r.1 == p).map(|r| r.0));
        i += 1;
    }
    rows.iter().filter(|r| tree.contains(&r.0)).fold((0.0, 0), |(c, m), r| (c + r.2, m + r.3))
}

fn soak(d: &D, minutes: u32) -> Result<(), String> {
    d.wait("ready", 20, |s| s.tray.click == Click::OpenPopover)?;
    let total = Duration::from_secs(u64::from(minutes) * 60);
    let pattern = cover_with_pattern(&d.app)?;
    let mut samples: Vec<(u64, f64, u64)> = Vec::new();
    let mut paused_ms: u64 = 0;
    let mut pauses = 0u32;
    let recorded = (|| {
        d.config()?;
        d.record()?;
        let start = Instant::now();
        let mut next_pause = start + Duration::from_secs(45);
        let mut next_sample = start;
        while start.elapsed() < total {
            if Instant::now() >= next_sample {
                let (cpu, rss) = tree_usage();
                samples.push((start.elapsed().as_secs(), cpu, rss));
                next_sample += Duration::from_secs(5);
            }
            if Instant::now() >= next_pause {
                click(&d.app, &d.rt);
                d.wait("paused", 10, |s| D::popover_id(s) == Some("paused"))?;
                let held = Instant::now();
                std::thread::sleep(Duration::from_secs(8));
                d.act(Action::Resume);
                d.wait("recording again", 10, |s| s.tray.state == Some("recording") && s.popover.is_none())?;
                paused_ms += held.elapsed().as_millis() as u64;
                pauses += 1;
                next_pause = Instant::now() + Duration::from_secs(45);
            }
            std::thread::sleep(Duration::from_millis(200));
        }
        let ids = snapshot(&d.rt).map(|s| s.state_ids()).unwrap_or_default();
        d.check("still recording at the end of the soak", snapshot(&d.rt).is_some_and(|s| s.tray.state == Some("recording")), format!("{ids:?}"));
        click(&d.app, &d.rt);
        d.wait("paused", 10, |s| D::popover_id(s) == Some("paused"))?;
        d.act(Action::Stop);
        Ok::<u64, String>(start.elapsed().as_millis() as u64)
    })();
    let _ = pattern.close();
    let wall_ms = recorded?;
    let (dir, publish) = published_take(d, 60)?;
    d.fact("rec", &publish.rec);
    let until = Instant::now() + Duration::from_secs(600);
    while !dir.join("uploaded").exists() {
        if Instant::now() > until {
            return Err("the soak's take never finished uploading".into());
        }
        std::thread::sleep(Duration::from_millis(500));
    }
    d.step("uploaded");
    let files = take_files(&dir);
    let length_ms = files["length_ms"].as_u64().unwrap_or(0);
    // The time recorded is the time the core held the take recording: from `started` or
    // `resumed` to the next pause click, read from the run log's monotonic stamps. (Wall
    // time less the pauses this drive saw also counts each pause's and resume's own
    // ~75 ms hand-over as recorded: 17 s over the 2-hour soak's 136 pauses — B70 c.)
    let recorded_ms = recording_intervals_ms(&std::fs::read_to_string(d.rt.home.root.join("logs/run.log")).unwrap_or_default());
    d.fact("wall_ms", wall_ms);
    d.fact("paused_ms", paused_ms);
    d.fact("wall_less_pauses_ms", wall_ms.saturating_sub(paused_ms));
    d.fact("recorded_ms", recorded_ms);
    d.fact("pauses", pauses);
    d.fact("take", &files);
    d.check("the take is as long as the time recorded (± 5 s)", recorded_ms > 0 && length_ms.abs_diff(recorded_ms) <= 5_000, format!("{length_ms} ms vs {recorded_ms} ms recorded"));
    d.check("every segment on disk (law 2)", files["missing"].as_array().is_some_and(|m| m.is_empty()), files["missing"].to_string());
    d.check("the whole take decodes", files["decodes"] == true, files["decode_errors"].to_string());
    d.check("pause-heavy: a pause at least every minute", u64::from(pauses) >= u64::from(minutes).saturating_sub(1), format!("{pauses} pauses"));
    // Memory after warm-up (the first two minutes) against the end; CPU on average.
    let warm: Vec<&(u64, f64, u64)> = samples.iter().filter(|s| s.0 >= 120).collect();
    let rss_start = warm.first().map(|s| s.2).unwrap_or(0);
    let rss_end = warm.last().map(|s| s.2).unwrap_or(0);
    let rss_max = samples.iter().map(|s| s.2).max().unwrap_or(0);
    let cpu_mean = if samples.is_empty() { 0.0 } else { samples.iter().map(|s| s.1).sum::<f64>() / samples.len() as f64 };
    let cpu_max = samples.iter().map(|s| s.1).fold(0.0, f64::max);
    d.fact("rss_kib", serde_json::json!({ "after_warmup": rss_start, "end": rss_end, "max": rss_max }));
    d.fact("cpu_pct", serde_json::json!({ "mean": cpu_mean, "max": cpu_max }));
    d.fact("samples", samples.len());
    d.check("memory doesn't grow past warm-up (≤ 200 MB more by the end)", rss_end <= rss_start + 200 * 1024, format!("{rss_start} KiB → {rss_end} KiB"));
    d.check("capture cost on average ≤ about one core (TESTING-PLAN §4)", cpu_mean <= 110.0, format!("{cpu_mean:.0}% mean, {cpu_max:.0}% max"));
    let ids = d.rt.trace.lock().unwrap().as_ref().map(|t| t.ids.join(" → ")).unwrap_or_default();
    d.check("never an interrupted or died take", !["interrupted", "capture-died", "recovered"].iter().any(|w| ids.contains(w)), "");
    Ok(())
}

// ── the signals ──

fn signals(d: &D) -> Result<(), String> {
    d.wait("ready", 20, |s| s.tray.click == Click::OpenPopover)?;
    let (displays, mics, cameras) = d.devices();
    d.fact("mic_access", format!("{:?}", probe::access(shroom_capture::mic_access())));
    d.fact("camera_access", format!("{:?}", probe::access(shroom_capture::camera_access())));
    d.fact("cameras", &cameras);
    // Continuity from the real device list (B48 a): an iPhone or iPad is never taken for an
    // ordinary camera, whatever type macOS reports it as.
    let raw = shroom_capture::cameras();
    let iphones: Vec<_> = raw.iter().filter(|c| ["iPhone", "iPad"].iter().any(|p| c.model.starts_with(p) || c.name.contains(p))).collect();
    d.fact("raw_cameras", &raw);
    d.fact("iphone_here", !iphones.is_empty());
    d.check("every iPhone or iPad here is Continuity (N39)", iphones.iter().all(|c| c.continuity && !c.builtin), format!("{iphones:?}"));
    d.check(
        "the core's lists say so too",
        iphones.iter().all(|i| cameras.iter().any(|c| c.uid == i.uid && c.continuity && !c.builtin)),
        format!("{cameras:?}"),
    );
    // A Mac with no camera shroom may open (a hosted runner has none) can't show the bubble
    // or Camera as the source: that is named as skipped, not failed. A camera that is there
    // and isn't granted is a failure (issue #8).
    let granted = probe::access(shroom_capture::camera_access()) == Access::Granted;
    let a_camera = cameras.iter().any(|c| !c.continuity);

    // 1. Every shroom window but the bubble is excluded from capture, from creation.
    windows_excluded(d)?;

    // 2. The bubble and Camera as the source (the camera only on this Mac's own grant).
    if !a_camera {
        d.fact("skipped", serde_json::json!([["the bubble, and Camera as the source", "no camera shroom may open on this Mac"]]));
    } else if !granted {
        d.check("the camera is granted here (bubble and Camera as the source)", false, "the camera isn't granted to shroom-dev — those steps were skipped");
    } else {
        d.fact("skipped", serde_json::json!([]));
        bubble(d, &cameras)?;
        camera_source(d, &displays, &mics)?;
    }

    // 3. Display takes, interrupted.
    let pattern = cover_with_pattern(&d.app)?;
    let result = display_takes(d, &displays, &mics, &cameras);
    let _ = pattern.close();
    result
}

fn window_number(app: &AppHandle, label: &'static str) -> Option<u32> {
    let w = app.get_webview_window(label)?;
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = app.run_on_main_thread(move || {
        let n = w.ns_window().ok().map(|p| {
            let ns: &objc2_app_kit::NSWindow = unsafe { &*(p as *const objc2_app_kit::NSWindow) };
            ns.windowNumber() as u32
        });
        let _ = tx.send(n);
    });
    rx.recv_timeout(Duration::from_secs(5)).ok().flatten()
}

fn windows_excluded(d: &D) -> Result<(), String> {
    d.config()?;
    d.act(Action::OpenLibrary);
    d.wait("the Library", 10, |s| s.library.is_some())?;
    d.config()?;
    d.act(Action::OpenSettings);
    d.wait("Settings", 10, |s| s.settings.is_some())?;
    std::thread::sleep(Duration::from_millis(300));
    let excluded = d.rt.excluded.lock().unwrap().clone();
    for (label, name) in [("library", "the Library"), ("settings", "Settings")] {
        let n = window_number(&d.app, label);
        d.check(&format!("{name}'s window is excluded from capture"), n.is_some_and(|n| excluded.contains(&n)), format!("{n:?} in {excluded:?}"));
    }
    // The popover's own panel — the window that shows — not the hidden host of its page.
    let app = d.app.clone();
    let n = d.on_main(move || crate::runtime::presenter::overlay_window(&app, crate::runtime::presenter::Overlay::Popover).map(|w| w.windowNumber() as u32)).flatten();
    d.check("the popover's window is excluded from capture", n.is_some_and(|n| excluded.contains(&n)), format!("{n:?} in {excluded:?}"));
    d.rt.post(Event::WindowClosed(WindowKind::Library));
    d.rt.post(Event::WindowClosed(WindowKind::Settings));
    d.wait("windows closed", 5, |s| s.library.is_none() && s.settings.is_none())?;
    Ok(())
}

/// Show yourself on the camera shroom would pick by itself: the built-in one, else the first
/// that isn't an iPhone (N39) — a drive never picks an iPhone either.
pub(crate) fn show_yourself_on(cameras: &[shroom_core::world::Device]) -> Action {
    let cam = cameras.iter().find(|c| c.builtin && !c.continuity).or_else(|| cameras.iter().find(|c| !c.continuity));
    Action::SetCamera(cam.map(|c| c.uid.clone()))
}

fn bubble(d: &D, cameras: &[shroom_core::world::Device]) -> Result<(), String> {
    let s = d.config()?;
    let on = matches!(s.popover.as_ref().map(|p| &p.body), Some(PopoverBody::Config(c)) if c.camera_picked().is_some());
    if !on {
        d.act(show_yourself_on(cameras));
    }
    let s = d.wait("the bubble", 5, |s| s.bubble.is_some())?;
    let want = s.bubble.clone().unwrap();
    std::thread::sleep(Duration::from_millis(1500));
    let seen = d.on_main(camera::bubble_seen).flatten().ok_or("no bubble on screen")?;
    d.fact("bubble", &seen);
    d.check("the bubble never takes the keyboard (law 5)", !seen.key && !seen.can_become_key && seen.nonactivating, format!("{seen:?}"));
    d.check("the bubble shows the core's camera", seen.camera == want.camera && seen.running, format!("{} running={}", seen.camera, seen.running));
    d.check("never an iPhone by itself (N39)", cameras.iter().any(|c| c.uid == seen.camera && !c.continuity), seen.camera.clone());
    d.check("it sits on the chosen display", seen.display == want.display, format!("{} vs {}", seen.display, want.display));
    let excluded = d.rt.excluded.lock().unwrap().clone();
    d.check("the bubble is recorded (never excluded)", !excluded.contains(&seen.window), "");
    // A drag: the window moves, AppKit says so, the core remembers the place.
    let before = seen.at;
    let _ = d.on_main(|| camera::drag_bubble_by(80.0, -60.0));
    let want_at = (before.0 + 80, before.1 - 60);
    let s = d.wait("the place remembered", 5, |s| s.bubble.as_ref().is_some_and(|b| b.at == Some(want_at)))?;
    let prefs: Option<shroom_core::world::Prefs> = d.rt.home.read_json("prefs.json");
    d.check("the place is in prefs.json", prefs.as_ref().and_then(|p| p.bubble_at) == Some(want_at), format!("{:?}", prefs.and_then(|p| p.bubble_at)));
    d.check("and the Scene", s.bubble.as_ref().and_then(|b| b.at) == Some(want_at), "");
    // The lid closes: only the Continuity cameras of this Mac's real list are left (the
    // observer's report, as `camera_source` posts it). No bubble, Show yourself greyed off,
    // no Camera source — nothing opens an iPhone by itself (N39, B48).
    let (displays, mics, _) = d.devices();
    let only_iphones: Vec<_> = cameras.iter().filter(|c| c.continuity).cloned().collect();
    d.fact("lid_closed_cameras", &only_iphones);
    d.rt.post(Event::Os(OsNews::Devices { displays: displays.clone(), mics: mics.clone(), cameras: only_iphones }));
    let s = d.wait("no bubble with only an iPhone", 5, |s| s.bubble.is_none())?;
    std::thread::sleep(Duration::from_millis(300));
    d.check("with only an iPhone the bubble's window closes", d.on_main(camera::bubble_seen).flatten().is_none(), "");
    let greyed = matches!(s.popover.as_ref().map(|p| &p.body), Some(PopoverBody::Config(c))
        if c.camera_picked().is_none() && !c.sources.iter().any(|x| x.action == Action::SetSource(SourceId::Camera)));
    d.check("Show yourself reads Camera off and Camera only leaves the sources", greyed, "");
    d.rt.post(Event::Os(OsNews::Devices { displays, mics, cameras: probe::cameras() }));
    let s = d.wait("the bubble back with the Mac's camera", 5, |s| s.bubble.is_some())?;
    std::thread::sleep(Duration::from_millis(1200));
    let back = d.on_main(camera::bubble_seen).flatten();
    d.check(
        "the wish was kept: the bubble returns on the core's camera",
        back.as_ref().is_some_and(|b| s.bubble.as_ref().is_some_and(|w| w.camera == b.camera) && b.running && cameras.iter().any(|c| c.uid == b.camera && !c.continuity)),
        format!("{back:?}"),
    );
    d.act(Action::SetCamera(None));
    d.wait("the bubble goes", 5, |s| s.bubble.is_none())?;
    let gone = d.after(300);
    let _ = gone;
    d.check("the bubble's window closed (the camera released)", d.on_main(camera::bubble_seen).flatten().is_none(), "");
    d.rt.post(Event::PopoverDismissed);
    Ok(())
}

fn camera_source(d: &D, displays: &[shroom_core::world::Display], mics: &[shroom_core::world::Device]) -> Result<(), String> {
    d.config()?;
    d.act(Action::SetSource(SourceId::Camera));
    let s = d.wait("Camera's preview", 5, |s| s.preview.is_some())?;
    let want = s.preview.clone().unwrap();
    std::thread::sleep(Duration::from_millis(1200));
    let seen = d.on_main(camera::preview_seen).flatten().ok_or("no preview on screen")?;
    d.fact("preview", &seen);
    d.check("Camera shows its preview at config, never key", seen.camera == want.camera && seen.running && !seen.can_become_key, format!("{seen:?}"));
    d.rt.post(Event::Key { surface: Surface::Popover, key: Key::Return });
    let s = d.wait("countdown", 5, |s| s.countdown.is_some())?;
    d.check("the count plays on the preview's display", s.countdown.as_ref().map(|c| c.display) == Some(want.display), format!("{:?} vs {}", s.countdown.as_ref().map(|c| c.display), want.display));
    d.wait("recording the camera", 15, |s| s.tray.state == Some("recording") && s.countdown.is_none())?;
    let s = d.after(800);
    // DEVIATIONS #42: the preview stays, live, on the camera the take records — and the take,
    // which opens the same camera beside it, still gets its frames: its file keeps growing.
    let seen = d.on_main(camera::preview_seen).flatten();
    d.check(
        "the preview stays while the take records, live, on the recorded camera",
        s.preview.as_ref() == Some(&want) && seen.as_ref().is_some_and(|p| p.camera == want.camera && p.running),
        format!("{:?}; {seen:?}", s.preview),
    );
    let growing = d.newest_take().map(|dir| dir.join("preview_0.mp4")).map(|f| {
        let size = || std::fs::metadata(&f).map(|m| m.len()).unwrap_or(0);
        let before = size();
        std::thread::sleep(Duration::from_millis(2500));
        (before, size())
    });
    d.check("…and the take keeps getting the camera's frames beside it", growing.is_some_and(|(a, b)| b > a && a > 0), format!("{growing:?} bytes, 2.5 s apart"));
    // The recorded camera disappears (as the observer reports it): camera-lost.
    let others: Vec<_> = probe::cameras().into_iter().filter(|c| c.uid != want.camera).collect();
    d.rt.post(Event::Os(OsNews::Devices { displays: displays.to_vec(), mics: mics.to_vec(), cameras: others }));
    let s = d.wait("camera-lost", 10, |s| s.popover.as_ref().is_some_and(|p| p.faces.contains(&"camera-lost")))?;
    let resume_off = matches!(s.popover.as_ref().map(|p| &p.body), Some(PopoverBody::Paused(b)) if !b.resume.enabled);
    d.check("Resume waits for that camera", resume_off, "");
    d.rt.post(Event::Os(OsNews::Devices { displays: displays.to_vec(), mics: mics.to_vec(), cameras: probe::cameras() }));
    let s = d.wait("the camera back", 5, |s| matches!(s.popover.as_ref().map(|p| &p.body), Some(PopoverBody::Paused(b)) if b.resume.enabled))?;
    let _ = s;
    // The room never outlives the drive: Discard.
    let dir = d.newest_take().ok_or("no camera take")?;
    d.act(Action::Discard);
    d.wait("discard-confirm", 5, |s| D::popover_id(s) == Some("discard-confirm"))?;
    d.act(Action::ConfirmDiscard);
    d.wait("discarded", 10, |s| s.tray.state == Some("idle-ready"))?;
    let until = Instant::now() + Duration::from_secs(5);
    while dir.exists() && Instant::now() < until {
        std::thread::sleep(Duration::from_millis(50));
    }
    d.check("the camera take is gone from disk", !dir.exists(), dir.display().to_string());
    // Back to the display for the rest.
    d.config()?;
    d.act(Action::SetSource(SourceId::Display(shroom_capture::main_display_id())));
    d.wait("the display chosen", 5, |s| s.preview.is_none())?;
    d.rt.post(Event::PopoverDismissed);
    Ok(())
}

// ── Camera as the source, a take kept (the hosted Mac's pattern camera only) ──

/// The one camera an unattended take may record (DECISIONS B121): OBS's virtual camera,
/// which `dev/mac/camera.sh` sets playing ffmpeg's test pattern on the hosted Mac.
const PATTERN_CAMERA: &str = "OBS Virtual Camera";

/// The pattern's colour bars (ffmpeg's `testsrc2`, six across) in a small RGB frame: how
/// many of the six read as the pattern has them at a third of the way down.
fn bars_seen(rgb: &[u8], width: usize, height: usize) -> usize {
    // red, green, yellow, blue, magenta, cyan: which of r, g, b is lit.
    const BARS: [(bool, bool, bool); 6] = [(true, false, false), (false, true, false), (true, true, false), (false, false, true), (true, false, true), (false, true, true)];
    let y = height / 3;
    BARS.iter()
        .enumerate()
        .filter(|(i, want)| {
            let x = (2 * i + 1) * width / 12;
            let p = &rgb.get((y * width + x) * 3..(y * width + x) * 3 + 3).unwrap_or(&[0, 0, 0]);
            (p[0] > 150, p[1] > 150, p[2] > 150) == **want && p.iter().all(|&c| c > 150 || c < 110)
        })
        .count()
}

/// Camera as the source through the real flow — `config`, Return, the count, five seconds,
/// the pause, Stop — and the take kept: its single file and a frame from its middle next to
/// the report, the frame read for the pattern's bars. Refused unless the camera the core
/// picked is the pattern camera; a take whose picture isn't the pattern is deleted.
fn camera_take(d: &D) -> Result<(), String> {
    d.wait("ready", 20, |s| s.tray.click == Click::OpenPopover)?;
    d.config()?;
    d.act(Action::SetSource(SourceId::Camera));
    let s = d.wait("Camera's preview", 5, |s| s.preview.is_some())?;
    let uid = s.preview.as_ref().map(|p| p.camera.clone()).unwrap_or_default();
    let name = probe::cameras().into_iter().find(|c| c.uid == uid).map(|c| c.name);
    d.fact("camera", &name);
    if name.as_deref() != Some(PATTERN_CAMERA) {
        return Err(format!("the take would record {name:?}, not {PATTERN_CAMERA} playing the pattern — refused (only the pattern is ever recorded)"));
    }
    d.rt.post(Event::Key { surface: Surface::Popover, key: Key::Return });
    d.wait("countdown", 5, |s| s.countdown.is_some())?;
    d.wait("recording the camera", 15, |s| s.tray.state == Some("recording") && s.countdown.is_none())?;
    std::thread::sleep(Duration::from_secs(5));
    click(&d.app, &d.rt);
    d.wait("paused", 10, |s| D::popover_id(s) == Some("paused"))?;
    d.act(Action::Stop);
    let (dir, _) = published_take(d, 30)?;
    let files = take_files(&dir);
    d.check("the camera take is whole and decodes", files["decodes"] == true && files["missing"].as_array().is_some_and(|m| m.is_empty()), files.to_string());
    d.fact("take", &files);

    let ffmpeg = crate::runtime::locate_ffmpeg().ok_or("no ffmpeg")?;
    let playlist = dir.join(shroom_encode::files::PLAYLIST);
    let here = d.out.parent().unwrap_or(Path::new(".")).to_path_buf();
    let (w, h) = (64, 36);
    let raw = std::process::Command::new(&ffmpeg)
        .args(["-v", "error", "-ss", "2.5", "-i"])
        .arg(&playlist)
        .args(["-frames:v", "1", "-vf", &format!("scale={w}:{h}"), "-f", "rawvideo", "-pix_fmt", "rgb24", "-"])
        .output()
        .map_err(|e| e.to_string())?
        .stdout;
    let bars = bars_seen(&raw, w, h);
    d.fact("bars_seen", bars);
    if bars < 5 {
        let _ = std::fs::remove_dir_all(&dir);
        d.check("the take's picture is the pattern's bars", false, format!("{bars} of 6 — the take was deleted"));
        return Ok(());
    }
    d.check("the take's picture is the pattern's bars", true, format!("{bars} of 6"));
    let frame = std::process::Command::new(&ffmpeg).args(["-v", "error", "-y", "-ss", "2.5", "-i"]).arg(&playlist).args(["-frames:v", "1"]).arg(here.join("camera-take.png")).status();
    let movie = std::process::Command::new(&ffmpeg).args(["-v", "error", "-y", "-i"]).arg(&playlist).args(["-c", "copy"]).arg(here.join("camera-take.mp4")).status();
    d.check("its frame and its file are kept beside the report", frame.is_ok_and(|s| s.success()) && movie.is_ok_and(|s| s.success()) && shroom_encode::take::playable(&ffmpeg, &here.join("camera-take.mp4")), here.display().to_string());
    Ok(())
}

/// Camera only, the way Bagrat made it at his 0.1.10 checkpoint: `config`, Camera only, the
/// preview dragged somewhere else, Return. The preview must stay where it was put — the same
/// window, never closed and reopened — through the count and the take; the take must keep
/// getting the camera's frames; and Stop must leave a whole take. Pattern camera only.
fn camera_drag(d: &D) -> Result<(), String> {
    d.wait("ready", 20, |s| s.tray.click == Click::OpenPopover)?;
    d.config()?;
    d.act(Action::SetSource(SourceId::Camera));
    let s = d.wait("Camera's preview", 5, |s| s.preview.is_some())?;
    let uid = s.preview.as_ref().map(|p| p.camera.clone()).unwrap_or_default();
    let name = probe::cameras().into_iter().find(|c| c.uid == uid).map(|c| c.name);
    d.fact("camera", &name);
    if name.as_deref() != Some(PATTERN_CAMERA) {
        return Err(format!("the take would record {name:?}, not {PATTERN_CAMERA} playing the pattern — refused (only the pattern is ever recorded)"));
    }
    std::thread::sleep(Duration::from_millis(1500));
    let first = d.on_main(camera::preview_seen).flatten().ok_or("no preview on screen")?;
    d.fact("preview_at_config", &first);
    // The person's drag: up and to the left.
    let _ = d.on_main(|| camera::drag_preview_by(-160.0, -140.0));
    std::thread::sleep(Duration::from_millis(400));
    let moved = d.on_main(camera::preview_seen).flatten().ok_or("no preview after the drag")?;
    d.fact("preview_dragged", &moved);
    d.check("the drag moved the preview", moved.at != first.at && moved.window == first.window, format!("{:?} → {:?}", first.at, moved.at));
    d.rt.post(Event::Key { surface: Surface::Popover, key: Key::Return });
    d.wait("countdown", 5, |s| s.countdown.is_some())?;
    // Through the count and into the take, the preview is looked at every 50 ms: any moment
    // it's gone, another window, or somewhere else is what the person saw snap back.
    let mut lapses: Vec<String> = Vec::new();
    let until = Instant::now() + Duration::from_secs(15);
    let mut recording_since: Option<Instant> = None;
    while Instant::now() < until {
        let seen = d.on_main(camera::preview_seen).flatten();
        match &seen {
            None => lapses.push(format!("{} ms: no preview", d.t0.elapsed().as_millis())),
            Some(p) if p.window != moved.window || p.at != moved.at => lapses.push(format!("{} ms: window {} at {:?}", d.t0.elapsed().as_millis(), p.window, p.at)),
            _ => {}
        }
        let recording = snapshot(&d.rt).is_some_and(|s| s.tray.state == Some("recording") && s.countdown.is_none());
        if recording && recording_since.is_none() {
            recording_since = Some(Instant::now());
            d.step("recording the camera");
        }
        if recording_since.is_some_and(|t| t.elapsed() > Duration::from_millis(2500)) {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    lapses.dedup();
    d.fact("preview_lapses", &lapses);
    if recording_since.is_none() {
        return Err(format!("never recorded; at {:?}", snapshot(&d.rt).map(|s| s.state_ids())));
    }
    d.check("the preview stays where it was put, the same window, through the count and the take", lapses.is_empty(), lapses.iter().take(6).cloned().collect::<Vec<_>>().join("; "));
    // Eight seconds of the take: still recording each second, its file growing.
    let segment = d.newest_take().map(|dir| dir.join("preview_0.mp4"));
    let size = || segment.as_ref().and_then(|f| std::fs::metadata(f).ok()).map(|m| m.len()).unwrap_or(0);
    let mut sizes = Vec::new();
    let mut died = None;
    for i in 0..8 {
        std::thread::sleep(Duration::from_secs(1));
        sizes.push(size());
        if !snapshot(&d.rt).is_some_and(|s| s.tray.state == Some("recording")) {
            died = Some(format!("after {} s: {:?}", i + 1, snapshot(&d.rt).map(|s| s.state_ids())));
            break;
        }
    }
    d.fact("take_sizes", &sizes);
    d.check("the take keeps recording (no capture-died)", died.is_none(), died.clone().unwrap_or_default());
    d.check("…and its file keeps growing", sizes.windows(2).filter(|w| w[1] > w[0]).count() >= 3, format!("{sizes:?}"));
    if died.is_some() {
        let took = d.newest_take();
        d.fact("take_dir_after_death", took.map(|t| take_files(&t)));
        return Ok(());
    }
    click(&d.app, &d.rt);
    d.wait("paused", 10, |s| D::popover_id(s) == Some("paused"))?;
    d.act(Action::Stop);
    let (dir, _) = published_take(d, 30)?;
    let files = take_files(&dir);
    d.check("the camera take is whole and decodes", files["decodes"] == true && files["missing"].as_array().is_some_and(|m| m.is_empty()), files.to_string());
    d.fact("take", &files);
    Ok(())
}

/// How long the show pass holds each surface: a hosted runner paints a new window two to
/// four seconds late, and the recording should show it whole.
const SEEN: Duration = Duration::from_millis(4500);

fn show(d: &D) -> Result<(), String> {
    let space: shroom_core::world::SpaceIdentity = d.rt.home.read_json("space.json").ok_or("no space.json — the show pass follows the wizard")?;
    if !d.rt.e2e() {
        return Err("the show pass runs only on the e2e run's test account (--e2e)".into());
    }
    d.wait("ready", 30, |s| s.tray.click == Click::OpenPopover)?;
    let camera = probe::cameras().into_iter().find(|c| c.name == PATTERN_CAMERA).ok_or_else(|| format!("no {PATTERN_CAMERA} — the show pass runs only where the bubble shows the test pattern"))?;
    d.config()?;
    std::thread::sleep(SEEN);
    // Nothing of a room: the microphone off, the bubble on the pattern camera.
    d.act(Action::SetMic(None));
    d.act(Action::SetCamera(Some(camera.uid.clone())));
    d.wait("the bubble", 15, |s| s.bubble.is_some())?;
    std::thread::sleep(SEEN);
    d.rt.post(Event::Key { surface: Surface::Popover, key: Key::Return });
    d.wait("countdown", 5, |s| s.countdown.is_some())?;
    d.wait("recording", 15, |s| s.tray.state == Some("recording") && s.countdown.is_none())?;
    std::thread::sleep(Duration::from_secs(6));
    click(&d.app, &d.rt);
    d.wait("paused", 10, |s| D::popover_id(s) == Some("paused"))?;
    std::thread::sleep(SEEN);
    d.act(Action::Stop);
    let (dir, publish) = published_take(d, 60)?;
    d.fact("link", format!("https://{}/{}", space.host, publish.rec));
    d.fact("take", take_files(&dir));
    // What the person sees after Stop: the popover with its link.
    std::thread::sleep(SEEN);
    d.check("the show take was published", true, publish.rec);
    Ok(())
}

/// This take's ffmpeg: a child of this process working in the take's folder.
fn take_encoder(dir: &Path) -> Vec<String> {
    let here = std::fs::canonicalize(dir).unwrap_or(dir.to_path_buf());
    let children = std::process::Command::new("pgrep").args(["-P", &std::process::id().to_string(), "ffmpeg"]).output();
    let Ok(children) = children else { return vec![] };
    String::from_utf8_lossy(&children.stdout)
        .split_whitespace()
        .filter(|pid| {
            let cwd = std::process::Command::new("lsof").args(["-a", "-d", "cwd", "-Fn", "-p", pid]).output();
            cwd.is_ok_and(|c| String::from_utf8_lossy(&c.stdout).lines().any(|l| l.strip_prefix('n').is_some_and(|p| Path::new(p) == here)))
        })
        .map(String::from)
        .collect()
}

fn display_takes(d: &D, displays: &[shroom_core::world::Display], mics: &[shroom_core::world::Device], cameras: &[shroom_core::world::Device]) -> Result<(), String> {
    let real = || OsNews::Grants {
        screen: shroom_capture::has_permission(),
        mic: probe::access(shroom_capture::mic_access()),
        camera: probe::access(shroom_capture::camera_access()),
    };
    let mic_granted = probe::access(shroom_capture::mic_access()) == Access::Granted;

    // ── the microphone leaves, then is switched off ──
    d.config()?;
    d.record()?;
    std::thread::sleep(Duration::from_secs(2));
    if mic_granted && !mics.is_empty() {
        d.rt.post(Event::Os(OsNews::Devices { displays: displays.to_vec(), mics: vec![], cameras: cameras.to_vec() }));
        let s = d.wait("mic-lost", 10, |s| s.popover.as_ref().is_some_and(|p| p.faces.contains(&"mic-lost")))?;
        let sub = match s.popover.as_ref().map(|p| &p.body) {
            Some(PopoverBody::Paused(b)) => b.sub.clone(),
            _ => None,
        };
        d.check("mic-lost says Resume continues without sound", sub.as_deref() == Some("Resume continues without sound."), format!("{sub:?}"));
        d.check("an unasked popover (law 5)", s.popover.as_ref().is_some_and(|p| !p.activates), "");
        let init = d.newest_take().map(|t| std::fs::read(t.join("init.mp4")).unwrap_or_default()).unwrap_or_default();
        d.act(Action::Resume);
        d.wait("recording again, silent", 10, |s| s.tray.state == Some("recording") && s.popover.is_none())?;
        std::thread::sleep(Duration::from_secs(3));
        // The observer's own path: the device list comes back.
        d.rt.post(Event::Os(OsNews::Devices { displays: displays.to_vec(), mics: mics.to_vec(), cameras: cameras.to_vec() }));
        observe::grants_if_changed(&d.rt, real);
        std::thread::sleep(Duration::from_millis(300));
        // Resuming without the mic left the take silent: put the mic back (a pause and Resume).
        click(&d.app, &d.rt);
        d.wait("paused", 10, |s| D::popover_id(s) == Some("paused"))?;
        // The silent stretch has ended: its encoder rewrote the shared init exactly.
        let after = d.newest_take().map(|t| std::fs::read(t.join("init.mp4")).unwrap_or_default()).unwrap_or_default();
        d.check("the shared init.mp4 is untouched by a stretch without the mic (B11)", !init.is_empty() && init == after, format!("{} vs {} bytes", init.len(), after.len()));
        d.act(Action::Resume);
        d.wait("recording with the mic", 10, |s| s.tray.state == Some("recording") && s.popover.is_none())?;
        std::thread::sleep(Duration::from_secs(1));
        observe::grants_if_changed(&d.rt, || OsNews::Grants { screen: true, mic: Access::Denied, camera: probe::access(shroom_capture::camera_access()) });
        let s = d.wait("mic-permission-lost", 10, |s| s.popover.as_ref().is_some_and(|p| p.faces.contains(&"mic-permission-lost")))?;
        d.check("macOS switching the microphone off pauses", s.popover.as_ref().is_some_and(|p| !p.activates), "");
        observe::grants_if_changed(&d.rt, real);
        d.act(Action::Resume);
        d.wait("recording after the grant came back", 10, |s| s.tray.state == Some("recording") && s.popover.is_none())?;
    } else {
        d.check("the microphone is granted here (mic-lost, permission-lost (microphone))", false, "skipped: no granted microphone");
    }

    // ── the encoder dies ──
    std::thread::sleep(Duration::from_secs(2));
    let dir = d.newest_take().ok_or("no take")?;
    let pids = take_encoder(&dir);
    d.check("found the take's encoder", pids.len() == 1, format!("{pids:?}"));
    for pid in &pids {
        let _ = std::process::Command::new("kill").args(["-9", pid]).status();
    }
    let killed = Instant::now();
    let s = d.wait("capture died → a saved take", 20, |s| s.tray.state != Some("recording") && s.popover.as_ref().is_some_and(|p| p.banner.as_ref().is_some_and(|b| b.id == "recovered-take")))?;
    d.fact("stall_to_saved_ms", killed.elapsed().as_millis());
    d.check("it opens by itself (DEVIATIONS #6)", s.popover.as_ref().is_some_and(|p| !p.activates), "");
    let files = take_files(&dir);
    d.check("the interrupted take is kept on disk (law 2)", dir.exists() && files["segments"].as_u64().unwrap_or(0) > 0, files.to_string());
    d.fact("stalled_take", files);
    d.rt.post(Event::PopoverDismissed);
    d.wait("dismissed", 5, |s| s.popover.is_none())?;

    // ── screen recording switched off ──
    d.config()?;
    d.record()?;
    std::thread::sleep(Duration::from_secs(2));
    observe::grants_if_changed(&d.rt, || OsNews::Grants { screen: false, mic: probe::access(shroom_capture::mic_access()), camera: probe::access(shroom_capture::camera_access()) });
    let s = d.wait("permission-lost", 15, |s| D::popover_id(s) == Some("permission-lost"))?;
    let line = match s.popover.as_ref().map(|p| &p.body) {
        Some(PopoverBody::Stopped(b)) => b.line.clone(),
        _ => String::new(),
    };
    d.check("a stop you didn't make, said so", line.starts_with("macOS switched off screen recording, so the recording stopped at "), line);
    let dir = d.newest_take().ok_or("no take")?;
    let files = take_files(&dir);
    d.check("finalized for the screen, never published", files["why"] == "screen-off" && files["published"] == false, files.to_string());
    d.check("it decodes", files["decodes"] == true, files["decode_errors"].to_string());
    Ok(())
}

// ── a take from a cold start, and what it leaves out of its picture ──

/// The pattern window sits at the screen saver's level (`harness::cover_with_pattern`); for
/// this drive the popover's and the count's panels sit just above it.
const ABOVE_THE_PATTERN: isize = 1001;
/// The least of the pattern's dark field the popover lights when it is in a picture — five
/// times what a clean frame may have lit (`tests/e2e/left-out.mjs`); on the clean VM's 4:3
/// display most of the popover hangs over the strips, and what is left of it lights 2–4 %
/// — and the least the count's scrim does, which covers the display (its digit is dark).
const POPOVER_SEEN: f64 = 0.01;
const COUNT_SEEN: f64 = 0.9;

/// How much of the pattern's dark field is lit in one real captured frame of `display` that
/// leaves nothing out (`shroom_testpattern::field_lit`).
fn lit(display: u32) -> Result<f64, String> {
    let (gray, w, h) = crate::harness::grab(display)?;
    Ok(shroom_testpattern::field_lit(&gray, w, h))
}

/// Looks, for up to `secs`, until such a frame has at least `bar` of the field lit: the most
/// it saw.
fn lit_within(display: u32, bar: f64, secs: u64) -> Result<f64, String> {
    let until = Instant::now() + Duration::from_secs(secs);
    let mut most: f64 = 0.0;
    loop {
        most = most.max(lit(display)?);
        if most >= bar || Instant::now() > until {
            return Ok(most);
        }
        std::thread::sleep(Duration::from_millis(100));
    }
}

fn pct(lit: f64) -> String {
    format!("{:.1} % of the pattern's dark field lit", lit * 100.0)
}

/// What a capture lit, beside what the pattern alone did: an overlay is seen only by what
/// it lights over that, never by what the bare pattern already lit (issue #4).
fn pct_over(lit: f64, alone: f64) -> String {
    format!("{} ({:.1} points over the pattern alone)", pct(lit), (lit - alone) * 100.0)
}

/// Does `f` to an overlay's panel on the main thread; whether there was a panel to do it to.
fn with_panel<T: Send + 'static>(d: &D, which: presenter::Overlay, f: impl FnOnce(&objc2_app_kit::NSWindow) -> T + Send + 'static) -> Option<T> {
    let app = d.app.clone();
    d.on_main(move || presenter::overlay_window(&app, which).map(|w| f(&w))).flatten()
}

/// `count_stays_in`: the reader's own proof — the count is taken off the list first.
fn left_out(d: &D, count_stays_in: bool) -> Result<(), String> {
    use presenter::Overlay;
    const PANELS: [(Overlay, &str); 2] = [(Overlay::Popover, "the popover"), (Overlay::Countdown, "the count")];
    d.wait("ready", 20, |s| s.tray.click == Click::OpenPopover)?;
    if !shroom_capture::has_permission() {
        return Err("recording isn't allowed for shroom on this Mac".into());
    }

    // 1. As shroom starts. The panels exist, are on the list of what a take leaves out, and
    // neither has shown — and ScreenCaptureKit already finds every window on that list, so
    // the check a take makes before it records (`ScreenSource::start`) would pass.
    let excluded = d.rt.excluded.lock().unwrap().clone();
    let scenes = d.rt.trace.lock().unwrap().as_ref().map(|t| t.ids.clone()).unwrap_or_default();
    let panels: Vec<Option<(u32, bool)>> = PANELS.iter().map(|(which, _)| with_panel(d, *which, |w| (w.windowNumber() as u32, w.isVisible()))).collect();
    let never_shown = !scenes.is_empty() && scenes.iter().all(|ids| !ids.ends_with(OVERLAY_UP)) && panels.iter().all(|p| p.is_some_and(|(_, visible)| !visible));
    let listed = panels.iter().all(|p| p.is_some_and(|(n, _)| excluded.contains(&n)));
    let missing = shroom_capture::not_shareable(&excluded);
    d.fact("as_shroom_starts", serde_json::json!({ "left_out_of_a_take": excluded, "the_panels": panels, "scenes_so_far": scenes, "not_found": missing }));
    d.check("as shroom starts both panels exist, on the list of what a take leaves out", listed, format!("the panels {panels:?} in {excluded:?}"));
    d.check("…and neither has shown yet", never_shown, format!("{scenes:?}, {panels:?}"));
    d.check(
        "…and ScreenCaptureKit already finds every window on that list: a take would start",
        missing.as_ref().is_ok_and(|m| m.is_empty()),
        format!("not found: {missing:?}"),
    );

    // A locked screen gives a capture no frames: nothing below can be looked at.
    if surfaces::locked() {
        d.fact("skipped", serde_json::json!([["the take from a cold start, and what is in its picture", "the screen is locked: nothing can be recorded"]]));
        return Ok(());
    }
    d.fact("skipped", serde_json::json!([]));
    if count_stays_in {
        let count = panels[1].map(|(n, _)| n);
        d.rt.excluded.lock().unwrap().retain(|n| Some(*n) != count);
        d.fact("taken_off_the_list", count);
    }
    warp_pointer_away();
    let display = shroom_capture::main_display_id();
    let pattern = cover_with_pattern(&d.app)?;
    // Both panels just above the pattern for the run (the popover above the count, as they
    // are): whatever a take doesn't leave out is now in its picture.
    let levels: Vec<Option<isize>> = PANELS
        .iter()
        .enumerate()
        .map(|(i, (which, _))| {
            with_panel(d, *which, move |w| {
                let was = w.level();
                w.setLevel(ABOVE_THE_PATTERN + 1 - i as isize);
                was
            })
        })
        .collect();
    let result = left_out_take(d, display);
    for ((which, _), was) in PANELS.iter().zip(levels) {
        if let Some(was) = was {
            with_panel(d, *which, move |w| w.setLevel(was));
        }
    }
    let _ = pattern.close();
    result
}

fn left_out_take(d: &D, display: u32) -> Result<(), String> {
    use presenter::Overlay;
    let alone = lit(display)?;
    d.check("the pattern alone lights none of its dark field", alone < 0.002, pct(alone));

    // 2. `config`: the popover is up, over the pattern.
    d.config()?;
    let seen = lit_within(display, alone + POPOVER_SEEN, 5)?;
    d.check("a capture that leaves nothing out sees the popover over the pattern", seen - alone >= POPOVER_SEEN, pct_over(seen, alone));
    d.act(Action::SetSource(SourceId::Display(display)));
    let s = d.wait("the pattern's display chosen", 5, |s| crate::runtime::records(s) == Some(SourceId::Display(display)))?;
    crate::runtime::guard_source(&s, display)?;

    // 3. Return: the count, over the pattern — and it runs to its end. The take starts as
    // the count leaves the screen: the moment its panel has to be found, and left out.
    d.rt.post(Event::Key { surface: Surface::Popover, key: Key::Return });
    d.wait("countdown", 5, |s| s.countdown.is_some())?;
    let seen = lit_within(display, alone + COUNT_SEEN, 2)?;
    d.check("a capture that leaves nothing out sees the count over the pattern", seen - alone >= COUNT_SEEN, pct_over(seen, alone));
    let log = || std::fs::read_to_string(d.rt.home.root.join("logs/run.log")).unwrap_or_default();
    if let Err(e) = d.wait("recording", 20, |s| s.tray.state == Some("recording") && s.countdown.is_none()) {
        let why: Vec<String> = log().lines().filter(|l| l.contains("recorder start failed")).map(|l| l.trim().to_string()).collect();
        d.check("the take starts once the count has run to its end", false, format!("{e}; {why:?}"));
        return Err(e);
    }
    d.check("the take starts once the count has run to its end", true, "");
    std::thread::sleep(Duration::from_millis(1500));

    // 4. In the middle of the take each panel is put up again, by hand. The Scene has
    // neither now, and its next turn would put one away — so each is held up until a
    // capture that leaves nothing out has seen it there, and a moment more.
    for (which, name, bar) in [(Overlay::Popover, "the popover", POPOVER_SEEN), (Overlay::Countdown, "the count", COUNT_SEEN)] {
        let mut seen = 0.0;
        for _ in 0..6 {
            with_panel(d, which, |w| w.orderFrontRegardless());
            std::thread::sleep(Duration::from_millis(500));
            seen = lit(display)?;
            if seen - alone >= bar {
                std::thread::sleep(Duration::from_millis(600));
                break;
            }
        }
        with_panel(d, which, |w| w.orderOut(None));
        d.check(&format!("mid-take, a capture that leaves nothing out sees {name} over the pattern"), seen - alone >= bar, pct_over(seen, alone));
        std::thread::sleep(Duration::from_millis(400));
    }
    let s = d.after(600);
    d.check("the take went on recording through it", s.tray.state == Some("recording"), s.state_ids().join(" | "));

    // 5. A pause: its popover comes up as the capture ends. Then Stop.
    click(&d.app, &d.rt);
    d.wait("paused", 10, |s| D::popover_id(s) == Some("paused"))?;
    let seen = lit_within(display, alone + POPOVER_SEEN, 5)?;
    d.check("a capture that leaves nothing out sees the pause's popover over the pattern", seen - alone >= POPOVER_SEEN, pct_over(seen, alone));
    d.act(Action::Stop);
    let (dir, _) = published_take(d, 30)?;

    // The take, as this Mac has it. What is in its picture is the runner's to read: every
    // frame, decoded by an ffmpeg that isn't the one that made it.
    let files = take_files(&dir);
    let frames: Vec<String> = log().lines().filter(|l| l.contains("recorder: take frames")).map(|l| l.trim().to_string()).collect();
    let received: u64 = frames.iter().filter_map(|l| l.split("take frames ").nth(1)?.split_whitespace().next()?.parse::<u64>().ok()).sum();
    d.check("frames arrived", received > 0, format!("{frames:?}"));
    d.check(
        "the take is whole on this Mac, and decodes",
        files["missing"].as_array().is_some_and(|m| m.is_empty()) && files["segments"].as_u64().unwrap_or(0) > 0 && files["decodes"] == true,
        files.to_string(),
    );
    let refused: Vec<String> = log().lines().filter(|l| l.contains("windows to exclude weren't all found")).map(|l| l.trim().to_string()).collect();
    d.check("no take was refused for a window it couldn't find", refused.is_empty(), format!("{refused:?}"));
    d.fact("take", files);
    d.fact("take_dir", dir.display().to_string());
    d.fact("frames", frames);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{bars_seen, recording_intervals_ms};

    /// A frame of the pattern's six bars, and OBS's own dark picture, as the camera take's
    /// reader sees them.
    #[test]
    fn the_camera_takes_reader_knows_the_patterns_bars() {
        let (w, h) = (64, 36);
        let colours = [[235, 20, 20], [20, 220, 20], [230, 230, 25], [25, 25, 230], [230, 25, 230], [25, 230, 230]];
        let bars: Vec<u8> = (0..w * h).flat_map(|i| colours[(i % w) * 6 / w]).collect();
        assert_eq!(bars_seen(&bars, w, h), 6);
        let navy: Vec<u8> = (0..w * h).flat_map(|_| [30u8, 40, 90]).collect();
        assert_eq!(bars_seen(&navy, w, h), 0);
        assert_eq!(bars_seen(&[], w, h), 0, "no frame at all");
    }

    /// The 2-hour soak's own log shape: the recording state's intervals, never the pauses
    /// or the hand-overs around them.
    #[test]
    fn the_time_recorded_is_the_recording_states_intervals() {
        let log = "    4918 core started [\"uploader\"]\n   50168 core click-pause [\"recorder\"]\n   50227 core paused []\n   58241 core resume [\"recorder\"]\n   58321 core resumed []\n   59244 core upload-progress []\n  103458 core click-pause [\"recorder\"]\n  103535 core paused []\nnot a core line\n";
        assert_eq!(recording_intervals_ms(log), (50168 - 4918) + (103458 - 58321));
        assert_eq!(recording_intervals_ms(""), 0);
    }
}
