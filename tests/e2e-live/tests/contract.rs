//! L1's contract (TESTING-PLAN §5), on the dedicated test account (D3): the in-process fake
//! must answer the calls shroom makes the way Cloudflare does. The same calls go to the run
//! (its token, with the product's powers) and to the fake; status codes, error codes and the
//! result's shape (keys and value kinds) must match. The bucket it makes is the run's own
//! (`shroom-space-<run>`), so the run's close removes it (DECISIONS B131).
//!
//! Runs only with SHROOM_E2E=1. It was `shroom-cloud`'s `contract_live` on the main account,
//! fenced to `shroom-test-…` names, until main-account testing retired (D3).

use std::time::{SystemTime, UNIX_EPOCH};

use serde_json::Value;
use shroom_cloud::{Cloud, CloudError, Policy};
use shroom_cloudfake::{Fake, State};
use shroom_e2e_live::{NAME, Run, config};

/// A value's shape: object keys (recursively, first array element only), kinds elsewhere.
fn shape(v: &Value) -> Value {
    match v {
        Value::Object(m) => Value::Object(m.iter().map(|(k, v)| (k.clone(), shape(v))).collect()),
        Value::Array(a) => Value::Array(a.first().map(shape).into_iter().collect()),
        Value::String(_) => Value::String("s".into()),
        Value::Number(_) => Value::String("n".into()),
        Value::Bool(_) => Value::String("b".into()),
        Value::Null => Value::Null,
    }
}

/// What a call answered, reduced to what the client reads.
#[derive(Debug, PartialEq)]
enum Answer {
    Ok(Value),
    Err { status: u16, codes: Vec<i64> },
}

fn answer(r: Result<Value, CloudError>, keys: &[&str]) -> Answer {
    match r {
        // Only the fields the client reads are held to the contract (Cloudflare returns
        // many more, and the fake needn't invent them).
        Ok(v) => {
            let result = v.get("result").cloned().unwrap_or(Value::Null);
            let picked = match &result {
                Value::Array(a) => Value::Array(a.first().map(|x| pick(x, keys)).into_iter().collect()),
                other => pick(other, keys),
            };
            Answer::Ok(shape(&picked))
        }
        Err(CloudError::Api { status, codes, .. }) => Answer::Err { status, codes },
        Err(e) => panic!("transport: {e}"),
    }
}

fn pick(v: &Value, keys: &[&str]) -> Value {
    match v {
        Value::Object(m) => Value::Object(m.iter().filter(|(k, _)| keys.contains(&k.as_str())).map(|(k, v)| (k.clone(), v.clone())).collect()),
        other => other.clone(),
    }
}

#[test]
fn the_fake_answers_like_cloudflare() {
    let Some(cfg) = config() else { return eprintln!("skipped: set SHROOM_E2E=1") };
    let run = Run::start(&cfg).unwrap_or_else(|e| panic!("the run didn't start: {e}"));
    let account = run.account_id.clone();
    let stamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
    let bucket = run.space.bucket.clone();

    let mut st = State::fresh();
    st.accounts = vec![(account.clone(), "x".into())];
    st.subdomain = Some(NAME.into());
    let fake_srv = Fake::start(st);
    let fake = Cloud::new("cfat_fake", Policy::Unfenced).with_endpoints(&fake_srv.api(), &fake_srv.r2());

    run.product.create_bucket(&account, &bucket).unwrap();
    fake.create_bucket(&account, &bucket).unwrap();

    let a = format!("/accounts/{account}");
    let calls: Vec<(&str, String, Option<Value>, &[&str])> = vec![
        ("GET", "/accounts".into(), None, &["id", "name"]),
        ("GET", format!("{a}/tokens/verify"), None, &["id", "status"]),
        ("GET", format!("{a}/workers/subdomain"), None, &["subdomain"]),
        ("GET", format!("{a}/r2/buckets?name_contains={bucket}"), None, &["buckets"]),
        ("GET", format!("{a}/workers/scripts/shroom-absent-{stamp}/settings"), None, &[]),
        ("POST", format!("{a}/r2/buckets"), Some(serde_json::json!({"name": bucket})), &[]),
    ];
    let mut mismatches = Vec::new();
    for (method, path, body, keys) in calls {
        let r = answer(run.product.raw(method, &path, body.clone()), keys);
        let f = answer(fake.raw(method, &path, body), keys);
        // Bucket objects: only `name` is read.
        let norm = |x: Answer| match x {
            Answer::Ok(Value::Object(mut m)) if m.contains_key("buckets") => {
                m.insert("buckets".into(), serde_json::json!([{"name": "s"}]));
                Answer::Ok(Value::Object(m))
            }
            other => other,
        };
        let (r, f) = (norm(r), norm(f));
        if r != f {
            mismatches.push(format!("{method} {path}\n  real: {r:?}\n  fake: {f:?}"));
        }
    }
    assert!(mismatches.is_empty(), "the fake drifted from Cloudflare:\n{}", mismatches.join("\n"));
}
