//! `shroom setup`'s words are the spec's (N89): each step's `say` is its state's agent line,
//! verbatim, with the spec's example link name and count; every step the app can answer
//! with has one.

#[cfg(test)]
use shroom_spec::Spec;

use crate::say::setup_says;

#[test]
fn every_say_is_its_states_agent_line() {
    let spec = Spec::load();
    let mut checked = 0;
    for id in crate::setup::STEPS {
        let state = spec.state(id).unwrap_or_else(|| panic!("{id}: not a state of the spec"));
        let line = state.agent.as_ref().unwrap_or_else(|| panic!("{id}: the spec has no agent line"));
        assert_eq!(setup_says(id, Some("v.myshroom.workers.dev"), Some(14)), line.text, "{id}: the command's words aren't the spec's");
        checked += 1;
    }
    assert!(checked >= 23);
}

#[test]
fn one_video_is_said_as_one() {
    assert_eq!(setup_says("space-found", Some("v.myshroom.workers.dev"), Some(1)), "Found your space — v.myshroom.workers.dev, 1 video. This Mac is joined.");
}
