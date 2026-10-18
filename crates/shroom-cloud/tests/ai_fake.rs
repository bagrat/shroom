//! L1: naming on and off (N66, DECISIONS B206 a) against the in-process Cloudflare fake. On
//! re-wires the space's page with Cloudflare's AI and a schedule each minute; off takes them
//! away. Whatever puts the page up again — a refresh after an update, a re-wiring — keeps
//! what the live page has, and the page's ping says which it is.

use std::time::Duration;

use shroom_cloud::member::{Ping, Space};
use shroom_cloud::wire::{self, PageCheck, Refresh, WireError, WireSpec};
use shroom_cloud::{Cloud, Policy};
use shroom_cloudfake::{Fake, State};
use serde_json::Value;

const TOKEN: &str = "cfat_0123456789abcdefghijklmnopqrstuvwxyzABCD";
const HOST: &str = "v.bagrat.workers.dev";

fn cloud(fake: &Fake) -> Cloud {
    Cloud::new(TOKEN, Policy::Unfenced).with_endpoints(&fake.api(), &fake.r2())
}

fn module(version: &str) -> String {
    format!("export const VERSION = \"{version}\";\nexport default {{}}\n")
}

fn spec<'a>(account: &'a str, token_id: &'a str, module: &'a str) -> WireSpec<'a> {
    WireSpec {
        account_id: account, token_id, label: "v", bucket: wire::BUCKET,
        worker_module: module, compatibility_date: "2026-09-01",
        assets: &[(".shroom/player/hls.min.js", b"/*hls*/", "text/javascript")],
    }
}

fn check<'a>(at: &'a dyn Fn(&str) -> Space, version: &'a str) -> PageCheck<'a> {
    PageCheck { at, version, every: Duration::from_millis(10), within: Duration::from_millis(1_000) }
}

/// The deployed page's binding types, in order.
fn bound(fake: &Fake) -> Vec<String> {
    fake.with(|s| s.scripts["v"].iter().filter_map(|b| b.get("type").and_then(Value::as_str).map(String::from)).collect())
}

fn ai_of(fake: &Fake) -> Option<bool> {
    match Space::at(HOST, &fake.page(HOST)).ping() {
        Ping::Shroom { ai, .. } => Some(ai),
        _ => None,
    }
}

fn wired(fake: &Fake, version: &str) -> (String, String, String) {
    let c = cloud(fake);
    let (acc, tok) = fake.with(|s| (s.account_id(), s.token_id.clone()));
    let m = module(version);
    wire::wire(&c, &spec(&acc, &tok, &m)).unwrap();
    (acc, tok, m)
}

#[test]
fn naming_on_puts_the_page_up_with_the_ai_and_a_schedule_each_minute_and_off_takes_them_away() {
    let fake = Fake::start(State { subdomain: Some("bagrat".into()), ..State::fresh() });
    let (acc, tok, m) = wired(&fake, "0.4.0");
    let c = cloud(&fake);
    assert_eq!(bound(&fake), ["r2_bucket", "secret_text"], "a fresh space: no AI");
    assert_eq!(ai_of(&fake), Some(false));
    assert!(fake.with(|s| s.schedules.get("v").is_none_or(Vec::is_empty)));

    wire::set_ai(&c, &spec(&acc, &tok, &m), true).unwrap();
    assert_eq!(bound(&fake), ["r2_bucket", "secret_text", "ai"]);
    assert_eq!(fake.with(|s| s.scripts["v"][2]["name"].clone()), "AI");
    assert_eq!(fake.with(|s| s.schedules["v"].clone()), ["* * * * *"]);
    assert_eq!(ai_of(&fake), Some(true), "the page says so");
    // Turning it on again is the same page.
    wire::set_ai(&c, &spec(&acc, &tok, &m), true).unwrap();
    assert_eq!(bound(&fake), ["r2_bucket", "secret_text", "ai"]);

    wire::set_ai(&c, &spec(&acc, &tok, &m), false).unwrap();
    assert_eq!(bound(&fake), ["r2_bucket", "secret_text"]);
    assert_eq!(fake.with(|s| s.schedules["v"].clone()), Vec::<String>::new());
    assert_eq!(ai_of(&fake), Some(false));
}

#[test]
fn a_refresh_after_an_update_keeps_naming_as_the_live_page_has_it() {
    for on in [true, false] {
        let fake = Fake::start(State { subdomain: Some("bagrat".into()), ..State::fresh() });
        let (acc, tok, old) = wired(&fake, "0.3.0");
        let c = cloud(&fake);
        if on {
            wire::set_ai(&c, &spec(&acc, &tok, &old), true).unwrap();
        }
        let ours = module("0.4.0");
        let at = |h: &str| Space::at(h, &fake.page(h));
        assert_eq!(wire::refresh_page(&c, &spec(&acc, &tok, &ours), &check(&at, "0.4.0"), HOST).unwrap(), Refresh::Deployed { was: "0.3.0".into() });
        assert_eq!(fake.with(|s| s.versions["v"].clone()), "0.4.0");
        assert_eq!(bound(&fake).contains(&"ai".to_string()), on, "naming {on}: kept");
        assert_eq!(fake.with(|s| s.schedules.get("v").is_some_and(|c| c == &["* * * * *"])), on);
        assert_eq!(ai_of(&fake), Some(on));
    }
}

#[test]
fn a_re_wiring_keeps_it_too() {
    let fake = Fake::start(State { subdomain: Some("bagrat".into()), ..State::fresh() });
    let (acc, tok, old) = wired(&fake, "0.3.0");
    let c = cloud(&fake);
    wire::set_ai(&c, &spec(&acc, &tok, &old), true).unwrap();
    // The repair mood's wiring, with a newer page in hand: the script goes up again, the AI with it.
    let ours = module("0.4.0");
    let at = |h: &str| Space::at(h, &fake.page(h));
    let w = wire::wire_with(&c, &spec(&acc, &tok, &ours), Some(&check(&at, "0.4.0"))).unwrap();
    assert!(w.deployed);
    assert!(bound(&fake).contains(&"ai".to_string()));
    assert_eq!(fake.with(|s| s.schedules["v"].clone()), ["* * * * *"]);
}

#[test]
fn naming_is_only_ever_turned_on_for_shrooms_own_page() {
    let fake = Fake::start(State { subdomain: Some("bagrat".into()), ..State::fresh() });
    fake.with(|s| s.scripts.insert("v".into(), vec![serde_json::json!({"type": "plain_text", "name": "X"})]));
    let c = cloud(&fake);
    let (acc, tok) = fake.with(|s| (s.account_id(), s.token_id.clone()));
    let m = module("0.4.0");
    assert!(matches!(wire::set_ai(&c, &spec(&acc, &tok, &m), true), Err(WireError::Foreign)));
    assert!(fake.with(|s| s.schedules.get("v").is_none()), "no schedule on someone else's Worker");
}

#[test]
fn the_schedule_is_fenced_like_every_other_write() {
    let c = Cloud::new(TOKEN, Policy::TestNamespace { prefix: "shroom-e2e-".into() });
    assert!(c.put_schedules("acc", "v", &["* * * * *"]).is_err(), "a run token never touches a name outside its own");
}
