//! Hooks the test harness drives. The app is always launched through LaunchServices
//! (`open … --args`), never as a child of a shell, so it answers to its own permission
//! grants rather than borrowing the terminal's (POC-LEARNINGS §4).

use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::Serialize;
use shroom_capture::{Access, LivenessReport, ScreenSource, main_display_id};
use shroom_encode::quality::Quality;
use shroom_record::{Finalized, Recording, TakeStats};
use tauri::{AppHandle, WebviewUrl, WebviewWindowBuilder};

pub use crate::drives::{Region5, Region6, drive_region5, drive_region6, seed_provisioned, seed_region6};

pub enum Mode {
    App,
    /// Ask macOS for every permission shroom needs, from a process that stays alive.
    Grant,
    /// Capture briefly, report what the grants allow, write JSON, exit.
    Selftest(PathBuf),
    /// Open the first non-Continuity camera as a recording source for a few seconds and
    /// measure it — frames, rate, size, brightness. Nothing is encoded or kept: the
    /// camera's frames are the room. Writes JSON, exits.
    CameraCheck { out: PathBuf, record: bool },
    /// Cover the main display with the test pattern, record two takes with a pause
    /// between them through the real path, write the stats as JSON, exit. With `tone`, one
    /// take of a synthetic picture and the loopback's tone instead (no screen at all).
    RecordCheck { out: PathBuf, seconds: f64, capture_fps: f32, audio: CheckAudio, exclude_pattern: bool },
    /// `--play-out <hz> <at_ms> <seconds>`: play a burst of `hz` at `at_ms` into every
    /// wall-clock second on the default output, then exit — the system-audio check's
    /// "other app" (B172). Started by the check's script, not by shroom: ScreenCaptureKit's
    /// `excludesCurrentProcessAudio` leaves out shroom's children too (probed on the hosted
    /// Mac, 2026-10-16).
    PlayOut { hz: f64, at_ms: u64, seconds: f64 },
    /// Tray + popover mechanics: the Dock rule (law 6) and an unasked popover that must
    /// not take the keyboard (law 5). Writes JSON, exits.
    ShellProbe(PathBuf),
    /// Open the camera bubble and report whether the webview got the camera.
    BubbleProbe(PathBuf),
    /// Every tray face drawn on every menu bar, its mushroom checked against the bar
    /// (`tray_probe.rs`); then faces put on at the scenario's asks. Writes JSON, exits.
    TrayProbe(PathBuf),
    /// Verify the pattern guard refuses when no pattern is on screen.
    GuardProbe(PathBuf),
    /// Drive the real runtime like a person: click → Return → record → click → Stop, then
    /// wait for the link and the upload. Writes JSON, exits.
    Drive { out: PathBuf, seconds: f64 },
    /// `--drive onboarding <out> [--taken <name>] [--gate-marker <file>]` (with `--home
    /// <tmp>` and `--cloud-fake <api> <r2>`): a fresh install walked through the welcome,
    /// the plan and the permissions chapter like a person, against the L1 fake — reacting to
    /// whatever the fake's state brings (a taken name: `--taken` claims it first; a gate:
    /// the drive writes the gate's id to `--gate-marker`, for whoever runs the fake to
    /// clear it; a space already there). Writes JSON, exits.
    ///
    /// With `--resume` (L3, DECISIONS B69): the person's own launch (no install flag) of a
    /// Mac whose first launch was `kill -9`ed mid-wire — whatever the window opens at, it's
    /// walked on from there to all-set, never pasting a token or claiming a name again.
    ///
    /// With `--doors` (law 6): the setup window's Dock icon, there and
    /// back — ✕ at the last step retreats to the menu bar and gives the keyboard back, the
    /// mushroom brings the window, the icon and the keyboard back; the walk ends there,
    /// before anything is asked of the Mac's grants (so a clean Mac can run it).
    ///
    /// With `--invite-someone`: all-set's Invite someone ends the walk instead of Close —
    /// setup is over, and Settings carries on without the icon, in front, with the keyboard,
    /// its name field holding the caret (N28): what is typed next is the name.
    /// (Never `--invite`: that is the install script's join hand-off, which `launch_kind`
    /// reads anywhere on the line — and this drive is a fresh install's launch.)
    ///
    /// With `--e2e --token-file <file>` (the pre-merge e2e run, TESTING-PLAN D3): against
    /// the dedicated test account, from the dev home, with the run's own token — read from
    /// the file (mode 600, written by the run), the file removed at once, the token pasted
    /// as a person pastes it. The gates are clear and the account has its name there, so
    /// the plan goes token → card → wiring.
    DriveOnboarding { out: PathBuf, taken: Option<String>, gate_marker: Option<PathBuf>, resume: bool, doors: bool, invite: bool, token_file: Option<PathBuf> },
    /// `--drive library|settings|update|update-relaunched <out>` (with `--home <tmp>` and
    /// `--cloud-fake <api> <r2>`; the update drive also `--releases <loopback>` and
    /// `--update-target <throwaway .app>`): Region 5 driven like a person on a provisioned
    /// Mac, against the fake. Writes JSON with the state sequence, exits.
    DriveRegion5 { which: Region5, out: PathBuf },
    /// `--drive join|members|switch <out>` (with `--home <tmp>`, `--cloud-fake <api> <r2>`
    /// and `--space-base <the local Worker>`): Region 6 driven like a person against
    /// `worker/dev/serve.mjs`. Writes JSON with the state sequence and its checks, exits.
    DriveRegion6 { which: Region6, out: PathBuf },
    /// `--drive launch <out> [provisioned]` (with `--home <tmp>`; `provisioned` seeds a
    /// set-up Mac and needs `--cloud-fake <api> <r2>`): boots however the process was
    /// launched — the install script's flags, the login launch event, nothing — waits, and
    /// reports the launch kind, what opened by itself, and where an opened popover sits
    /// against the mushroom. Writes JSON, exits.
    /// `late`: the start-up keeps the main thread for longer than the mushroom's settle
    /// wait before the event loop first turns (some starts do by themselves).
    DriveLaunch { out: PathBuf, provisioned: bool, late: bool },
    /// `--drive take <out> disk|signals` (with `--home <tmp>` and `--cloud-fake <api> <r2>`):
    /// Region 3's takes interrupted the ways they really are (`take_drive.rs`). Writes JSON
    /// with the state sequence and its checks, exits.
    DriveTake { which: crate::take_drive::TakeDrive, out: PathBuf },
}

pub fn from_args(args: Vec<String>) -> Mode {
    let after = |flag: &str| args.iter().position(|a| a == flag).map(|i| args[i + 1..].to_vec());
    if let Some(rest) = after("--selftest") {
        return Mode::Selftest(PathBuf::from(rest.first().expect("--selftest needs an output path")));
    }
    if let Some(rest) = after("--camera-check") {
        let out = PathBuf::from(rest.first().expect("--camera-check needs an output path"));
        return Mode::CameraCheck { out, record: rest.get(1).is_some_and(|a| a == "record") };
    }
    if let Some(rest) = after("--record-check") {
        let out = PathBuf::from(rest.first().expect("--record-check needs an output path"));
        let seconds = rest.get(1).and_then(|s| s.parse().ok()).unwrap_or(6.0);
        let capture_fps = rest.get(2).and_then(|s| s.parse().ok()).unwrap_or(60.0);
        let audio = if rest.iter().any(|a| a == "tone") {
            CheckAudio::Tone
        } else if rest.iter().any(|a| a == "loopback") {
            CheckAudio::Loopback
        } else if rest.iter().any(|a| a == "system") {
            CheckAudio::System
        } else if rest.iter().any(|a| a == "mic") {
            CheckAudio::Mic
        } else {
            CheckAudio::None
        };
        let exclude_pattern = rest.iter().any(|a| a == "exclude-pattern");
        return Mode::RecordCheck { out, seconds, capture_fps, audio, exclude_pattern };
    }
    if let Some(rest) = after("--play-out") {
        let num = |i: usize, d: f64| rest.get(i).and_then(|s| s.parse().ok()).unwrap_or(d);
        return Mode::PlayOut { hz: num(0, 440.0), at_ms: num(1, 500.0) as u64, seconds: num(2, 10.0) };
    }
    if let Some(rest) = after("--guard-probe") {
        return Mode::GuardProbe(PathBuf::from(rest.first().expect("--guard-probe needs an output path")));
    }
    if let Some(rest) = after("--drive") {
        if rest.first().map(String::as_str) == Some("take") {
            let out = PathBuf::from(rest.get(1).expect("--drive take needs an output path"));
            let which = rest.get(2).and_then(|w| crate::take_drive::TakeDrive::parse(w)).expect("--drive take <out> disk|signals|movies|upload-kill|upload-resume|soak:<minutes>|surfaces|left-out|not-left-out|camera-take");
            return Mode::DriveTake { which, out };
        }
        if let Some(which) = rest.first().and_then(|w| Region6::parse(w)) {
            let out = PathBuf::from(rest.get(1).unwrap_or_else(|| panic!("--drive {} needs an output path", which.name())));
            return Mode::DriveRegion6 { which, out };
        }
        if let Some(which) = rest.first().and_then(|w| Region5::parse(w)) {
            let out = PathBuf::from(rest.get(1).unwrap_or_else(|| panic!("--drive {} needs an output path", which.name())));
            return Mode::DriveRegion5 { which, out };
        }
        if rest.first().map(String::as_str) == Some("launch") {
            let out = PathBuf::from(rest.get(1).expect("--drive launch needs an output path"));
            let word = |w: &str| rest.iter().skip(2).take(2).any(|a| a == w);
            return Mode::DriveLaunch { out, provisioned: word("provisioned"), late: word("late") };
        }
        if rest.first().map(String::as_str) == Some("onboarding") {
            let out = PathBuf::from(rest.get(1).expect("--drive onboarding needs an output path"));
            let value = |flag: &str| args.iter().position(|a| a == flag).and_then(|i| args.get(i + 1)).cloned();
            return Mode::DriveOnboarding { out, taken: value("--taken"), gate_marker: value("--gate-marker").map(PathBuf::from), resume: args.iter().any(|a| a == "--resume"), doors: args.iter().any(|a| a == "--doors"), invite: args.iter().any(|a| a == "--invite-someone"), token_file: value("--token-file").map(PathBuf::from) };
        }
        let out = PathBuf::from(rest.first().expect("--drive needs an output path"));
        // "NaN" parses as a float: a drive with no end would record (and cover the screen)
        // forever. Only a finite, sane duration is taken.
        let seconds = rest.get(1).and_then(|s| s.parse::<f64>().ok()).filter(|s| s.is_finite() && (1.0..=600.0).contains(s)).unwrap_or(8.0);
        return Mode::Drive { out, seconds };
    }
    if let Some(rest) = after("--bubble-probe") {
        return Mode::BubbleProbe(PathBuf::from(rest.first().expect("--bubble-probe needs an output path")));
    }
    if let Some(rest) = after("--tray-probe") {
        return Mode::TrayProbe(PathBuf::from(rest.first().expect("--tray-probe needs an output path")));
    }
    if let Some(rest) = after("--shell-probe") {
        return Mode::ShellProbe(PathBuf::from(rest.first().expect("--shell-probe needs an output path")));
    }
    if args.iter().any(|a| a == "--grant") { Mode::Grant } else { Mode::App }
}

#[derive(Serialize)]
struct SelftestReport {
    version: &'static str,
    executable: String,
    mic: Access,
    camera: Access,
    screen: LivenessReport,
}

pub fn selftest(out: &Path) -> i32 {
    let report = SelftestReport {
        version: env!("CARGO_PKG_VERSION"),
        executable: std::env::current_exe().map(|p| p.display().to_string()).unwrap_or_default(),
        mic: shroom_capture::mic_access(),
        camera: shroom_capture::camera_access(),
        screen: shroom_capture::liveness_probe(Duration::from_secs(3), 30.0),
    };
    write_json(out, &report)
}

#[derive(Serialize)]
struct CameraCheckReport {
    access: shroom_capture::Access,
    cameras: Vec<shroom_capture::Camera>,
    chosen: Option<String>,
    native: Option<(u32, u32)>,
    sample: Option<shroom_record::probe::Sample>,
    /// With `record`: a 3 s take through the real encode path, measured, then deleted.
    recorded: Option<serde_json::Value>,
    error: Option<String>,
}

pub fn camera_check(out: &Path, record: bool) -> i32 {
    let cameras = shroom_capture::cameras();
    let mut report = CameraCheckReport { access: shroom_capture::camera_access(), cameras: cameras.clone(), chosen: None, native: None, sample: None, recorded: None, error: None };
    match cameras.iter().find(|c| c.builtin && !c.continuity).or_else(|| cameras.iter().find(|c| !c.continuity)) {
        None => report.error = Some("no camera".into()),
        Some(cam) => {
            report.chosen = Some(cam.name.clone());
            match shroom_capture::CameraSource::new(&cam.uid, shroom_encode::quality::Quality::Normal) {
                Err(e) => report.error = Some(e),
                Ok(mut source) => {
                    report.native = Some(source.native_size());
                    report.sample = Some(shroom_record::probe::sample(&mut source, Duration::from_secs(3)));
                    if record {
                        report.recorded = Some(camera_take(&cam.uid).unwrap_or_else(|e| serde_json::json!({ "error": e })));
                    }
                }
            }
        }
    }
    write_json(out, &report)
}

/// A short take of the camera through the real path (frame clock → ffmpeg → segments →
/// finalize), decoded and measured, then deleted: the frames are the room, so nothing of
/// it outlives this function.
fn camera_take(uid: &str) -> Result<serde_json::Value, String> {
    let ffmpeg = crate::runtime::locate_ffmpeg().ok_or("no ffmpeg")?;
    let dir = std::env::temp_dir().join(format!("shroom-camera-take-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    let result = (|| {
        let mut rec = Recording::create(&dir, &ffmpeg, Quality::Normal.video_kbps()).map_err(|e| e.to_string())?;
        let source = shroom_capture::CameraSource::new(uid, Quality::Normal)?;
        rec.start_take(Box::new(source), None).map_err(|e| e.to_string())?;
        std::thread::sleep(Duration::from_secs(3));
        let take = rec.end_take().map_err(|e| e.to_string())?;
        let fin = rec.finalize().map_err(|e| e.to_string())?;
        let probe = std::process::Command::new(ffmpeg.with_file_name("ffprobe"))
            .args(["-v", "error", "-select_streams", "v:0", "-count_frames", "-show_entries", "stream=width,height,nb_read_frames", "-of", "csv=p=0"])
            .arg(dir.join(shroom_encode::files::PLAYLIST))
            .output()
            .map_err(|e| e.to_string())?;
        Ok(serde_json::json!({ "take": take, "seconds": fin.seconds, "decoded": String::from_utf8_lossy(&probe.stdout).trim() }))
    })();
    let _ = std::fs::remove_dir_all(&dir);
    result
}

pub fn grant() {
    shroom_capture::request_mic();
    shroom_capture::request_camera();
    shroom_capture::request_permission();
}

/// The record check's sound.
#[derive(Debug, Clone, Copy, PartialEq, Serialize, Default)]
#[serde(rename_all = "kebab-case")]
pub enum CheckAudio {
    /// None at all.
    #[default]
    None,
    /// The default microphone — the room; only ever asked for by hand.
    Mic,
    /// The loopback (BlackHole) through the native input path, carrying the drives' beep on
    /// the pattern's clock: A/V through ScreenCaptureKit and CoreAudio (DECISIONS B59 e).
    Loopback,
    /// The loopback carrying a continuous tone, under a synthetic picture: TESTING-PLAN §3's
    /// tone capture (sample gaps, the duration against the wall clock).
    Tone,
    /// The Mac's own sound mixed with the drives' beep (spec N47, DECISIONS B172). Take 0:
    /// other apps play — `--play-out`'s [`SYSTEM_BURST_HZ`] half a second after each beep,
    /// started by the check's script, never by this process. Take 1: nothing else plays.
    /// Both: this process plays [`SELF_TONE_HZ`] all along, which must never be in a take.
    System,
}

/// The other process's burst, and where in each wall-clock second it starts.
pub const SYSTEM_BURST_HZ: f64 = 440.0;
pub const SYSTEM_BURST_AT_MS: u64 = 500;
/// shroom's own sound in the system-audio check: kept out of every take.
pub const SELF_TONE_HZ: f64 = 2_000.0;

/// `--play-out`: plays, then exits with 0, or 1 when there was nothing to play on.
pub fn play_out(hz: f64, at_ms: u64, seconds: f64) -> i32 {
    match shroom_capture::loopback::play_out(shroom_capture::loopback::Signal::Burst { hz, at_ms }) {
        Ok(speaker) => {
            // For the check: its bursts leave the output this much after their instants.
            eprintln!("device_latency_ms={:.1}", speaker.device_latency.as_secs_f64() * 1000.0);
            std::thread::sleep(Duration::from_secs_f64(seconds));
            0
        }
        Err(e) => {
            eprintln!("play-out: {e}");
            1
        }
    }
}

#[derive(Serialize, Default)]
struct SystemReport {
    /// The default output both sounds play on.
    output: Option<String>,
    rate: u32,
    output_gaps: u64,
    /// What reached each take's handle from ScreenCaptureKit.
    per_take: Vec<shroom_record::mix::SystemCounts>,
    error: Option<String>,
}

/// The tone the tone check plays: not a divisor of any common rate, so a lost or added
/// sample always moves its phase (by 7.5° at 48 kHz).
pub const TONE_HZ: f64 = 997.0;

#[derive(Serialize, Default)]
struct LoopbackReport {
    input: String,
    rate: u32,
    signal: String,
    /// The player's own gaps (not the input's).
    output_gaps: u64,
}

#[derive(Serialize, Default)]
struct RecordCheckReport {
    version: &'static str,
    display_id: u32,
    capture_fps: f32,
    cpu_seconds: f64,
    /// Capturing's cost while the takes ran — this process and its encoders, per second
    /// recorded (TESTING-PLAN §4: ≤ ~1 core).
    capture_cores: Option<f64>,
    /// The last take's end and the finalize, as Stop runs them (TESTING-PLAN §4: ≤ 200 ms).
    finalize_ms: Option<u64>,
    mic_devices: Vec<shroom_capture::mic::MicDevice>,
    excluded: Vec<u32>,
    mic: Option<shroom_capture::mic::MicDevice>,
    audio: CheckAudio,
    loopback: Option<LoopbackReport>,
    system: Option<SystemReport>,
    /// Each take's wall-clock length, from its start to its end returning.
    wall_seconds: Vec<f64>,
    takes: Vec<TakeStats>,
    finalized: Option<Finalized>,
    error: Option<String>,
}

/// Runs on its own thread; exits the app when done.
pub fn record_check(app: AppHandle, out: PathBuf, seconds: f64, capture_fps: f32, audio: CheckAudio, exclude_pattern: bool) {
    let mut report = RecordCheckReport { version: env!("CARGO_PKG_VERSION"), audio, ..Default::default() };
    let result = if audio == CheckAudio::Tone {
        run_tone_check(&out, seconds, &mut report)
    } else {
        run_record_check(&app, &out, seconds, capture_fps, audio, exclude_pattern, &mut report)
    };
    if let Err(e) = result {
        report.error = Some(e);
    }
    let code = write_json(&out, &report);
    app.exit(code);
}

#[allow(clippy::too_many_arguments)]
fn run_record_check(app: &AppHandle, out: &Path, seconds: f64, capture_fps: f32, audio: CheckAudio, exclude_pattern: bool, report: &mut RecordCheckReport) -> Result<(), String> {
    let ffmpeg = locate_ffmpeg().ok_or("no ffmpeg found (set SHROOM_FFMPEG)")?;
    let dir = out.parent().unwrap_or(Path::new(".")).join("recording");
    let _ = std::fs::remove_dir_all(&dir);
    // The beep on the loopback for both takes (and the pause), before anything records.
    let loopback = if audio == CheckAudio::Loopback { Some(shroom_capture::loopback::play(shroom_capture::loopback::Signal::Beep)?) } else { None };
    if let Some(lb) = &loopback {
        report.loopback = Some(LoopbackReport { input: lb.input.name.clone(), rate: lb.rate, signal: "beep".into(), output_gaps: 0 });
    }
    // shroom's own sound, out loud for the whole check: never in a take (B172).
    let speaker = if audio == CheckAudio::System { Some(shroom_capture::loopback::play_out(shroom_capture::loopback::Signal::Tone(SELF_TONE_HZ))?) } else { None };
    if let Some(sp) = &speaker {
        report.system = Some(SystemReport { output: Some(sp.device.clone()), rate: sp.rate, ..Default::default() });
    }

    // The pattern covers the whole display, proven before anything records.
    let pattern = cover_with_pattern(app)?;
    let excluded: Vec<u32> = if exclude_pattern { window_number(app, &pattern).into_iter().collect() } else { Vec::new() };
    report.excluded = excluded.clone();

    let result = (|| {
        let display = main_display_id();
        report.display_id = display;
        report.capture_fps = capture_fps;
        let mut rec = Recording::create(&dir, &ffmpeg, Quality::Normal.video_kbps()).map_err(|e| e.to_string())?;
        let half = Duration::from_secs_f64(seconds / 2.0);
        let cpu_at = cpu_split();
        for take in 0..2 {
            if take > 0 {
                std::thread::sleep(Duration::from_secs(1)); // the pause
            }
            let source = ScreenSource::new(display, Quality::Normal)?
                .with_cursor(false)
                .with_capture_fps(capture_fps)
                .excluding(excluded.clone());
            let system = (audio == CheckAudio::System).then(shroom_record::mix::SystemAudio::new);
            let source = match &system {
                Some(system) => source.with_system_audio(system.clone()),
                None => source,
            };
            let audio: Option<Box<dyn shroom_record::AudioSource>> = match (audio, &loopback) {
                (CheckAudio::Mic, _) => {
                    let devices = shroom_capture::mic::input_devices();
                    let default = shroom_capture::mic::default_input_uid();
                    let chosen = shroom_capture::mic::choose(&devices, None, default.as_deref()).cloned().ok_or("no usable microphone")?;
                    report.mic_devices = devices.clone();
                    report.mic = Some(chosen.clone());
                    Some(Box::new(shroom_capture::mic::MicSource::open(chosen)?))
                }
                (CheckAudio::Loopback, Some(lb)) => {
                    report.mic = Some(lb.input.clone());
                    Some(Box::new(shroom_capture::mic::MicSource::open(lb.input.clone())?))
                }
                // The beep is the voice; the Mac's sound is mixed in.
                (CheckAudio::System, _) => Some(Box::new(shroom_record::mix::Mixed::new(
                    Some(Box::new(shroom_capture::beep::BeepSource::new(48_000))),
                    system.clone().expect("the system check has the Mac's sound"),
                ))),
                _ => None,
            };
            let began = std::time::Instant::now();
            rec.start_take(Box::new(source), audio).map_err(|e| e.to_string())?;
            std::thread::sleep(half);
            let stop = std::time::Instant::now();
            report.takes.push(rec.end_take().map_err(|e| e.to_string())?);
            report.wall_seconds.push(began.elapsed().as_secs_f64());
            if let (Some(system), Some(r)) = (&system, report.system.as_mut()) {
                r.per_take.push(system.counts());
            }
            // The system check's other apps play outside this process (a child of shroom's is
            // shroom's own sound, left out like the rest): `system-audio-check.mjs` plays them
            // through take 0 and, told it ended, stops them before take 1.
            if system.is_some() && take == 0 {
                std::fs::write(out.with_extension("take0-ended"), b"").map_err(|e| e.to_string())?;
                let quiet = out.with_extension("quiet");
                let asked = std::time::Instant::now();
                while !quiet.exists() && asked.elapsed() < Duration::from_secs(15) {
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
            if take == 1 {
                report.finalized = Some(rec.finalize().map_err(|e| e.to_string())?);
                report.finalize_ms = Some(stop.elapsed().as_millis() as u64);
            }
        }
        let cpu_now = cpu_split();
        let recorded: f64 = report.takes.iter().map(|t| t.seconds).sum();
        report.capture_cores = Some((cpu_now.0 - cpu_at.0 + cpu_now.1 - cpu_at.1) / recorded.max(0.001));
        Ok(())
    })();
    let _ = pattern.close();
    if let (Some(lb), Some(r)) = (&loopback, report.loopback.as_mut()) {
        r.output_gaps = lb.output_gaps.load(std::sync::atomic::Ordering::Relaxed);
    }
    if let (Some(sp), Some(r)) = (&speaker, report.system.as_mut()) {
        r.output_gaps = sp.output_gaps.load(std::sync::atomic::Ordering::Relaxed);
    }
    report.cpu_seconds = cpu_seconds();
    result
}

/// A still picture, offered every tick: the tone check's video, so its take is the sound's
/// (no screen is captured, nothing is covered).
struct StillPicture {
    stop: std::sync::Arc<std::sync::atomic::AtomicBool>,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl shroom_record::FrameSource for StillPicture {
    fn size(&self) -> (u32, u32) {
        (640, 360)
    }
    fn start(&mut self, sink: shroom_record::FrameSink) -> Result<(), String> {
        let stop = self.stop.clone();
        self.thread = Some(std::thread::spawn(move || {
            let mut due = std::time::Instant::now();
            while !stop.load(std::sync::atomic::Ordering::SeqCst) {
                sink.offer(|b| b.fill(0x40));
                due += Duration::from_millis(16);
                std::thread::sleep(due.saturating_duration_since(std::time::Instant::now()));
            }
        }));
        Ok(())
    }
    fn stop(&mut self) -> Result<(), String> {
        self.stop.store(true, std::sync::atomic::Ordering::SeqCst);
        if let Some(t) = self.thread.take() {
            let _ = t.join();
        }
        Ok(())
    }
}

/// TESTING-PLAN §3's tone capture (DECISIONS B59 e): one take of `seconds` whose sound is
/// the loopback's continuous tone through the native input path — the device, the
/// recorder's sound writer, the encoder — under a still synthetic picture. The recording is
/// left beside the report for `tests/e2e/tone-check.mjs` to read (sample gaps, the length
/// against the wall clock), which then deletes it.
fn run_tone_check(out: &Path, seconds: f64, report: &mut RecordCheckReport) -> Result<(), String> {
    let ffmpeg = locate_ffmpeg().ok_or("no ffmpeg found (set SHROOM_FFMPEG)")?;
    let dir = out.parent().unwrap_or(Path::new(".")).join("recording");
    let _ = std::fs::remove_dir_all(&dir);
    let lb = shroom_capture::loopback::play(shroom_capture::loopback::Signal::Tone(TONE_HZ))?;
    report.mic = Some(lb.input.clone());
    // The player settles before the take opens its input.
    std::thread::sleep(Duration::from_millis(500));
    let mut rec = Recording::create(&dir, &ffmpeg, Quality::Normal.video_kbps()).map_err(|e| e.to_string())?;
    let picture = StillPicture { stop: Default::default(), thread: None };
    let began = std::time::Instant::now();
    rec.start_take(Box::new(picture), Some(Box::new(shroom_capture::mic::MicSource::open(lb.input.clone())?))).map_err(|e| e.to_string())?;
    std::thread::sleep(Duration::from_secs_f64(seconds));
    report.takes.push(rec.end_take().map_err(|e| e.to_string())?);
    report.wall_seconds.push(began.elapsed().as_secs_f64());
    report.finalized = Some(rec.finalize().map_err(|e| e.to_string())?);
    report.loopback = Some(LoopbackReport {
        input: lb.input.name.clone(),
        rate: lb.rate,
        signal: format!("tone {TONE_HZ} Hz"),
        output_gaps: lb.output_gaps.load(std::sync::atomic::Ordering::Relaxed),
    });
    report.cpu_seconds = cpu_seconds();
    Ok(())
}

/// The harness runs the encoder the app would (B82): one road, `runtime::find_encoder`.
fn locate_ffmpeg() -> Option<PathBuf> {
    crate::runtime::locate_ffmpeg()
}

pub(crate) fn write_json(out: &Path, value: &impl Serialize) -> i32 {
    let json = serde_json::to_string_pretty(value).expect("report serializes");
    match std::fs::write(out, json) {
        Ok(()) => 0,
        Err(_) => 1,
    }
}

/// CPU time this process has used (user + system), for cost comparisons.
fn cpu_seconds() -> f64 {
    let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
    unsafe { libc::getrusage(libc::RUSAGE_SELF, &mut usage) };
    let t = |v: libc::timeval| v.tv_sec as f64 + v.tv_usec as f64 / 1e6;
    t(usage.ru_utime) + t(usage.ru_stime)
}

#[derive(Serialize, Default)]
struct ShellReport {
    tray_rect: Option<String>,
    frontmost_before: Option<String>,
    policy_with_no_window: Option<String>,
    unasked_popover_visible: Option<bool>,
    app_active_after_unasked: Option<bool>,
    frontmost_after_unasked: Option<String>,
    popover_is_key_after_unasked: Option<bool>,
    policy_with_window: Option<String>,
    error: Option<String>,
}

tauri_nspanel::tauri_panel! {
    panel!(ProbePopover {
        config: {
            can_become_key_window: true,
            is_floating_panel: true
        }
    })
}

fn frontmost() -> Option<String> {
    use objc2_app_kit::NSWorkspace;
    let app = NSWorkspace::sharedWorkspace().frontmostApplication()?;
    app.bundleIdentifier().map(|s| s.to_string())
}

pub(crate) fn policy() -> String {
    use objc2_app_kit::{NSApplication, NSApplicationActivationPolicy};
    let mtm = objc2_foundation::MainThreadMarker::new().expect("main thread");
    match NSApplication::sharedApplication(mtm).activationPolicy() {
        NSApplicationActivationPolicy::Regular => "regular (Dock icon)".into(),
        NSApplicationActivationPolicy::Accessory => "accessory (no Dock icon)".into(),
        _ => "prohibited".into(),
    }
}

pub(crate) fn app_active() -> bool {
    use objc2_app_kit::NSApplication;
    let mtm = objc2_foundation::MainThreadMarker::new().expect("main thread");
    NSApplication::sharedApplication(mtm).isActive()
}

/// Runs on a helper thread, hopping to the main thread for every AppKit call.
pub fn shell_probe(app: AppHandle, out: PathBuf) {
    use std::sync::mpsc::channel;
    use tauri::Manager;
    use tauri_nspanel::{ManagerExt, PanelBuilder, PanelLevel};
    let on_main = |app: &AppHandle, f: Box<dyn FnOnce(&AppHandle) + Send>| {
        let (tx, rx) = channel();
        let h = app.clone();
        let _ = app.run_on_main_thread(move || {
            f(&h);
            let _ = tx.send(());
        });
        let _ = rx.recv_timeout(Duration::from_secs(10));
    };
    let report = std::sync::Arc::new(std::sync::Mutex::new(ShellReport::default()));

    // 1. Menu-bar only: the Dock icon appears only with a window (law 6).
    {
        let r = report.clone();
        on_main(&app, Box::new(move |app| {
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.hide();
            }
            let tray = tauri::tray::TrayIconBuilder::with_id("shroom")
                .icon(app.default_window_icon().cloned().expect("an icon"))
                .tooltip("shroom")
                .build(app);
            let mut rep = r.lock().unwrap();
            rep.tray_rect = tray.ok().and_then(|t| t.rect().ok().flatten()).map(|rect| format!("{rect:?}"));
            rep.policy_with_no_window = Some(policy());
        }));
    }

    // 2. Another app takes the front, as when the user is working elsewhere.
    let _ = std::process::Command::new("open").args(["-a", "Finder"]).status();
    std::thread::sleep(Duration::from_millis(1500));
    report.lock().unwrap().frontmost_before = frontmost();

    // 3. An unasked popover (an auto-pause): shown, never made key, never activating.
    {
        let r = report.clone();
        on_main(&app, Box::new(move |app| {
            let built = PanelBuilder::<_, ProbePopover>::new(app, "probe-popover")
                .url(WebviewUrl::App("index.html".into()))
                .level(PanelLevel::Floating)
                .size(tauri::Size::Logical(tauri::LogicalSize { width: 320.0, height: 200.0 }))
                .build();
            match built {
                Ok(panel) => {
                    panel.show();
                    r.lock().unwrap().unasked_popover_visible = Some(panel.is_visible());
                }
                Err(e) => r.lock().unwrap().error = Some(format!("panel: {e}")),
            }
        }));
    }
    std::thread::sleep(Duration::from_millis(1500));
    {
        let r = report.clone();
        on_main(&app, Box::new(move |app| {
            let mut rep = r.lock().unwrap();
            rep.app_active_after_unasked = Some(app_active());
            rep.frontmost_after_unasked = frontmost();
            rep.popover_is_key_after_unasked = app.get_webview_panel("probe-popover").ok().map(|p| p.as_panel().isKeyWindow());
        }));
    }

    // 4. A real window brings the Dock icon (law 6).
    {
        let r = report.clone();
        on_main(&app, Box::new(move |app| {
            let _ = app.set_activation_policy(tauri::ActivationPolicy::Regular);
            if let Some(w) = app.get_webview_window("main") {
                let _ = w.show();
            }
            r.lock().unwrap().policy_with_window = Some(policy());
        }));
    }

    let code = write_json(&out, &*report.lock().unwrap());
    app.exit(code);
}

/// Opens the native bubble on the camera the core would pick (the built-in one, else the
/// first that isn't an iPhone — N39) and reports what's on screen: the camera, whether it
/// delivers, and that the panel can never take the keyboard (law 5). Closes it, exits.
pub fn bubble_probe(app: AppHandle, out: PathBuf) {
    let cameras = crate::runtime::probe::cameras();
    let pick = cameras.iter().find(|c| c.builtin && !c.continuity).or_else(|| cameras.iter().find(|c| !c.continuity)).cloned();
    let Some(cam) = pick else {
        let _ = std::fs::write(&out, serde_json::json!({ "ok": false, "error": "no camera shroom may pick", "cameras": cameras }).to_string());
        app.exit(0);
        return;
    };
    let bubble = shroom_core::scene::Bubble { at: None, display: main_display_id(), camera: cam.uid.clone() };
    let report = crate::runtime::camera::probe_bubble(&app, bubble, Duration::from_secs(2));
    let _ = std::fs::write(&out, serde_json::json!({ "ok": report.as_ref().is_some_and(|s| s.running && !s.can_become_key && !s.key), "camera": cam, "seen": report }).to_string());
    app.exit(0);
}

/// A Tauri window's AppKit window number (what ScreenCaptureKit calls its window id).
fn window_number(app: &AppHandle, w: &tauri::WebviewWindow) -> Option<u32> {
    let (tx, rx) = std::sync::mpsc::channel();
    let w = w.clone();
    let _ = app.run_on_main_thread(move || {
        let n = w.ns_window().ok().map(|p| {
            let ns: &objc2_app_kit::NSWindow = unsafe { &*(p as *const objc2_app_kit::NSWindow) };
            ns.windowNumber() as u32
        });
        let _ = tx.send(n);
    });
    rx.recv_timeout(Duration::from_secs(5)).ok().flatten()
}

#[derive(Serialize, Default)]
struct DriveReport {
    /// `live` once the video is fully uploaded and the owner's key is in the folder (the
    /// heartbeat plays it, titles it, then asks for the delete); `deleted` at the end.
    phase: &'static str,
    steps: Vec<(String, u128)>,
    states: Vec<Vec<&'static str>>,
    take: Option<String>,
    rec: Option<String>,
    host: Option<String>,
    stop_to_published_ms: Option<u128>,
    stop_to_uploaded_ms: Option<u128>,
    /// Stop → the app opens the owner's page (a quiet run logs the open instead of doing
    /// it: the drive reads when, then does what the browser would).
    stop_to_open_ms: Option<u128>,
    /// Stop → the clean link first answers 200, polled from the moment of Stop.
    stop_to_page_200_ms: Option<u128>,
    /// Stop → the keyed page's first 200 (TESTING-PLAN §4's “Stop → page answers 200”,
    /// ≤ 1 s): the open, the one-time key, its redirect, the owner's page.
    stop_to_keyed_page_ms: Option<u128>,
    /// That page was the owner's (the title field), not a recipient's.
    keyed_page_is_owner: Option<bool>,
    /// Where the keyed page's time went, from Stop: the open, the one-time key minted, its
    /// redirect answered, the owner's page answered.
    keyed_steps_ms: Option<(u128, u128, u128, u128)>,
    /// The encoder's finalize at the pause, and Stop's finalize (TESTING-PLAN §4: ≤ 200 ms).
    take_end_ms: Option<u64>,
    finalize_ms: Option<u64>,
    /// Each take's frame accounting, as the recorder logged it: where repeats came from.
    take_frames: Vec<String>,
    /// What capturing cost while the take ran (TESTING-PLAN §4: ≤ ~1 core).
    capture: Option<CaptureCost>,
    /// The library's delete went through: the row left and the record is a tombstone.
    deleted: Option<bool>,
    /// The member leg's owner steps (B68): the invite, then the removal — what each did.
    member: Vec<serde_json::Value>,
    error: Option<String>,
}

#[derive(Serialize, Default, Clone)]
struct CaptureCost {
    seconds: f64,
    /// This process (capture, frame clock, hand-off) and its encoder (ffmpeg, a child —
    /// counted once the take's end has reaped it), in CPU seconds.
    app_cpu: f64,
    encoder_cpu: f64,
    /// Both, per second of the take: cores.
    cores: f64,
    display_pixels: (u32, u32),
}

/// The file beside the report where the drive leaves the owner's one-time key for the
/// heartbeat (mode 600; never on a command line, never in a log), and the one the
/// heartbeat touches to ask for the delete.
pub const OWNER_KEY_FILE: &str = "owner-key";
pub const DELETE_NOW_FILE: &str = "delete-now";
/// The heartbeat's member leg (B68): make the invite now; remove the member now.
pub const MEMBER_NOW_FILE: &str = "member-now";
pub const REMOVE_NOW_FILE: &str = "remove-now";

/// Runs on its own thread against the real runtime: the same core, lanes and presenter a
/// person's clicks would reach.
pub fn drive(app: AppHandle, rt: crate::runtime::Runtime, out: PathBuf, seconds: f64) {
    let mut report = DriveReport { phase: "recording", ..DriveReport::default() };
    let started = std::time::Instant::now();
    let result = run_drive(&app, &rt, &out, seconds, &mut report, started);
    if let Err(e) = result {
        report.error = Some(e);
    }
    let code = write_json(&out, &report);
    rt.ending_ours.store(true, std::sync::atomic::Ordering::SeqCst);
    app.exit(code);
}

/// CPU seconds this process and its reaped children have used (user + system).
fn cpu_split() -> (f64, f64) {
    let read = |who: i32| {
        let mut usage: libc::rusage = unsafe { std::mem::zeroed() };
        unsafe { libc::getrusage(who, &mut usage) };
        let t = |v: libc::timeval| v.tv_sec as f64 + v.tv_usec as f64 / 1e6;
        t(usage.ru_utime) + t(usage.ru_stime)
    };
    (read(libc::RUSAGE_SELF), read(libc::RUSAGE_CHILDREN))
}

/// The run log's lines written since `from` (a byte offset), as (mono ms, text).
fn log_since(rt: &crate::runtime::Runtime, from: u64) -> Vec<(u64, String)> {
    use std::io::{Read, Seek, SeekFrom};
    let Ok(mut f) = std::fs::File::open(rt.home.root.join("logs/run.log")) else { return Vec::new() };
    let _ = f.seek(SeekFrom::Start(from));
    let mut text = String::new();
    let _ = f.read_to_string(&mut text);
    text.lines()
        .filter_map(|l| {
            let l = l.trim_start();
            let (mono, rest) = l.split_once(' ')?;
            Some((mono.parse().ok()?, rest.to_string()))
        })
        .collect()
}

fn log_len(rt: &crate::runtime::Runtime) -> u64 {
    std::fs::metadata(rt.home.root.join("logs/run.log")).map(|m| m.len()).unwrap_or(0)
}

fn agent(timeout: Duration) -> ureq::Agent {
    ureq::Agent::config_builder()
        .http_status_as_error(false)
        .max_redirects(0)
        .timeout_global(Some(timeout))
        .tls_config(ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build())
        .build()
        .into()
}

/// Polls the clean link from the moment of Stop until it first answers 200.
fn first_200(url: String, stop: std::time::Instant) -> std::thread::JoinHandle<Option<u128>> {
    std::thread::spawn(move || {
        let agent = agent(Duration::from_secs(2));
        while stop.elapsed() < Duration::from_secs(15) {
            if agent.get(&url).call().is_ok_and(|r| r.status() == 200) {
                return Some(stop.elapsed().as_millis());
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        None
    })
}

/// What the owner's browser does when the app opens the page: the app's own keyed URL (its
/// one-time key, minted ahead while the take ran — DECISIONS B59 g), its 302 (and the cookie
/// it sets), then the clean page as the owner. Returns when that page answered 200, and
/// whether it was the owner's face (its title field). A fresh connection, as a browser
/// opening a link it hasn't been to would make.
fn open_keyed(space: &shroom_core::world::SpaceIdentity, rec: &str, stop: std::time::Instant) -> Result<(u128, bool, (u128, u128)), String> {
    let page = format!("https://{}/{rec}", space.host);
    let keyed = crate::runtime::cloud::take_harness_open().ok_or("the app opened the page without its owner key")?;
    if !keyed.starts_with(&format!("{page}?k=")) {
        return Err("the app opened the page without its owner key".into());
    }
    let minted = stop.elapsed().as_millis();
    let agent = agent(Duration::from_secs(5));
    let mut res = agent.get(&keyed).call().map_err(|e| format!("the keyed page: {e}"))?;
    let redirected = stop.elapsed().as_millis();
    // The redirect's (empty) body read through, so its connection goes back to the pool and
    // the page comes over it — as a browser follows a redirect on the connection it has.
    let _ = res.body_mut().read_to_vec();
    if res.status() != 302 {
        return Err(format!("the keyed page answered {} (not its redirect)", res.status()));
    }
    let cookie = res
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|v| v.to_str().ok())
        .find_map(|v| v.split(';').next().filter(|c| c.starts_with("shroom_owner=")).map(str::to_string))
        .ok_or("the key set no owner cookie")?;
    let mut attempts: Vec<(u16, u128)> = Vec::new();
    let until = std::time::Instant::now() + Duration::from_secs(10);
    loop {
        let mut res = agent.get(&page).header("cookie", &cookie).call().map_err(|e| format!("the page: {e}"))?;
        if res.status() == 200 {
            let at = stop.elapsed().as_millis();
            let body = res.body_mut().read_to_string().unwrap_or_default();
            return Ok((at, body.contains("id=\"t\""), (minted, redirected)));
        }
        attempts.push((res.status().as_u16(), stop.elapsed().as_millis()));
        if std::time::Instant::now() > until {
            return Err(format!("the page answered {} ({attempts:?})", res.status()));
        }
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// Enrolls this Mac's device key under member #1 of the dev space, as wiring does for a
/// person (`cloud::enroll_this_mac`, from the token the wizard saved). The dev space's tool
/// wires the page without it, and a dev home keeps its token in the harness's file, which
/// the wizard's enroll never reads — so without this the owner's key would be B25's
/// device-less grant, a recipient's. The parent's proof is made in memory, never logged.
fn enroll_dev_mac(rt: &crate::runtime::Runtime, space: &shroom_core::world::SpaceIdentity) -> Result<(), String> {
    use shroom_cloud::member::EnrollNext;
    let token = rt.home.token().ok_or("no token for the dev space")?;
    let cloud = rt.cloud(&token);
    let token_id = cloud.verify(&space.account_id).map_err(|e| format!("the token's verify: {e}"))?.id;
    let key = crate::runtime::member::ensure_device_key(rt, &space.host).map_err(|e| format!("device.key: {e}"))?;
    let page = rt.space(&space.host).ok_or("the dev space's page is out of this build's reach")?;
    let secret = cloud.s3_credentials(&token_id).secret_access_key;
    // A page just deployed can take a few seconds to answer as itself on its new address
    // (wiring waits for it the same way): not shroom-shaped yet, or no answer, is retried.
    for _ in 0..30 {
        let ts = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        match EnrollNext::of(page.enroll(&secret, &key, None, None, Some(&space.owner_name), ts)) {
            EnrollNext::Enrolled(_) => return Ok(()),
            EnrollNext::Transient | EnrollNext::Unanswered(_) | EnrollNext::Reping => std::thread::sleep(Duration::from_secs(1)),
            other => return Err(format!("the enroll: {}", match other {
                EnrollNext::Clock(_) => "the clock",
                EnrollNext::ReplaceParent => "the page's parent",
                EnrollNext::Seed => "no roster",
                EnrollNext::Broken => "a broken roster",
                EnrollNext::Conflict => "another device's id",
                EnrollNext::Repair(_) => "the space",
                _ => "no answer",
            })),
        }
    }
    Err("the enroll didn't land".into())
}

/// The heartbeat's dev home holds this Mac's device key before the core boots (B68): the
/// core learns of a device key only from disk at boot, or from its own wiring — and the dev
/// space's tool wires without one. Without it the Members face and `/me` would never ask
/// the space (the member leg's removal would read the roster as the invite left it).
/// `enroll_dev_mac` then enrolls this same key (re-enrolling it is idempotent).
pub fn prepare_drive_home(home: &crate::runtime::home::Home) -> Result<(), String> {
    use shroom_cloud::device::DeviceKey;
    let identity: shroom_core::world::SpaceIdentity = home.read_json("space.json").ok_or("no space.json — run dev-space up first")?;
    if home.read_json::<DeviceKey>("device.key").is_some_and(|k| k.host == identity.host) {
        return Ok(());
    }
    home.write_json("device.key", &DeviceKey::generate(&identity.host)).map_err(|e| format!("device.key: {e}"))
}

/// Writes `text` to `path`, readable by this user only.
fn write_private(path: &Path, text: &str) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::OpenOptionsExt;
    let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(path)?;
    f.write_all(text.as_bytes())
}

pub(crate) fn snapshot(rt: &crate::runtime::Runtime) -> Option<shroom_core::Scene> {
    rt.snapshot.lock().unwrap().clone()
}

fn wait_for(rt: &crate::runtime::Runtime, what: &str, secs: u64, report: &mut DriveReport, t0: std::time::Instant, pred: impl Fn(&shroom_core::Scene) -> bool) -> Result<(), String> {
    let until = std::time::Instant::now() + Duration::from_secs(secs);
    while std::time::Instant::now() < until {
        if let Some(s) = snapshot(rt) {
            if pred(&s) {
                report.steps.push((what.to_string(), t0.elapsed().as_millis()));
                report.states.push(s.state_ids());
                return Ok(());
            }
        }
        std::thread::sleep(Duration::from_millis(40));
    }
    Err(format!("timed out waiting for {what}; last {:?}", snapshot(rt).map(|s| s.state_ids())))
}

pub(crate) fn click(app: &AppHandle, rt: &crate::runtime::Runtime) {
    let rt = rt.clone();
    let _ = app.run_on_main_thread(move || {
        if let Some(mtm) = objc2_foundation::MainThreadMarker::new() {
            let (quick, _) = crate::runtime::probe::quick(mtm, &rt.home.root);
            rt.post(shroom_core::event::Event::TrayClicked { right: false, quick });
        }
    });
}

fn run_drive(app: &AppHandle, rt: &crate::runtime::Runtime, out: &Path, seconds: f64, report: &mut DriveReport, t0: std::time::Instant) -> Result<(), String> {
    use shroom_core::event::{Event, Key, Surface};
    use shroom_core::scene::{Action, Click};
    #[repr(C)]
    struct CGPoint {
        x: f64,
        y: f64,
    }
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGWarpMouseCursorPosition(p: CGPoint) -> i32;
    }
    // This Mac joins its space's roster the way wiring does it for a person (the dev
    // space's tool wires without it): so the timed open after Stop is the everyday one.
    let identity: shroom_core::world::SpaceIdentity = rt.home.read_json("space.json").ok_or("no space.json — run dev-space up first")?;
    enroll_dev_mac(rt, &identity)?;
    // Stop → the owner's page is timed through the app's own path: the key minted ahead
    // while the take records, the open (left for this drive instead of a browser).
    crate::runtime::cloud::MEASURE_OPEN.store(true, std::sync::atomic::Ordering::SeqCst);
    crate::runtime::cloud::owner_key(rt, &identity, &mut None).ok_or("this Mac couldn't get an owner key from its space")?;
    report.steps.push(("enrolled".into(), t0.elapsed().as_millis()));
    // This drive's own lines of the run log (the dev home's log keeps earlier runs').
    let drive_log = log_len(rt);
    // The pattern covers the display, proven before anything records; the pointer moves
    // off the counter strip.
    let pattern = cover_with_pattern(app)?;
    let covered = main_display_id();
    unsafe { CGWarpMouseCursorPosition(CGPoint { x: 4000.0, y: 3000.0 }) };
    let result = (|| -> Result<(), String> {
        wait_for(rt, "ready", 20, report, t0, |s| s.tray.click == Click::OpenPopover)?;
        click(app, rt);
        wait_for(rt, "config", 5, report, t0, |s| s.popover.as_ref().is_some_and(|p| p.id == "config"))?;
        // Only the pattern is measured: Show yourself starts on (N43), so its bubble goes.
        if snapshot(rt).is_some_and(|s| s.bubble.is_some()) {
            rt.post(Event::Act { surface: Surface::Popover, action: Action::SetCamera(None), rev: 0 });
            wait_for(rt, "the bubble off", 5, report, t0, |s| s.bubble.is_none())?;
        }
        // What records is the display the pattern covers — whatever the prefs said (a
        // camera picked in the dev app, another display): chosen as a person would, then
        // checked on the Scene the Return will start from (b4 #16).
        rt.post(Event::Act { surface: Surface::Popover, action: Action::SetSource(shroom_core::world::SourceId::Display(covered)), rev: 0 });
        wait_for(rt, "the pattern's display chosen", 5, report, t0, |s| crate::runtime::records(s) == Some(shroom_core::world::SourceId::Display(covered)))?;
        let scene = snapshot(rt).ok_or("no Scene")?;
        crate::runtime::guard_source(&scene, covered)?;
        rt.post(Event::Key { surface: Surface::Popover, key: Key::Return });
        wait_for(rt, "countdown", 3, report, t0, |s| s.countdown.is_some())?;
        wait_for(rt, "recording", 10, report, t0, |s| s.tray.state == Some("recording") && s.countdown.is_none())?;
        let (cpu_at, take_at) = (cpu_split(), std::time::Instant::now());
        // ⌘Q mid-take is declined silently (law 4): the app menu's Quit, exactly as AppKit
        // sends it. Were it not, this process would end here, with no report.
        std::thread::sleep(Duration::from_secs_f64(seconds / 2.0));
        terminate_like_cmd_q(app);
        std::thread::sleep(Duration::from_millis(400));
        wait_for(rt, "cmd-q declined", 2, report, t0, |s| s.tray.state == Some("recording") && s.popover.is_none())?;
        std::thread::sleep(Duration::from_secs_f64(seconds / 2.0));
        click(app, rt);
        wait_for(rt, "paused", 10, report, t0, |s| s.popover.as_ref().is_some_and(|p| p.id == "paused"))?;
        // The take has ended (its encoder reaped): what capturing it cost.
        let (cpu_now, wall) = (cpu_split(), take_at.elapsed().as_secs_f64());
        let (app_cpu, encoder_cpu) = (cpu_now.0 - cpu_at.0, cpu_now.1 - cpu_at.1);
        let display = display_pixels(covered);
        report.capture = Some(CaptureCost { seconds: wall, app_cpu, encoder_cpu, cores: (app_cpu + encoder_cpu) / wall.max(0.001), display_pixels: display });
        // Paused, ⌘Q brings the paused popover back (with its note) instead of quitting.
        rt.post(Event::PopoverDismissed);
        wait_for(rt, "paused, dismissed", 5, report, t0, |s| s.popover.is_none())?;
        terminate_like_cmd_q(app);
        wait_for(rt, "cmd-q paused popover", 5, report, t0, |s| s.popover.as_ref().is_some_and(|p| p.id == "paused" && p.activates))?;
        // The video's id (its take.json, written at Start) and the log so far: what Stop
        // brings is read from here on.
        let rec_id = crate::runtime::takes::dirs(&rt.home.takes())
            .into_iter()
            .last()
            .and_then(|d| crate::runtime::home::read_json::<crate::runtime::takes::TakeJson>(&d.join("take.json")))
            .map(|t| t.rec)
            .ok_or("no take.json for the take")?;
        let log_from = log_len(rt);
        let stop = std::time::Instant::now();
        let stop_mono = rt.mono().0;
        let page_200 = first_200(format!("https://{}/{rec_id}", identity.host), stop);
        rt.post(Event::Act { surface: Surface::Popover, action: Action::Stop, rev: 0 });
        // The app opens the owner's page: when it would, the drive does what the browser does.
        let opened = {
            let (rt, identity, rec_id) = (rt.clone(), identity.clone(), rec_id.clone());
            std::thread::spawn(move || -> Result<(u128, u128, bool, (u128, u128)), String> {
                let needle = format!("cloud would open the owner page for {}", crate::runtime::short(&rec_id));
                let until = std::time::Instant::now() + Duration::from_secs(10);
                let at = loop {
                    if let Some((mono, _)) = log_since(&rt, log_from).into_iter().find(|(_, l)| l.contains(&needle)) {
                        break mono.saturating_sub(stop_mono) as u128;
                    }
                    if std::time::Instant::now() > until {
                        return Err("the app never opened the page after Stop".into());
                    }
                    std::thread::sleep(Duration::from_millis(5));
                };
                let (keyed, owner, steps) = open_keyed(&identity, &rec_id, stop)?;
                Ok((at, keyed, owner, steps))
            })
        };
        // The link: publish.json in the newest take.
        let takes = rt.home.takes();
        let newest = || crate::runtime::takes::dirs(&takes).into_iter().last();
        let until = std::time::Instant::now() + Duration::from_secs(15);
        let dir = loop {
            if let Some(d) = newest().filter(|d| d.join("publish.json").exists()) {
                break d;
            }
            if std::time::Instant::now() > until {
                return Err("no publish.json after Stop".into());
            }
            std::thread::sleep(Duration::from_millis(20));
        };
        report.stop_to_published_ms = Some(stop.elapsed().as_millis());
        report.take = dir.file_name().map(|n| n.to_string_lossy().into_owned());
        let publish: Option<crate::runtime::takes::PublishJson> = crate::runtime::home::read_json(&dir.join("publish.json"));
        report.rec = publish.map(|p| p.rec);
        let identity: Option<shroom_core::world::SpaceIdentity> = rt.home.read_json("space.json");
        report.host = identity.map(|i| i.host);
        // The single-file copy goes after the go-live: a longer take's takes longer.
        let upload_wait = Duration::from_secs_f64(120.0 + seconds / 2.0);
        let until = std::time::Instant::now() + upload_wait;
        while !dir.join("uploaded").exists() {
            if std::time::Instant::now() > until {
                return Err(format!("the upload didn't finish in {} s", upload_wait.as_secs()));
            }
            std::thread::sleep(Duration::from_millis(50));
        }
        report.stop_to_uploaded_ms = Some(stop.elapsed().as_millis());
        // The queue drained: the popover closed and the tray back to idle — the status line just goes (DEVIATIONS #52).
        wait_for(rt, "upload drained", 10, report, t0, |s| s.popover.is_none() && s.tray.state == Some("idle-ready"))?;
        let (open_ms, keyed_ms, owner, (minted, redirected)) = opened.join().map_err(|_| "the open's thread panicked".to_string())??;
        report.keyed_steps_ms = Some((open_ms, minted, redirected, keyed_ms));
        report.stop_to_open_ms = Some(open_ms);
        report.stop_to_keyed_page_ms = Some(keyed_ms);
        report.keyed_page_is_owner = Some(owner);
        report.stop_to_page_200_ms = page_200.join().ok().flatten();
        // (A log rolled since then starts over from its top.)
        let from = if log_len(rt) >= drive_log { drive_log } else { 0 };
        for (_, line) in log_since(rt, from) {
            if let Some(ms) = line.strip_prefix("recorder: take ended in ").and_then(|r| r.strip_suffix(" ms")).and_then(|v| v.parse().ok()) {
                report.take_end_ms = Some(ms);
            }
            if let Some(ms) = line.strip_prefix("recorder: finalized in ").and_then(|r| r.strip_suffix(" ms")).and_then(|v| v.parse().ok()) {
                report.finalize_ms = Some(ms);
            }
            if let Some(frames) = line.strip_prefix("recorder: take frames ") {
                report.take_frames.push(frames.to_string());
            }
        }
        Ok(())
    })();
    let _ = pattern.close();
    result?;

    // The video is fully uploaded: the heartbeat plays it and, as its owner, titles it — with a
    // one-time key minted here and left beside the report for it.
    let rec = report.rec.clone().ok_or("no video id")?;
    let k = crate::runtime::cloud::owner_key(rt, &identity, &mut None).ok_or("no owner key for the heartbeat")?;
    let dir = out.parent().unwrap_or(Path::new("."));
    write_private(&dir.join(OWNER_KEY_FILE), &k).map_err(|e| format!("the owner key's file: {e}"))?;
    drop(k);
    report.phase = "live";
    write_json(out, &*report);
    // The heartbeat plays, decodes and titles it first: longer for a longer take. Between
    // that and the delete, the member leg (B68): an invite made in Members for the
    // heartbeat's second instance to join, then its removal — each on the heartbeat's word.
    let until = std::time::Instant::now() + Duration::from_secs_f64(900.0 + seconds * 2.0);
    while !dir.join(DELETE_NOW_FILE).exists() {
        if std::time::Instant::now() > until {
            return Err("the heartbeat never asked for the delete".into());
        }
        let leg = if report.phase == "live" && dir.join(MEMBER_NOW_FILE).exists() {
            Some((crate::drives::HbOwner::Invite, "invited"))
        } else if report.phase == "invited" && dir.join(REMOVE_NOW_FILE).exists() {
            Some((crate::drives::HbOwner::Remove, "removed"))
        } else {
            None
        };
        if let Some((step, phase)) = leg {
            let done = crate::drives::heartbeat_owner(app, rt, dir, step);
            let ok = done["ok"] == true;
            report.member.push(done);
            report.phase = if ok { phase } else { "member-leg-failed" };
            write_json(out, &*report);
            if !ok {
                return Err(format!("the member leg's {phase} step failed"));
            }
        }
        std::thread::sleep(Duration::from_millis(100));
    }

    // Delete, through the Library like a person: open it, the row's Delete…, the sheet's answer.
    use shroom_core::world::{RecId, WindowKind};
    click(app, rt);
    wait_for(rt, "config again", 10, report, t0, |s| s.popover.as_ref().is_some_and(|p| p.id == "config"))?;
    rt.post(Event::Act { surface: Surface::Popover, action: Action::OpenLibrary, rev: 0 });
    let has_row = |s: &shroom_core::Scene| s.library.as_ref().is_some_and(|l| l.body.rows.iter().any(|r| r.key == rec));
    wait_for(rt, "the video in the Library", 30, report, t0, has_row)?;
    rt.post(Event::Act { surface: Surface::Library, action: Action::DeleteRow(RecId(rec.clone())), rev: 0 });
    wait_for(rt, "the delete sheet", 5, report, t0, |s| s.library.as_ref().is_some_and(|l| l.body.sheet.is_some()))?;
    let log_from = log_len(rt);
    rt.post(Event::Act { surface: Surface::Library, action: Action::ConfirmDelete, rev: 0 });
    wait_for(rt, "the row is gone", 30, report, t0, |s| !has_row(s) && s.library.as_ref().is_some_and(|l| l.body.sheet.is_none()))?;
    // The row hides while the delete runs: it's done when the lane says so — the tombstone
    // read back, the bytes gone (closing the app before then would cut it short).
    let short = crate::runtime::short(&rec);
    let until = std::time::Instant::now() + Duration::from_secs(60);
    loop {
        let lines = log_since(rt, log_from);
        if lines.iter().any(|(_, l)| l.starts_with("library deleted") && l.contains(&short)) {
            report.steps.push(("the delete landed".into(), t0.elapsed().as_millis()));
            break;
        }
        if let Some((_, l)) = lines.iter().find(|(_, l)| l.starts_with(&format!("library delete {short}:"))) {
            return Err(format!("the Library's delete failed: {l}"));
        }
        if std::time::Instant::now() > until {
            return Err("the Library's delete didn't land in a minute".into());
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    rt.post(Event::WindowClosed(WindowKind::Library));
    wait_for(rt, "the Library closed", 5, report, t0, |s| s.library.is_none())?;
    report.deleted = Some(true);
    report.phase = "deleted";
    Ok(())
}

/// Asks AppKit to quit the way the app menu's Quit (⌘Q) does: `-[NSApplication terminate:]`.
fn terminate_like_cmd_q(app: &AppHandle) {
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = app.run_on_main_thread(move || {
        if let Some(mtm) = objc2_foundation::MainThreadMarker::new() {
            objc2_app_kit::NSApplication::sharedApplication(mtm).terminate(None);
        }
        let _ = tx.send(());
    });
    let _ = rx.recv_timeout(Duration::from_secs(5));
}

/// Covers the main display with the test pattern — a borderless window above everything
/// (screen-saver level) on every Space — and proves it before anything records: one real
/// captured frame must carry a readable strip, or nothing is recorded at all. (A native
/// full-screen Space was left the moment the popover took the keyboard, and a take once
/// recorded the desktop instead — 2026-10-02. This guard makes that impossible.)
pub fn cover_with_pattern(app: &AppHandle) -> Result<tauri::WebviewWindow, String> {
    #[repr(C)]
    #[derive(Clone, Copy)]
    struct CGRect {
        x: f64,
        y: f64,
        w: f64,
        h: f64,
    }
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGDisplayBounds(display: u32) -> CGRect;
    }
    let display = main_display_id();
    let b = unsafe { CGDisplayBounds(display) };
    let w = WebviewWindowBuilder::new(app, "pattern", WebviewUrl::App("pattern.html".into()))
        .decorations(false)
        .resizable(false)
        .skip_taskbar(true)
        .visible(false)
        .position(b.x, b.y)
        .inner_size(b.w, b.h)
        .build()
        .map_err(|e| format!("pattern window: {e}"))?;
    let (tx, rx) = std::sync::mpsc::channel();
    let w2 = w.clone();
    let _ = app.run_on_main_thread(move || {
        if let Ok(ptr) = w2.ns_window() {
            let ns: &objc2_app_kit::NSWindow = unsafe { &*(ptr as *const objc2_app_kit::NSWindow) };
            ns.setLevel(1000); // NSScreenSaverWindowLevel: above the menu bar, the Dock, and shroom's own panels
            ns.setCollectionBehavior(
                objc2_app_kit::NSWindowCollectionBehavior::CanJoinAllSpaces
                    | objc2_app_kit::NSWindowCollectionBehavior::Stationary
                    | objc2_app_kit::NSWindowCollectionBehavior::IgnoresCycle
                    | objc2_app_kit::NSWindowCollectionBehavior::FullScreenAuxiliary,
            );
        }
        let _ = w2.show();
        let _ = tx.send(());
    });
    let _ = rx.recv_timeout(Duration::from_secs(5));
    std::thread::sleep(Duration::from_millis(1200));

    // The proof: what the screen capture sees must be the pattern. Its page has drawn by now
    // on a Mac at rest; on a slower one (a VM without a graphics card of its own) it is
    // sometimes a moment later, so the guard looks again for a few seconds before it
    // refuses. Nothing records until a look has seen the pattern.
    let began = std::time::Instant::now();
    let mut looks = 1;
    while let Err(e) = verify_pattern(display) {
        if began.elapsed() > PATTERN_PATIENCE {
            let _ = w.close();
            return Err(e);
        }
        std::thread::sleep(Duration::from_millis(250));
        looks += 1;
    }
    if looks > 1 {
        eprintln!("shroom: the pattern covered the display {} ms later than a first look ({looks} looks)", began.elapsed().as_millis());
    }
    Ok(w)
}

/// How long the pattern guard goes on looking after its first look.
const PATTERN_PATIENCE: Duration = Duration::from_secs(8);

/// The display's size in pixels (its current mode).
fn display_pixels(display: u32) -> (u32, u32) {
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGDisplayCopyDisplayMode(display: u32) -> *mut std::ffi::c_void;
        fn CGDisplayModeGetPixelWidth(mode: *mut std::ffi::c_void) -> usize;
        fn CGDisplayModeGetPixelHeight(mode: *mut std::ffi::c_void) -> usize;
        fn CGDisplayModeRelease(mode: *mut std::ffi::c_void);
    }
    unsafe {
        let mode = CGDisplayCopyDisplayMode(display);
        let size = (CGDisplayModeGetPixelWidth(mode) as u32, CGDisplayModeGetPixelHeight(mode) as u32);
        CGDisplayModeRelease(mode);
        size
    }
}

/// One real captured frame of `display` with nothing left out of it, in gray, at the size a
/// take records it: (the frame, its width, its height).
pub(crate) fn grab(display: u32) -> Result<(Vec<u8>, u32, u32), String> {
    let (pw, ph) = display_pixels(display);
    let (ow, oh) = Quality::Normal.output_size(pw, ph);
    Ok((shroom_capture::grab_gray(display, ow, oh)?, ow, oh))
}

/// Reads the strip from one real captured frame of `display`; refuses if it isn't there.
pub fn verify_pattern(display: u32) -> Result<(), String> {
    let (gray, ow, oh) = grab(display)?;
    match shroom_testpattern::read_gray(&gray, ow, oh) {
        Some(_) => Ok(()),
        None => Err("the test pattern isn't covering the display — nothing was recorded".into()),
    }
}

/// The guard must be seen to refuse: with no pattern on screen, verification fails.
pub fn guard_probe(app: AppHandle, out: PathBuf) {
    let result = verify_pattern(main_display_id());
    let json = serde_json::json!({ "guard_refused_without_pattern": result.is_err(), "reason": result.err() });
    let _ = std::fs::write(&out, json.to_string());
    app.exit(0);
}

#[derive(Serialize, Default)]
struct OnboardingReport {
    /// Every onboarding state id the Scene showed, in order ("closed" = no window).
    states: Vec<String>,
    /// What the drive waited for, and when (ms since start).
    steps: Vec<(String, u128)>,
    /// The first launch's faces on the welcome.
    welcome_faces: Vec<&'static str>,
    /// Law 6: the activation policy with the window open, and after Close.
    dock_with_window: Option<String>,
    dock_after_close: Option<String>,
    /// `--doors`, `--invite-someone`: each check of the setup window's Dock icon and keyboard
    /// (what, passed, what was seen).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    doors: Vec<(String, bool, String)>,
    /// What this run couldn't look at, and why (a locked screen keeps every app from the
    /// keyboard).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    unseen: Vec<String>,
    host: Option<String>,
    token_json_mode: Option<String>,
    status: Option<serde_json::Value>,
    files: Vec<String>,
    /// Files under the home (or run.log) holding the pasted token's text — must be empty.
    token_leaks: Vec<String>,
    /// The runner's stash after the token was saved — consumed, so empty.
    stash_empty: bool,
    /// A resumed run (`--resume`): the window the relaunch opened at, by itself.
    resumed_at: Option<&'static str>,
    /// Effects quiet mode logged instead of doing.
    would: Vec<String>,
    ok: bool,
    error: Option<String>,
}

/// Runs on its own thread against the real runtime (core, lanes, presenter), posting
/// exactly what the onboarding webview would: Act, Key, paste_secret, field.
pub fn drive_onboarding(app: AppHandle, rt: crate::runtime::Runtime, out: PathBuf, taken: Option<String>, gate_marker: Option<PathBuf>, resume: bool, (doors, invite): (bool, bool), token_file: Option<PathBuf>) {
    let mut report = OnboardingReport::default();
    let t0 = std::time::Instant::now();
    // The fake token: token-shaped, random, known only to this process — or, resumed, the
    // one the killed first launch saved.
    let mut raw = [0u8; 30];
    getrandom::fill(&mut raw).expect("the OS random source works");
    let alphabet = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    // The e2e run's token: handed over by a file only this user reads, gone once read — it
    // then lives in token.json alone (law 13), as a pasted one does.
    let live = token_file.is_some();
    let handed = token_file.map(|f| {
        let t = std::fs::read_to_string(&f).map(|t| t.trim().to_string()).unwrap_or_default();
        let _ = std::fs::remove_file(&f);
        t
    });
    let token: String = if let Some(t) = handed {
        t
    } else if resume {
        rt.home.token().unwrap_or_default()
    } else {
        "cfat_".chars().chain(raw.iter().map(|b| alphabet[*b as usize % alphabet.len()] as char)).collect()
    };
    let result = if token.is_empty() {
        Err(if resume { "resumed, but the first launch saved no token" } else { "the run's token file was empty or unreadable" }.to_string())
    } else {
        run_onboarding(&app, &rt, &token, OnboardingPath { taken: taken.as_deref(), gate_marker: gate_marker.as_deref(), resume, doors, invite, live }, &mut report, t0)
    };
    if let Err(e) = result {
        report.error = Some(e);
    }
    report.states = rt.trace.lock().unwrap().as_ref().map(|t| t.ids.clone()).unwrap_or_default();
    // Law 13: the token's text is in token.json and nowhere else under the home.
    report.token_leaks = leaks(&rt.home.root, &token);
    report.stash_empty = rt.stash.lock().unwrap().is_empty();
    report.files = files(&rt.home.root).into_iter().map(|p| p.strip_prefix(&rt.home.root).unwrap_or(&p).display().to_string()).collect();
    report.files.sort();
    report.would = std::fs::read_to_string(rt.home.root.join("logs/run.log"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.split_once("would ").map(|(_, w)| w.to_string()))
        .collect();
    // Every path shares these; the path-specific states (name, wiring, a gate, a found
    // space) are the caller's to check against the fake's state.
    let expected: &[&str] = if doors {
        // The doors' walk ends at the last step: closed, back, closed.
        &["welcome", "account", "token", "card", "done", "closed", "done", "closed"]
    } else if resume {
        &["done", "perm-priming", "all-set", "closed"]
    } else {
        &["welcome", "account", "token", "card", "done", "perm-priming", "all-set", "closed"]
    };
    let mut it = report.states.iter();
    let in_order = expected.iter().all(|e| it.any(|s| s == e));
    report.ok = report.error.is_none() && report.token_leaks.is_empty() && report.stash_empty && in_order && report.doors.iter().all(|(_, ok, _)| *ok);
    if report.error.is_none() && !in_order {
        report.error = Some(format!("states out of order: {:?}", report.states));
    }
    drop(token);
    let code = write_json(&out, &report);
    rt.ending_ours.store(true, std::sync::atomic::Ordering::SeqCst);
    app.exit(code);
}

fn onboarding_id(rt: &crate::runtime::Runtime) -> Option<&'static str> {
    snapshot(rt).and_then(|s| s.onboarding.map(|o| o.id))
}

fn wait_onboarding(rt: &crate::runtime::Runtime, what: &str, secs: u64, report: &mut OnboardingReport, t0: std::time::Instant, pred: impl Fn(&shroom_core::Scene) -> bool) -> Result<shroom_core::Scene, String> {
    let until = std::time::Instant::now() + Duration::from_secs(secs);
    while std::time::Instant::now() < until {
        if let Some(s) = snapshot(rt) {
            if pred(&s) {
                report.steps.push((what.to_string(), t0.elapsed().as_millis()));
                return Ok(s);
            }
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    Err(format!("timed out waiting for {what}; at {:?}", onboarding_id(rt)))
}

#[derive(Serialize, Default)]
struct LaunchReport {
    /// The launch kind this process read (its argv, its launch event).
    kind: String,
    /// What opened by itself once the boot settled: the popover's and the onboarding
    /// window's ids, the window's faces, whether either took the keyboard, and the Dock.
    popover: Option<&'static str>,
    onboarding: Option<&'static str>,
    faces: Vec<&'static str>,
    activates: bool,
    dock: Option<String>,
    tray: Option<&'static str>,
    /// Where the opened popover was put (its top-left corner, under the mushroom or at the
    /// fallback corner, how long it waited for a just-made mushroom to settle).
    placed: Option<crate::runtime::presenter::Placed>,
    /// The popover's panel on screen: its top-left corner, and whether it shows.
    panel: Option<(f64, f64, bool)>,
    /// The mushroom as it is when the report is written: its frame (x, y, w, h) and
    /// whether macOS shows it.
    mushroom: Option<((f64, f64, f64, f64), bool)>,
    /// Where the popover's corner belongs, from that mushroom: under it, centred, on its
    /// screen (the presenter's own geometry).
    expected: Option<(f64, f64, bool)>,
    /// The same popover dismissed, then opened again by a click on the mushroom (a
    /// popover opens more than once): the panel after the dismissal, and after the click
    /// with where it was put.
    dismissed: Option<crate::runtime::presenter::PanelCorner>,
    reopened: Option<crate::runtime::presenter::PanelCorner>,
    reopened_placed: Option<crate::runtime::presenter::Placed>,
    /// This process: the runner tells it from any other shroom, and waits for it to end.
    pid: u32,
    /// Why no window can have the keyboard on this Mac right now, if none can (a locked
    /// screen): the runner then lists what it couldn't look at.
    no_keyboard: Option<&'static str>,
    /// Why the mushroom can't be seen on this Mac right now, if it can't (a locked screen:
    /// the lock screen is over the menu bar, and macOS counts no menu-bar item as seen —
    /// `front::behind_the_lock`). The popover a launch opens then takes the corner, and the
    /// runner lists "under the mushroom" among what it couldn't look at.
    behind_the_lock: Option<&'static str>,
    /// Where the launch left the keyboard, once the boot settled. AppKit calls shroom the
    /// active app; the setup window the launch opened, as the keyboard reader has it; the
    /// popover it opened is the window with the keyboard, and the process the window server
    /// sends keys to is shroom (unknown where that can't be read).
    active: bool,
    window: Option<crate::front::Front>,
    popover_key: Option<bool>,
    keys_ours: Option<bool>,
}

/// `--drive launch`: what a launch opens by itself — nothing is clicked.
pub fn drive_launch(app: AppHandle, rt: crate::runtime::Runtime, kind: shroom_core::world::LaunchKind, out: PathBuf) {
    let mut report = LaunchReport { kind: format!("{kind:?}"), ..Default::default() };
    // The boot's Scene, and anything it opens, settle well inside this.
    std::thread::sleep(Duration::from_millis(2500));
    if let Some(s) = snapshot(&rt) {
        report.popover = s.popover.as_ref().map(|p| p.id);
        report.onboarding = s.onboarding.as_ref().map(|o| o.id);
        report.faces = s.onboarding.as_ref().map(|o| o.faces.clone()).unwrap_or_default();
        report.activates = s.popover.as_ref().is_some_and(|p| p.activates) || s.onboarding.as_ref().is_some_and(|o| o.activates);
        report.tray = s.tray.state;
    }
    report.dock = on_main_policy(&app);
    let (tx, rx) = std::sync::mpsc::channel();
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || {
        let _ = tx.send(crate::runtime::presenter::placement(&app2));
    });
    if let Ok((placed, panel, m)) = rx.recv_timeout(Duration::from_secs(5)) {
        use crate::runtime::presenter::{Frame, popover_corner};
        report.placed = placed;
        report.panel = panel;
        report.mushroom = m.map(|m| ((m.frame.x, m.frame.y, m.frame.w, m.frame.h), m.visible));
        let off = Frame { x: 0.0, y: 0.0, w: 0.0, h: 0.0 };
        report.expected = m.filter(|m| m.visible).map(|m| popover_corner(Some(m), off, 320.0));
    }
    // The keyboard, as the launch left it — before anything here touches the popover.
    report.pid = std::process::id();
    report.no_keyboard = no_keyboard(&app);
    report.behind_the_lock = on_main_lock(&app);
    report.window = report.onboarding.is_some().then(|| front_of(&app, crate::runtime::presenter::ONBOARDING));
    let (tx, rx) = std::sync::mpsc::channel();
    let (app2, shows) = (app.clone(), report.popover.is_some());
    let _ = app.run_on_main_thread(move || {
        use crate::runtime::presenter::{Overlay, overlay_window};
        let key = shows.then(|| overlay_window(&app2, Overlay::Popover).is_some_and(|w| w.isKeyWindow()));
        let _ = tx.send((app_active(), key, crate::take_drive::key_focus_pid()));
    });
    if let Ok((active, key, keys)) = rx.recv_timeout(Duration::from_secs(5)) {
        report.active = active;
        report.popover_key = key;
        report.keys_ours = keys.map(|pid| pid == std::process::id() as i32);
    }
    if report.popover.is_some() {
        let read = || {
            let (tx, rx) = std::sync::mpsc::channel();
            let app2 = app.clone();
            let _ = app.run_on_main_thread(move || {
                let _ = tx.send(crate::runtime::presenter::placement(&app2));
            });
            rx.recv_timeout(Duration::from_secs(5)).ok()
        };
        // The panel once it is hidden, or shown: a Mac kept busy (a scenario before this one
        // still rendering) lands the dismissal and the click late, so each is waited for, a
        // few seconds at most — then whatever is there is what the runner judges.
        let once = |shows: bool| crate::drives::poll(5, || read().filter(|(_, panel, _)| panel.is_some_and(|p| p.2 == shows))).or_else(read);
        rt.post(shroom_core::event::Event::PopoverDismissed);
        report.dismissed = once(false).and_then(|(_, panel, _)| panel);
        click(&app, &rt);
        // An opening places the panel before it shows it, so shown again after hidden is a
        // fresh placement.
        if let Some((placed, panel, _)) = once(true) {
            report.reopened = panel;
            report.reopened_placed = placed;
        }
    }
    let code = write_json(&out, &report);
    rt.ending_ours.store(true, std::sync::atomic::Ordering::SeqCst);
    app.exit(code);
}

/// Why nothing of any app's can be seen right now (a locked screen), if nothing can.
fn on_main_lock(app: &AppHandle) -> Option<&'static str> {
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = app.run_on_main_thread(move || {
        let _ = tx.send(crate::front::behind_the_lock());
    });
    rx.recv_timeout(Duration::from_secs(5)).ok().flatten()
}

fn on_main_policy(app: &AppHandle) -> Option<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = app.run_on_main_thread(move || {
        let _ = tx.send(policy());
    });
    rx.recv_timeout(Duration::from_secs(5)).ok()
}

/// What the runner asked of an onboarding drive: a taken name to claim first, where to
/// mark a gate, and whether this is a relaunch resuming after a kill.
struct OnboardingPath<'a> {
    taken: Option<&'a str>,
    gate_marker: Option<&'a Path>,
    resume: bool,
    doors: bool,
    invite: bool,
    /// The e2e run's real account: wiring waits out workers.dev switching the name on.
    live: bool,
}

/// Where one of shroom's windows stands, read on the main thread.
fn front_of(app: &AppHandle, label: &'static str) -> crate::front::Front {
    let (tx, rx) = std::sync::mpsc::channel();
    let app2 = app.clone();
    let _ = app.run_on_main_thread(move || {
        let _ = tx.send(crate::front::front(&app2, label));
    });
    rx.recv_timeout(Duration::from_secs(5)).unwrap_or_default()
}

/// Waits a few seconds for the window to be in front with the keyboard; what it saw last.
fn keyboard_at(app: &AppHandle, label: &'static str) -> crate::front::Front {
    let until = std::time::Instant::now() + Duration::from_secs(4);
    loop {
        let f = front_of(app, label);
        if f.has_keyboard() || std::time::Instant::now() > until {
            return f;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
}

/// The app macOS sends the keyboard to, by bundle id (read on the main thread).
fn frontmost_now(app: &AppHandle) -> Option<String> {
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = app.run_on_main_thread(move || {
        let _ = tx.send(frontmost());
    });
    rx.recv_timeout(Duration::from_secs(5)).ok().flatten()
}

/// Why nothing can take the keyboard right now (a locked screen), if nothing can.
fn no_keyboard(app: &AppHandle) -> Option<&'static str> {
    let (tx, rx) = std::sync::mpsc::channel();
    let _ = app.run_on_main_thread(move || {
        let _ = tx.send(crate::front::no_keyboard());
    });
    rx.recv_timeout(Duration::from_secs(5)).ok().flatten()
}

fn run_onboarding(
    app: &AppHandle,
    rt: &crate::runtime::Runtime,
    token: &str,
    path: OnboardingPath,
    report: &mut OnboardingReport,
    t0: std::time::Instant,
) -> Result<(), String> {
    let OnboardingPath { taken, gate_marker, resume, doors, invite, live } = path;
    use crate::runtime::presenter::{ONBOARDING, SETTINGS};
    use shroom_core::event::{Event, Field, SecretField, Surface};
    use shroom_core::world::WindowKind;
    use shroom_core::scene::{Action, Nest, OnboardingBody};
    let is = |id: &'static str| move |s: &shroom_core::Scene| s.onboarding.as_ref().is_some_and(|o| o.id == id);
    let act = |action: Action| rt.post(Event::Act { surface: Surface::Onboarding, action, rev: 0 });

    if resume {
        // The person opens shroom again after it died mid-wire: a launch you did is a click
        // (N41), and the plan re-derives where it stands from what's on disk and a fresh
        // probe — nothing pasted, nothing claimed again.
        let s = wait_onboarding(rt, "the relaunch's window", 20, report, t0, |s| s.onboarding.is_some())?;
        report.resumed_at = s.onboarding.as_ref().map(|o| o.id);
        std::thread::sleep(Duration::from_millis(300));
        report.dock_with_window = on_main_policy(app);
    } else {
        // A fresh install's first launch opens the welcome.
        let s = wait_onboarding(rt, "welcome", 20, report, t0, is("welcome"))?;
        report.welcome_faces = s.onboarding.map(|o| o.faces).unwrap_or_default();
        std::thread::sleep(Duration::from_millis(300));
        report.dock_with_window = on_main_policy(app);

        act(Action::SetUp);
        wait_onboarding(rt, "account", 5, report, t0, is("account"))?;
        act(Action::HaveOne);
        wait_onboarding(rt, "token", 5, report, t0, |s| {
            s.onboarding.as_ref().is_some_and(|o| matches!(&o.body, OnboardingBody::Plan(p) if matches!(p.nest, Nest::Token { .. })))
        })?;

        // The paste, through the same path the webview's paste handler takes.
        rt.paste_secret(SecretField::Token, token.to_string());
    }

    // From here the fake's state decides the path: react to what the window shows until
    // row six ("Allow recording") is current.
    let random_name = || {
        let mut suffix = [0u8; 4];
        getrandom::fill(&mut suffix).expect("the OS random source works");
        format!("drive-{}", suffix.iter().map(|b| format!("{b:02x}")).collect::<String>())
    };
    // Typed like a person, a key at a time (from an empty field), then claimed.
    let claim = |name: &str, report: &mut OnboardingReport| -> Result<(), String> {
        rt.field(Field::Name, String::new());
        for i in 1..=name.len() {
            rt.field(Field::Name, name[..i].to_string());
            std::thread::sleep(Duration::from_millis(15));
        }
        wait_onboarding(rt, &format!("typed {name}"), 5, report, t0, |s| {
            s.onboarding.as_ref().is_some_and(|o| matches!(&o.body, OnboardingBody::Plan(p) if matches!(&p.nest, Nest::Name { value, claim, .. } if value == name && claim.enabled)))
        })?;
        act(Action::Claim);
        Ok(())
    };
    let mut tried_taken = taken.is_none() || resume;
    // A resumed run never claims a name: the killed launch's claim stands (were the plan
    // to ask again, the run times out at `name` and says so).
    let mut named = resume;
    let mut gates_seen: Vec<&'static str> = Vec::new();
    // The fake answers at once; a real account's wiring waits for workers.dev to switch a
    // fresh name on ("about a minute", the plan says) and for the page to answer as itself.
    let until = std::time::Instant::now() + Duration::from_secs(if live { 300 } else { 120 });
    let mut last = None;
    loop {
        if std::time::Instant::now() > until {
            return Err(format!("timed out before row six; at {:?}", onboarding_id(rt)));
        }
        let id = onboarding_id(rt);
        let fresh = id != last;
        last = id;
        match id {
            Some("done") => {
                report.steps.push(("done".into(), t0.elapsed().as_millis()));
                break;
            }
            Some("name") if !tried_taken => {
                tried_taken = true;
                claim(taken.unwrap_or_default(), report)?;
                wait_onboarding(rt, "name-taken", 30, report, t0, is("name-taken"))?;
            }
            Some("name" | "name-taken") if !named => {
                named = true;
                claim(&random_name(), report)?;
                wait_onboarding(rt, "claimed", 30, report, t0, |s| s.onboarding.as_ref().is_some_and(|o| !matches!(o.id, "name" | "name-taken")))?;
            }
            Some(g @ ("gate-email" | "gate-card")) if !gates_seen.contains(&g) => {
                gates_seen.push(g);
                report.steps.push((g.to_string(), t0.elapsed().as_millis()));
                if let Some(m) = gate_marker {
                    let _ = std::fs::write(m, g);
                }
            }
            Some(other) if fresh => report.steps.push((other.to_string(), t0.elapsed().as_millis())),
            _ => {}
        }
        std::thread::sleep(Duration::from_millis(20));
    }

    // The keyboard's side of the doors needs a screen nothing is locked in front of.
    let hands = (doors || invite) && match no_keyboard(app) {
        None => true,
        Some(why) => {
            report.unseen.push(format!("{} in front with the keyboard — {why}", if invite { "Settings, after Invite someone," } else { "the setup window, closed and brought back," }));
            false
        }
    };
    let policy = || on_main_policy(app).unwrap_or_default();
    let ours = || {
        let (tx, rx) = std::sync::mpsc::channel();
        let _ = app.run_on_main_thread(move || {
            let _ = tx.send(crate::front::ours_in_front());
        });
        rx.recv_timeout(Duration::from_secs(5)).unwrap_or(false)
    };
    if doors {
        // ✕ on the setup window is the retreat (N12): the Dock icon goes with it, the
        // keyboard goes back to the app the person was in, and shroom is a menu-bar app
        // until the mushroom brings the window back.
        let before = policy();
        rt.post(Event::WindowClosed(WindowKind::Onboarding));
        wait_onboarding(rt, "closed at the last step", 5, report, t0, |s| s.onboarding.is_none())?;
        std::thread::sleep(Duration::from_millis(500));
        let after = policy();
        report.doors.push(("closing the setup window retreats to the menu bar: the Dock icon goes".into(), before.starts_with("regular") && after.starts_with("accessory"), format!("{before} → {after}")));
        if hands {
            let until = std::time::Instant::now() + Duration::from_secs(3);
            while ours() && std::time::Instant::now() < until {
                std::thread::sleep(Duration::from_millis(100));
            }
            report.doors.push(("and the keyboard goes back to the app the person was in".into(), !ours(), format!("{:?} has it", frontmost_now(app))));
        }
        // Coming back to finish setup: the click is the door (DEVIATIONS #5), and the icon
        // is back with the window — in front, with the keyboard.
        click(app, rt);
        crate::runtime::presenter::bring_forward(app);
        wait_onboarding(rt, "done again", 10, report, t0, is("done"))?;
        std::thread::sleep(Duration::from_millis(500));
        let back = policy();
        report.doors.push(("coming back to finish setup, the Dock icon is back with the window".into(), back.starts_with("regular"), back));
        if hands {
            let f = keyboard_at(app, ONBOARDING);
            report.doors.push(("the setup window comes back in front with the keyboard".into(), f.has_keyboard(), f.says()));
        }
        // The walk ends here: nothing is asked of this Mac's grants.
        rt.post(Event::WindowClosed(WindowKind::Onboarding));
        wait_onboarding(rt, "closed", 5, report, t0, |s| s.onboarding.is_none())?;
    } else {
        // This Mac's grants are already given: no prompt — the warm-up runs, then all-set.
        act(Action::AllowRecording);
        wait_onboarding(rt, "all-set", 30, report, t0, is("all-set"))?;
        if invite {
            // all-set's Invite someone: setup is over, and Settings carries on in the same
            // turn — the Dock icon staying with it (law 6), in front, with the keyboard,
            // and still with it a moment later.
            let before = policy();
            act(Action::InviteSomeone);
            wait_onboarding(rt, "closed", 5, report, t0, |s| s.onboarding.is_none() && s.settings.is_some())?;
            std::thread::sleep(Duration::from_millis(500));
            let after = policy();
            report.doors.push(("Invite someone: setup is over, and Settings carries on with the Dock icon".into(), before.starts_with("regular") && after.starts_with("regular"), format!("{before} → {after}")));
            if hands {
                let f = keyboard_at(app, SETTINGS);
                report.doors.push(("Settings takes over in front with the keyboard".into(), f.has_keyboard(), f.says()));
                std::thread::sleep(Duration::from_millis(1500));
                let f = front_of(app, SETTINGS);
                report.doors.push(("and keeps it a moment later".into(), f.has_keyboard(), f.says()));
                // N28: the Members face arrives with “Who's this for?” focused — what the
                // person types next is the name, with no click first. The drive never puts
                // the caret there: its keys go where a keyboard's go, to whichever window
                // holds the keyboard, one at a time, each seen by the core before the next.
                let name = |s: &shroom_core::Scene| match s.settings.as_ref().map(|w| &w.body.pane) {
                    Some(shroom_core::scene::SettingsPane::Members(m)) => Some(m.name.value.clone()),
                    _ => None,
                };
                // On a busy Mac the page can still be blank seconds after the window has the
                // keyboard, and a key that arrives before it is drawn goes nowhere: the drive
                // waits for the field to be there — never putting the caret in it — and says
                // which field had the caret when it began to type.
                let drawn = crate::front::drawn_with(app, SETTINGS, "[data-field=\"invite-name\"]", 10);
                let mut typed = String::new();
                for c in "ani".chars() {
                    let (tx, rx) = std::sync::mpsc::channel();
                    let _ = app.run_on_main_thread(move || {
                        let _ = tx.send(crate::front::press(c));
                    });
                    if !matches!(rx.recv_timeout(Duration::from_secs(5)), Ok(Ok(()))) {
                        break;
                    }
                    typed.push(c);
                    let want = typed.clone();
                    if wait_onboarding(rt, &format!("typed {want}"), 3, report, t0, |s| name(s).as_deref() == Some(want.as_str())).is_err() {
                        break;
                    }
                }
                let reads = snapshot(rt).and_then(|s| name(&s));
                report.doors.push(("and what's typed next is the name: “Who's this for?” arrived with the caret".into(), reads.as_deref() == Some("ani"), format!("typed {typed:?}; the field reads {reads:?}; before typing, {}", drawn.unwrap_or_else(|e| e))));
            }
            rt.post(Event::WindowClosed(WindowKind::Settings));
            wait_onboarding(rt, "Settings closed", 5, report, t0, |s| s.settings.is_none())?;
        } else {
            act(Action::CloseWindow);
            wait_onboarding(rt, "closed", 5, report, t0, |s| s.onboarding.is_none())?;
        }
    }
    std::thread::sleep(Duration::from_millis(500));
    report.dock_after_close = on_main_policy(app);

    use std::os::unix::fs::PermissionsExt;
    report.token_json_mode = std::fs::metadata(rt.home.path("token.json")).ok().map(|m| format!("{:o}", m.permissions().mode() & 0o777));
    report.status = std::fs::read(rt.home.path("status.json")).ok().and_then(|b| serde_json::from_slice(&b).ok());
    let identity: Option<shroom_core::world::SpaceIdentity> = rt.home.read_json("space.json");
    report.host = identity.map(|i| i.host);
    Ok(())
}

pub(crate) fn files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    let mut stack = vec![root.to_path_buf()];
    while let Some(dir) = stack.pop() {
        for e in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
            let p = e.path();
            if p.is_dir() { stack.push(p) } else { out.push(p) }
        }
    }
    out
}

/// Every file under the home but token.json whose bytes hold the token.
pub(crate) fn leaks(root: &Path, token: &str) -> Vec<String> {
    files(root)
        .into_iter()
        .filter(|p| p != &root.join("token.json"))
        // Regular files only: a take's audio FIFO would block the read.
        .filter(|p| std::fs::symlink_metadata(p).is_ok_and(|m| m.is_file()))
        .filter(|p| std::fs::read(p).is_ok_and(|b| b.windows(token.len()).any(|w| w == token.as_bytes())))
        .map(|p| p.display().to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        std::iter::once("shroom").chain(s.split_whitespace()).map(String::from).collect()
    }

    /// The L3 kill drives and the heartbeat's member leg parse to their modes (B68, B69).
    #[test]
    fn the_kill_and_member_drives_parse() {
        assert!(matches!(from_args(args("--home /tmp/h --drive hb-member /tmp/o.json --invite")), Mode::DriveRegion6 { which: Region6::HbMember, .. }));
        assert!(matches!(from_args(args("--drive take /tmp/o.json capture-kill")), Mode::DriveTake { which: crate::take_drive::TakeDrive::CaptureKill, .. }));
        assert!(matches!(from_args(args("--drive take /tmp/o.json capture-resume")), Mode::DriveTake { which: crate::take_drive::TakeDrive::CaptureResume, .. }));
        assert!(matches!(from_args(args("--drive onboarding /tmp/o.json --resume")), Mode::DriveOnboarding { resume: true, .. }));
        assert!(matches!(from_args(args("--drive onboarding /tmp/o.json --relaunched=install")), Mode::DriveOnboarding { resume: false, .. }));
    }

    /// The onboarding runner's own lines (app/test/onboarding-drive.mjs), each as it hands
    /// it to the app. `main` takes `--drive onboarding` only on the install script's first
    /// launch and ends any other before its home is made — and `launch_kind` reads the
    /// installer's flags wherever they stand on the line. So a flag of the drive's own is
    /// never one of the installer's: `--invite` is its join hand-off, and the path that
    /// passed it for all-set's Invite someone was a launch the drive refused.
    #[test]
    fn every_path_of_the_onboarding_runner_is_a_launch_the_drive_takes() {
        use shroom_core::world::LaunchKind;
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
        let runner = std::fs::read_to_string(root.join("app/test/onboarding-drive.mjs")).unwrap();
        // `const PATHS = {` … `};` — a path a line: `name: { …, drive: ["--flag", "value"], … },`.
        let table: Vec<&str> = runner.lines().skip_while(|l| !l.starts_with("const PATHS = {")).skip(1).take_while(|l| !l.starts_with("};")).collect();
        let mut seen = Vec::new();
        for row in table {
            let name = row.split(':').next().unwrap().trim().trim_matches('"').to_string();
            let flags: Vec<String> = match row.split_once("drive: [") {
                Some((_, rest)) => rest.split(']').next().unwrap().split(',').map(|w| w.trim().trim_matches('"').to_string()).collect(),
                None => Vec::new(),
            };
            // The runner's line: its own flags, then the install script's mark, last.
            let line = format!("--home /tmp/h --cloud-fake http://a/client/v4 http://a/s3 --drive onboarding /tmp/o.json --gate-marker /tmp/gate {} --relaunched=install", flags.join(" "));
            let Mode::DriveOnboarding { taken, gate_marker, resume, doors, invite, .. } = from_args(args(&line)) else { panic!("{name}: not the onboarding drive: {line}") };
            assert_eq!((taken.is_some(), doors, invite, resume), (name == "taken", name == "doors", name == "invite", false), "{name}: the drive reads its path from {line}");
            assert_eq!(gate_marker, Some(PathBuf::from("/tmp/gate")), "{name}");
            assert_eq!(crate::runtime::launch_kind(&args(&line), false), LaunchKind::Install { join: false }, "{name}: the app ends this launch before the drive starts — a flag on it is the installer's: {line}");
            seen.push(name);
        }
        // The walk read the table (it isn't passing on nothing).
        assert_eq!(seen, ["fresh", "taken", "email-gate", "card-gate", "found", "doors", "invite"], "the runner's paths");
    }

    /// The heartbeat's owner steps are the files the heartbeat writes, beside its report.
    #[test]
    fn the_member_legs_signals_are_distinct() {
        let names = [OWNER_KEY_FILE, DELETE_NOW_FILE, MEMBER_NOW_FILE, REMOVE_NOW_FILE];
        let set: std::collections::BTreeSet<_> = names.iter().collect();
        assert_eq!(set.len(), names.len());
    }
}
