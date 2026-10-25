//! The voice-cut lane (spec N65, DECISIONS B201): once a take this Mac made is live, it waits
//! for the space's words and lands the take's voice cuts in its record
//! (`shroom_upload::voice`), on a thread of its own, so the upload never waits on it. It
//! goes through the take's own credential (`cred.json`: its record and its files). A take
//! whose words came late is looked at again by the sweep: two minutes after launch, then
//! every six hours awake, for two days after its go-live. Each cue's plan is weighed by the
//! judgment above the planner, asked of the space with this Mac's key (DECISIONS B202); when
//! it can't answer, the code's own plan stands. Nothing on screen answers it: the owner page
//! shows the cuts (N65 (5)). Each step leaves a line in the run log under the take's short id
//! — what was heard, each cue's judgment and cuts, a press on the owner page — never a word
//! said, so a report shows what the lane did (Bagrat, 2026-10-18).

use std::path::Path;
use std::time::Duration;

use shroom_cloud::device::{CredFile, DeviceKey};
use shroom_cloud::member::Space;
use shroom_core::event::{EditorNews, Event};
use shroom_core::world::{RecId, SpaceIdentity};
use shroom_edit::Question;
use shroom_storage::s3::Client;
use shroom_upload::voice::{self, Clock, Mark, Wait, Wall, Weighed};

use super::Runtime;
use super::takes::{Kind, PublishJson, classify, dirs};

/// The sweep's first look after launch, and every one after.
const SWEEP_FIRST: Duration = Duration::from_secs(120);
const SWEEP_EVERY: Duration = Duration::from_secs(6 * 3600);
/// A take's words are looked for this long after its go-live, then never again.
const LOOK_FOR_MS: u64 = 2 * 24 * 3600 * 1000;

/// The take's own credential as a bucket client.
fn client(rt: &Runtime, dir: &Path) -> Option<Client> {
    let identity: SpaceIdentity = rt.home.read_json("space.json")?;
    let cred: CredFile = super::home::read_json(&dir.join("cred.json"))?;
    Some(super::uploader::client(rt, &identity, &cred))
}

/// The space and this Mac's key for it: what the judgment is asked through.
struct Door {
    space: Option<Space>,
    key: Option<DeviceKey>,
}

fn door(rt: &Runtime) -> Door {
    let identity = rt.home.read_json::<SpaceIdentity>("space.json");
    Door {
        space: identity.as_ref().and_then(|id| rt.space(&id.host)),
        key: identity.and_then(|id| super::member::device_key(rt).filter(|k| k.host == id.host)),
    }
}

/// The judgment above the planner for `rec`'s cues (DECISIONS B202): a score per candidate
/// start and how long the space took, or why it can't answer — no space, no key for it, or
/// the space can't answer now — and the cue keeps the code's own plan. The lane notes it.
fn judgment<'a>(door: &'a Door, rec: &'a str) -> impl Fn(&Question) -> Weighed + 'a {
    move |q: &Question| match (door.space.as_ref(), door.key.as_ref()) {
        (Some(s), Some(k)) => match s.judge(k, rec, q.cue, &q.after, &q.candidates) {
            Ok((scores, ms)) => Weighed::scored(scores, ms),
            Err(why) => Weighed::unanswered(why),
        },
        (None, _) => Weighed::unanswered("no space"),
        (_, None) => Weighed::unanswered("no key for the space"),
    }
}

/// The lane's lines, in the run log under the take's short id.
fn noting<'a>(rt: &'a Runtime, rec: &'a str) -> impl Fn(&str) + 'a {
    let short = super::short(rec);
    move |line: &str| rt.log(&format!("voice: {short} {line}"))
}

/// Right after `rec`'s go-live, from the uploader (its single-file copy and teaser still to
/// send): the take's lane, on its own thread. The space is asked once whether it hears the
/// take.
pub(crate) fn after_go_live(rt: &Runtime, dir: &Path, rec: &str) {
    if voice::mark(dir).is_some_and(|m| !matches!(m, Mark::Waiting { .. })) {
        return;
    }
    let (lane, dir, rec) = (rt.clone(), dir.to_path_buf(), rec.to_string());
    let spawned = std::thread::Builder::new().name("shroom-voice".into()).spawn(move || {
        let rt = lane;
        let Some(s3) = client(&rt, &dir) else {
            rt.log(&format!("voice: {} has no credential to read its words with", super::short(&rec)));
            return;
        };
        let door = door(&rt);
        let coming = door.space.as_ref().and_then(|s| s.cuts_coming(&rec));
        let m = voice::after_go_live_judged(&s3, &rec, &dir, coming, Wait::default(), &mut Wall, &judgment(&door, &rec), &noting(&rt, &rec));
        // Waiting, the lane has said how long it looked.
        if !matches!(m, Mark::Waiting { .. }) {
            say(&rt, &rec, &m);
        }
    });
    if spawned.is_err() {
        rt.log("voice: the lane didn't start");
    }
}

/// The sweep, for the life of the app.
pub(crate) fn spawn_sweep(rt: Runtime) {
    let _ = std::thread::Builder::new().name("shroom-voice-sweep".into()).spawn(move || {
        std::thread::sleep(SWEEP_FIRST);
        loop {
            sweep(&rt);
            std::thread::sleep(SWEEP_EVERY);
        }
    });
}

/// One look at every take still waiting for its words: uploaded, its record's id in its
/// `publish.json`; past two days it stops waiting. A take planned in those two days has its
/// cuts read back, so a press on the owner page since shows in the log.
fn sweep(rt: &Runtime) {
    let now = Wall.now_ms();
    let door = door(rt);
    for dir in dirs(&rt.home.takes()) {
        let since = match voice::mark(&dir) {
            Some(Mark::Waiting { since }) => since,
            Some(Mark::Planned { at, .. }) if now.saturating_sub(at) <= LOOK_FOR_MS => {
                let Some(p) = super::home::read_json::<PublishJson>(&dir.join("publish.json")) else { continue };
                if let Some(s3) = client(rt, &dir) {
                    if voice::pressed(&s3, &p.rec, &dir, &noting(rt, &p.rec)).is_some() {
                        landed(rt, &p.rec);
                    }
                }
                continue;
            }
            _ => continue,
        };
        if classify(&dir) != Kind::Done {
            continue;
        }
        let Some(p) = super::home::read_json::<PublishJson>(&dir.join("publish.json")) else { continue };
        let m = if now.saturating_sub(since) > LOOK_FOR_MS {
            voice::give_up(&dir, now)
        } else if let Some(s3) = client(rt, &dir) {
            voice::look_again_judged(&s3, &p.rec, &dir, now, &judgment(&door, &p.rec), &noting(rt, &p.rec))
        } else {
            continue;
        };
        say(rt, &p.rec, &m);
    }
}

/// Cuts landed on `rec`, or changed: the core makes its edited copy at the next upkeep,
/// due at once, so what viewers get leaves the cuts out without waiting for the upkeep's turn.
fn landed(rt: &Runtime, rec: &str) {
    rt.post(Event::Editor(EditorNews::CutsLanded { rec: RecId(rec.to_string()) }));
}

/// What the lane came to, when its own lines haven't said it: a take planned has (what was
/// heard, landed and cut).
fn say(rt: &Runtime, rec: &str, m: &Mark) {
    if matches!(m, Mark::Planned { cuts, .. } if !cuts.is_empty()) {
        landed(rt, rec);
    }
    let rec = super::short(rec);
    match m {
        Mark::Planned { .. } => {}
        Mark::Skipped { why, .. } => rt.log(&format!("voice: {rec} left as it is ({why})")),
        Mark::Waiting { .. } => rt.log(&format!("voice: {rec} waits for its words")),
    }
}
