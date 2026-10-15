//! L1: an e2e run's own space on the shared test account (TESTING-PLAN D3, DECISIONS
//! B131). The product's names are `v` (or its fallbacks) over `shroom-space`, always; a
//! run's are `v-<run>` over `shroom-space-<run>` — the label and the bucket as a pair, so
//! two runs side by side never share a page, a video or a delete.

use shroom_cloud::sweep::{self, WorkerFact};
use shroom_cloud::wire::{self, SpaceNames, WireSpec};
use shroom_cloud::{Cloud, Policy};
use shroom_cloudfake::{Fake, State};

const TOKEN: &str = "cfat_0123456789abcdefghijklmnopqrstuvwxyzABCD";
const RUN: &str = "1792000000-ab12";

#[test]
fn the_products_names_are_the_defaults() {
    let p = SpaceNames::product();
    assert_eq!(p.labels(), ["v", "shroom", "shroom-v"]);
    assert_eq!(p.labels(), sweep::LABELS);
    assert_eq!(p.bucket, wire::BUCKET);
}

#[test]
fn a_runs_names_move_together() {
    let r = SpaceNames::of_run(RUN).expect("a run id");
    assert_eq!(r.labels(), ["v-1792000000-ab12"], "no fallback: the run's label is its own");
    assert_eq!(r.bucket, "shroom-space-1792000000-ab12");
    assert_eq!(wire::run_started(RUN), Some(1_792_000_000));
    for bad in ["", "v", "1792000000", "1792000000-", "1792000000-AB12", "1792000000-ab1", "1792000000-ab123", "179200000-ab12", "x792000000-ab12", "../00000000-ab12", "1792000000-ab12\n"] {
        assert_eq!(SpaceNames::of_run(bad), None, "{bad:?}");
        assert_eq!(wire::run_started(bad), None, "{bad:?}");
    }
}

#[test]
fn a_name_says_which_run_it_is() {
    for name in ["v-1792000000-ab12", "shroom-space-1792000000-ab12", "shroom-e2e-run-1792000000-ab12", "shroom-test-dev-1792000000-ab12"] {
        assert_eq!(wire::run_of(name), Some(RUN), "{name}");
    }
    for name in ["v", "shroom-space", "shroom-probe", "shroom-e2e-run-1792000000", "1792000000-ab12", "v1792000000-ab12", "v-1792000000-ab12-x", "é-1792000000-ab12x"] {
        assert_eq!(wire::run_of(name), None, "{name}");
    }
}

fn on_named_account() -> (Fake, Cloud, String, String) {
    let fake = Fake::start(State { subdomain: Some("shroom-e2e".into()), ..State::fresh() });
    let c = Cloud::new(TOKEN, Policy::Unfenced).with_endpoints(&fake.api(), &fake.r2());
    let (acc, tok) = fake.with(|s| (s.account_id(), s.token_id.clone()));
    (fake, c, acc, tok)
}

fn wire_under<'a>(c: &Cloud, acc: &'a str, tok: &'a str, label: &'a str, bucket: &'a str) -> wire::Wired {
    wire::wire(c, &WireSpec {
        account_id: acc, token_id: tok, label, bucket,
        worker_module: "export default {}", compatibility_date: "2026-09-01", assets: &[],
    })
    .expect("wiring")
}

/// The setup path is the product's, with the run's strings: the survey, wiring, and the
/// provisioned space's own survey all read the run's pair — and another run's space beside
/// it (or the product's `v`) is neither ours nor someone else's `v`.
#[test]
fn a_run_is_set_up_beside_another_without_seeing_it() {
    let (fake, c, acc, tok) = on_named_account();
    let other = SpaceNames::of_run("1792000100-cd34").unwrap();
    wire_under(&c, &acc, &tok, &other.labels[0], &other.bucket);
    wire_under(&c, &acc, &tok, "v", wire::BUCKET);

    let mine = SpaceNames::of_run(RUN).unwrap();
    let before = sweep::survey(&c, &acc, &tok, &mine.labels(), &mine.bucket).unwrap();
    assert_eq!((before.label.as_deref(), before.foreign_v, before.bucket, before.worker, before.found), (Some("v-1792000000-ab12"), false, false, WorkerFact::Absent, None));

    let wired = wire_under(&c, &acc, &tok, &mine.labels[0], &mine.bucket);
    assert_eq!(wired.host, "v-1792000000-ab12.shroom-e2e.workers.dev");
    fake.with(|s| {
        assert_eq!(s.scripts["v-1792000000-ab12"][0]["bucket_name"], "shroom-space-1792000000-ab12");
        assert!(s.object("shroom-space-1792000000-ab12", wire::MANIFEST).is_some());
        assert_eq!(s.scripts["v"][0]["bucket_name"], "shroom-space", "the product's space is untouched");
    });

    let own = sweep::survey_own_among(&c, &acc, &tok, &mine.labels[0], &mine.labels(), &mine.bucket).unwrap();
    assert_eq!((own.worker, own.foreign_v, own.found), (WorkerFact::Ours, false, Some(0)), "a run's label is its first: no foreign `v` is said");
    // The product's own survey reads the same way it always has.
    let product = sweep::survey_own(&c, &acc, &tok, "shroom", wire::BUCKET).unwrap();
    assert!(product.foreign_v, "a product space under a fallback still says so");
}
