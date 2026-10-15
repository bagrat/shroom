//! L1: the Mac's side of the registry (WORKER-INTERFACE §2.2, §6.11, §7.1 "Rust side")
//! against the in-process Cloudflare fake — seeding is create-only and wiring does it
//! before the page goes up, with the mint parent bound; a broken roster is set aside,
//! create-only, and never deleted or rewritten. Plus the page's version stamp.

use shroom_cloud::registry::{self, Aside};
use shroom_cloud::wire::{self, WireSpec};
use shroom_cloud::{Cloud, Policy};
use shroom_cloudfake::{Fake, State};

const TOKEN: &str = "cfat_0123456789abcdefghijklmnopqrstuvwxyzABCD";

fn wired() -> (Fake, Cloud, String, String) {
    let fake = Fake::start(State { subdomain: Some("bagrat".into()), ..State::fresh() });
    let c = Cloud::new(TOKEN, Policy::Unfenced).with_endpoints(&fake.api(), &fake.r2());
    let acc = fake.with(|s| s.account_id());
    let token_id = fake.with(|s| s.token_id.clone());
    wire::wire(&c, &WireSpec {
        account_id: &acc, token_id: &token_id, label: "v", bucket: wire::BUCKET,
        worker_module: "export default {}", compatibility_date: "2026-09-01", assets: &[],
    }).unwrap();
    (fake, c, acc, token_id)
}

#[test]
fn wiring_seeds_the_roster_and_binds_the_parent() {
    let (fake, c, acc, token_id) = wired();
    let roster = fake.with(|s| s.object(wire::BUCKET, registry::ROSTER)).expect("wiring seeded the roster");
    assert!(registry::is_roster(&roster));
    let r: serde_json::Value = serde_json::from_slice(&roster).unwrap();
    assert_eq!(r["members"]["m1"]["role"], "owner");
    fake.with(|s| {
        let bindings = &s.scripts["v"];
        assert_eq!(bindings[0]["bucket_name"], wire::BUCKET, "the bucket stays the first binding");
        assert!(bindings.iter().any(|b| b["type"] == "secret_text" && b["name"] == wire::PARENT_BINDING), "{bindings:?}");
    });
    // The parent is the token's derived pair, named for the space's bucket.
    let parent: serde_json::Value = serde_json::from_str(&wire::parent_text(&c, &acc, wire::BUCKET, &token_id)).unwrap();
    let pair = c.s3_credentials(&token_id);
    assert_eq!(parent, serde_json::json!({"account_id": acc, "bucket": wire::BUCKET, "key_id": pair.access_key_id, "secret": pair.secret_access_key}));
}

#[test]
fn seeding_is_create_only_and_wiring_again_keeps_the_roster() {
    let (fake, c, acc, token_id) = wired();
    let s3 = wire::bucket_client(&c, &acc, wire::BUCKET, &token_id);
    let members = br#"{"v":1,"createdAt":"x","members":{"m1":{"role":"owner"},"m_Mher____________":{"role":"member"}},"devices":{},"invites":{}}"#;
    fake.with(|s| s.put_object(wire::BUCKET, registry::ROSTER, members));
    assert!(!registry::seed(&s3, Some("Bagrat"), 0).unwrap(), "a roster already there is never replaced");
    wire::wire(&c, &WireSpec {
        account_id: &acc, token_id: &token_id, label: "v", bucket: wire::BUCKET,
        worker_module: "export default {}", compatibility_date: "2026-09-01", assets: &[],
    }).unwrap();
    assert_eq!(fake.with(|s| s.object(wire::BUCKET, registry::ROSTER)).unwrap(), members.to_vec());
}

#[test]
fn a_broken_roster_is_set_aside_create_only_and_never_deleted() {
    let (fake, c, acc, token_id) = wired();
    let s3 = wire::bucket_client(&c, &acc, wire::BUCKET, &token_id);
    assert_eq!(registry::set_aside(&s3, 100).unwrap(), Aside::Intact, "a roster that parses is left alone");
    fake.with(|s| s.put_object(wire::BUCKET, registry::ROSTER, b"{ hand-edited, oops"));
    let Aside::SetAside(key) = registry::set_aside(&s3, 100).unwrap() else { panic!("set aside") };
    assert_eq!(key, ".shroom/registry/roster.broken-100.json");
    assert_eq!(fake.with(|s| s.object(wire::BUCKET, &key)).unwrap(), b"{ hand-edited, oops".to_vec());
    assert_eq!(fake.with(|s| s.object(wire::BUCKET, registry::ROSTER)).unwrap(), b"{ hand-edited, oops".to_vec(), "never deleted");
    // A second pass at the same second never overwrites the first copy.
    fake.with(|s| s.put_object(wire::BUCKET, registry::ROSTER, b"worse"));
    registry::set_aside(&s3, 100).unwrap();
    assert_eq!(fake.with(|s| s.object(wire::BUCKET, &key)).unwrap(), b"{ hand-edited, oops".to_vec());
}

#[test]
fn the_page_reports_the_apps_own_version() {
    let worker = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../worker/src/worker.js")).unwrap();
    let conf: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../app/src-tauri/tauri.conf.json")).unwrap()).unwrap();
    let app = conf["version"].as_str().unwrap();
    assert!(worker.contains(&format!("export const VERSION = \"{app}\";")), "worker.js's VERSION must be the app's {app}");
    assert!(worker.contains("export const IFACE = 2;"));
}
