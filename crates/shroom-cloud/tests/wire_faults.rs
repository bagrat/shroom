//! L3 (TESTING-PLAN §5; DECISIONS B64; tests-sys #4): wiring under the transport fault seam
//! (the L1 fake's `fault` rules on every call wiring makes — Cloudflare's API and the
//! bucket) and killed mid-wire (`kill -9` of the process wiring). Wiring is idempotent end
//! to end and the app retries it on a backoff (`Op::Wire`), so the claim is: whatever broke
//! where, wiring again converges on exactly the space one clean wiring makes — one page,
//! one route, the same objects, the owner cookie's secret written once and never replaced
//! (law 2's cousin: rewriting it would sign every browser out), and a space cut short never
//! reads as one that's already there.

use std::collections::{BTreeMap, BTreeSet};
use std::process::Command;
use std::time::{Duration, Instant};

use shroom_cloud::sweep;
use shroom_cloud::wire::{self, WireError, WireSpec};
use shroom_cloud::{Cloud, CloudError, Policy, owner};
use shroom_cloudfake::{Fake, State};

const TOKEN: &str = "cfat_0123456789abcdefghijklmnopqrstuvwxyzABCD";

fn fake() -> Fake {
    Fake::start(State { subdomain: Some("bagrat".into()), ..State::fresh() })
}

fn cloud(api: &str, r2: &str) -> Cloud {
    Cloud::new(TOKEN, Policy::Unfenced).with_endpoints(api, r2)
}

fn spec<'a>(account: &'a str, token_id: &'a str) -> WireSpec<'a> {
    WireSpec {
        account_id: account, token_id, label: "v", bucket: wire::BUCKET,
        worker_module: "export const VERSION = \"9.9.9\";\nexport default {}", compatibility_date: "2026-09-01",
        assets: &[(".shroom/player/hls.min.js", b"/*hls*/", "text/javascript")],
    }
}

/// The page's bindings by Worker, the routes, the bucket's keys, the placed secrets.
type Space = (BTreeMap<String, Vec<String>>, BTreeSet<String>, BTreeSet<String>, BTreeMap<String, BTreeSet<String>>);

/// What a wiring leaves: the page's bindings, the routes, the bucket's keys (the cookie
/// secret's bytes aside: random per space), the placed secrets.
fn space_of(f: &Fake) -> Space {
    f.with(|s| {
        let scripts = s.scripts.iter().map(|(k, b)| (k.clone(), b.iter().map(|x| x["name"].as_str().unwrap_or("").to_string()).collect())).collect();
        let keys = s.buckets.get(wire::BUCKET).map(|b| b.keys().cloned().collect()).unwrap_or_default();
        (scripts, s.routes.clone(), keys, s.secrets.clone())
    })
}

fn transient(e: &WireError) -> bool {
    match e {
        WireError::Cloud(CloudError::Transport(_)) | WireError::Storage(_) => true,
        WireError::Cloud(CloudError::Api { status, .. }) => *status >= 500,
        _ => false,
    }
}

/// Wires until it lands, as `Op::Wire`'s backoff would: every failure on the way must be a
/// transient one (never `Foreign`, never a wrong answer), within a few tries.
fn wire_until_done(c: &Cloud, sp: &WireSpec, f: &Fake) -> usize {
    for tries in 1..=12 {
        match wire::wire(c, sp) {
            Ok(w) => {
                assert_eq!(w.host, "v.bagrat.workers.dev");
                return tries;
            }
            Err(e) => {
                assert!(transient(&e), "a fault read as something else: {e:?}");
                // Until the manifest is up, the space never reads as found.
                let acc = f.with(|s| s.account_id());
                let tok = f.with(|s| s.token_id.clone());
                if f.with(|s| s.object(wire::BUCKET, wire::MANIFEST).is_none()) {
                    let survey = sweep::survey_space(c, &acc, &tok);
                    if let Ok(sv) = survey {
                        assert_eq!(sv.found, None, "a wiring cut short reads as a found space");
                    }
                }
            }
        }
    }
    panic!("wiring never converged");
}

fn reference() -> Space {
    let f = fake();
    let (acc, tok) = f.with(|s| (s.account_id(), s.token_id.clone()));
    wire::wire(&cloud(&f.api(), &f.r2()), &spec(&acc, &tok)).unwrap();
    space_of(&f)
}

#[test]
fn every_fault_on_every_wiring_call_converges_on_the_clean_space() {
    let clean = reference();
    // Each kind on each kind of call wiring makes: the bucket's creation, the page's PUT,
    // the route, and the bucket's own writes (the player, the cookie secret, the roster, the
    // self-check, the manifest).
    let targets = [
        ("POST", "/client/v4/accounts/ACC/r2/buckets"),
        ("PUT", "/client/v4/accounts/ACC/workers/scripts/v"),
        ("POST", "/client/v4/accounts/ACC/workers/scripts/v/subdomain"),
        ("PUT", "/s3/shroom-space/.shroom/player/"),
        ("PUT", "/s3/shroom-space/.shroom/cookie.key"),
        ("PUT", "/s3/shroom-space/.shroom/registry/"),
        ("PUT", "/s3/shroom-space/.shroom/self-check"),
        ("PUT", "/s3/shroom-space/.shroom/manifest"),
    ];
    for kind in ["503", "drop", "lost-reply", "reset", "delay:200"] {
        for (method, path) in targets {
            let f = fake();
            let (acc, tok) = f.with(|s| (s.account_id(), s.token_id.clone()));
            let path = path.replace("ACC", &acc);
            f.with(|s| s.control(&format!("fault {kind} {method} {path} 1")).unwrap());
            let c = cloud(&f.api(), &f.r2());
            let tries = wire_until_done(&c, &spec(&acc, &tok), &f);
            assert_eq!(f.with(|s| s.faulted()), 1, "the fault was met: {kind} {method} {path}");
            assert_eq!(space_of(&f), clean, "{kind} on {method} {path} ({tries} tries)");
            // The cookie secret was written once: a later wiring leaves it as it is.
            let secret = f.with(|s| s.object(wire::BUCKET, owner::COOKIE_KEY)).expect("the cookie secret");
            wire::wire(&c, &spec(&acc, &tok)).unwrap();
            assert_eq!(f.with(|s| s.object(wire::BUCKET, owner::COOKIE_KEY)).unwrap(), secret, "{kind} {path}");
        }
    }
}

/// The process the kill test kills: one wiring, from the environment the parent set.
#[test]
#[ignore = "run only as the kill test's child"]
fn child_wire_pass() {
    let (Ok(api), Ok(r2), Ok(acc), Ok(tok)) =
        (std::env::var("SHROOM_WIRE_API"), std::env::var("SHROOM_WIRE_R2"), std::env::var("SHROOM_WIRE_ACC"), std::env::var("SHROOM_WIRE_TOK"))
    else {
        return;
    };
    let _ = wire::wire(&cloud(&api, &r2), &spec(&acc, &tok));
}

#[test]
fn kill_9_mid_wire_then_wiring_again_converges_and_keeps_the_cookie_secret() {
    let clean = reference();
    // Kill points: after the page went up but before the route; after the route, before the
    // self-check and the manifest.
    for (after, label) in [("PUT /client/v4/accounts/ACC/workers/scripts/v", "page up, no route"), ("POST /client/v4/accounts/ACC/workers/scripts/v/subdomain", "route on, no manifest")] {
        let f = fake();
        let (acc, tok) = f.with(|s| (s.account_id(), s.token_id.clone()));
        let after = after.replace("ACC", &acc);
        // A slow path everywhere, so the kill lands between two calls.
        f.with(|s| s.control("fault delay:250 * / always").unwrap());
        let mut child = Command::new(std::env::current_exe().unwrap())
            .args(["--exact", "child_wire_pass", "--ignored", "--nocapture", "--test-threads=1"])
            .env("SHROOM_WIRE_API", f.api())
            .env("SHROOM_WIRE_R2", f.r2())
            .env("SHROOM_WIRE_ACC", &acc)
            .env("SHROOM_WIRE_TOK", &tok)
            .spawn()
            .unwrap();
        let until = Instant::now() + Duration::from_secs(30);
        while !f.with(|s| s.calls.iter().any(|c| c == &after)) {
            assert!(Instant::now() < until, "{label}: never got there");
            assert!(child.try_wait().unwrap().is_none(), "{label}: wiring finished before the kill");
            std::thread::sleep(Duration::from_millis(10));
        }
        child.kill().unwrap(); // SIGKILL
        child.wait().unwrap();
        f.with(|s| s.control("faults-clear").unwrap());
        assert!(f.with(|s| s.object(wire::BUCKET, wire::MANIFEST)).is_none(), "{label}: killed before the manifest");
        let c = cloud(&f.api(), &f.r2());
        let survey = sweep::survey_space(&c, &acc, &tok).unwrap();
        assert_eq!(survey.found, None, "{label}: a wiring cut short never reads as a found space");
        let secret = f.with(|s| s.object(wire::BUCKET, owner::COOKIE_KEY));
        // The relaunch wires again (the wizard's row five, or Set it up fresh).
        wire_until_done(&c, &spec(&acc, &tok), &f);
        assert_eq!(space_of(&f), clean, "{label}");
        if let Some(secret) = secret {
            assert_eq!(f.with(|s| s.object(wire::BUCKET, owner::COOKIE_KEY)).unwrap(), secret, "{label}: the cookie secret survived the kill");
        }
        assert_eq!(sweep::survey_space(&c, &acc, &tok).unwrap().found, Some(0), "{label}: now a whole space");
    }
}
