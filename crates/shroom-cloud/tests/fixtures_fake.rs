//! L1 against reality: the fake answers the calls recorded on the real test account
//! (`tests/fixtures/`) the way Cloudflare did — status, `success`, error codes and result
//! keys. Messages are Cloudflare's copy and change without notice, so they aren't held.

use serde_json::Value;
use shroom_cloud::{Cloud, CloudError, Policy};
use shroom_cloudfake::{Fake, State};

const TOKEN: &str = "cfat_0123456789abcdefghijklmnopqrstuvwxyzABCD";

fn recorded(path: &str) -> (u16, Value) {
    let file = format!("{}/tests/fixtures/{path}", env!("CARGO_MANIFEST_DIR"));
    let d: Value = serde_json::from_str(&std::fs::read_to_string(&file).unwrap()).unwrap();
    (d["http_status"].as_u64().unwrap() as u16, serde_json::from_str(d["body"].as_str().unwrap()).unwrap())
}

fn keys(v: &Value) -> Vec<String> {
    v.as_object().map(|o| o.keys().cloned().collect()).unwrap_or_default()
}

/// The fake's answer to `<method> /accounts/{id}/<rest>`, as (status, success, codes, result).
fn fake_call(state: State, method: &str, rest: &str, body: Option<Value>) -> (u16, bool, Vec<i64>, Value) {
    let fake = Fake::start(state);
    let c = Cloud::new(TOKEN, Policy::Unfenced).with_endpoints(&fake.api(), &fake.r2());
    let acc = fake.with(|s| s.account_id());
    match c.raw(method, &format!("/accounts/{acc}/{rest}"), body) {
        Ok(v) => (200, v["success"].as_bool().unwrap_or(false), vec![], v["result"].clone()),
        Err(CloudError::Api { status, codes, .. }) => (status, false, codes, Value::Null),
        Err(e) => panic!("{rest}: {e:?}"),
    }
}

fn holds(fixture: &str, state: State, rest: &str) {
    holds_for(fixture, state, "GET", rest, None);
}

fn holds_for(fixture: &str, state: State, method: &str, rest: &str, send: Option<Value>) {
    let (status, body) = recorded(fixture);
    let codes: Vec<i64> = body["errors"].as_array().unwrap().iter().map(|e| e["code"].as_i64().unwrap()).collect();
    let (f_status, f_success, f_codes, f_result) = fake_call(state, method, rest, send);
    assert_eq!(f_status, status, "{fixture}: status");
    assert_eq!(f_success, body["success"].as_bool().unwrap(), "{fixture}: success");
    assert_eq!(f_codes, codes, "{fixture}: error codes");
    assert_eq!(keys(&f_result), keys(&body["result"]), "{fixture}: result keys");
}

fn bare() -> State {
    State { r2_enabled: false, subdomain: None, ..State::fresh() }
}

#[test]
fn bare_account_token_verify() {
    holds("bare-account/verify.json", bare(), "tokens/verify");
}

#[test]
fn bare_account_card_gate() {
    holds("bare-account/r2-buckets.json", bare(), "r2/buckets");
}

#[test]
fn bare_account_has_no_subdomain() {
    holds("bare-account/subdomain.json", bare(), "workers/subdomain");
}

#[test]
fn card_on_lists_no_buckets() {
    holds("card-on/r2-buckets.json", State { subdomain: None, ..State::fresh() }, "r2/buckets");
}

fn named() -> State {
    State { subdomain: Some("shroom-e2e".into()), ..State::fresh() }
}

#[test]
fn a_named_account_reads_its_name() {
    holds("named/subdomain.json", named(), "workers/subdomain");
}

#[test]
fn a_named_account_refuses_a_second_claim() {
    holds_for("named/claim-another.json", named(), "PUT", "workers/subdomain", Some(serde_json::json!({"subdomain": "myshroom"})));
}
