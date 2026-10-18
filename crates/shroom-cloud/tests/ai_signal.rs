//! L1: a Mac telling its space a take has another minute up, or is done (N66, DECISIONS
//! B206 b), against a loopback stand-in for the page that answers as scripted and keeps what
//! it was sent. The device key is the door; the setup Mac signs with the parent proof enroll
//! takes. A step at a time, asked again while the space says there's more; an answer of any
//! kind settles it, and only no answer at all keeps it owed.

use std::io::{BufRead, BufReader, Read, Write};
use std::net::TcpListener;
use std::sync::{Arc, Mutex};

use serde_json::{Value, json};
use shroom_cloud::device::DeviceKey;
use shroom_cloud::member::{AiAuth, AiSignal, AiSent, Space, ai_signal, parent_proof};

const HOST: &str = "v.myshroom.workers.dev";
const REC: &str = "AbCdEfGhIjKlMnOp";

#[derive(Debug, Clone)]
struct Got {
    path: String,
    auth: String,
    body: Value,
    raw: Vec<u8>,
}

/// A page that answers each request with the next of `answers` (status, body), then 500s.
fn stand_in(answers: Vec<(u16, Value)>) -> (String, Arc<Mutex<Vec<Got>>>) {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let got = Arc::new(Mutex::new(Vec::new()));
    let seen = got.clone();
    std::thread::spawn(move || {
        let mut answers = answers.into_iter();
        for stream in listener.incoming() {
            let Ok(mut stream) = stream else { break };
            let mut reader = BufReader::new(stream.try_clone().unwrap());
            let mut line = String::new();
            reader.read_line(&mut line).unwrap();
            let path = line.split_whitespace().nth(1).unwrap_or_default().to_string();
            let (mut len, mut auth) = (0usize, String::new());
            loop {
                let mut h = String::new();
                reader.read_line(&mut h).unwrap();
                let h = h.trim_end();
                if h.is_empty() {
                    break;
                }
                let (k, v) = h.split_once(':').unwrap_or((h, ""));
                match k.to_ascii_lowercase().as_str() {
                    "content-length" => len = v.trim().parse().unwrap_or(0),
                    "authorization" => auth = v.trim().to_string(),
                    _ => {}
                }
            }
            let mut raw = vec![0u8; len];
            reader.read_exact(&mut raw).unwrap();
            seen.lock().unwrap().push(Got { path, auth, body: serde_json::from_slice(&raw).unwrap_or(Value::Null), raw });
            let (status, body) = answers.next().unwrap_or((500, json!({"shroom": 1, "error": "space"})));
            let text = body.to_string();
            let _ = write!(stream, "HTTP/1.1 {status} X\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{text}", text.len());
        }
    });
    (base, got)
}

fn key() -> DeviceKey {
    DeviceKey::generate(HOST)
}

#[test]
fn another_minute_is_one_ask_with_the_device_key_and_again_while_theres_more() {
    let (base, got) = stand_in(vec![
        (200, json!({"shroom": 1, "ai": true, "step": "heard", "more": true})),
        (200, json!({"shroom": 1, "ai": true, "step": "heard", "more": false})),
    ]);
    let k = key();
    let sent = ai_signal(&Space::at(HOST, &base), &AiAuth::Device(&k), REC, AiSignal::Through(20));
    assert_eq!(sent, AiSent::On);
    let got = got.lock().unwrap().clone();
    assert_eq!(got.len(), 2, "asked again while the space said there was more");
    for g in &got {
        assert_eq!(g.path, "/_shroom/api/ai");
        assert_eq!(g.body, json!({"rec": REC, "through": 20}));
        assert_eq!(g.auth, k.bearer());
    }
}

#[test]
fn done_is_asked_until_the_take_is_named_with_the_setup_macs_proof() {
    let (base, got) = stand_in(vec![
        (200, json!({"shroom": 1, "ai": true, "step": "heard", "more": true})),
        (200, json!({"shroom": 1, "ai": true, "step": "heard", "more": true})),
        (200, json!({"shroom": 1, "ai": true, "step": "named", "more": false})),
    ]);
    let secret = "a1b2c3d4e5f60718293a4b5c6d7e8f90a1b2c3d4e5f60718293a4b5c6d7e8f90";
    let sent = ai_signal(&Space::at(HOST, &base), &AiAuth::Parent(secret), REC, AiSignal::Done);
    assert_eq!(sent, AiSent::On);
    let got = got.lock().unwrap().clone();
    assert_eq!(got.len(), 3);
    for g in &got {
        assert_eq!(g.body, json!({"rec": REC, "done": true}));
        // The proof enroll takes, over this request: method, host, path, time and body.
        let ts: u64 = g.auth.strip_prefix("ShroomParent ").and_then(|p| p.split_once('.')).map(|(t, _)| t.parse().unwrap()).expect("a parent proof");
        assert_eq!(g.auth, parent_proof(secret, "POST", HOST, "/_shroom/api/ai", ts, &g.raw));
    }
}

#[test]
fn a_space_without_naming_answers_once_and_is_left_be() {
    let (base, got) = stand_in(vec![(200, json!({"shroom": 1, "ai": false}))]);
    assert_eq!(ai_signal(&Space::at(HOST, &base), &AiAuth::Device(&key()), REC, AiSignal::Done), AiSent::Off);
    assert_eq!(got.lock().unwrap().len(), 1);
}

#[test]
fn a_take_waiting_for_the_free_day_is_settled_the_space_keeps_it() {
    let (base, got) = stand_in(vec![(200, json!({"shroom": 1, "ai": true, "later": "daily"}))]);
    assert_eq!(ai_signal(&Space::at(HOST, &base), &AiAuth::Device(&key()), REC, AiSignal::Done), AiSent::On);
    assert_eq!(got.lock().unwrap().len(), 1, "the space's schedule finishes it");
}

#[test]
fn a_take_the_space_doesnt_have_any_more_is_settled_too() {
    for (status, error) in [(404, "absent"), (410, "deleted"), (403, "not-yours")] {
        let (base, _) = stand_in(vec![(status, json!({"shroom": 1, "error": error}))]);
        assert_eq!(ai_signal(&Space::at(HOST, &base), &AiAuth::Device(&key()), REC, AiSignal::Done), AiSent::Settled, "{error}");
    }
}

#[test]
fn no_answer_keeps_it_owed() {
    // Nothing listens here.
    let free = TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", free.local_addr().unwrap());
    drop(free);
    assert_eq!(ai_signal(&Space::at(HOST, &base), &AiAuth::Device(&key()), REC, AiSignal::Done), AiSent::Owed);
    // A page that isn't shroom's, or the space's own trouble, is the same: try again later.
    let (base, _) = stand_in(vec![(503, json!({"shroom": 1, "error": "space", "reason": "storage"}))]);
    assert_eq!(ai_signal(&Space::at(HOST, &base), &AiAuth::Device(&key()), REC, AiSignal::Done), AiSent::Owed);
}

#[test]
fn it_never_asks_without_end() {
    let (base, got) = stand_in((0..40).map(|_| (200, json!({"shroom": 1, "ai": true, "step": "heard", "more": true}))).collect());
    assert_eq!(ai_signal(&Space::at(HOST, &base), &AiAuth::Device(&key()), REC, AiSignal::Done), AiSent::On);
    assert!(got.lock().unwrap().len() <= 12, "the space's schedule finishes what's left");
}
