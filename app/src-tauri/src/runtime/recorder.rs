//! The recorder lane: serial Start / Pause / Resume / Finalize / Confirm / Discard /
//! Abandon / Marker / Prime, each answered with an event (Warm and Cool aren't). Holds the
//! take's LOCK while it's live (a crash releases it; the scan then reads the take as
//! interrupted — law 2).
//!
//! Between commands, while a take runs, its watchdog looks every second (DECISIONS B47):
//! the takes volume's free space against the floor (`Rec(DiskFloor)`, once, and the free
//! space itself as `Os(Disk)`), the take's liveness (`Rec(FrameStall)` — or, when screen
//! recording was switched off under it, the grants that say so), and the microphone's
//! own lost flag (`Rec(MicLost)`). The core decides what each means (N39).
//!
//! The microphone opens with the count (`Warm`), not at its zero, and a Bluetooth one stays
//! open through a pause: a headset wakes about two seconds after its mic opens, and that
//! wake was every take's silent head (B173).

use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{RecvTimeoutError, Sender, channel};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

use shroom_capture::{ScreenSource, mic};
use shroom_core::effect::RecCmd;
use shroom_core::event::{Event, OsNews, RecNews};
use shroom_core::world::{FailClass, Quality, RecId, SourceId, TakeDir, Why};
use shroom_encode::quality::Quality as EncQuality;
use shroom_record::mix::{Mixed, SystemAudio};
use shroom_record::warm::WarmMic;
use shroom_record::{AudioSource, FrameSource, Recording};

use super::Runtime;
use super::home::write_json;
use super::takes::{FinalizedJson, TakeJson, TakeLock, new_dir_name};

/// The open take, shared with the uploader (it uploads segments while recording).
#[derive(Debug, Clone)]
pub struct LiveTake {
    pub dir: PathBuf,
    pub rec: String,
}

struct Active {
    recording: Recording,
    dir: PathBuf,
    name: String,
    _lock: TakeLock,
    source: SourceId,
    quality: Quality,
    mic: Option<String>,
    /// Camera as the source: the camera recorded (the core's pick), reopened on Resume.
    camera: Option<String>,
    length_ms: u64,
    running: bool,
    markers: Vec<u64>,
    /// The running stretch: when it began, and its microphone's lost flag.
    since: Instant,
    mic_lost: Option<Arc<AtomicBool>>,
    /// A Bluetooth microphone, held open through a pause so Resume is heard at once (B173).
    held: Option<Opened>,
    /// What the watchdog already said this stretch (each is said once).
    said: Said,
}

/// A microphone opened ahead of its take: the count's, or a Bluetooth one held through a
/// pause. Each take records from a clone of `mic`; the device closes with the last one.
#[derive(Clone)]
struct Opened {
    uid: String,
    mic: WarmMic,
    lost: Arc<AtomicBool>,
    bluetooth: bool,
}

impl Opened {
    fn open(uid: &str) -> Option<Opened> {
        if shroom_capture::mic_access() != shroom_capture::Access::Granted {
            return None;
        }
        let info = mic::input_devices().into_iter().find(|d| d.uid == uid)?;
        let bluetooth = info.transport == mic::Transport::Bluetooth;
        let source = mic::MicSource::open(info).ok()?;
        let lost = source.lost_flag();
        let mic = WarmMic::warm(Box::new(source)).ok()?;
        Some(Opened { uid: uid.to_string(), mic, lost, bluetooth })
    }

    /// Still the device `uid` names, and still delivering.
    fn serves(&self, uid: &str) -> bool {
        self.uid == uid && !self.lost.load(Ordering::SeqCst)
    }
}

#[derive(Default)]
struct Said {
    stall: bool,
    mic: bool,
    floor: bool,
    /// The free space last posted.
    disk: Option<u64>,
}

/// How often the watchdog looks while a take runs.
pub const WATCH_EVERY: Duration = Duration::from_secs(1);
/// No progress for this long is capture dying (DEVIATIONS #6, #8: the recorder steps' 5 s).
pub const STALL_AFTER: Duration = Duration::from_secs(5);
/// The free space is re-posted mid-take when it moved this much (the core's `disk_free`).
const DISK_STEP: u64 = 32 * 1024 * 1024;

fn enc(q: Quality) -> EncQuality {
    match q {
        Quality::Normal => EncQuality::Normal,
        Quality::TwoK => EncQuality::TwoK,
        Quality::FourK => EncQuality::FourK,
    }
}

pub fn spawn(rt: Runtime, live: Arc<Mutex<Option<LiveTake>>>, excluded: Arc<Mutex<Vec<u32>>>) -> Sender<RecCmd> {
    let (tx, rx) = channel::<RecCmd>();
    std::thread::Builder::new()
        .name("shroom-recorder".into())
        .spawn(move || {
            let mut active: Option<Active> = None;
            // The microphone the count opened, until the count fires or is cancelled.
            let mut warm: Option<Opened> = None;
            loop {
                match rx.recv_timeout(WATCH_EVERY) {
                    Ok(cmd) => {
                        let news = handle(&rt, &mut active, &mut warm, &live, &excluded, cmd);
                        if let Some(n) = news {
                            rt.post(Event::Rec(n));
                        }
                    }
                    Err(RecvTimeoutError::Timeout) => {}
                    Err(RecvTimeoutError::Disconnected) => return,
                }
                if let Some(a) = active.as_mut() {
                    // While it records, the key Stop's page opens with is minted ahead
                    // (DECISIONS B59 g) — off this thread, once per grant's life.
                    if a.running {
                        super::cloud::mint_ahead(&rt);
                    }
                    for e in watch(a, super::probe::disk_free(&a.dir), super::os::grants) {
                        rt.log(&format!("recorder watchdog: {}", describe(&e)));
                        rt.post(e);
                    }
                }
            }
        })
        .expect("the recorder lane starts");
    tx
}

/// A take's frame accounting, for the media-truth checks (TESTING-PLAN §4): where every
/// repeated frame came from. Counts only.
fn log_frames(rt: &Runtime, t: &shroom_record::TakeStats) {
    rt.log(&format!(
        "recorder: take frames {} received, {} written, {} repeated ({} stale, {} late, {} backlog)",
        t.frames_received, t.frames_written, t.repeats, t.stale_ticks, t.late_ticks, t.backlog_overflow
    ));
}

/// The watchdog's look at a running take: what to tell the core, each thing once per
/// stretch. `free` is the takes volume's free space now; `grants` reads macOS's grants.
fn watch(a: &mut Active, free: u64, grants: impl Fn() -> OsNews) -> Vec<Event> {
    if !a.running {
        return Vec::new();
    }
    let seen = Seen {
        stall: a.recording.stall(STALL_AFTER),
        mic_lost: a.mic_lost.as_ref().is_some_and(|f| f.load(Ordering::SeqCst)),
        length_ms: a.length_ms + a.since.elapsed().as_millis() as u64,
        quality: a.quality,
        display: matches!(a.source, SourceId::Display(_)),
        free,
    };
    decide(&mut a.said, &seen, grants)
}

/// What the watchdog saw of a running take this second.
struct Seen {
    stall: Option<shroom_record::Stall>,
    mic_lost: bool,
    length_ms: u64,
    quality: Quality,
    /// A display is the source (only then does screen recording matter).
    display: bool,
    free: u64,
}

/// The watchdog's verdict, pure: capture dying is `FrameStall` — unless screen recording
/// was switched off under a display take, which is the grants' news (`permission-lost`);
/// the microphone's lost flag is `MicLost`; free space under the take's floor is
/// `DiskFloor` (with the free space itself); the free space is re-posted as it moves.
fn decide(said: &mut Said, seen: &Seen, grants: impl Fn() -> OsNews) -> Vec<Event> {
    let mut out = Vec::new();
    if !said.stall && seen.stall.is_some() {
        said.stall = true;
        let g = grants();
        let screen_off = seen.display && matches!(g, OsNews::Grants { screen: false, .. });
        out.push(if screen_off { Event::Os(g) } else { Event::Rec(RecNews::FrameStall) });
        return out;
    }
    if !said.mic && seen.mic_lost {
        said.mic = true;
        out.push(Event::Rec(RecNews::MicLost));
    }
    if !said.floor && seen.free < shroom_core::derive::take_floor_bytes(seen.quality, seen.length_ms) {
        said.floor = true;
        said.disk = Some(seen.free);
        out.push(Event::Os(OsNews::Disk(seen.free)));
        out.push(Event::Rec(RecNews::DiskFloor));
    } else if said.disk.is_none_or(|d| d.abs_diff(seen.free) >= DISK_STEP) {
        said.disk = Some(seen.free);
        out.push(Event::Os(OsNews::Disk(seen.free)));
    }
    out
}

fn describe(e: &Event) -> String {
    match e {
        Event::Os(OsNews::Disk(free)) => format!("free {} MB", free / 1_000_000),
        Event::Os(OsNews::Grants { screen, mic, .. }) => format!("grants screen={screen} mic={mic:?}"),
        Event::Rec(n) => format!("{n:?}"),
        other => format!("{other:?}"),
    }
}

type Sources = (Box<dyn FrameSource>, Option<Box<dyn AudioSource>>, Option<Arc<AtomicBool>>, Option<Opened>);

/// Where a take's sound comes from: the microphone the core named — or, in an unattended
/// harness run, the test pattern's beep in its place, or the voice-cut drive's speech
/// (`--voice`, N65), never the room: what a drive records may be published (TESTING-PLAN §4,
/// b4 #17). No microphone named: silence. A display's take
/// mixes the Mac's own sound in (spec N47, DECISIONS B172) — with no microphone, it alone.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Ears {
    Device,
    DriveBeep,
}

fn ears(rt: &Runtime) -> Ears {
    // `harness` is a quiet dev run only (never a release build, never a person's dev app).
    if rt.harness { Ears::DriveBeep } else { Ears::Device }
}

/// `ready`: a microphone already open (the count's, or one held through the pause) — used
/// when it's the one named and still delivering, else the named one opens now.
fn sources(ears: Ears, a_source: SourceId, quality: Quality, mic_uid: &Option<String>, ready: Option<Opened>, camera: &Option<String>, excluded: &[u32]) -> Result<Sources, String> {
    let system = matches!(a_source, SourceId::Display(_)).then(SystemAudio::new);
    let video: Box<dyn FrameSource> = match a_source {
        SourceId::Display(display) => {
            let screen = ScreenSource::new(display, enc(quality))?.excluding(excluded.to_vec());
            Box::new(match &system {
                Some(system) => screen.with_system_audio(system.clone()),
                None => screen,
            })
        }
        // The camera as the whole picture: the one the core picked (N39: never an iPhone by
        // itself) — this lane never chooses.
        SourceId::Camera => {
            let uid = camera.as_deref().ok_or("no camera")?;
            Box::new(shroom_capture::CameraSource::new(uid, enc(quality))?)
        }
    };
    let (voice, lost, opened) = if ears == Ears::DriveBeep {
        let drive: Option<Box<dyn AudioSource>> = match (mic_uid, crate::harness::drive_voice()) {
            (None, _) => None,
            (Some(_), Some(wav)) => Some(Box::new(shroom_capture::speech::SpeechSource::open(wav, 48_000)?)),
            (Some(_), None) => Some(Box::new(shroom_capture::beep::BeepSource::new(48_000))),
        };
        (drive, None, None)
    } else {
        let opened = mic_uid.as_deref().and_then(|uid| ready.filter(|o| o.serves(uid)).or_else(|| Opened::open(uid)));
        let voice = opened.as_ref().map(|o| Box::new(o.mic.clone()) as Box<dyn AudioSource>);
        (voice, opened.as_ref().map(|o| o.lost.clone()), opened)
    };
    let audio = match system {
        Some(system) => Some(Box::new(Mixed::new(voice, system)) as Box<dyn AudioSource>),
        None => voice,
    };
    Ok((video, audio, lost, opened))
}

fn handle(rt: &Runtime, active: &mut Option<Active>, warm: &mut Option<Opened>, live: &Arc<Mutex<Option<LiveTake>>>, excluded: &Arc<Mutex<Vec<u32>>>, cmd: RecCmd) -> Option<RecNews> {
    match cmd {
        RecCmd::Start { source, quality, mic, camera } => {
            let Some(ffmpeg) = super::locate_ffmpeg() else { return Some(RecNews::Failed) };
            let wall = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as i64).unwrap_or(0);
            let name = new_dir_name(wall);
            let dir = rt.home.takes().join(&name);
            let started = (|| -> Result<Active, String> {
                std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
                let lock = TakeLock::acquire(&dir).map_err(|e| e.to_string())?;
                let rec = shroom_upload::new_id();
                write_json(&dir.join("take.json"), &TakeJson { v: 1, rec: rec.clone(), created: wall }).map_err(|e| e.to_string())?;
                let mut recording = Recording::create(&dir, &ffmpeg, enc(quality).video_kbps()).map_err(|e| e.to_string())?;
                let (screen, audio, mic_lost, opened) = sources(ears(rt), source, quality, &mic, warm.take(), &camera, &excluded.lock().unwrap())?;
                recording.start_take(screen, audio).map_err(|e| e.to_string())?;
                Ok(Active {
                    recording,
                    dir: dir.clone(),
                    name: name.clone(),
                    _lock: lock,
                    source,
                    quality,
                    mic,
                    camera,
                    length_ms: 0,
                    running: true,
                    markers: Vec::new(),
                    since: Instant::now(),
                    mic_lost,
                    held: opened.filter(|o| o.bluetooth),
                    said: Said::default(),
                })
                    .inspect(|_| {
                        *live.lock().unwrap() = Some(LiveTake { dir: dir.clone(), rec: rec.clone() });
                    })
            })();
            match started {
                Ok(a) => {
                    let rec = live.lock().unwrap().as_ref().map(|l| l.rec.clone()).unwrap_or_default();
                    *active = Some(a);
                    Some(RecNews::Started { dir: TakeDir(name), rec: RecId(rec) })
                }
                Err(e) => {
                    rt.log(&format!("recorder start failed: {e}"));
                    let _ = std::fs::remove_dir_all(&dir);
                    Some(RecNews::Failed)
                }
            }
        }
        RecCmd::Pause => {
            let a = active.as_mut()?;
            if a.running {
                let ending = Instant::now();
                match a.recording.end_take() {
                    Ok(stats) => {
                        a.length_ms += (stats.seconds * 1000.0) as u64;
                        a.running = false;
                        // The encoder's finalize, timed (TESTING-PLAN §4: ≤ 200 ms).
                        rt.log(&format!("recorder: take ended in {} ms", ending.elapsed().as_millis()));
                        log_frames(rt, &stats);
                    }
                    Err(e) => {
                        rt.log(&format!("recorder pause failed: {e}"));
                        return Some(RecNews::Failed);
                    }
                }
            }
            Some(RecNews::Paused { length_ms: a.length_ms })
        }
        RecCmd::Resume { mic } => {
            let a = active.as_mut()?;
            // The microphone the core named: the take's own, the fallback, or none — the
            // recording keeps its audio format either way (one init.mp4, DECISIONS B47).
            a.mic = mic;
            let started = sources(ears(rt), a.source, a.quality, &a.mic, a.held.take(), &a.camera, &excluded.lock().unwrap())
                .and_then(|(s, m, lost, opened)| a.recording.start_take(s, m).map(|()| (lost, opened)).map_err(|e| e.to_string()));
            match started {
                Ok((lost, opened)) => {
                    a.running = true;
                    a.since = Instant::now();
                    a.mic_lost = lost;
                    a.held = opened.filter(|o| o.bluetooth);
                    a.said = Said::default();
                    Some(RecNews::Resumed)
                }
                Err(e) => {
                    rt.log(&format!("recorder resume failed: {e}"));
                    Some(RecNews::Failed)
                }
            }
        }
        RecCmd::Finalize { why } => {
            let Some(mut a) = active.take() else { return Some(RecNews::Finalized(Err(FailClass::Unclassified))) };
            *live.lock().unwrap() = None;
            let finalizing = Instant::now();
            if a.running {
                if let Ok(stats) = a.recording.end_take() {
                    a.length_ms += (stats.seconds * 1000.0) as u64;
                    log_frames(rt, &stats);
                }
            }
            match a.recording.finalize() {
                Ok(_) => {
                    // Stop → a whole recording on disk, timed (TESTING-PLAN §4: ≤ 200 ms).
                    rt.log(&format!("recorder: finalized in {} ms", finalizing.elapsed().as_millis()));
                    let why = match why {
                        Why::DiskFull => "disk-full",
                        Why::ScreenRevoked => "screen-off",
                        _ => "stop",
                    };
                    let _ = write_json(&a.dir.join("finalized.json"), &FinalizedJson { v: 1, length_ms: a.length_ms, why: why.into() });
                    // Already on disk since each drop; written once more, whole.
                    write_markers(&a.dir, &a.markers);
                    Some(RecNews::Finalized(Ok(a.length_ms)))
                }
                Err(e) => {
                    rt.log(&format!("recorder finalize failed: {e}"));
                    Some(RecNews::Finalized(Err(FailClass::Unclassified)))
                }
            }
        }
        RecCmd::Confirm(dir) => {
            let f: Option<FinalizedJson> = super::home::read_json(&rt.home.takes().join(&dir.0).join("finalized.json"));
            Some(RecNews::Finalized(f.map(|f| f.length_ms).ok_or(FailClass::Unclassified)))
        }
        RecCmd::Discard(dir) => {
            if active.as_ref().is_some_and(|a| a.name == dir.0) {
                *active = None; // dropping the recording kills its ffmpeg
                *live.lock().unwrap() = None;
            }
            let _ = std::fs::remove_dir_all(rt.home.takes().join(&dir.0));
            Some(RecNews::Deleted(dir))
        }
        RecCmd::Abandon => {
            // Kill capture, keep every byte: the scan reads it as interrupted (law 2) — once
            // the lock is released, which `Abandoned` says (a scan before it sees it live).
            let a = active.take()?;
            *live.lock().unwrap() = None;
            let Active { recording, dir, source, _lock: lock, .. } = a;
            let camera = source == SourceId::Camera;
            drop(recording); // its ffmpeg killed and waited for; the lock still held
            // Whether anything of it can be kept: the scan's own repair, run now, under the
            // take's lock (a scan meanwhile reads it live, and never repairs it alongside).
            // A folder no repair can save (no frame ever reached it) is no take — and no
            // banner will say so: the core says it instead (DEVIATIONS #53). The folder stays.
            use super::takes::{Kind, classify_held, repair};
            let kept = match classify_held(&dir) {
                Kind::Interrupted => {
                    repair(&dir);
                    !matches!(classify_held(&dir), Kind::Interrupted | Kind::Unreadable)
                }
                Kind::Unreadable => false,
                _ => true,
            };
            drop(lock);
            if kept {
                Some(RecNews::Abandoned)
            } else {
                rt.log(&format!("recorder: the abandoned take {} held nothing to keep", dir.file_name().map(|n| n.to_string_lossy().into_owned()).unwrap_or_default()));
                Some(RecNews::NothingKept { camera })
            }
        }
        RecCmd::Marker => {
            let a = active.as_mut()?;
            a.markers.push(a.length_ms);
            // On disk at once, atomically: a take that dies, or comes back as a recovered
            // one, keeps every marker dropped so far (whole-spec audit #28).
            write_markers(&a.dir, &a.markers);
            None
        }
        RecCmd::Warm { mic } => {
            // Restart's count reuses the paused take's held microphone rather than open it
            // twice. A harness run records the test pattern's beep: nothing to open.
            *warm = match mic {
                Some(uid) if ears(rt) == Ears::Device => {
                    let ready = warm.take().or_else(|| active.as_ref().and_then(|a| a.held.clone())).filter(|o| o.serves(&uid));
                    ready.or_else(|| Opened::open(&uid))
                }
                _ => None,
            };
            if let Some(o) = warm.as_ref() {
                rt.log(&format!("recorder: microphone open for the count (bluetooth: {})", o.bluetooth));
            }
            None
        }
        RecCmd::Cool => {
            *warm = None;
            None
        }
        RecCmd::Prime => {
            let report = shroom_capture::liveness_probe(Duration::from_secs(1), 30.0);
            Some(RecNews::Primed(report.frames_lockable > 0))
        }
    }
}

/// `markers.json`: the take's markers as seconds in the joined timeline — the pause point
/// each was dropped at (N24) — the record's `markers` at publish (DECISIONS B49).
fn write_markers(dir: &std::path::Path, markers: &[u64]) {
    if !markers.is_empty() {
        let seconds: Vec<f64> = markers.iter().map(|m| *m as f64 / 1000.0).collect();
        let _ = write_json(&dir.join(shroom_upload::upload::MARKERS_FILE), &serde_json::json!({ "v": 1, "seconds": seconds }));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn markers_are_on_disk_from_their_drop_as_seconds_in_the_joined_timeline() {
        let tmp = tempfile::tempdir().unwrap();
        write_markers(tmp.path(), &[]);
        assert!(!tmp.path().join("markers.json").exists(), "none dropped, no file");
        write_markers(tmp.path(), &[12_500]);
        write_markers(tmp.path(), &[12_500, 40_000]);
        assert_eq!(shroom_upload::upload::local_markers(tmp.path()), vec![12.5, 40.0], "the uploader reads what each drop wrote");
        assert!(!tmp.path().join("markers.tmp").exists(), "written atomically");
    }

    use super::*;
    use shroom_core::world::Access;
    use shroom_record::Stall;

    const GB: u64 = 1_000_000_000;

    fn seen() -> Seen {
        Seen { stall: None, mic_lost: false, length_ms: 60_000, quality: Quality::Normal, display: true, free: 50 * GB }
    }

    fn granted() -> OsNews {
        OsNews::Grants { screen: true, mic: Access::Granted, camera: Access::Granted }
    }

    #[test]
    fn the_disk_floor_is_said_once_with_the_free_space_and_the_space_as_it_moves() {
        let mut said = Said::default();
        assert_eq!(decide(&mut said, &seen(), granted), vec![Event::Os(OsNews::Disk(50 * GB))], "the first look posts the free space");
        assert!(decide(&mut said, &Seen { free: 50 * GB - 1_000_000, ..seen() }, granted).is_empty(), "a small move says nothing");
        assert_eq!(decide(&mut said, &Seen { free: 49 * GB, ..seen() }, granted), vec![Event::Os(OsNews::Disk(49 * GB))]);
        // The floor: both tee legs for a minute, the single-file copy of what's recorded, the margin.
        let floor = shroom_core::derive::take_floor_bytes(Quality::Normal, 60_000);
        assert!(floor > shroom_core::derive::DISK_MARGIN + shroom_core::derive::bytes_per_minute(Quality::Normal));
        assert_eq!(decide(&mut said, &Seen { free: floor - 1, ..seen() }, granted), vec![Event::Os(OsNews::Disk(floor - 1)), Event::Rec(RecNews::DiskFloor)]);
        assert!(decide(&mut said, &Seen { free: floor - 2, ..seen() }, granted).is_empty(), "said once");
        // A longer take needs more room for its single-file copy.
        assert!(shroom_core::derive::take_floor_bytes(Quality::Normal, 600_000) > floor);
    }

    #[test]
    fn a_stall_is_capture_dying_unless_screen_recording_was_switched_off() {
        let mut said = Said::default();
        let stalled = Seen { stall: Some(Stall::Source), ..seen() };
        assert_eq!(decide(&mut said, &stalled, granted), vec![Event::Rec(RecNews::FrameStall)]);
        assert!(decide(&mut said, &stalled, granted).iter().all(|e| !matches!(e, Event::Rec(RecNews::FrameStall))), "said once");
        let off = || OsNews::Grants { screen: false, mic: Access::Granted, camera: Access::Granted };
        let mut said = Said::default();
        assert_eq!(decide(&mut said, &stalled, off), vec![Event::Os(off())], "the stream stopped because macOS switched it off");
        let mut said = Said::default();
        assert_eq!(decide(&mut said, &Seen { display: false, ..stalled }, off), vec![Event::Rec(RecNews::FrameStall)], "a camera take doesn't need the screen");
    }

    #[test]
    fn a_harness_run_never_hears_the_room() {
        let tmp = tempfile::tempdir().unwrap();
        let (rt, _rx) = Runtime::for_test(tmp.path().join("h"), None, None);
        assert_eq!(super::ears(&rt), super::Ears::DriveBeep, "a drive records the pattern's beep");
    }

    #[test]
    fn the_microphones_lost_flag_is_said_once() {
        let mut said = Said { disk: Some(50 * GB), ..Said::default() };
        assert_eq!(decide(&mut said, &Seen { mic_lost: true, ..seen() }, granted), vec![Event::Rec(RecNews::MicLost)]);
        assert!(decide(&mut said, &Seen { mic_lost: true, ..seen() }, granted).is_empty());
    }
}
