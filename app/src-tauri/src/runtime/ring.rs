//! The core's trace ring, kept by the runner (ARCHITECTURE §3.3 “Trace”, DECISIONS B93):
//! `shroom_core::trace::Ring` — the last 512 steps, kinds and names only (law 13) — filled
//! on the core thread and written to the private `logs/run.log` (700/600) when it matters:
//!
//! - a law breaks (`trace::breach`, checked after every batch in a dev build);
//! - a take turns out to have been rescued — a crash-recovery launch finds an interrupted
//!   take, or a capture died under one (the steps that led there, read beside the log);
//! - anything panics (a hook chained before the default one);
//! - a harness run (a drive, the heartbeat, the soak) ends — the harness *is* the flag: a
//!   drive's run log always carries the ring it ended on.
//!
//! The ring is a process-wide value behind a lock; the panic hook only ever `try_lock`s it,
//! so a panic on the core thread mid-push never deadlocks the dump.

use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{Duration, Instant};

use shroom_core::effect::Effect;
use shroom_core::scene::Scene;
use shroom_core::trace::Ring;
use shroom_core::world::{SavedCause, World};

use super::Runtime;

static RING: Mutex<Option<Ring>> = Mutex::new(None);
static LOGS: OnceLock<PathBuf> = OnceLock::new();

/// Where the ring lands, and the panic hook. Call once, as the runtime starts.
pub fn install(rt: &Runtime) {
    RING.lock().unwrap().get_or_insert_with(Ring::default);
    if LOGS.set(rt.home.root.join("logs")).is_err() {
        return;
    }
    let before = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        if let (Some(logs), Ok(ring)) = (LOGS.get(), RING.try_lock()) {
            if let Some(r) = ring.as_ref() {
                write(logs, r, "a panic");
            }
        }
        before(info);
    }));
}

/// One step of the core, its event named before `update` took it.
pub fn step(mono: u64, event: String, arm: &shroom_core::Arm, effects: &[Effect]) {
    RING.lock().unwrap().get_or_insert_with(Ring::default).step_named(mono, event, arm, effects);
}

/// The ring's lines, framed, into `run.log`.
fn write(logs: &Path, ring: &Ring, why: &str) {
    let mut out = format!("trace: {why} — the last {} rows of the core, oldest first\n", ring.len());
    for l in ring.lines() {
        out.push_str("trace ");
        out.push_str(&l);
        out.push('\n');
    }
    out.push_str("trace: end\n");
    super::log_line(logs, &out);
}

/// Writes the ring to `run.log` now.
pub fn dump(rt: &Runtime, why: &str) {
    write(&rt.home.root.join("logs"), RING.lock().unwrap().get_or_insert_with(Ring::default), why);
}

/// What the runner remembers between batches to know when to dump.
#[derive(Default)]
pub struct Watch {
    /// Interrupted takes known after the last batch.
    rescued: usize,
    /// When each law last dumped (a breach that keeps breaking dumps once a minute).
    breached: Vec<(&'static str, Instant)>,
}

impl Watch {
    /// After a batch: the screen into the ring, then the reasons to write it down.
    /// `prev` is the screen before the batch; `take_was_open` the take before it.
    pub fn after(&mut self, rt: &Runtime, w: &World, take_was_open: bool, prev: Option<&Scene>, next: &Scene, effects: &[Effect]) -> Option<String> {
        RING.lock().unwrap().get_or_insert_with(Ring::default).screen(rt.mono().0, next);
        let why = self.reason(rt.dev, w, take_was_open, prev, next, effects);
        if let Some(why) = &why {
            dump(rt, why);
        }
        why
    }

    fn reason(&mut self, dev: bool, w: &World, take_was_open: bool, prev: Option<&Scene>, next: &Scene, effects: &[Effect]) -> Option<String> {
        let rescued = w.saved.iter().filter(|s| s.cause == SavedCause::Interrupted).count();
        let more = rescued > self.rescued;
        self.rescued = rescued;
        if more {
            return Some("a take was rescued (a crash-recovery launch, or a capture that died)".into());
        }
        if !dev {
            return None;
        }
        let law = shroom_core::trace::breach(take_was_open, prev?, w, next, effects)?;
        let now = Instant::now();
        if self.breached.iter().any(|(l, at)| *l == law && now.duration_since(*at) < Duration::from_secs(60)) {
            return None;
        }
        self.breached.retain(|(l, _)| *l != law);
        self.breached.push((law, now));
        Some(format!("{law} (dev build)"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use shroom_core::event::Event;
    use shroom_core::world::{RecId, SavedTake, TakeDir};

    fn rt() -> (tempfile::TempDir, Runtime) {
        let tmp = tempfile::tempdir().unwrap();
        let (rt, _rx) = Runtime::for_test(tmp.path().join("h"), None, None);
        (tmp, rt)
    }

    fn log(rt: &Runtime) -> String {
        std::fs::read_to_string(rt.home.root.join("logs/run.log")).unwrap_or_default()
    }

    /// The ring lands in the private log, framed, kinds only: a rescued take dumps once.
    #[test]
    fn a_rescued_take_writes_the_ring_down_once() {
        let (_tmp, rt) = rt();
        step(5, shroom_core::trace::event_kind(&Event::Tick), &shroom_core::Arm::named("tick"), &[]);
        let mut w = World::blank();
        let scene = shroom_core::derive(&w);
        let mut watch = Watch::default();
        assert_eq!(watch.after(&rt, &w, false, Some(&scene), &scene, &[]), None);
        w.saved.push(SavedTake {
            dir: TakeDir("20261002-120000-ab12".into()),
            rec: RecId("AbCdEfGhIjKlMnOp".into()),
            length_ms: 4_000,
            cause: SavedCause::Interrupted,
            created: shroom_core::time::Wall(1),
            movies: false,
        });
        let why = watch.after(&rt, &w, false, Some(&scene), &shroom_core::derive(&w), &[]);
        assert!(why.is_some_and(|w| w.contains("rescued")));
        let text = log(&rt);
        assert!(text.contains("trace: a take was rescued") && text.contains("trace ") && text.contains("Tick → tick") && text.contains("trace: end"), "{text}");
        assert!(!text.contains("AbCdEfGhIjKlMnOp"), "law 13: never a link's code");
        assert_eq!(watch.after(&rt, &w, false, Some(&scene), &shroom_core::derive(&w), &[]), None, "once");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            assert_eq!(std::fs::metadata(rt.home.root.join("logs/run.log")).unwrap().permissions().mode() & 0o777, 0o600);
        }
    }

    /// A dev build writes the ring down when a law breaks — once a minute per law.
    #[test]
    fn a_law_breaking_in_a_dev_build_writes_the_ring_down() {
        let (_tmp, rt) = rt();
        let w = World::blank();
        let prev = shroom_core::derive(&w);
        // A Scene that lies about the Dock (law 6) — what the check exists to catch.
        let mut next = prev.clone();
        next.dock = !next.dock;
        let mut watch = Watch::default();
        let why = watch.after(&rt, &w, false, Some(&prev), &next, &[]);
        assert_eq!(why.as_deref(), Some("law 6: the dock and the windows disagree (dev build)"));
        assert!(log(&rt).contains("trace: law 6"));
        assert_eq!(watch.after(&rt, &w, false, Some(&prev), &next, &[]), None, "not again within the minute");
        assert_eq!(watch.reason(false, &w, false, Some(&prev), &next, &[]), None, "a release build never checks");
    }
}
