//! N35's first open: “the window opens on the local cache at once; the bucket refresh runs
//! behind it.” An empty cache is not an empty space — “No videos yet” waits for the bucket
//! to say so (law 1), with the laws checked at every step.

#[path = "common/laws.rs"]
mod laws;
#[path = "common/support.rs"]
mod support;

use shroom_core::effect::LibCmd;
use shroom_core::event::{Event, LibraryNews};
use shroom_core::scene::Action;
use shroom_core::time::Wall;
use shroom_core::Effect;
use shroom_core::world::*;
use support::*;

fn row(id: &str, title: &str) -> LibraryRow {
    LibraryRow { rec: RecId(id.into()), title: title.into(), created: Wall(1_787_313_600_000), length_ms: Some(272_000), live: true, owner: Some("m1".into()), poster: false }
}

/// Opens the Library the way a person does: the mushroom, then config's footer.
fn opened() -> Run {
    let mut r = Run::new(ready());
    r.click();
    r.act(Action::OpenLibrary);
    assert!(r.scene.library.is_some(), "the Library opens");
    r
}

fn faces(r: &Run) -> Vec<&'static str> {
    r.scene.library.as_ref().map(|l| l.faces.clone()).unwrap_or_default()
}

#[test]
fn an_empty_cache_never_says_no_videos_before_the_bucket_answers() {
    let mut r = opened();
    assert!(r.has(|e| *e == Effect::Library(LibCmd::Refresh)), "the refresh runs behind the window");
    r.send(Event::Library(LibraryNews::Rows { rows: vec![], fresh: false }));
    let lib = r.scene.library.as_ref().expect("the Library");
    assert!(!faces(&r).contains(&"library-empty"), "{:?}", faces(&r));
    assert_eq!(lib.body.empty, None, "no line on an unknown space");
    assert!(lib.body.rows.is_empty() && lib.body.footer.is_none());
    // The bucket answers with videos: they slide in, and the empty line never showed.
    r.send(Event::Library(LibraryNews::Rows { rows: vec![row("r502", "Where the 502s come from")], fresh: true }));
    assert_eq!(r.scene.library.as_ref().unwrap().body.rows.len(), 1);
    assert!(!faces(&r).contains(&"library-empty"));
}

#[test]
fn a_space_the_bucket_says_is_empty_says_no_videos_yet() {
    let mut r = opened();
    r.send(Event::Library(LibraryNews::Rows { rows: vec![], fresh: false }));
    r.send(Event::Library(LibraryNews::Rows { rows: vec![], fresh: true }));
    assert!(faces(&r).contains(&"library-empty"), "{:?}", faces(&r));
    assert!(r.scene.library.as_ref().unwrap().body.empty.as_deref().is_some_and(|e| e.starts_with("No videos yet")));
}

#[test]
fn a_failed_refresh_leaves_an_unknown_space_unclaimed() {
    let mut r = opened();
    r.send(Event::Library(LibraryNews::Rows { rows: vec![], fresh: false }));
    r.send(Event::Failed { op: Op::LibRefresh, class: FailClass::Transport });
    assert!(!faces(&r).contains(&"library-empty"), "{:?}", faces(&r));
}

#[test]
fn this_macs_own_takes_show_while_the_bucket_is_unknown() {
    let mut w = ready();
    w.queue.push(Queued {
        dir: TakeDir("q1".into()),
        rec: RecId("u1".into()),
        title: "Untitled — Sep 30".into(),
        stopped: Wall(1_790_769_600_000),
        done: 1,
        total: 4,
        blocked: None,
        length_ms: Some(48_000),
    });
    let mut r = Run::new(w);
    r.click();
    r.act(Action::OpenLibrary);
    r.send(Event::Library(LibraryNews::Rows { rows: vec![], fresh: false }));
    let lib = r.scene.library.as_ref().expect("the Library");
    assert_eq!(lib.body.rows.len(), 1, "an uploading take is a row before any refresh");
    assert!(lib.body.empty.is_none());
}
