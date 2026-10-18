//! L1 (spec N65): the Mac that made a take lands its voice cuts once the space has heard it
//! whole (N66's transcript beside it, `rec/<id>/ai/transcript.json`), in the record's edit
//! list (N63's `meta.edits`), once — and what it writes is what the real Worker's owner page
//! reads (`worker/src/worker.js`, under node). The lane runs against the L1 fake's bucket,
//! on a clock the test turns.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Duration;

use serde_json::{Value, json};
use shroom_cloudfake::{Fake, Fault, FaultKind, State};
use shroom_storage::s3::{Bucket, Client};
use shroom_storage::sigv4::Credentials;
use shroom_edit::Question;
use shroom_upload::voice::{self, Clock, Mark, Wait};

const ID: &str = "AbCdEfGhIjKlMnOp";
const BUCKET: &str = "shroom-space";
const NOW: u64 = 1_792_329_066_000;

fn space() -> (Fake, Client) {
    let mut st = State::fresh();
    st.buckets.insert(BUCKET.into(), Default::default());
    let fake = Fake::start(st);
    let s3 = Client::new(Bucket {
        endpoint: fake.r2(),
        region: "auto".into(),
        name: BUCKET.into(),
        credentials: Credentials { access_key_id: "k".into(), secret_access_key: "s".into(), session_token: None },
    });
    (fake, s3)
}

/// The record of a take that went live a few seconds ago, with `over` laid on it.
fn record(fake: &Fake, over: Value) {
    let mut m = json!({
        "schema": 1, "id": ID, "state": "live", "createdAt": "2026-10-18T13:10:00Z", "publishedAt": "2026-10-18T13:11:00Z",
        "deletedAt": null, "title": "Setting up shroom", "durationSec": 8.709, "captions": null, "owner": "m1",
        "media": { "hls": format!("rec/{ID}/master.m3u8"), "mp4": format!("rec/{ID}/video.mp4"), "poster": null, "teaser": null, "width": 1920, "height": 1080 },
        "progress": { "done": 9, "total": 9, "updatedAt": "2026-10-18T13:11:05Z" },
    });
    for (k, v) in over.as_object().unwrap() {
        m[k] = v.clone();
    }
    put(fake, &format!("meta/{ID}.json"), m.to_string().as_bytes());
}

fn put(fake: &Fake, key: &str, bytes: &[u8]) {
    fake.with(|s| s.put_object(BUCKET, key, bytes));
}

fn stored(fake: &Fake) -> Value {
    serde_json::from_slice(&fake.with(|s| s.object(BUCKET, &format!("meta/{ID}.json"))).expect("the record")).unwrap()
}

/// One of the POC's Workers AI takes, as the space keeps what it heard (N66).
fn heard(take: &str, done: bool, clock: Option<&str>) -> Vec<u8> {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../shroom-edit/tests/takes").join(take).join("transcript.json");
    let t: Value = serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap();
    let words: Vec<Value> = t["words"].as_array().unwrap().iter().map(|w| json!([w["text"], w["start"], w["end"]])).collect();
    let d = t["duration"].as_f64().unwrap();
    json!({ "v": 1, "model": "@cf/openai/whisper-large-v3-turbo", "language": "en", "duration": d, "heard_through": d, "done": done, "windows": 1, "clock": clock, "words": words })
        .to_string()
        .into_bytes()
}

fn transcript_key() -> String {
    format!("rec/{ID}/ai/transcript.json")
}

/// The test's clock: sleeping moves it on.
struct Turned {
    now: u64,
    slept: Vec<Duration>,
}

impl Turned {
    fn at(now: u64) -> Turned {
        Turned { now, slept: Vec::new() }
    }
}

impl Clock for Turned {
    fn now_ms(&mut self) -> u64 {
        self.now
    }
    fn sleep(&mut self, d: Duration) {
        self.slept.push(d);
        self.now += d.as_millis() as u64;
    }
}

/// A take folder as the Mac keeps it, without its sound: the planner then sets the edges by
/// the words alone.
fn take() -> tempfile::TempDir {
    tempfile::tempdir().unwrap()
}

/// The real Worker's owner-page strip and status over a record — `None` when node isn't on
/// this Mac (the Worker's own suite covers its side then).
fn worker(meta: &Value, now: u64) -> Option<(String, Value)> {
    let worker = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../worker/src/worker.js").canonicalize().ok()?;
    let script = format!(
        "import {{ status, cutsStrip }} from {:?};\nlet s='';process.stdin.on('data',d=>s+=d).on('end',()=>{{const m=JSON.parse(s);console.log(JSON.stringify([cutsStrip(m.edits),status(m,{now},true)]))}});",
        format!("file://{}", worker.display())
    );
    let mut child = Command::new("node").args(["--input-type=module", "-e", &script]).stdin(Stdio::piped()).stdout(Stdio::piped()).spawn().ok()?;
    use std::io::Write;
    child.stdin.take()?.write_all(meta.to_string().as_bytes()).ok()?;
    let out = child.wait_with_output().ok()?;
    assert!(out.status.success(), "node ran the Worker: {}", String::from_utf8_lossy(&out.stderr));
    let v: Value = serde_json::from_slice(&out.stdout).unwrap();
    Some((v[0].as_str().unwrap().to_string(), v[1].clone()))
}

#[test]
fn a_take_heard_whole_gets_its_voice_cuts_once_and_the_owner_page_shows_them() {
    let (fake, s3) = space();
    record(&fake, json!({}));
    put(&fake, &transcript_key(), &heard("wai-en_US-lessac-medium-1-the-example", true, None));
    let dir = take();
    let mut clock = Turned::at(NOW);

    let mark = voice::after_go_live(&s3, ID, dir.path(), Some(true), Wait::default(), &mut clock);
    assert_eq!(mark, Mark::Planned { at: NOW, cues: 1, cuts: "v1+".into() });
    assert_eq!(voice::mark(dir.path()), Some(mark.clone()));
    let m = stored(&fake);
    let edits = &m["edits"];
    assert_eq!(edits["voice"], json!({ "planned_at": NOW, "cues": 1 }));
    assert_eq!(edits["source_duration"], json!(8.709));
    let cuts = edits["cuts"].as_array().unwrap();
    assert_eq!(cuts.len(), 1);
    assert_eq!((cuts[0]["id"].as_str(), cuts[0]["kind"].as_str(), cuts[0]["applied"].as_bool()), (Some("v1"), Some("retake"), Some(true)));
    assert_eq!(cuts[0]["removed"], "Hello, that is Shroom. Shroom cut.");
    assert_eq!(edits["rev"], 1);
    // The rest of the record is as it was.
    assert_eq!((m["title"].as_str(), m["state"].as_str(), m["owner"].as_str()), (Some("Setting up shroom"), Some("live"), Some("m1")));

    // Once per take: a second pass writes nothing — nor one that lost the take's own mark
    // (the Mac quit between the write and the mark), which finds the plan in the record.
    let before = fake.with(|s| s.buckets[BUCKET][&format!("meta/{ID}.json")].clone());
    assert_eq!(voice::after_go_live(&s3, ID, dir.path(), Some(true), Wait::default(), &mut clock), Mark::Planned { at: NOW, cues: 1, cuts: "v1+".into() });
    assert_eq!(voice::look_again(&s3, ID, dir.path(), NOW + 1), Mark::Planned { at: NOW, cues: 1, cuts: "v1+".into() });
    std::fs::remove_file(dir.path().join(voice::MARK)).unwrap();
    assert_eq!(voice::look_again(&s3, ID, dir.path(), NOW + 2), Mark::Planned { at: NOW, cues: 1, cuts: "v1+".into() });
    assert_eq!(fake.with(|s| s.buckets[BUCKET][&format!("meta/{ID}.json")].clone()), before);

    // What the Mac wrote is what the owner page reads: the cut, its words, Put back.
    if let Some((strip, status)) = worker(&m, NOW) {
        assert!(strip.contains(r#"<li class="cut on" data-cut="v1">"#), "{strip}");
        assert!(strip.contains("Hello, that is Shroom. Shroom cut."), "{strip}");
        let tag = status["cuts"].as_str().unwrap_or_default();
        assert!(!tag.is_empty() && tag != "coming" && tag.chars().all(|c| c.is_ascii_alphanumeric()), "{status}");
    }
}

#[test]
fn the_editors_list_stays_beside_the_voice_cuts() {
    let (fake, s3) = space();
    let saved = json!({ "version": 1, "source_duration": 8.709, "in": 0.0, "out": 8.0, "rev": 2,
        "cuts": [{ "id": "m1", "start": 7.0, "end": 7.5, "kind": "manual", "confidence": "high", "applied": true, "removed": "" }] });
    record(&fake, json!({ "edits": saved }));
    put(&fake, &transcript_key(), &heard("wai-en_US-lessac-medium-1-the-example", true, None));
    let dir = take();
    assert_eq!(voice::after_go_live(&s3, ID, dir.path(), Some(true), Wait::default(), &mut Turned::at(NOW)), Mark::Planned { at: NOW, cues: 1, cuts: "v1+".into() });
    let edits = stored(&fake)["edits"].clone();
    let ids: Vec<&str> = edits["cuts"].as_array().unwrap().iter().map(|c| c["id"].as_str().unwrap()).collect();
    assert_eq!(ids, ["m1", "v1"]);
    assert_eq!((edits["out"].as_f64(), edits["rev"].as_u64()), (Some(8.0), Some(3)));
}

#[test]
fn a_space_that_doesnt_listen_is_never_asked() {
    let (fake, s3) = space();
    record(&fake, json!({}));
    let dir = take();
    let mut clock = Turned::at(NOW);
    let mark = voice::after_go_live(&s3, ID, dir.path(), Some(false), Wait::default(), &mut clock);
    assert!(matches!(mark, Mark::Skipped { at: NOW, .. }), "{mark:?}");
    assert!(clock.slept.is_empty());
    assert!(fake.with(|s| s.calls.iter().all(|c| !c.contains("/ai/"))), "the words were looked for");
    assert!(stored(&fake).get("edits").is_none());
}

#[test]
fn words_not_there_yet_are_waited_for_then_left_for_a_later_look() {
    let (fake, s3) = space();
    record(&fake, json!({}));
    let dir = take();
    let mut clock = Turned::at(NOW);
    let wait = Wait { first: Duration::from_secs(3), most: Duration::from_secs(15), until: Duration::from_secs(120) };
    // Nobody said whether the space listens (offline): it's waited for all the same.
    assert_eq!(voice::after_go_live(&s3, ID, dir.path(), None, wait, &mut clock), Mark::Waiting { since: NOW });
    assert_eq!(clock.slept.first(), Some(&Duration::from_secs(3)));
    assert!(clock.slept.iter().all(|d| *d <= Duration::from_secs(15)), "{:?}", clock.slept);
    assert!(clock.now >= NOW + 120_000 && clock.now < NOW + 136_000, "it waited {} ms", clock.now - NOW);
    assert_eq!(voice::mark(dir.path()), Some(Mark::Waiting { since: NOW }));

    // The words come later (a long take, or a free day used up): the sweep's look lands them.
    put(&fake, &transcript_key(), &heard("wai-en_US-lessac-medium-4-no-retake", true, None));
    let later = NOW + 6 * 3_600_000;
    assert_eq!(voice::look_again(&s3, ID, dir.path(), later), Mark::Planned { at: later, cues: 1, cuts: "v1+ v2-".into() });
    let kinds: Vec<String> = stored(&fake)["edits"]["cuts"].as_array().unwrap().iter().map(|c| c["kind"].as_str().unwrap().to_string()).collect();
    assert_eq!(kinds, ["cue-only", "line-before-cue"]);
}

#[test]
fn a_transcript_not_whole_yet_is_never_planned() {
    let (fake, s3) = space();
    record(&fake, json!({}));
    put(&fake, &transcript_key(), &heard("wai-en_US-lessac-medium-1-the-example", false, None));
    let dir = take();
    let wait = Wait { first: Duration::from_secs(3), most: Duration::from_secs(15), until: Duration::from_secs(30) };
    assert_eq!(voice::after_go_live(&s3, ID, dir.path(), Some(true), wait, &mut Turned::at(NOW)), Mark::Waiting { since: NOW });
    assert!(stored(&fake).get("edits").is_none());
}

#[test]
fn a_read_that_fails_is_tried_again() {
    let (fake, s3) = space();
    record(&fake, json!({}));
    put(&fake, &transcript_key(), &heard("wai-en_US-lessac-medium-1-the-example", true, None));
    fake.with(|s| {
        s.faults.push(Fault { kind: FaultKind::Status(503), method: Some("GET".into()), prefix: format!("/s3/{BUCKET}/rec/{ID}/ai/"), left: Some(2) })
    });
    let dir = take();
    let mut clock = Turned::at(NOW);
    assert_eq!(voice::after_go_live(&s3, ID, dir.path(), Some(true), Wait::default(), &mut clock), Mark::Planned { at: clock.now, cues: 1, cuts: "v1+".into() });
    assert!(!clock.slept.is_empty());
}

#[test]
fn a_take_removed_baked_or_heard_on_another_clock_is_left_alone() {
    let cases: [(&str, Value, Option<&str>); 4] = [
        ("removed", json!({ "state": "deleted" }), None),
        ("baked", json!({ "bakedFrom": [{ "at": "2026-10-18T13:20:00Z", "keep": [[0.0, 3.0], [4.0, 8.7]] }] }), None),
        ("another clock", json!({}), Some("2026-10-18T13:20:00Z")),
        ("an edit list this build can't read", json!({ "edits": 42 }), None),
    ];
    for (case, over, clock) in cases {
        let (fake, s3) = space();
        record(&fake, over);
        put(&fake, &transcript_key(), &heard("wai-en_US-lessac-medium-1-the-example", true, clock));
        let before = fake.with(|s| s.object(BUCKET, &format!("meta/{ID}.json")));
        let dir = take();
        let mark = voice::after_go_live(&s3, ID, dir.path(), Some(true), Wait::default(), &mut Turned::at(NOW));
        assert!(matches!(mark, Mark::Skipped { .. }), "{case}: {mark:?}");
        assert_eq!(fake.with(|s| s.object(BUCKET, &format!("meta/{ID}.json"))), before, "{case}: the record was touched");
    }
}

/// A take only the judgment cuts whole: “Click on this link to see it.” said again after the
/// cue in other words. The judge answers as Clef did for it (its recorded judged.json).
fn reworded(fake: &Fake) -> Vec<f64> {
    put(fake, &transcript_key(), &heard("wai-en_US-ryan-medium-9-clean-reworded", true, None));
    let judged: Value = serde_json::from_slice(&std::fs::read(Path::new(env!("CARGO_MANIFEST_DIR")).join("../shroom-edit/tests/takes/wai-en_US-ryan-medium-9-clean-reworded/judged.json")).unwrap()).unwrap();
    serde_json::from_value(judged["answers"][0]["scores"].clone()).unwrap()
}

#[test]
fn the_judgment_reaches_back_where_the_words_alone_dont_and_is_asked_once() {
    let (fake, s3) = space();
    record(&fake, json!({ "durationSec": 13.0 }));
    let scores = reworded(&fake);
    let asked = std::cell::RefCell::new(Vec::<Question>::new());
    let judge = |q: &Question| {
        asked.borrow_mut().push(q.clone());
        voice::Weighed::scored(scores.clone(), Some(700))
    };
    let dir = take();
    let mark = voice::after_go_live_judged(&s3, ID, dir.path(), Some(true), Wait::default(), &mut Turned::at(NOW), &judge, &|_: &str| {});
    assert_eq!(mark, Mark::Planned { at: NOW, cues: 1, cuts: "v1+".into() });
    let cuts = stored(&fake)["edits"]["cuts"].as_array().unwrap().clone();
    assert_eq!(cuts.len(), 1, "{cuts:?}");
    assert_eq!((cuts[0]["kind"].as_str(), cuts[0]["applied"].as_bool(), cuts[0]["removed"].as_str()), (Some("retake"), Some(true), Some("Click on this link to see it. Shroom cut.")));
    let asked = asked.into_inner();
    assert_eq!(asked.len(), 1);
    assert_eq!(asked[0].after, "To get your video, use this link and you'll see it right away.");
    assert_eq!(asked[0].candidates[0], "Click on this link to see it.");
}

#[test]
fn a_judgment_that_doesnt_come_leaves_the_codes_own_plan() {
    let (fake, s3) = space();
    record(&fake, json!({ "durationSec": 13.0 }));
    reworded(&fake);
    let dir = take();
    // The free day used up, a busy model, no answer: the cue goes and the line is offered.
    let mark = voice::after_go_live_judged(&s3, ID, dir.path(), Some(true), Wait::default(), &mut Turned::at(NOW), &|_: &Question| voice::Weighed::unanswered("busy"), &|_: &str| {});
    assert_eq!(mark, Mark::Planned { at: NOW, cues: 1, cuts: "v1+ v2-".into() });
    let kinds: Vec<(String, bool)> = stored(&fake)["edits"]["cuts"].as_array().unwrap().iter().map(|c| (c["kind"].as_str().unwrap().to_string(), c["applied"].as_bool().unwrap())).collect();
    assert_eq!(kinds, [("cue-only".to_string(), true), ("line-before-cue".to_string(), false)]);
}

/// What the lane noted, line by line.
fn notes() -> (std::cell::RefCell<Vec<String>>, impl Fn(&std::cell::RefCell<Vec<String>>, &str)) {
    (std::cell::RefCell::new(Vec::new()), |into: &std::cell::RefCell<Vec<String>>, line: &str| into.borrow_mut().push(line.to_string()))
}

#[test]
fn a_take_planned_leaves_a_line_of_what_was_heard_and_one_per_cue_never_the_words() {
    // What a report shows of a take (Bagrat, 2026-10-18: enough in the log to report a
    // problem): the words' count and length, the looks, the quiet, the cues, the landing's
    // time; then each cue: where, how many starts, the judgment's scores and time, and what was
    // cut where. Never a word said.
    let (fake, s3) = space();
    record(&fake, json!({ "durationSec": 13.0 }));
    let scores = reworded(&fake);
    let (said, push) = notes();
    let note = |l: &str| push(&said, l);
    let dir = take();
    let judge = |_: &Question| voice::Weighed::scored(scores.clone(), Some(700));
    voice::after_go_live_judged(&s3, ID, dir.path(), Some(true), Wait::default(), &mut Turned::at(NOW), &judge, &note);
    let said = said.into_inner();
    assert_eq!(said.len(), 2, "{said:?}");
    // This take's folder holds no sound: the line says so, as it would of a copy gone missing.
    assert!(said[0].starts_with("heard 32 words over 10 s after 1 look; its sound unread (NotFound); 1 cue; landed 1 cut in "), "{}", said[0]);
    assert!(said[1].starts_with("cue 1 at ") && said[1].contains(" starts weighed in 700 ms (") && said[1].contains("→ v1 retake ") && said[1].ends_with(" s cut"), "{}", said[1]);
    let words: Vec<String> = serde_json::from_slice::<Value>(&heard("wai-en_US-ryan-medium-9-clean-reworded", true, None)).unwrap()["words"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|w| w[0].as_str().map(|t| t.trim_matches(|c: char| !c.is_alphabetic()).to_lowercase()))
        .filter(|w| w.len() > 3)
        .collect();
    for line in &said {
        let l = line.to_lowercase();
        assert!(words.iter().all(|w| !l.split(|c: char| !c.is_alphabetic()).any(|x| x == w)), "a word said in the log: {line}");
    }
}

#[test]
fn a_judgment_that_doesnt_come_and_a_landing_that_fails_each_leave_their_line() {
    let (fake, s3) = space();
    record(&fake, json!({ "durationSec": 13.0 }));
    reworded(&fake);
    // The record's write refused once (a credential that lost its power, say): the next look lands it.
    fake.with(|s| s.faults.push(Fault { kind: FaultKind::Status(403), method: Some("PUT".into()), prefix: format!("/s3/{BUCKET}/meta/{ID}"), left: Some(1) }));
    let (said, push) = notes();
    let note = |l: &str| push(&said, l);
    let dir = take();
    let mark = voice::after_go_live_judged(&s3, ID, dir.path(), Some(true), Wait::default(), &mut Turned::at(NOW), &|_: &Question| voice::Weighed::unanswered("busy"), &note);
    assert!(matches!(mark, Mark::Planned { cues: 1, .. }), "{mark:?}");
    let said = said.into_inner();
    assert_eq!(said[0], "landing failed (status 403); looks again");
    assert!(said[1].contains(" after 2 looks; "), "{}", said[1]);
    assert!(said[2].starts_with("cue 1 at ") && said[2].contains("starts, the code's own plan (busy) → v1 cue-only ") && said[2].contains(", v2 line-before-cue ") && said[2].ends_with(" s offered"), "{said:?}");
}

#[test]
fn a_press_on_the_owner_page_shows_in_the_log_once() {
    let (fake, s3) = space();
    record(&fake, json!({ "durationSec": 13.0 }));
    reworded(&fake);
    let dir = take();
    let mark = voice::after_go_live_judged(&s3, ID, dir.path(), Some(true), Wait::default(), &mut Turned::at(NOW), &|_: &Question| voice::Weighed::unanswered("busy"), &|_: &str| {});
    assert_eq!(mark, Mark::Planned { at: NOW, cues: 1, cuts: "v1+ v2-".into() });
    // Cut it too on the offered line, as the Worker's press writes it.
    let press = |id: &str, on: bool| {
        let mut m = stored(&fake);
        for c in m["edits"]["cuts"].as_array_mut().unwrap() {
            if c["id"] == id {
                c["applied"] = json!(on);
            }
        }
        put(&fake, &format!("meta/{ID}.json"), &serde_json::to_vec(&m).unwrap());
    };
    press("v2", true);
    let (said, push) = notes();
    let note = |l: &str| push(&said, l);
    assert_eq!(voice::pressed(&s3, ID, dir.path(), &note), Some(Mark::Planned { at: NOW, cues: 1, cuts: "v1+ v2+".into() }));
    assert_eq!(voice::pressed(&s3, ID, dir.path(), &note), None, "nothing new");
    press("v1", false);
    assert!(voice::pressed(&s3, ID, dir.path(), &note).is_some());
    assert_eq!(said.into_inner(), ["on the page since: v2 cut", "on the page since: v1 put back"]);
}
