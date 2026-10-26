//! L1: the wizard's probes, claim and wiring against the in-process Cloudflare fake —
//! the paths that must never run on the real account (claiming a name, a taken name,
//! the gates on a fresh account, a space that's already there).

use shroom_cloud::sweep::{self, Claim, Gate, Gates, Verdict, WorkerFact};
use shroom_cloud::wire::{self, WireSpec};
use shroom_cloud::{Cloud, Policy, owner};
use shroom_cloudfake::{Fake, State};

const TOKEN: &str = "cfat_0123456789abcdefghijklmnopqrstuvwxyzABCD";

fn cloud(fake: &Fake) -> Cloud {
    Cloud::new(TOKEN, Policy::Unfenced).with_endpoints(&fake.api(), &fake.r2())
}

fn spec<'a>(account: &'a str, token_id: &'a str, label: &'a str) -> WireSpec<'a> {
    WireSpec {
        account_id: account, token_id, label, bucket: wire::BUCKET,
        worker_module: "export default {}", compatibility_date: "2026-09-01",
        assets: &[(".shroom/player/hls.min.js", b"/*hls*/", "text/javascript")],
    }
}

#[test]
fn a_fresh_account_from_paste_to_a_wired_space() {
    let fake = Fake::start(State::fresh());
    let c = cloud(&fake);
    assert_eq!(sweep::shape(TOKEN), sweep::Shape::Account);
    let Verdict::Valid { token_id, accounts } = sweep::verify(&c).unwrap() else { panic!() };
    let acc = accounts[0].id.clone();
    assert!(sweep::powers(&c, &acc).unwrap().both());
    assert_eq!(sweep::gates(&c, &acc, "shroom-probe").unwrap(), Gates { email: false, card: false });
    assert!(fake.with(|s| !s.scripts.contains_key("shroom-probe")), "the email probe cleans up");
    assert!(fake.with(|s| !s.calls.iter().any(|c| c.starts_with("FORCED "))), "without force: it made the probe, nothing hangs on it");

    let before = sweep::survey_space(&c, &acc, &token_id).unwrap();
    assert_eq!((before.subdomain.clone(), before.label.as_deref(), before.foreign_v, before.found), (None, Some("v"), false, None));

    // The name step: taken, then claimed.
    fake.with(|s| s.taken.insert("myshroom".into()));
    assert_eq!(sweep::claim(&c, &acc, "myshroom").unwrap(), Claim::Taken);
    assert_eq!(sweep::claim(&c, &acc, "bagrat").unwrap(), Claim::Claimed);

    let wired = wire::wire(&c, &spec(&acc, &token_id, "v")).unwrap();
    assert_eq!(wired.host, "v.bagrat.workers.dev");
    fake.with(|s| {
        assert!(s.routes.contains("v"));
        assert_eq!(s.scripts["v"][0]["bucket_name"], "shroom-space");
        assert!(s.object("shroom-space", wire::MANIFEST).is_some());
        assert!(s.object("shroom-space", owner::COOKIE_KEY).is_some());
        assert!(s.object("shroom-space", ".shroom/self-check").is_none(), "the self-check cleans up");
    });

    // Re-entry finds the whole space; wiring again changes nothing that matters.
    let secret = fake.with(|s| s.object("shroom-space", owner::COOKIE_KEY));
    fake.with(|s| s.put_object("shroom-space", "meta/AbCdEfGhIjKlMnOp.json", b"{}"));
    let after = sweep::survey_space(&c, &acc, &token_id).unwrap();
    assert_eq!((after.worker, after.found), (WorkerFact::Ours, Some(1)));
    wire::wire(&c, &spec(&acc, &token_id, "v")).unwrap();
    assert_eq!(fake.with(|s| s.object("shroom-space", owner::COOKIE_KEY)), secret);
}

#[test]
fn a_wiring_cut_short_never_reads_as_a_found_space() {
    let fake = Fake::start(State { subdomain: Some("bagrat".into()), ..State::fresh() });
    let c = cloud(&fake);
    let acc = fake.with(|s| s.account_id());
    let token_id = fake.with(|s| s.token_id.clone());
    // The route can't switch on (the Worker deploy fails): wiring stops partway.
    fake.with(|s| s.powers.0 = false);
    assert!(wire::wire(&c, &spec(&acc, &token_id, "v")).is_err());
    fake.with(|s| s.powers.0 = true);
    let survey = sweep::survey_space(&c, &acc, &token_id).unwrap();
    assert_eq!((survey.bucket, survey.found), (true, None));
}

#[test]
fn the_gates_surface_email_first_then_card() {
    let fake = Fake::start(State { email_verified: false, r2_enabled: false, ..State::fresh() });
    let c = cloud(&fake);
    let acc = fake.with(|s| s.account_id());
    assert!(sweep::powers(&c, &acc).unwrap().both(), "R2 switched off is a gate, not a missing power");
    assert_eq!(sweep::gates(&c, &acc, "shroom-probe").unwrap(), Gates { email: true, card: true });
    fake.with(|s| s.email_verified = true);
    assert_eq!(sweep::gates(&c, &acc, "shroom-probe").unwrap(), Gates { email: false, card: true });
    fake.with(|s| s.r2_enabled = true);
    assert_eq!(sweep::gates(&c, &acc, "shroom-probe").unwrap(), Gates { email: false, card: false });
    // Wiring keeps the same classification as a fallback.
    fake.with(|s| s.email_verified = false);
    let e = c.put_worker(&acc, "v", "w.js", "x", &[], "2026-09-01").unwrap_err();
    assert_eq!(sweep::gate_of(&e), Some(Gate::Email));
}

/// The email probe writes only a Worker it made: one already under the probe's name stays
/// exactly as it is, through a gates pass and the gate's own poll, and the probe goes under
/// a fresh name of its own (`<probe>-<8 hex>`), removed again without force.
#[test]
fn a_worker_already_named_as_the_probe_survives_a_gates_pass() {
    let fake = Fake::start(State::fresh());
    let c = cloud(&fake);
    let acc = fake.with(|s| s.account_id());
    let theirs = vec![serde_json::json!({"type": "plain_text", "name": "KEEP"})];
    fake.with(|s| s.scripts.insert("shroom-probe".into(), theirs.clone()));
    assert_eq!(sweep::gates(&c, &acc, "shroom-probe").unwrap(), Gates { email: false, card: false });
    fake.with(|s| {
        assert_eq!(s.scripts.get("shroom-probe"), Some(&theirs), "untouched");
        let writes = |name: &str| s.calls.iter().filter(|c| !c.starts_with("GET ") && c.ends_with(&format!("/workers/scripts/{name}"))).count();
        assert_eq!(writes("shroom-probe"), 0, "never written or deleted: {:?}", s.calls);
        let fresh: Vec<&str> = s.calls.iter().filter(|c| c.starts_with("PUT ")).filter_map(|c| c.rsplit_once("/workers/scripts/").map(|(_, n)| n)).collect();
        assert_eq!(fresh.len(), 1, "{:?}", s.calls);
        let suffix = fresh[0].strip_prefix("shroom-probe-").unwrap_or_else(|| panic!("{}", fresh[0]));
        assert!(suffix.len() == 8 && suffix.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase()), "{}", fresh[0]);
        assert_eq!(writes(fresh[0]), 2, "made, then removed");
        assert!(!s.scripts.contains_key(fresh[0]), "the probe it made is gone again");
        assert!(!s.calls.iter().any(|c| c.starts_with("FORCED ")), "removed without force");
    });
    // The gate still reads while the name is someone else's.
    fake.with(|s| s.email_verified = false);
    assert!(sweep::email_gate(&c, &acc, "shroom-probe").unwrap());
    fake.with(|s| assert_eq!(s.scripts.get("shroom-probe"), Some(&theirs)));
    fake.with(|s| s.email_verified = true);
    assert!(!sweep::email_gate(&c, &acc, "shroom-probe").unwrap());
    fake.with(|s| assert_eq!((s.scripts.get("shroom-probe"), s.scripts.len()), (Some(&theirs), 1)));
}

#[test]
fn a_token_cloudflare_rejects_and_one_missing_a_power() {
    let fake = Fake::start(State { token_active: false, ..State::fresh() });
    assert_eq!(sweep::verify(&cloud(&fake)).unwrap(), Verdict::Rejected);
    let fake = Fake::start(State { powers: (true, false), ..State::fresh() });
    let acc = fake.with(|s| s.account_id());
    assert_eq!(sweep::powers(&cloud(&fake), &acc).unwrap(), sweep::Powers { workers: true, r2: false });
}

#[test]
fn several_accounts_are_listed_for_the_pick() {
    let fake = Fake::start(State {
        accounts: vec![("acc1".into(), "Personal".into()), ("acc2".into(), "Work".into())],
        ..State::fresh()
    });
    let Verdict::Valid { accounts, .. } = sweep::verify(&cloud(&fake)).unwrap() else { panic!() };
    assert_eq!(accounts.iter().map(|a| a.name.as_str()).collect::<Vec<_>>(), ["Personal", "Work"]);
}

#[test]
fn someone_elses_v_is_left_alone_and_the_page_takes_shroom() {
    let mut state = State { subdomain: Some("myshroom".into()), ..State::fresh() };
    state.scripts.insert("v".into(), vec![serde_json::json!({"type": "r2_bucket", "name": "B", "bucket_name": "shroom"})]);
    let fake = Fake::start(state);
    let c = cloud(&fake);
    let acc = fake.with(|s| s.account_id());
    let token_id = fake.with(|s| s.token_id.clone());
    let survey = sweep::survey_space(&c, &acc, &token_id).unwrap();
    assert_eq!((survey.foreign_v, survey.label.as_deref(), survey.worker), (true, Some("shroom"), WorkerFact::Absent));
    let wired = wire::wire(&c, &spec(&acc, &token_id, "shroom")).unwrap();
    assert_eq!(wired.host, "shroom.myshroom.workers.dev");
    fake.with(|s| assert_eq!(s.scripts["v"][0]["bucket_name"], "shroom", "the foreign v is untouched"));
}

// ── The page follows the app, and wiring tells the truth (whole-spec audit #1, #4, #5,
// #6, #21, #23, #25, #27; DECISIONS B50) ──

use std::time::Duration;

use shroom_cloud::member::Space;
use shroom_cloud::wire::{PageCheck, Refresh, WireError};

const BUNDLED: &str = "0.4.0";

fn module(version: &str) -> String {
    format!("export const VERSION = \"{version}\";\nexport default {{}}\n")
}

fn page_spec<'a>(account: &'a str, token_id: &'a str, label: &'a str, module: &'a str) -> WireSpec<'a> {
    WireSpec { worker_module: module, ..spec(account, token_id, label) }
}

fn check<'a>(at: &'a dyn Fn(&str) -> Space, within_ms: u64) -> PageCheck<'a> {
    PageCheck { at, version: BUNDLED, every: Duration::from_millis(10), within: Duration::from_millis(within_ms) }
}

fn script_puts(fake: &Fake, label: &str) -> usize {
    fake.with(|s| s.calls.iter().filter(|c| c.starts_with("PUT /client/v4/accounts/") && c.ends_with(&format!("/workers/scripts/{label}"))).count())
}

#[test]
fn wiring_waits_for_the_page_to_answer_before_it_says_done() {
    let fake = Fake::start(State { subdomain: Some("bagrat".into()), propagation: 4, ..State::fresh() });
    let c = cloud(&fake);
    let (acc, tok) = fake.with(|s| (s.account_id(), s.token_id.clone()));
    let m = module(BUNDLED);
    let at = |h: &str| Space::at(h, &fake.page(h));
    let wired = wire::wire_with(&c, &page_spec(&acc, &tok, "v", &m), Some(&check(&at, 5_000))).unwrap();
    assert!(wired.deployed);
    assert!(fake.with(|s| s.pings["v"]) >= 5, "it asked until the page answered");
    assert!(fake.with(|s| s.object("shroom-space", wire::MANIFEST).is_some()));
}

/// The setup shows wiring's parts as they begin (N74): in order, the name's before its
/// wait, and a retry says them again from the start.
#[test]
fn wiring_says_its_parts_in_order() {
    let fake = Fake::start(State { subdomain: Some("bagrat".into()), propagation: 4, ..State::fresh() });
    let c = cloud(&fake);
    let (acc, tok) = fake.with(|s| (s.account_id(), s.token_id.clone()));
    let m = module(BUNDLED);
    let at = |h: &str| Space::at(h, &fake.page(h));
    let said = std::cell::RefCell::new(Vec::new());
    let pings_at_name = std::cell::Cell::new(None);
    wire::wire_parts(&c, &page_spec(&acc, &tok, "v", &m), Some(&check(&at, 5_000)), &|p| {
        said.borrow_mut().push(p);
        if p == wire::WirePart::Name {
            pings_at_name.set(Some(fake.with(|s| s.pings.get("v").copied().unwrap_or(0))));
        }
    })
    .unwrap();
    use wire::WirePart::*;
    assert_eq!(*said.borrow(), [Storage, Page, Name, Check]);
    assert_eq!(pings_at_name.get(), Some(0), "the name's part begins before its wait");
}

#[test]
fn a_page_that_never_answers_is_a_retry_never_done() {
    let fake = Fake::start(State { subdomain: Some("bagrat".into()), propagation: 10_000, ..State::fresh() });
    let c = cloud(&fake);
    let (acc, tok) = fake.with(|s| (s.account_id(), s.token_id.clone()));
    let m = module(BUNDLED);
    let at = |h: &str| Space::at(h, &fake.page(h));
    let err = wire::wire_with(&c, &page_spec(&acc, &tok, "v", &m), Some(&check(&at, 60))).unwrap_err();
    assert!(matches!(err, WireError::NotAnswering), "{err}");
    // Nothing reads as a whole space yet: the manifest comes after the page answers.
    assert!(fake.with(|s| s.object("shroom-space", wire::MANIFEST).is_none()));
}

#[test]
fn an_older_page_is_refreshed_the_same_untouched_a_newer_never_lowered() {
    for (live, deployed) in [("0.3.9", true), (BUNDLED, false), ("0.10.0", false), ("1.0.0-beta.1", false)] {
        let fake = Fake::start(State { subdomain: Some("bagrat".into()), ..State::fresh() });
        let c = cloud(&fake);
        let (acc, tok) = fake.with(|s| (s.account_id(), s.token_id.clone()));
        let old = module(live);
        wire::wire(&c, &page_spec(&acc, &tok, "v", &old)).unwrap();
        let before = script_puts(&fake, "v");
        let ours = module(BUNDLED);
        let at = |h: &str| Space::at(h, &fake.page(h));
        let wired = wire::wire_with(&c, &page_spec(&acc, &tok, "v", &ours), Some(&check(&at, 1_000))).unwrap();
        assert_eq!(wired.deployed, deployed, "live {live}");
        assert_eq!(script_puts(&fake, "v") - before, usize::from(deployed), "live {live}");
        let now = fake.with(|s| s.versions["v"].clone());
        assert_eq!(now, if deployed { BUNDLED.to_string() } else { live.to_string() }, "never an older version over a newer one");
        // Either way the parent is this token's.
        assert!(deployed || fake.with(|s| s.secrets.get("v").is_some_and(|n| n.contains(wire::PARENT_BINDING))), "live {live}");
    }
    assert!(wire::may_deploy_over("0.9.9", "0.10.0") && !wire::may_deploy_over("0.10.0", "0.9.9") && !wire::may_deploy_over("garbage", "0.1.0"));
    // Early builds (B220): above the release before, below their own, each above the last.
    let up = [("0.1.24", "0.1.25-early.1"), ("0.1.25-early.1", "0.1.25-early.2"), ("0.1.25-early.2", "0.1.25-early.10"), ("0.1.25-early.10", "0.1.25"), ("0.1.25", "0.1.26-early.1")];
    for (older, newer) in up {
        assert!(wire::may_deploy_over(older, newer), "{newer} goes over {older}");
        assert!(!wire::may_deploy_over(newer, older), "{older} never goes over {newer}");
    }
    assert!(!wire::may_deploy_over("0.1.25-early.1", "0.1.25-early.1"), "the same page stays");
}

#[test]
fn the_refresh_follows_the_app_and_touches_nothing_else() {
    let fake = Fake::start(State { subdomain: Some("bagrat".into()), ..State::fresh() });
    let c = cloud(&fake);
    let (acc, tok) = fake.with(|s| (s.account_id(), s.token_id.clone()));
    let old = module("0.1.0");
    wire::wire(&c, &page_spec(&acc, &tok, "v", &old)).unwrap();
    // A space wired before Region 6: no roster yet.
    fake.with(|s| s.buckets.get_mut("shroom-space").unwrap().remove(shroom_cloud::registry::ROSTER));
    let ours = module(BUNDLED);
    let at = |h: &str| Space::at(h, &fake.page(h));
    let host = "v.bagrat.workers.dev";
    let spec = page_spec(&acc, &tok, "v", &ours);
    assert_eq!(wire::refresh_page(&c, &spec, &check(&at, 1_000), host).unwrap(), Refresh::Deployed { was: "0.1.0".into() });
    assert!(fake.with(|s| s.object("shroom-space", shroom_cloud::registry::ROSTER).is_some()), "the roster is seeded before the new page");
    assert_eq!(wire::refresh_page(&c, &spec, &check(&at, 1_000), host).unwrap(), Refresh::Current { version: BUNDLED.into() });
    // A page that doesn't answer is the repair mood's, never a refresh's: no storage made.
    fake.with(|s| s.control("drop-bucket shroom-space").unwrap());
    fake.with(|s| s.routes.clear());
    let puts = script_puts(&fake, "v");
    assert_eq!(wire::refresh_page(&c, &spec, &check(&at, 1_000), host).unwrap(), Refresh::NotAnswering);
    assert_eq!(script_puts(&fake, "v"), puts);
    assert!(fake.with(|s| !s.buckets.contains_key("shroom-space")), "a refresh never sets up storage");
}

#[test]
fn wiring_never_writes_over_a_worker_that_isnt_shrooms() {
    let fake = Fake::start(State { subdomain: Some("bagrat".into()), ..State::fresh() });
    fake.with(|s| s.scripts.insert("v".into(), vec![serde_json::json!({"type": "plain_text", "name": "X"})]));
    let c = cloud(&fake);
    let (acc, tok) = fake.with(|s| (s.account_id(), s.token_id.clone()));
    let m = module(BUNDLED);
    let at = |h: &str| Space::at(h, &fake.page(h));
    let err = wire::wire_with(&c, &page_spec(&acc, &tok, "v", &m), Some(&check(&at, 100))).unwrap_err();
    assert!(matches!(err, WireError::Foreign), "{err}");
    assert_eq!(script_puts(&fake, "v"), 0);
    assert!(matches!(wire::wire(&c, &spec(&acc, &tok, "v")), Err(WireError::Foreign)), "the plain wire refuses too");
}

#[test]
fn a_provisioned_space_is_probed_under_its_own_label() {
    // (a) The space is `v`; someone else's `v` appears: the space's page reads foreign —
    // never “absent under shroom”, which would rewire `v` over a stranger's Worker.
    let fake = Fake::start(State { subdomain: Some("bagrat".into()), ..State::fresh() });
    let c = cloud(&fake);
    let (acc, tok) = fake.with(|s| (s.account_id(), s.token_id.clone()));
    wire::wire(&c, &spec(&acc, &tok, "v")).unwrap();
    fake.with(|s| s.scripts.insert("v".into(), vec![serde_json::json!({"type": "r2_bucket", "name": "B", "bucket_name": "someone-elses"})]));
    let own = sweep::survey_own(&c, &acc, &tok, "v", wire::BUCKET).unwrap();
    assert_eq!((own.worker, own.label.as_deref()), (WorkerFact::Foreign, Some("v")));
    let chain = sweep::survey_space(&c, &acc, &tok).unwrap();
    assert_eq!((chain.worker, chain.label.as_deref()), (WorkerFact::Absent, Some("shroom")), "the chain would have said a fallback is free");

    // (b) The space is `shroom` (set up beside a foreign `v`); the foreign `v` is deleted:
    // the space stays sound under its own label.
    let fake = Fake::start(State { subdomain: Some("bagrat".into()), ..State::fresh() });
    let c = cloud(&fake);
    fake.with(|s| s.scripts.insert("v".into(), vec![serde_json::json!({"type": "plain_text", "name": "X"})]));
    wire::wire(&c, &spec(&acc, &tok, "shroom")).unwrap();
    fake.with(|s| s.scripts.remove("v"));
    let own = sweep::survey_own(&c, &acc, &tok, "shroom", wire::BUCKET).unwrap();
    assert_eq!((own.worker, own.label.as_deref(), own.foreign_v), (WorkerFact::Ours, Some("shroom"), true));
    assert_eq!(own.found, Some(0));
}

#[test]
fn space_found_counts_videos_never_tombstones() {
    let fake = Fake::start(State { subdomain: Some("bagrat".into()), ..State::fresh() });
    let c = cloud(&fake);
    let (acc, tok) = fake.with(|s| (s.account_id(), s.token_id.clone()));
    wire::wire(&c, &spec(&acc, &tok, "v")).unwrap();
    fake.with(|s| {
        s.put_object("shroom-space", "meta/AAAAAAAAAAAAAAAA.json", br#"{"state":"live"}"#);
        s.put_object("shroom-space", "meta/BBBBBBBBBBBBBBBB.json", br#"{"state":"uploading"}"#);
        s.put_object("shroom-space", "meta/CCCCCCCCCCCCCCCC.json", br#"{"state":"deleted"}"#);
    });
    assert_eq!(sweep::survey_space(&c, &acc, &tok).unwrap().found, Some(2));
}
