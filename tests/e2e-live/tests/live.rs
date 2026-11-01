//! L2 on the dedicated test account (TESTING-PLAN D3), Region 2's states live. Each test
//! is one pinned run in its own space (`v-<run>` over `shroom-space-<run>`), driven by a
//! per-run token with the product's powers.
//!
//! Runs only with SHROOM_E2E=1. `claim_the_name_once` also needs SHROOM_E2E_CLAIM=1 —
//! Bagrat's go: it ends the account's no-name state for good. The tests that need the
//! name skip until it's claimed.

use shroom_cloud::sweep::{self, Claim, Gates, Verdict};
use shroom_cloud::wire::{self, WireSpec};
use shroom_cloud::CloudError;
use shroom_e2e_live::{MAIN_NAME, NAME, Run, config};

fn run() -> Option<Run> {
    let cfg = config()?;
    Some(Run::start(&cfg).unwrap_or_else(|e| panic!("the run didn't start: {e}")))
}

/// The run's account has its workers.dev name, or the test says why it skipped.
fn named(run: &Run) -> bool {
    let name = run.product.subdomain(&run.account_id).expect("the workers.dev name");
    if name.as_deref() != Some(NAME) {
        eprintln!("skipped: the account's workers.dev name isn't claimed yet (claim_the_name_once)");
    }
    name.as_deref() == Some(NAME)
}

#[test]
fn the_run_token_has_exactly_the_products_powers() {
    let Some(run) = run() else { return eprintln!("skipped: set SHROOM_E2E=1") };
    let Verdict::Valid { token_id, accounts } = sweep::verify(&run.product).expect("verify") else { panic!("the run token verifies") };
    assert_eq!(token_id, run.token_id);
    assert_eq!(accounts.iter().map(|a| a.id.as_str()).collect::<Vec<_>>(), [run.account_id.as_str()]);
    assert!(sweep::powers(&run.product, &run.account_id).expect("powers").both());
    // The account token's extra power stays with the harness.
    match run.product.raw("GET", &format!("/accounts/{}/tokens", run.account_id), None) {
        Err(CloudError::Api { status: 401 | 403, .. }) => {}
        other => panic!("the run token can read the account's tokens: {:?}", other.map(|_| "listed")),
    }
}

#[test]
fn no_gate_stands() {
    let Some(run) = run() else { return eprintln!("skipped: set SHROOM_E2E=1") };
    let gates = sweep::gates(&run.product, &run.account_id, "shroom-probe").expect("gates");
    assert_eq!(gates, Gates { email: false, card: false });
}

/// `name`: the account has no workers.dev name, and the wizard's claim gives it one.
/// One-shot (D3) — afterwards the no-name state is gone from this account.
#[test]
fn claim_the_name_once() {
    let Some(run) = run() else { return eprintln!("skipped: set SHROOM_E2E=1") };
    if std::env::var("SHROOM_E2E_CLAIM").ok().as_deref() != Some("1") {
        return eprintln!("skipped: claiming the name is one-shot — set SHROOM_E2E_CLAIM=1 with Bagrat's go");
    }
    let before = sweep::survey(&run.product, &run.account_id, &run.token_id, &run.space.labels(), &run.space.bucket).expect("survey");
    assert_eq!(before.subdomain, None, "the account already has a name");
    assert_eq!(sweep::claim(&run.product, &run.account_id, NAME).expect("claim"), Claim::Claimed);
    assert_eq!(run.product.subdomain(&run.account_id).expect("the name").as_deref(), Some(NAME));
}

/// A named account keeps its name: Cloudflare refuses a second claim with `10036`
/// whatever it asks for — even a name someone else holds — so `name-taken` (`10031`)
/// only answers an account without a name, and stays on the fakes (D3).
#[test]
fn a_named_account_keeps_its_name() {
    let Some(run) = run() else { return eprintln!("skipped: set SHROOM_E2E=1") };
    if !named(&run) {
        return;
    }
    match sweep::claim(&run.product, &run.account_id, MAIN_NAME) {
        Err(e) if e.has_code(10036) => {}
        other => panic!("a second claim answered {other:?}"),
    }
    assert_eq!(run.product.subdomain(&run.account_id).expect("the name").as_deref(), Some(NAME));
}

/// Wiring from nothing (the run's own space), then `space-found`: the survey sees the whole
/// space.
#[test]
fn wiring_from_nothing_is_found_after() {
    let Some(run) = run() else { return eprintln!("skipped: set SHROOM_E2E=1") };
    if !named(&run) {
        return;
    }
    let (labels, bucket) = (run.space.labels(), run.space.bucket.as_str());
    let before = sweep::survey(&run.product, &run.account_id, &run.token_id, &labels, bucket).expect("survey");
    assert_eq!((before.bucket, before.found), (false, None), "a fresh run's space is already there");
    let label = before.label.expect("a fresh run's label is free");
    let wired = wire::wire(&run.product, &WireSpec {
        account_id: &run.account_id,
        token_id: &run.token_id,
        label: &label,
        bucket,
        worker_module: "export default { fetch() { return new Response('shroom e2e'); } }",
        compatibility_date: "2026-09-01",
        assets: &[],
    })
    .expect("wiring");
    assert_eq!(wired.subdomain, NAME);
    let after = sweep::survey(&run.product, &run.account_id, &run.token_id, &labels, bucket).expect("survey");
    assert!(after.bucket, "{after:?}");
    assert!(after.found.is_some(), "{after:?}");
}

/// The AI switch's parts, timed (N66, N76): the run's space wired with the real page, then
/// AI turned on and off three times through the same calls `set_ai` makes — the page's
/// version asked, the token proved, the page read as ours, the script put up again, its
/// schedule — each part's time printed, and how long after it the page says so. What the
/// switch's progress names comes from these.
#[test]
fn the_ai_switch_parts_are_timed() {
    use shroom_cloud::member::{Ping, Space};
    use shroom_cloud::Binding;
    use std::time::{Duration, Instant};
    let Some(run) = run() else { return eprintln!("skipped: set SHROOM_E2E=1") };
    if !named(&run) {
        return;
    }
    const MODULE: &str = include_str!("../../../worker/src/worker.js");
    let (labels, bucket) = (run.space.labels(), run.space.bucket.as_str());
    let before = sweep::survey(&run.product, &run.account_id, &run.token_id, &labels, bucket).expect("survey");
    let label = before.label.expect("a fresh run's label is free");
    let spec = WireSpec { account_id: &run.account_id, token_id: &run.token_id, label: &label, bucket, worker_module: MODULE, compatibility_date: "2026-09-01", assets: &[] };
    let wired = wire::wire(&run.product, &spec).expect("wiring");
    let page = Space::new(&wired.host);
    let ping = |want: Option<bool>, within: Duration| {
        let t = Instant::now();
        loop {
            if let Ping::Shroom { ai, .. } = page.ping() {
                if want.is_none_or(|w| w == ai) {
                    return t.elapsed();
                }
            }
            assert!(t.elapsed() < within, "the page didn't answer as wanted ({want:?}) in {within:?}");
            std::thread::sleep(Duration::from_millis(250));
        }
    };
    eprintln!("ai-switch: the page answered after wiring in {:?}", ping(None, Duration::from_secs(180)));
    let ms = |d: Duration| d.as_millis();
    for round in 1..=3 {
        for on in [true, false] {
            let all = Instant::now();
            let t = Instant::now();
            let _ = page.ping();
            let version = t.elapsed();
            let t = Instant::now();
            assert!(matches!(sweep::verify(&run.product).expect("verify"), Verdict::Valid { .. }));
            assert!(sweep::powers(&run.product, &run.account_id).expect("powers").both());
            let prove = t.elapsed();
            let t = Instant::now();
            assert_eq!(sweep::worker_fact(&run.product, &run.account_id, &label, bucket).expect("worker fact"), sweep::WorkerFact::Ours);
            let fact = t.elapsed();
            let mut bindings = vec![
                Binding::R2Bucket { name: "BUCKET".into(), bucket: bucket.into() },
                Binding::SecretText { name: wire::PARENT_BINDING.into(), text: wire::parent_text(&run.product, &run.account_id, bucket, &run.token_id) },
            ];
            if on {
                bindings.push(Binding::Ai { name: "AI".into() });
            }
            let t = Instant::now();
            run.product.put_worker(&run.account_id, &label, "worker.js", MODULE, &bindings, "2026-09-01").expect("the script");
            let script = t.elapsed();
            let t = Instant::now();
            run.product.put_schedules(&run.account_id, &label, if on { &[wire::AI_SCHEDULE] } else { &[] }).expect("the schedule");
            let schedule = t.elapsed();
            let calls = all.elapsed();
            let says = ping(Some(on), Duration::from_secs(120));
            eprintln!(
                "ai-switch: round {round} {}: version {}ms, prove {}ms, fact {}ms, script {}ms, schedule {}ms — calls {}ms; the page says so {}ms later",
                if on { "on " } else { "off" },
                ms(version), ms(prove), ms(fact), ms(script), ms(schedule), ms(calls), ms(says)
            );
        }
    }
}
