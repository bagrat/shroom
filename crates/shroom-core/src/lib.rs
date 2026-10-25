//! shroom's core (ARCHITECTURE §3, B22): one `World` of facts, `update` as its only
//! writer, a pure `derive` to the `Scene` every surface renders, and `settle` for what's
//! due. No Tauri, no AppKit, no network, no threads, no clock reads.

pub mod apply;
pub mod copy;
pub mod derive;
pub mod effect;
pub mod event;
pub mod report;
pub mod scene;
pub mod settle;
pub mod time;
pub mod trace;
pub mod world;

pub use apply::Arm;
pub use derive::derive;
pub use effect::Effect;
pub use event::Event;
pub use scene::Scene;
pub use time::{Envelope, Mono, Wall};
pub use world::World;

/// The one way the World changes: stamp the time, apply the event, settle what's due, and
/// give every started op a deadline.
pub fn update(w: &mut World, env: Envelope) -> (Vec<Effect>, Arm) {
    w.now = env.mono;
    w.wall = env.wall;
    let take_was = w.take.is_some();
    let (mut fx, arm) = apply::apply(w, env.event);
    // A display picked by hand holds until the take ends (N56).
    if take_was && w.take.is_none() {
        w.display_pick.by_hand = false;
    }
    // Pin both's word lasts while the popover stays open (N74).
    if w.session.popover.is_none() {
        w.session.fields.library.pinned_note = None;
    }
    mark(w, &fx);
    let more = settle::settle(w);
    mark(w, &more);
    fx.extend(more);
    (fx, arm)
}

fn mark(w: &mut World, fx: &[Effect]) {
    for e in fx {
        if let Some(op) = e.op() {
            w.inflight.ops.insert(op, w.now.plus(op.deadline_ms()));
        }
        // The keep-awake assertion is held from the moment it's asked for (N41).
        if let Effect::Shell(effect::ShellCmd::PowerAssertion(on)) = e {
            w.mac.awake_held = *on;
        }
        // macOS's will-sleep is answered once: the hold ends with its answer (B92).
        if let Effect::Shell(effect::ShellCmd::AllowSleep) = e {
            w.mac.sleep_hold = None;
        }
        // A permanent copy is asked once (N63): its answer, or its failure, settles it.
        if let Effect::Editor(effect::EditCmd::Copy { rec, rev }) = e {
            w.library_ops.copies.insert(rec.clone(), world::CopyOwed { rev: *rev, asked: true });
        }
        // An editor's mark for the space's auto-cleanup (N74): again in five minutes.
        if let Effect::Library(effect::LibCmd::Editing(recs)) = e {
            for r in recs {
                w.library_ops.marked.insert(r.clone(), w.now);
            }
        }
    }
    // A closed editor's video is marked no more.
    let open: Vec<world::RecId> = w.session.fields.editors.values().map(|e| e.rec.clone()).collect();
    w.library_ops.marked.retain(|r, _| open.contains(r));
}
