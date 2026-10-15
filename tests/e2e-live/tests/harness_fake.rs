//! L1: the e2e harness's guards against the Cloudflare fake — the pin refuses any account
//! but the e2e one and the main account's name; a run's close takes its own space and nothing
//! else; a crashed run's leftovers go by the age in its id.

use std::collections::BTreeMap;
use std::time::{SystemTime, UNIX_EPOCH};

use shroom_cloud::{Cloud, Policy};
use shroom_cloudfake::{Fake, State};
use serde_json::json;

use shroom_e2e_live::{MAIN_NAME, RUN_FRESH_SECS, new_run_id, parse_rfc3339, pin, reap, rfc3339, run_token_name, stale_run_tokens, sweep_run};

const TOKEN: &str = "cfat_0123456789abcdefghijklmnopqrstuvwxyzABCD";

fn on(state: State) -> (Fake, Cloud, String) {
    let fake = Fake::start(state);
    let c = Cloud::new(TOKEN, Policy::Unfenced).with_endpoints(&fake.api(), &fake.r2());
    let acc = fake.with(|s| s.account_id());
    (fake, c, acc)
}

#[test]
fn the_pin_holds_on_the_e2e_account() {
    let (_fake, c, acc) = on(State::fresh());
    assert_eq!(pin(&c, &acc), Ok(()));
}

#[test]
fn the_pin_refuses_another_account() {
    let (_fake, c, _acc) = on(State::fresh());
    assert!(pin(&c, "acc0000000000000000000000000002").is_err());
}

#[test]
fn the_pin_refuses_a_token_that_reaches_more_than_one_account() {
    let mut state = State::fresh();
    state.accounts.push(("acc0000000000000000000000000002".into(), "Another".into()));
    let (_fake, c, acc) = on(state);
    assert!(pin(&c, &acc).is_err());
}

#[test]
fn the_pin_refuses_the_main_account() {
    let (_fake, c, acc) = on(State { subdomain: Some(MAIN_NAME.into()), ..State::fresh() });
    let refused = pin(&c, &acc).unwrap_err();
    assert!(refused.contains(MAIN_NAME), "{refused}");
}

const RUN: &str = "1792000000-ab12";
const OTHER: &str = "1792000300-cd34";

/// An account with two runs' spaces on it, and the product's names (a run never touches
/// what isn't a run's).
fn two_runs(crashed: &str) -> State {
    let mut state = State { subdomain: Some("shroom-e2e".into()), ..State::fresh() };
    let mut objects = BTreeMap::new();
    objects.insert("rec/a.ts".to_string(), (b"x".to_vec(), 1));
    for run in [RUN, OTHER, crashed] {
        state.scripts.insert(format!("v-{run}"), vec![]);
        state.buckets.insert(format!("shroom-space-{run}"), objects.clone());
    }
    state.scripts.insert("v".into(), vec![]);
    state.scripts.insert("shroom-probe".into(), vec![]);
    state.buckets.insert("shroom-space".into(), objects);
    state
}

#[test]
fn a_runs_close_takes_its_own_space_and_nothing_else() {
    let (fake, c, acc) = on(two_runs(OTHER));
    let token_id = fake.with(|s| s.token_id.clone());
    let gone = sweep_run(&c, &acc, &token_id, RUN).unwrap();
    assert_eq!(gone, [format!("bucket shroom-space-{RUN}"), format!("worker v-{RUN}")]);
    fake.with(|s| {
        assert_eq!(s.scripts.keys().collect::<Vec<_>>(), ["shroom-probe", "v", &format!("v-{OTHER}")]);
        assert_eq!(s.buckets.keys().collect::<Vec<_>>(), ["shroom-space", &format!("shroom-space-{OTHER}")]);
        assert_eq!(s.subdomain.as_deref(), Some("shroom-e2e"), "the name is never swept");
    });
}

#[test]
fn a_crashed_runs_leftovers_are_reaped_by_the_age_in_its_id() {
    let now = 1_792_000_000 + 600;
    let crashed = format!("{}-ef56", now - RUN_FRESH_SECS);
    let (fake, c, acc) = on(two_runs(&crashed));
    let token_id = fake.with(|s| s.token_id.clone());
    let gone = reap(&c, &acc, &token_id, now).unwrap();
    assert_eq!(gone, [format!("bucket shroom-space-{crashed}"), format!("worker v-{crashed}")], "runs under 45 min old are still going");
    fake.with(|s| {
        assert!(s.scripts.contains_key(&format!("v-{RUN}")) && s.scripts.contains_key(&format!("v-{OTHER}")));
        assert!(s.scripts.contains_key("v") && s.scripts.contains_key("shroom-probe"), "a name without a run id is never a run's to take");
        assert!(s.buckets.contains_key("shroom-space"));
    });
}

#[test]
fn a_run_is_named_by_when_it_began() {
    let before = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let run = new_run_id();
    let began = shroom_cloud::wire::run_started(&run).expect("a run id");
    assert!(began >= before && began <= before + 5, "{run}");
    assert_eq!(run_token_name(RUN), "shroom-e2e-run-1792000000-ab12");
}

#[test]
fn expiry_is_rfc3339() {
    assert_eq!(rfc3339(0), "1970-01-01T00:00:00Z");
    assert_eq!(rfc3339(1_792_000_000), "2026-10-14T17:46:40Z");
    assert_eq!(rfc3339(951_825_600), "2000-02-29T12:00:00Z");
}

#[test]
fn rfc3339_reads_back() {
    for secs in [0, 951_825_600, 1_792_000_000, 4_102_444_799] {
        assert_eq!(parse_rfc3339(&rfc3339(secs)), Some(secs));
    }
    assert_eq!(parse_rfc3339("2026-10-14T12:00:00.123Z"), Some(1_791_979_200));
    for bad in ["", "2026-10-14", "2026-13-01T00:00:00Z", "2026-10-14T12:00:00+01:00"] {
        assert_eq!(parse_rfc3339(bad), None, "{bad}");
    }
}

#[test]
fn a_crashed_runs_token_is_reaped_and_a_live_ones_never() {
    let now = 1_792_000_000;
    let tokens = json!({"result": [
        {"id": "acct", "name": "shroom-e2e-cloud", "issued_on": rfc3339(now - 90_000)},
        {"id": "live", "name": format!("shroom-e2e-run-{}-ab12", now - 600), "issued_on": rfc3339(now - 600)},
        {"id": "crashed", "name": format!("shroom-e2e-run-{}-cd34", now - RUN_FRESH_SECS), "issued_on": rfc3339(now - 90_000)},
        {"id": "old-by-date", "name": "shroom-e2e-run-2", "issued_on": rfc3339(now - RUN_FRESH_SECS - 1)},
        {"id": "old-by-name", "name": format!("shroom-e2e-run-{}", now - RUN_FRESH_SECS)},
        {"id": "young-by-name", "name": format!("shroom-e2e-run-{}", now - 120)},
        {"id": "unreadable", "name": "shroom-e2e-run-x"},
    ]});
    assert_eq!(stale_run_tokens(&tokens, now), ["crashed", "old-by-date", "old-by-name"], "a run's id says its age first; one whose age can't be read is left be");
    assert!(stale_run_tokens(&json!({"result": [{"id": "acct", "name": "shroom-e2e-ci"}]}), now).is_empty(), "the account token is never a run's");
}
