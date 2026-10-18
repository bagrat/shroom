//! World builders with the spec's example values, and a driver that runs events through
//! `update` with law checks at every step.
#![allow(dead_code)]

use shroom_core::event::{Event, Key, Surface};
use shroom_core::scene::{Action, Scene};
use shroom_core::time::{Envelope, Fact, Mono, Wall};
use shroom_core::world::*;
use shroom_core::{Arm, Effect, derive, update};
use shroom_spec::Spec;

pub fn quick() -> QuickProbe {
    QuickProbe {
        screen: true,
        mic: Access::Granted,
        camera: Access::Granted,
        displays: vec![Display { id: 1, name: "Built-in Display".into(), is_main: true, x: 0, y: 0 }],
        mics: vec![Device { uid: "BuiltInMicrophoneDevice".into(), name: "MacBook Pro Microphone".into(), builtin: true, continuity: false, system_default: false }],
        cameras: vec![Device { uid: "cam".into(), name: "FaceTime HD Camera".into(), builtin: true, continuity: false, system_default: false }],
        disk_free: 200_000_000_000,
        online: Some(true),
        os: OsSession::default(),
        status_item_visible: true,
        login_item: None,
        clicked_display: None,
    }
}

pub fn identity() -> SpaceIdentity {
    SpaceIdentity {
        account_id: "acct".into(),
        label: "v".into(),
        subdomain: "myshroom".into(),
        host: "v.myshroom.workers.dev".into(),
        bucket: "shroom".into(),
        role: Role::TokenHolder,
        member_id: "m1".into(),
        owner_name: "Bagrat".into(),
        account_name: None,
        foreign_v: false,
    }
}

/// A ready Mac: provisioned, permissions chapter done, every local fact known.
pub fn ready() -> World {
    let mut w = World::blank();
    w.space.identity = Some(identity());
    w.space.token = true;
    // Primed a day ago: a ready Mac's monthly re-confirm isn't due (N38).
    w.mac.primed = Some(Wall(1_790_000_000_000 - 86_400_000));
    let q = quick();
    let at = Mono(0);
    w.mac.screen = Fact::known(q.screen, at);
    w.mac.mic = Fact::known(q.mic, at);
    w.mac.camera = Fact::known(q.camera, at);
    w.mac.displays = Fact::known(q.displays, at);
    w.mac.mics = Fact::known(q.mics, at);
    w.mac.cameras = Fact::known(q.cameras, at);
    w.mac.disk_free = Fact::known(q.disk_free, at);
    w.mac.os = Fact::known(q.os, at);
    w.net.online = Fact::known(true, at);
    w.now = Mono(1_000);
    w.wall = Wall(1_790_000_000_000);
    w
}

pub fn take(phase: Phase, length_ms: u64) -> OpenTake {
    OpenTake {
        dir: TakeDir("20261002-120000-ab12".into()),
        rec: RecId("AbCdEfGhIjKlMnOp".into()),
        opened: Mono(500),
        source: SourceId::Display(1),
        phase,
        since: Mono(500),
        length_ms,
        markers: 0,
        mic: None,
        camera: None,
    }
}

pub fn saved(cause: SavedCause, length_ms: u64) -> SavedTake {
    SavedTake {
        dir: TakeDir("20261001-090000-cd34".into()),
        rec: RecId("QrStUvWxYz012345".into()),
        length_ms,
        cause,
        created: Wall(1_789_000_000_000),
        movies: false,
    }
}

/// Drives a World through events, checking the laws at every step.
pub struct Run {
    pub w: World,
    pub scene: Scene,
    pub effects: Vec<Effect>,
    /// The arm the last event ran, and the machine's edge it names.
    pub arm: Arm,
    pub t: u64,
    /// Wall time the Mac spent asleep: the wall clock runs ahead of `t`, which (like the
    /// app's monotonic clock on macOS) doesn't count sleep.
    pub slept: i64,
}

impl Run {
    pub fn new(w: World) -> Run {
        let scene = derive(&w);
        let t = w.now.0;
        Run { w, scene, effects: vec![], arm: Arm::named("start"), t, slept: 0 }
    }

    pub fn at(&mut self, ms: u64) -> &mut Run {
        self.t = ms;
        self
    }

    pub fn send(&mut self, event: Event) -> &mut Run {
        self.t += 10;
        let prev_w = self.w.clone();
        let prev = self.scene.clone();
        let (fx, arm) = update(&mut self.w, Envelope { mono: Mono(self.t), wall: Wall(1_790_000_000_000 + self.t as i64 + self.slept), event: event.clone() });
        let next = derive(&self.w);
        crate::laws::check(&prev_w, &prev, &event, &self.w, &next, &fx);
        // The runner's own dev-build law check (DECISIONS B93) agrees at every step: a
        // breach it reports would be a false alarm here, where every law holds.
        assert_eq!(shroom_core::trace::breach(prev_w.take_open(), &prev, &self.w, &next, &fx), None, "the runner's law check disagrees after {event:?}");
        // ARCHITECTURE §3's independent oracle: the fixtures' entry conditions, read on
        // their own, name what `derive` shows (DECISIONS B89).
        crate::laws::entry::check(&self.w, &next);
        // TESTING-PLAN §2.2: the core names no edge the spec doesn't draw.
        if let Some(edge) = arm.edge {
            assert!(edge_listed(edge), "arm {:?} names {edge:?}: not an edge of spec/machine.json", arm.name);
        }
        self.arm = arm;
        self.effects = fx;
        self.scene = next;
        self
    }

    pub fn click(&mut self) -> &mut Run {
        self.send(Event::TrayClicked { right: false, quick: quick() })
    }

    /// A click on the mushroom in this display's menu bar (N56).
    pub fn click_on(&mut self, display: u32) -> &mut Run {
        let displays = self.w.mac.displays.get().cloned().unwrap_or_default();
        self.send(Event::TrayClicked { right: false, quick: QuickProbe { displays, clicked_display: Some(display), ..quick() } })
    }

    pub fn act(&mut self, action: Action) -> &mut Run {
        self.send(Event::Act { surface: Surface::Popover, action, rev: 0 })
    }

    pub fn key(&mut self, surface: Surface, key: Key) -> &mut Run {
        self.send(Event::Key { surface, key })
    }

    pub fn tick_to(&mut self, ms: u64) -> &mut Run {
        self.t = ms - 10;
        self.send(Event::Tick)
    }

    pub fn ids(&self) -> Vec<&'static str> {
        self.scene.state_ids()
    }

    pub fn has(&self, pred: impl Fn(&Effect) -> bool) -> bool {
        self.effects.iter().any(pred)
    }
}

/// The takes a Save to Movies effect copies (its names aside).
pub fn saved_dirs(e: &Effect) -> Option<Vec<TakeDir>> {
    match e {
        Effect::Takes(shroom_core::effect::TakeCmd::SaveToMovies(c)) => Some(c.iter().map(|c| c.dir.clone()).collect()),
        _ => None,
    }
}

// ── the machine's edges (TESTING-PLAN §2.2, ARCHITECTURE §3.7 item 4) ──

/// The machine, loaded once per test binary.
pub fn spec() -> &'static Spec {
    static SPEC: std::sync::OnceLock<Spec> = std::sync::OnceLock::new();
    SPEC.get_or_init(Spec::load)
}

/// Whether the core may name this edge: it is an edge of the machine (spec/machine.json).
pub fn edge_listed((from, to): (&str, &str)) -> bool {
    from == to || spec().edge(from, to).is_some()
}

/// The region a state lives in (0 when it isn't a state).
pub fn region_of(id: &str) -> u32 {
    spec().state(id).map_or(0, |s| s.region)
}

/// The machine's drawn edges (the map's, kept in spec/machine.json) that a test's slice takes
/// in: each test realizes them, or says where they're realized or why they're out of scope.
pub fn drawn_edges(in_scope: impl Fn(&str, &str) -> bool) -> std::collections::BTreeSet<(String, String)> {
    spec().edges.iter().filter(|e| e.source.iter().any(|s| s == "map") && in_scope(&e.from, &e.to)).map(|e| (e.from.clone(), e.to.clone())).collect()
}

/// An event's kind, as the machine's rules name events: its variant, and the action, news,
/// op, key or field inside it — `Act(Restart)`, `Cloud(Swept)`, `Failed(Wire)`, `Key(Esc)`,
/// `TrayClicked`.
pub fn event_kind(ev: &Event) -> String {
    let s = format!("{ev:?}");
    let head: String = s.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
    let inner_after = |key: &str| s.split_once(key).map(|(_, rest)| rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect::<String>());
    let inner = match head.as_str() {
        "Act" => inner_after("action: "),
        "Cloud" | "Rec" | "Os" | "Join" => inner_after("("),
        "Failed" => inner_after("op: "),
        "Key" => inner_after("key: "),
        "SecretPasted" | "Typed" => inner_after("field: "),
        _ => None,
    };
    match inner {
        Some(i) if !i.is_empty() => format!("{head}({i})"),
        _ => head,
    }
}

// ── law 12's converse over the inventory (the machine's law 12; ARCHITECTURE §3.7 item 7) ──

/// Every way an op can fail: its deadline going by (any op), and each failure class its
/// lane posts as `Failed` (the runner's lanes: the wizard's, the member lane, the Library's,
/// the updater, the uploader — app/src-tauri/src/runtime/*.rs). The recorder, the stop's
/// reach probe, the page opening and the relaunch only ever fail by their deadline.
const FAIL_CLASSES: &[FailClass] = &[FailClass::Transport, FailClass::Refused, FailClass::Timeout, FailClass::Panicked, FailClass::Unclassified];
fn lane_fails(op: Op) -> &'static [FailClass] {
    match op {
        Op::Validate | Op::SaveToken | Op::Sweep | Op::Claim | Op::Wire | Op::Refresh => FAIL_CLASSES,
        Op::Peek | Op::Join | Op::Standing | Op::Roster | Op::Invite | Op::Remove | Op::Mark => FAIL_CLASSES,
        Op::LibRefresh | Op::Rename | Op::Delete | Op::UpdateCheck | Op::UpdateDownload | Op::UpdateApply => FAIL_CLASSES,
        // N68: the suggestions, a keep and the play-count switch — the Library lane's classes.
        Op::Suggest | Op::Keep | Op::Plays => FAIL_CLASSES,
        // The token Mac's cleanup sweep (B91): the Library lane's classes.
        Op::Cleanup => FAIL_CLASSES,
        // No space on this Mac is the cache read's one failure.
        Op::LibCache => &[FailClass::Refused],
        Op::Upload => &[FailClass::Transport, FailClass::Refused],
        Op::RecStart | Op::RecPause | Op::RecResume | Op::RecFinalize | Op::Prime | Op::Reach | Op::OpenPage | Op::Relaunch => &[],
    }
}

/// From every inventory row — as it stands, after each button it offers, and after its
/// next wake — every op in flight is failed every way (each class, and its deadline gone
/// by), with the laws checked at every step: law 12 then asks each failure to land (a face,
/// a line, its note read, a noted retry). Returns each break, named for its row and path.
pub fn every_failure_lands(rows: Vec<(&'static str, World)>) -> Vec<String> {
    use std::panic::{AssertUnwindSafe, catch_unwind};
    let mut out = Vec::new();
    let mut tried = 0;
    let quietly = |f: &mut dyn FnMut()| -> Option<String> {
        catch_unwind(AssertUnwindSafe(f)).err().map(|e| e.downcast_ref::<String>().cloned().or_else(|| e.downcast_ref::<&str>().map(|s| s.to_string())).unwrap_or_default())
    };
    for (row, w) in rows {
        let base = Run::new(w);
        let mut starts: Vec<(String, Run)> = vec![("as it stands".into(), base.clone_run())];
        for (surface, action) in offered(&base.scene) {
            let mut r = base.clone_run();
            if quietly(&mut || {
                r.send(Event::Act { surface, action: action.clone(), rev: 0 });
            })
            .is_none()
            {
                starts.push((format!("after {action:?}"), r));
            }
        }
        if let Some(wake) = base.scene.next_wake {
            let mut r = base.clone_run();
            if quietly(&mut || {
                r.tick_to(wake.0 + 10);
            })
            .is_none()
            {
                starts.push(("after its next wake".into(), r));
            }
        }
        for (path, start) in &starts {
            for (op, deadline) in start.w.inflight.ops.clone() {
                for class in lane_fails(op) {
                    tried += 1;
                    let mut r = start.clone_run();
                    if let Some(why) = quietly(&mut || {
                        r.send(Event::Failed { op, class: *class });
                    }) {
                        out.push(format!("{row}, {path}: {op:?} failing {class:?}: {}", why.lines().next().unwrap_or("")));
                    }
                }
                tried += 1;
                let mut r = start.clone_run();
                if let Some(why) = quietly(&mut || {
                    r.tick_to(deadline.0.max(r.t) + 20);
                }) {
                    out.push(format!("{row}, {path}: {op:?} past its deadline: {}", why.lines().next().unwrap_or("")));
                }
            }
        }
    }
    assert!(tried > 0, "no row had an op in flight to fail");
    out
}

/// Every enabled button a Scene offers, with its surface.
pub fn offered(scene: &Scene) -> Vec<(Surface, Action)> {
    let mut out: Vec<(Surface, Action)> = Vec::new();
    let mut add = |surface: Surface, bs: Vec<&shroom_core::scene::Button>| out.extend(bs.into_iter().filter(|b| b.enabled).map(|b| (surface, b.action.clone())));
    if let Some(p) = &scene.popover {
        add(Surface::Popover, p.buttons());
    }
    if let Some(o) = &scene.onboarding {
        add(Surface::Onboarding, o.buttons());
    }
    if let Some(l) = &scene.library {
        add(Surface::Library, l.buttons());
    }
    if let Some(s) = &scene.settings {
        add(Surface::Settings, s.buttons());
    }
    out
}

impl Run {
    pub fn clone_run(&self) -> Run {
        Run { w: self.w.clone(), scene: self.scene.clone(), effects: self.effects.clone(), arm: self.arm, t: self.t, slept: self.slept }
    }
}
