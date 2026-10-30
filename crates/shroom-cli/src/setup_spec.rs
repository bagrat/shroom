//! `shroom setup`'s words are the spec's (N88): each step's `say` is its state's agent line,
//! verbatim, with the spec's example link name and count; every step the app can answer
//! with has one.

use shroom_spec::Spec;

use crate::setup::{Facts, says};

#[test]
fn every_say_is_its_states_agent_line() {
    let spec = Spec::load();
    let facts = Facts { host: Some("v.myshroom.workers.dev".into()), videos: Some(14) };
    let mut checked = 0;
    for id in crate::setup::STEPS {
        let state = spec.state(id).unwrap_or_else(|| panic!("{id}: not a state of the spec"));
        let line = state.agent.as_ref().unwrap_or_else(|| panic!("{id}: the spec has no agent line"));
        assert_eq!(says(id, &facts), line.text, "{id}: the command's words aren't the spec's");
        checked += 1;
    }
    assert!(checked >= 23);
}

#[test]
fn one_video_is_said_as_one() {
    let facts = Facts { host: Some("v.myshroom.workers.dev".into()), videos: Some(1) };
    assert_eq!(says("space-found", &facts), "Found your space — v.myshroom.workers.dev, 1 video. This Mac is joined.");
}
