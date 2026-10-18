//! Voice cuts (spec N65): once the space has heard a take whole — its transcript beside the
//! take, `rec/<id>/ai/transcript.json` (N66) — the Mac that made it finds each spoken cue and
//! lands the cuts in the record's edit list (N63's `meta.edits`), once per take, by a
//! read-modify-write under `If-Match` that changes nothing else in the record. The quiet a
//! cut's edges are set in comes from the take's own sound, still on this Mac. What the lane
//! came to is kept in the take (`voice.json`), so a take is planned once and one whose words
//! came late is looked at again.

use std::path::Path;
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use shroom_edit::{Answer, Levels, Options, Question, Saved, Silence, Transcript, questions};
use shroom_storage::s3::{Client, S3Error};

use crate::keys;
use crate::upload::{UploadError, with_retry};

/// What the lane came to, kept in the take folder.
pub const MARK: &str = "voice.json";

/// The space's words for a take (N66), beside it.
pub fn transcript_key(id: &str) -> String {
    format!("{}ai/transcript.json", keys::prefix(id))
}

/// The space's transcript of a take, as N66 keeps it: the words heard on the recording's
/// clock, written once, whole.
#[derive(Debug, Clone, Deserialize)]
pub struct Heard {
    #[serde(default)]
    pub done: bool,
    #[serde(default)]
    pub duration: f64,
    /// `[text, start, end]`, seconds on the recording's clock.
    #[serde(default)]
    pub words: Vec<(String, f64, f64)>,
    /// The bake whose clock the words are on (N66, after N63's bake): `None` on the
    /// recording's own.
    #[serde(default)]
    pub clock: Option<String>,
}

/// What a take's voice lane came to.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "kebab-case")]
pub enum Mark {
    /// Its cues are planned and in the record (`at`: when, by the record's mark).
    Planned { at: u64, cues: u32 },
    /// Never to be planned: the space doesn't listen, or the take is gone, baked or
    /// unreadable to this build.
    Skipped { at: u64, why: String },
    /// Its words weren't there yet: a later look lands them.
    Waiting { since: u64 },
}

/// The take's mark, if the lane has been at it.
pub fn mark(dir: &Path) -> Option<Mark> {
    serde_json::from_slice(&std::fs::read(dir.join(MARK)).ok()?).ok()
}

fn set_mark(dir: &Path, m: Mark) -> Mark {
    let tmp = dir.join(format!("{MARK}.tmp"));
    if std::fs::write(&tmp, serde_json::to_vec(&m).expect("a mark serializes")).is_ok() {
        let _ = std::fs::rename(&tmp, dir.join(MARK));
    }
    m
}

/// How long a take waits for its words after its go-live, and how often it looks: the
/// first look again after `first`, each wait doubling up to `most`, until `until`.
#[derive(Debug, Clone, Copy)]
pub struct Wait {
    pub first: Duration,
    pub most: Duration,
    pub until: Duration,
}

impl Default for Wait {
    /// The space hears a take as it uploads, so its words are whole seconds after its
    /// go-live (N66's last window and naming); a long take's schedule can take minutes.
    fn default() -> Self {
        Wait { first: Duration::from_secs(3), most: Duration::from_secs(15), until: Duration::from_secs(15 * 60) }
    }
}

/// The lane's time, turned by a test.
pub trait Clock {
    fn now_ms(&mut self) -> u64;
    fn sleep(&mut self, d: Duration);
}

/// The wall clock.
pub struct Wall;

impl Clock for Wall {
    fn now_ms(&mut self) -> u64 {
        std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_millis() as u64).unwrap_or(0)
    }
    fn sleep(&mut self, d: Duration) {
        std::thread::sleep(d);
    }
}

/// The judgment above the planner (DECISIONS B202): Clef's score per candidate start for one
/// question, on the space's own account, or `None` when it can't answer now — then that cue
/// keeps the code's own plan.
pub type Judge<'a> = &'a dyn Fn(&Question) -> Option<Vec<f64>>;

/// No judgment: the code's own plan.
fn unjudged(_: &Question) -> Option<Vec<f64>> {
    None
}

/// Right after a take's go-live, from the Mac that made it: waits for the space's words
/// and lands the take's voice cuts. `coming`: the space's own word on whether it hears the
/// take (its status, `cuts: "coming"`) — `Some(false)` ends it at once, unasked; `None`
/// (not reachable) waits all the same. A take whose words don't come in time is left
/// `Waiting`, for [`look_again`]. The code's own plan: [`after_go_live_judged`] with no
/// judgment.
pub fn after_go_live(s3: &Client, id: &str, dir: &Path, coming: Option<bool>, wait: Wait, clock: &mut impl Clock) -> Mark {
    after_go_live_judged(s3, id, dir, coming, wait, clock, &unjudged)
}

/// [`after_go_live`], each cue's plan weighed by `judge` (asked once per question).
pub fn after_go_live_judged(s3: &Client, id: &str, dir: &Path, coming: Option<bool>, wait: Wait, clock: &mut impl Clock, judge: Judge) -> Mark {
    let since = match mark(dir) {
        Some(Mark::Waiting { since }) => since,
        Some(done) => return done,
        None => clock.now_ms(),
    };
    if coming == Some(false) {
        return set_mark(dir, Mark::Skipped { at: since, why: "the space doesn't listen".into() });
    }
    set_mark(dir, Mark::Waiting { since });
    let deadline = clock.now_ms().saturating_add(wait.until.as_millis() as u64);
    let mut memo = Memo::default();
    let mut pause = wait.first;
    loop {
        if let Some(m) = look(s3, id, dir, clock.now_ms(), &mut memo, judge) {
            return set_mark(dir, m);
        }
        if clock.now_ms() >= deadline {
            return Mark::Waiting { since };
        }
        clock.sleep(pause);
        pause = (pause * 2).min(wait.most);
    }
}

/// A later look at a take still waiting for its words (the sweep): one read of them, and
/// the cuts landed if they're whole. The code's own plan: [`look_again_judged`] with no
/// judgment.
pub fn look_again(s3: &Client, id: &str, dir: &Path, now_ms: u64) -> Mark {
    look_again_judged(s3, id, dir, now_ms, &unjudged)
}

/// [`look_again`], each cue's plan weighed by `judge`.
pub fn look_again_judged(s3: &Client, id: &str, dir: &Path, now_ms: u64, judge: Judge) -> Mark {
    let since = match mark(dir) {
        Some(Mark::Waiting { since }) => since,
        Some(done) => return done,
        None => now_ms,
    };
    let m = look(s3, id, dir, now_ms, &mut Memo::default(), judge).unwrap_or(Mark::Waiting { since });
    set_mark(dir, m)
}

/// What one lane has read and been told, so a retry neither reads the take's sound again nor
/// asks the judgment twice.
#[derive(Default)]
struct Memo {
    sound: Option<Sound>,
    answers: Option<Vec<Answer>>,
}

/// The take's quiet and its loudness, as the planner reads them.
#[derive(Clone)]
struct Sound {
    silences: Vec<Silence>,
    levels: Option<Levels>,
}

/// A take whose words never came in the time it was looked for: never looked for again.
pub fn give_up(dir: &Path, now_ms: u64) -> Mark {
    match mark(dir) {
        Some(Mark::Waiting { .. }) | None => set_mark(dir, Mark::Skipped { at: now_ms, why: "its words never came".into() }),
        Some(done) => done,
    }
}

/// The space's words for `id`, when they're whole. `None`: not yet (not there, not whole,
/// or not reachable now).
fn whole_words(s3: &Client, id: &str) -> Result<Option<Heard>, &'static str> {
    match s3.get(&transcript_key(id)) {
        Ok(Some(bytes)) => match serde_json::from_slice::<Heard>(&bytes) {
            Ok(h) => Ok(h.done.then_some(h)),
            Err(_) => Err("its words are in a form this build can't read"),
        },
        Ok(None) | Err(_) => Ok(None),
    }
}

/// One look: `Some` when the take is planned or never will be, `None` to look again later.
fn look(s3: &Client, id: &str, dir: &Path, now: u64, memo: &mut Memo, judge: Judge) -> Option<Mark> {
    let h = match whole_words(s3, id) {
        Ok(Some(h)) => h,
        Ok(None) => return None,
        Err(why) => return Some(Mark::Skipped { at: now, why: why.into() }),
    };
    if h.clock.is_some() {
        return Some(Mark::Skipped { at: now, why: "its words are on a clock a bake replaced".into() });
    }
    let sound = memo.sound.get_or_insert_with(|| sound(dir)).clone();
    let mut t = Transcript::heard(h.duration, &h.words, sound.silences);
    if let Some(levels) = sound.levels {
        t = t.with_levels(levels);
    }
    let answers = memo.answers.get_or_insert_with(|| {
        questions(&t, Options::default()).iter().filter_map(|q| judge(q).map(|scores| Answer { cue: q.cue, scores })).collect()
    });
    match land(s3, id, &t, now, answers) {
        Ok(Landed::Planned { at, cues }) => Some(Mark::Planned { at, cues }),
        Ok(Landed::Refused(why)) => Some(Mark::Skipped { at: now, why: why.into() }),
        Ok(Landed::NotLive) | Err(_) => None,
    }
}

/// The quiet and the loudness of the take's own sound (its single-file copy), or none known.
fn sound(dir: &Path) -> Sound {
    match shroom_encode::quiet::read(&dir.join(shroom_encode::files::PREVIEW)) {
        Ok(q) => Sound {
            silences: q.silences.into_iter().map(|(start, end)| Silence { start, end }).collect(),
            levels: (!q.db.is_empty()).then(|| Levels { step: shroom_encode::quiet::STEP, db: q.db }),
        },
        Err(_) => Sound { silences: Vec::new(), levels: None },
    }
}

/// What landing a take's cues came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Landed {
    /// In the record: planned now, or planned already (`at`: the record's own mark).
    Planned { at: u64, cues: u32 },
    /// Not live yet: try again later.
    NotLive,
    /// Never to be landed, and the record left as it was.
    Refused(&'static str),
}

/// Lands the take's planned cues in its record: the editor's list read from `edits` (its
/// fields this build doesn't know kept), [`shroom_edit::land_with`]'s result laid over it
/// (the plan weighed by the judgment's `answers`), and nothing else in the record touched.
/// A record already planned is not rewritten.
pub fn land(s3: &Client, id: &str, t: &Transcript, now_ms: u64, answers: &[Answer]) -> Result<Landed, UploadError> {
    for _ in 0..8 {
        let Some((bytes, etag)) = with_retry(|| s3.get_with_etag(&keys::meta(id)))? else {
            return Ok(Landed::Refused("the take has no record"));
        };
        let Ok(Value::Object(mut meta)) = serde_json::from_slice::<Value>(&bytes) else {
            return Ok(Landed::Refused("its record is in a form this build can't read"));
        };
        match meta.get("state").and_then(Value::as_str) {
            Some("live") => {}
            Some("deleted") => return Ok(Landed::Refused("the take was removed")),
            _ => return Ok(Landed::NotLive),
        }
        if meta.get("bakedFrom").is_some_and(|b| !b.is_null() && b.as_array().is_none_or(|a| !a.is_empty())) {
            return Ok(Landed::Refused("its edit became the video"));
        }
        let original: Option<Map<String, Value>> = match meta.get("edits") {
            None | Some(Value::Null) => None,
            Some(Value::Object(o)) => Some(o.clone()),
            Some(_) => return Ok(Landed::Refused("its edit list is in a form this build can't read")),
        };
        let saved = match &original {
            None => None,
            Some(o) => match serde_json::from_value::<Saved>(Value::Object(o.clone())) {
                Ok(s) => Some(s),
                Err(_) => return Ok(Landed::Refused("its edit list is in a form this build can't read")),
            },
        };
        if let Some(v) = saved.as_ref().and_then(|s| s.list.voice) {
            return Ok(Landed::Planned { at: v.planned_at, cues: v.cues });
        }
        let length = meta.get("durationSec").and_then(Value::as_f64).filter(|d| *d > 0.0).unwrap_or(t.duration);
        let Some(next) = shroom_edit::land_with(saved, t, length, now_ms, answers) else {
            return Ok(Landed::Planned { at: now_ms, cues: 0 });
        };
        let cues = next.list.voice.map_or(0, |v| v.cues);
        let mut edits = original.unwrap_or_default();
        if let Value::Object(fresh) = serde_json::to_value(&next).expect("an edit list serializes") {
            edits.extend(fresh);
        }
        meta.insert("edits".into(), Value::Object(edits));
        let body = serde_json::to_vec(&Value::Object(meta)).expect("a record serializes");
        match with_retry(|| s3.put_if_match(&keys::meta(id), &body, "application/json", &etag)) {
            Ok(true) => return Ok(Landed::Planned { at: now_ms, cues }),
            Ok(false) => continue,
            Err(S3Error::Status { status: 404, .. }) => return Ok(Landed::Refused("the take has no record")),
            Err(e) => return Err(e.into()),
        }
    }
    Err(UploadError::Incomplete("the record kept changing under us".into()))
}
