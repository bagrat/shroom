//! Pin to keep (N74): the record's `keep`, set and taken back from a row's menu, at once and
//! with no sheet; several rows selected, it acts on all; a pinned row shows a small pin; the
//! suggestions and auto-cleanup never take a pinned video.

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use std::sync::Arc;

use shroom_core::effect::{Effect, LibCmd};
use shroom_core::event::{Event, LibraryNews, OsNews, Surface};
use shroom_core::scene::{Action, LibraryRowView, Scene};
use shroom_core::time::{Fact, Mono, Wall};
use shroom_core::world::*;
use support::*;

const NOW: i64 = 1_790_000_000_000;
const DAY: i64 = 86_400_000;

fn rec(id: &str) -> RecId {
    RecId(id.into())
}

fn remote(id: &str, title: &str, days_ago: i64, pinned: bool) -> LibraryRow {
    LibraryRow { rec: rec(id), title: title.into(), created: Wall(NOW - days_ago * DAY), length_ms: Some(272_000), live: true, owner: Some("m1".into()), poster: false, edited_ms: None, pinned }
}

fn library() -> World {
    let mut w = ready();
    w.session.windows.insert(WindowKind::Library, Door::asked(Mono(900)));
    w.library = Fact::known(
        Arc::new(vec![
            remote("onb", "Onboarding: your first week", 2, true),
            remote("r502", "Where the 502s come from", 5, false),
            remote("arch", "Architecture tour", 20, false),
        ]),
        Mono(950),
    );
    w.library_ops.refreshed = Some(Mono(950));
    w.suggestions = Fact::known(Suggestions { counting: true, space_bytes: 34_000_000_000, rows: vec![Suggestion { rec: rec("arch"), bytes: 1_100_000_000, plays: 1, quiet_days: 150, stopped_early: false }], ..Default::default() }, Mono(960));
    w
}

fn lib(r: &mut Run, action: Action) {
    r.send(Event::Act { surface: Surface::Library, action, rev: 0 });
}

fn row<'a>(s: &'a Scene, title: &str) -> &'a LibraryRowView {
    s.library.as_ref().unwrap().body.rows.iter().find(|r| r.title.as_deref() == Some(title)).unwrap_or_else(|| panic!("no row {title}"))
}

fn menu(s: &Scene, title: &str) -> Vec<String> {
    row(s, title).menu.iter().map(|b| b.label.clone()).collect()
}

#[test]
fn a_pinned_row_shows_its_pin_and_its_menu_says_unpin() {
    let r = Run::new(library());
    assert_eq!(row(&r.scene, "Onboarding: your first week").kept.as_deref(), Some("Pinned"), "the small pin, said to VoiceOver and on hover");
    assert_eq!(row(&r.scene, "Where the 502s come from").kept, None);
    assert_eq!(menu(&r.scene, "Where the 502s come from"), ["Copy link", "Copy thumbnail", "Open page", "Edit…", "Rename", "Pin", "Delete…"], "Pin after Rename");
    assert_eq!(menu(&r.scene, "Onboarding: your first week"), ["Copy link", "Copy thumbnail", "Open page", "Edit…", "Rename", "Unpin", "Delete…"]);
    assert!(!row(&r.scene, "Onboarding: your first week").pinned, "a pin isn't the recovered take's place at the top: the list stays newest first");
}

#[test]
fn pin_and_unpin_take_effect_at_once_and_a_failed_one_comes_back() {
    let mut r = Run::new(library());
    lib(&mut r, Action::PinRow(rec("r502"), true));
    assert!(r.has(|e| *e == Effect::Library(LibCmd::Pin(vec![rec("r502")], true))));
    assert!(r.scene.library.as_ref().unwrap().body.sheet.is_none(), "no sheet");
    assert_eq!(row(&r.scene, "Where the 502s come from").kept.as_deref(), Some("Pinned"), "it reads pinned while the write runs");
    r.send(Event::Library(LibraryNews::Pinned(vec![rec("r502")], true)));
    assert_eq!(row(&r.scene, "Where the 502s come from").kept.as_deref(), Some("Pinned"));
    // A refresh begun before the pin landed still lists it unpinned: the landed pin reads over it.
    r.send(Event::Library(LibraryNews::Rows { rows: library().library.get().unwrap().to_vec(), fresh: true }));
    assert_eq!(row(&r.scene, "Where the 502s come from").kept.as_deref(), Some("Pinned"));

    lib(&mut r, Action::PinRow(rec("onb"), false));
    assert!(r.has(|e| *e == Effect::Library(LibCmd::Pin(vec![rec("onb")], false))));
    assert_eq!(row(&r.scene, "Onboarding: your first week").kept, None);
    r.send(Event::Failed { op: Op::Pin, class: FailClass::Transport });
    assert_eq!(row(&r.scene, "Onboarding: your first week").kept.as_deref(), Some("Pinned"), "an unpin that didn't land: the pin is back (law 12's truth)");
}

#[test]
fn several_rows_selected_pin_says_pin_while_any_isnt_and_acts_on_all() {
    let mut r = Run::new(library());
    lib(&mut r, Action::SelectRow(rec("onb")));
    lib(&mut r, Action::ToggleRow(rec("r502")));
    assert_eq!(menu(&r.scene, "Onboarding: your first week")[5], "Pin", "one of them isn't pinned");
    let pin = row(&r.scene, "Onboarding: your first week").menu[5].action.clone();
    lib(&mut r, pin);
    assert!(r.has(|e| matches!(e, Effect::Library(LibCmd::Pin(recs, true)) if recs.len() == 2 && recs.contains(&rec("onb")) && recs.contains(&rec("r502")))));
    r.send(Event::Library(LibraryNews::Pinned(vec![rec("onb"), rec("r502")], true)));
    assert_eq!(menu(&r.scene, "Where the 502s come from")[5], "Unpin", "all of them pinned: Unpin");
}

#[test]
fn offline_pin_greys_as_rename_does() {
    let mut r = Run::new(library());
    r.send(Event::Os(OsNews::NetPath(false)));
    let b = row(&r.scene, "Where the 502s come from").menu.iter().find(|b| b.label == "Pin").cloned().unwrap();
    assert!(!b.enabled);
    lib(&mut r, Action::PinRow(rec("r502"), true));
    assert!(!r.has(|e| matches!(e, Effect::Library(LibCmd::Pin(..)))), "a greyed Pin does nothing");
}

#[test]
fn a_pinned_video_is_never_suggested() {
    let mut r = Run::new(library());
    assert!(r.scene.library.as_ref().unwrap().body.cleanup_line.is_some());
    lib(&mut r, Action::PinRow(rec("arch"), true));
    assert!(r.scene.library.as_ref().unwrap().body.cleanup_line.is_none(), "pinned from its menu, it leaves the suggestions too");
    let mut w = library();
    if let Fact::Known { v, .. } = &mut w.library {
        Arc::make_mut(v).iter_mut().find(|r| r.rec == rec("arch")).unwrap().pinned = true;
    }
    assert!(Run::new(w).scene.library.as_ref().unwrap().body.cleanup_line.is_none(), "a record pinned elsewhere, even if a stale read still suggests it");
}

#[test]
fn the_pin_op_has_a_deadline() {
    assert_eq!(Op::Pin.deadline_ms(), 60_000);
}
