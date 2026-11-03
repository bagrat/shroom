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
use shroom_e2e_live::{MAIN_NAME, NAME, Run, close, config, open};

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

/// The video's bytes as an iPhone asks for them, from the real page on the real R2 (a
/// phone played nothing while desktop Chrome played: Safari plays a file only from a server
/// that answers its Range asks with 206 and the right `Content-Range`, and asks for two bytes
/// first). The run's space is wired with the real worker.js behind a thin front that seeds
/// one recording into the run's own bucket; then each range an iPhone asks is answered with
/// 206, the span it names (clamped to the file), and that many bytes.
#[test]
fn the_video_answers_an_iphones_ranges() {
    use shroom_cloud::member::{Ping, Space};
    use std::time::{Duration, Instant};
    let Some(run) = run() else { return eprintln!("skipped: set SHROOM_E2E=1") };
    if !named(&run) {
        return;
    }
    const MODULE: &str = include_str!("../../../worker/src/worker.js");
    assert_eq!(MODULE.matches("\nexport default {").count(), 1);
    let module = MODULE.replacen("\nexport default {", "\nconst shroomDefault = {", 1) + SEED_FRONT;
    let (labels, bucket) = (run.space.labels(), run.space.bucket.as_str());
    let before = sweep::survey(&run.product, &run.account_id, &run.token_id, &labels, bucket).expect("survey");
    let label = before.label.expect("a fresh run's label is free");
    let spec = WireSpec { account_id: &run.account_id, token_id: &run.token_id, label: &label, bucket, worker_module: &module, compatibility_date: "2026-09-01", assets: &[] };
    let wired = wire::wire(&run.product, &spec).expect("wiring");
    let t = Instant::now();
    while !matches!(Space::new(&wired.host).ping(), Ping::Shroom { .. }) {
        assert!(t.elapsed() < Duration::from_secs(180), "the page never answered");
        std::thread::sleep(Duration::from_millis(500));
    }
    let agent: ureq::Agent = ureq::Agent::config_builder().http_status_as_error(false).timeout_global(Some(Duration::from_secs(60))).build().into();
    let base = format!("https://{}", wired.host);
    // workers.dev may route an ask to an edge that hasn't the new script yet (1042's 404).
    let t = Instant::now();
    loop {
        let res = agent.post(&format!("{base}/__probe/seed")).send_empty();
        let status = res.as_ref().map(|r| r.status().as_u16()).unwrap_or(0);
        if status == 200 {
            break;
        }
        assert!(t.elapsed() < Duration::from_secs(180), "the seed never answered (last {status})");
        std::thread::sleep(Duration::from_secs(1));
    }
    let mut raw = agent.get(&format!("{base}/__probe/r2")).call().expect("r2 ranges");
    eprintln!("iphone-ranges: R2's own answers {}", raw.body_mut().read_to_string().unwrap_or_default());
    const SIZE: u64 = 1000;
    let mut wrong = Vec::new();
    for (ask, from, to) in [("bytes=0-1", 0, 1), ("bytes=0-", 0, 999), ("bytes=0-999", 0, 999), ("bytes=500-", 500, 999), ("bytes=990-1999", 990, 999), ("bytes=-100", 900, 999)] {
        let mut res = agent.get(&format!("{base}/m/{SEED_ID}/video.mp4")).header("range", ask).header("accept-encoding", "identity").call().expect("a range");
        let status = res.status().as_u16();
        let head = |k: &str| res.headers().get(k).and_then(|v| v.to_str().ok()).unwrap_or("-").to_string();
        let (cr, cl, ce, te) = (head("content-range"), head("content-length"), head("content-encoding"), head("transfer-encoding"));
        let body = res.body_mut().read_to_vec().unwrap_or_default();
        eprintln!("iphone-ranges: {ask} -> {status} content-range {cr} content-length {cl} content-encoding {ce} transfer-encoding {te}, {} bytes, first {:?}", body.len(), body.first());
        let want = format!("bytes {from}-{to}/{SIZE}");
        if status != 206 || cr != want || body.len() as u64 != to - from + 1 || body.first().copied() != Some((from % 256) as u8) {
            wrong.push(format!("{ask}: {status} {cr} {} bytes (want 206 {want} {} bytes)", body.len(), to - from + 1));
        }
    }
    assert!(wrong.is_empty(), "the video's ranges, as an iPhone asks them: {wrong:#?}");
}

const SEED_ID: &str = "PhoneRangePhoneR";

/// The front of the_video_answers_an_iphones_ranges: seeds the run's own bucket with one
/// live recording whose single-file copy is 1000 known bytes, and reports R2's own `range`
/// for each ask; everything else is the real page.
const SEED_FRONT: &str = r#"
const SEED_ID = "PhoneRangePhoneR";
const ASKS = ["bytes=0-1", "bytes=0-", "bytes=0-999", "bytes=500-", "bytes=990-1999", "bytes=-100"];
export default {
  async fetch(request, env, ctx) {
    const path = new URL(request.url).pathname;
    if (path === "/__probe/seed" && request.method === "POST") {
      await env.BUCKET.put(`rec/${SEED_ID}/video.mp4`, new Uint8Array(1000).map((_, i) => i % 256), { httpMetadata: { contentType: "video/mp4" } });
      await env.BUCKET.put(`meta/${SEED_ID}.json`, JSON.stringify({ schema: 1, id: SEED_ID, state: "live", createdAt: new Date(Date.now() - 3600000).toISOString(), title: "Ranges", durationSec: 5,
        media: { hls: `rec/${SEED_ID}/master.m3u8`, mp4: `rec/${SEED_ID}/video.mp4`, poster: null, width: 1280, height: 800 }, owner: "m1" }));
      return new Response("seeded");
    }
    if (path === "/__probe/r2") {
      const out = [];
      for (const r of ASKS) {
        const o = await env.BUCKET.get(`rec/${SEED_ID}/video.mp4`, { range: new Headers({ range: r }) });
        const b = o ? new Uint8Array(await o.arrayBuffer()) : null;
        out.push({ ask: r, range: o ? o.range ?? null : "no object", size: o?.size, bytes: b?.length, first: b?.[0] });
      }
      return Response.json(out);
    }
    return shroomDefault.fetch(request, env, ctx);
  },
  scheduled(controller, env, ctx) { return shroomDefault.scheduled(controller, env, ctx); },
};
"#;

/// A space kept up for a phone to open (the iPhone Simulator on a held Mac): the real page
/// on the real R2, with two recordings of the same 5 s take — one that plays its single-file
/// copy, one that plays the stream. Runs only while `fixtures/phone/HOLD` holds
/// `<minutes> <run id>`: the run id is chosen ahead (a run id's shape, so the account's
/// reaper knows it), so the space is at `v-<run>.shroom-e2e.workers.dev` before it exists.
#[test]
fn a_space_held_for_a_phone() {
    use shroom_cloud::member::{Ping, Space};
    use shroom_cloud::{Cloud, Policy};
    use std::time::{Duration, Instant};
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/phone");
    let Ok(hold) = std::fs::read_to_string(dir.join("HOLD")) else { return eprintln!("skipped: no fixtures/phone/HOLD") };
    let mut words = hold.split_whitespace();
    let minutes: u64 = words.next().and_then(|m| m.parse().ok()).expect("HOLD: <minutes> <run id>");
    let run_id = words.next().expect("HOLD: <minutes> <run id>").to_string();
    let Some(cfg) = config() else { return eprintln!("skipped: set SHROOM_E2E=1") };
    let opened = open(&cfg, &run_id).unwrap_or_else(|e| panic!("the run didn't open: {e}"));
    struct Closing<'a>(&'a shroom_e2e_live::Config, String);
    impl Drop for Closing<'_> {
        fn drop(&mut self) {
            eprintln!("phone-hold: closed {:?}", close(self.0, Some(&self.1)));
        }
    }
    let _closing = Closing(&cfg, run_id.clone());
    let product = Cloud::new(&opened.value, Policy::Unfenced);
    if product.subdomain(&cfg.account_id).expect("the workers.dev name").as_deref() != Some(NAME) {
        return eprintln!("skipped: the account's workers.dev name isn't claimed yet");
    }
    const MODULE: &str = include_str!("../../../worker/src/worker.js");
    let logger = std::fs::read_to_string(dir.join("logger.js")).expect("the logger");
    let module = MODULE.replacen("\nexport default {", "\nconst shroomDefault = {", 1) + &format!("\nconst LOGGER = {};\n", serde_json::to_string(&logger).unwrap()) + PUT_FRONT;
    let space = opened.space();
    let (labels, bucket) = (space.labels(), space.bucket.as_str());
    let before = sweep::survey(&product, &cfg.account_id, &opened.token_id, &labels, bucket).expect("survey");
    let label = before.label.expect("the run's label is free");
    let spec = WireSpec { account_id: &cfg.account_id, token_id: &opened.token_id, label: &label, bucket, worker_module: &module, compatibility_date: "2026-09-01", assets: &[] };
    let wired = wire::wire(&product, &spec).expect("wiring");
    eprintln!("phone-hold: wired {}", wired.host);
    let t = Instant::now();
    while !matches!(Space::new(&wired.host).ping(), Ping::Shroom { .. }) {
        assert!(t.elapsed() < Duration::from_secs(180), "the page never answered");
        std::thread::sleep(Duration::from_millis(500));
    }
    let agent: ureq::Agent = ureq::Agent::config_builder().http_status_as_error(false).timeout_global(Some(Duration::from_secs(60))).build().into();
    let base = format!("https://{}", wired.host);
    let put = |key: &str, bytes: &[u8]| {
        let t = Instant::now();
        loop {
            let status = agent.put(&format!("{base}/__probe/put/{key}")).send(bytes).map(|r| r.status().as_u16()).unwrap_or(0);
            if status == 200 {
                return;
            }
            assert!(t.elapsed() < Duration::from_secs(180), "{key} never went up (last {status})");
            std::thread::sleep(Duration::from_secs(1));
        }
    };
    let files = ["master.m3u8", "poster.jpg", "video.mp4", "v0/stream.m3u8", "v0/init.mp4", "v0/seg_00000.m4s"];
    for (id, mp4) in [("PhoneMp4PhoneMp4", true), ("PhoneHlsPhoneHls", false), ("PhoneCapPhoneCap", true)] {
        for f in files {
            put(&format!("rec/{id}/{f}"), &std::fs::read(dir.join(f)).expect("a fixture"));
        }
        let meta = serde_json::json!({ "schema": 1, "id": id, "state": "live", "createdAt": "2026-11-01T12:00:00Z", "title": "A take for a phone", "durationSec": 5,
            "media": { "hls": format!("rec/{id}/master.m3u8"), "mp4": if mp4 { serde_json::json!(format!("rec/{id}/video.mp4")) } else { serde_json::Value::Null },
                "poster": format!("rec/{id}/poster.jpg"), "width": 1280, "height": 800 }, "owner": "m1" });
        let mut meta = meta;
        if id == "PhoneCapPhoneCap" {
            // Captions as a named take has them: the record says so and the transcript is done.
            meta["captions"] = serde_json::json!({ "language": "en" });
            let words = serde_json::json!([["Hello", 0.2, 0.6], ["there,", 0.7, 1.1], ["this", 1.3, 1.5], ["is", 1.5, 1.6], ["a", 1.6, 1.7], ["take.", 1.7, 2.2], ["Thanks", 3.0, 3.4], ["for", 3.4, 3.6], ["watching.", 3.6, 4.3]]);
            let t = serde_json::json!({ "v": 1, "model": "test", "language": "en", "duration": 5, "heard_through": 5, "done": true, "windows": 1, "clock": null, "words": words });
            put(&format!("rec/{id}/ai/transcript.json"), t.to_string().as_bytes());
        }
        put(&format!("meta/{id}.json"), meta.to_string().as_bytes());
        eprintln!("phone-hold: {base}/{id}");
    }
    eprintln!("phone-hold: up for {minutes} min from now");
    std::thread::sleep(Duration::from_secs(minutes * 60));
}

/// The front of a_space_held_for_a_phone: `PUT /__probe/put/<key>` stores its body in the
/// run's own bucket; every HTML page gets `fixtures/phone/logger.js` under the page's own
/// nonce, whose lines `POST /__probe/log` keeps and `GET /__probe/logs` hands back (the
/// session reads them during the hold; they never reach the run's log). Everything else is
/// the real page.
const PUT_FRONT: &str = r#"
export default {
  async fetch(request, env, ctx) {
    const path = new URL(request.url).pathname;
    if (path.startsWith("/__probe/put/") && request.method === "PUT") {
      await env.BUCKET.put(decodeURIComponent(path.slice("/__probe/put/".length)), await request.arrayBuffer());
      return new Response("stored");
    }
    if (path === "/__probe/log" && request.method === "POST") {
      const line = (await request.text()).slice(0, 4000);
      await env.BUCKET.put(`probe/log/${Date.now()}-${Math.random().toString(36).slice(2, 8)}`, line);
      return new Response(null, { status: 204 });
    }
    if (path === "/__probe/logs") {
      const keys = [];
      let cursor;
      do { const page = await env.BUCKET.list({ prefix: "probe/log/", cursor }); keys.push(...page.objects.map((o) => o.key)); cursor = page.truncated ? page.cursor : undefined; } while (cursor);
      keys.sort();
      const lines = [];
      for (const k of keys) { const o = await env.BUCKET.get(k); if (o) lines.push(`${k.slice(10)} ${await o.text()}`); }
      return new Response(lines.join("\n") + "\n", { headers: { "content-type": "text/plain; charset=utf-8", "cache-control": "no-store" } });
    }
    const res = await shroomDefault.fetch(request, env, ctx);
    const nonce = /'nonce-([^']+)'/.exec(res.headers.get("content-security-policy") ?? "")?.[1];
    if (!nonce || !(res.headers.get("content-type") ?? "").startsWith("text/html")) return res;
    const page = (await res.text()).replace("<head>", `<head><script nonce="${nonce}">${LOGGER}</script>`);
    const headers = new Headers(res.headers);
    headers.delete("content-length");
    return new Response(page, { status: res.status, headers });
  },
  scheduled(controller, env, ctx) { return shroomDefault.scheduled(controller, env, ctx); },
};
"#;

