//! Law 1 by construction: nothing the World holds can name a screen — no state ids, no
//! Scene — and the session stays small enough to read (ARCHITECTURE §3.1).

#[test]
fn the_world_cannot_name_a_screen() {
    let world = include_str!("../src/world.rs");
    for forbidden in ["StateId", "Scene", "&'static str"] {
        let hits: Vec<_> = world.lines().filter(|l| !l.trim_start().starts_with("//") && l.contains(forbidden)).collect();
        assert!(hits.is_empty(), "world.rs mentions {forbidden}: {hits:?}");
    }
}

#[test]
fn the_session_holds_at_most_twelve_intents() {
    let world = include_str!("../src/world.rs");
    let body = world.split("pub struct Session {").nth(1).unwrap().split("\n}").next().unwrap();
    let fields = body.lines().filter(|l| l.trim_start().starts_with("pub ")).count();
    assert!(fields <= 12, "Session has {fields} fields");
}
