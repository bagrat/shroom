//! The voice-cut lane (spec N65, DECISIONS B201): once a take this Mac made is live, it waits
//! for the space's words and lands the take's voice cuts in its record
//! (`shroom_upload::voice`), on a thread of its own, so the upload never waits on it. It
//! goes through the take's own credential (`cred.json`: its record and its files). A take
//! whose words came late is looked at again by the sweep: two minutes after launch, then
//! every six hours awake, for two days after its go-live. Nothing on screen answers it: the
//! owner page shows the cuts (N65 (5)).

use std::path::Path;
use std::time::Duration;

use shroom_cloud::device::CredFile;
use shroom_core::world::SpaceIdentity;
use shroom_storage::s3::Client;
use shroom_upload::voice::{self, Clock, Mark, Wait, Wall};

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
        let space = rt.home.read_json::<SpaceIdentity>("space.json").and_then(|id| rt.space(&id.host));
        let coming = space.and_then(|s| s.cuts_coming(&rec));
        let m = voice::after_go_live(&s3, &rec, &dir, coming, Wait::default(), &mut Wall);
        say(&rt, &rec, &m);
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
/// `publish.json`; past two days it stops waiting.
fn sweep(rt: &Runtime) {
    let now = Wall.now_ms();
    for dir in dirs(&rt.home.takes()) {
        let Some(Mark::Waiting { since }) = voice::mark(&dir) else { continue };
        if classify(&dir) != Kind::Done {
            continue;
        }
        let Some(p) = super::home::read_json::<PublishJson>(&dir.join("publish.json")) else { continue };
        let m = if now.saturating_sub(since) > LOOK_FOR_MS {
            voice::give_up(&dir, now)
        } else if let Some(s3) = client(rt, &dir) {
            voice::look_again(&s3, &p.rec, &dir, now)
        } else {
            continue;
        };
        say(rt, &p.rec, &m);
    }
}

fn say(rt: &Runtime, rec: &str, m: &Mark) {
    let rec = super::short(rec);
    match m {
        Mark::Planned { cues, .. } => rt.log(&format!("voice: {rec} planned ({cues} cue{})", if *cues == 1 { "" } else { "s" })),
        Mark::Skipped { why, .. } => rt.log(&format!("voice: {rec} left as it is ({why})")),
        Mark::Waiting { .. } => rt.log(&format!("voice: {rec} waits for its words")),
    }
}
