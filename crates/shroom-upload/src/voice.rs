//! Voice cuts (spec N65): once the space has heard a take whole — its transcript beside the
//! take, `rec/<id>/ai/transcript.json` (N66) — the Mac that made it finds each spoken cue and
//! lands the cuts in the record's edit list (N63's `meta.edits`), once per take, by a
//! read-modify-write under `If-Match` that changes nothing else in the record. The quiet a
//! cut's edges are set in comes from the take's own sound, still on this Mac. What the lane
//! came to is kept in the take (`voice.json`), so a take is planned once and one whose words
//! came late is looked at again. Each step leaves one line for the app's log ([`Note`]):
//! ids, times, counts and scores, never the words said.

use std::path::Path;
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use shroom_edit::{Answer, Cut, CutKind, Levels, Options, Question, Saved, Silence, Transcript, find_cues, questions};
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
    /// Its cues are planned and in the record (`at`: when, by the record's mark). `cuts`:
    /// each voice cut's id and whether it plays cut (`v1+ v2-`), as last seen in the record,
    /// so a press on the owner page shows in the log ([`pressed`]).
    Planned {
        at: u64,
        cues: u32,
        #[serde(default, skip_serializing_if = "String::is_empty")]
        cuts: String,
    },
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

/// What the judgment above the planner said for one question (DECISIONS B202): Clef's score
/// per candidate start and how long the space took, or why it couldn't answer now — then that
/// cue keeps the code's own plan. `why` is a word or two, never the words said.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Weighed {
    pub scores: Option<Vec<f64>>,
    pub ms: Option<u64>,
    pub why: Option<String>,
}

impl Weighed {
    pub fn scored(scores: Vec<f64>, ms: Option<u64>) -> Weighed {
        Weighed { scores: Some(scores), ms, why: None }
    }
    pub fn unanswered(why: impl Into<String>) -> Weighed {
        Weighed { scores: None, ms: None, why: Some(why.into()) }
    }
}

/// The judgment, asked once per question on the space's own account.
pub type Judge<'a> = &'a dyn Fn(&Question) -> Weighed;

/// Where the lane's lines go (the app's run log, under the take's short id): one per step.
pub type Note<'a> = &'a dyn Fn(&str);

/// No judgment: the code's own plan.
fn unjudged(_: &Question) -> Weighed {
    Weighed::unanswered("no judgment")
}

fn unnoted(_: &str) {}

/// Right after a take's go-live, from the Mac that made it: waits for the space's words
/// and lands the take's voice cuts. `coming`: the space's own word on whether it hears the
/// take (its status, `cuts: "coming"`) — `Some(false)` ends it at once, unasked; `None`
/// (not reachable) waits all the same. A take whose words don't come in time is left
/// `Waiting`, for [`look_again`]. The code's own plan: [`after_go_live_judged`] with no
/// judgment.
pub fn after_go_live(s3: &Client, id: &str, dir: &Path, coming: Option<bool>, wait: Wait, clock: &mut impl Clock) -> Mark {
    after_go_live_judged(s3, id, dir, coming, wait, clock, &unjudged, &unnoted)
}

/// [`after_go_live`], each cue's plan weighed by `judge` (asked once per question), each step
/// noted.
#[allow(clippy::too_many_arguments)]
pub fn after_go_live_judged(s3: &Client, id: &str, dir: &Path, coming: Option<bool>, wait: Wait, clock: &mut impl Clock, judge: Judge, note: Note) -> Mark {
    let since = match mark(dir) {
        Some(Mark::Waiting { since }) => since,
        Some(done) => return done,
        None => clock.now_ms(),
    };
    if coming == Some(false) {
        return set_mark(dir, Mark::Skipped { at: since, why: "the space doesn't listen".into() });
    }
    set_mark(dir, Mark::Waiting { since });
    let started = clock.now_ms();
    let deadline = started.saturating_add(wait.until.as_millis() as u64);
    let mut memo = Memo::default();
    let mut pause = wait.first;
    loop {
        if let Some(m) = look(s3, id, dir, clock.now_ms(), &mut memo, judge, note) {
            return set_mark(dir, m);
        }
        if clock.now_ms() >= deadline {
            let waited = clock.now_ms().saturating_sub(started) / 1000;
            note(&format!("{} after {} looks in {waited} s", if memo.heard { "not landed" } else { "no words yet" }, memo.looks));
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
    look_again_judged(s3, id, dir, now_ms, &unjudged, &unnoted)
}

/// [`look_again`], each cue's plan weighed by `judge`, each step noted.
pub fn look_again_judged(s3: &Client, id: &str, dir: &Path, now_ms: u64, judge: Judge, note: Note) -> Mark {
    let since = match mark(dir) {
        Some(Mark::Waiting { since }) => since,
        Some(done) => return done,
        None => now_ms,
    };
    let m = look(s3, id, dir, now_ms, &mut Memo::default(), judge, note).unwrap_or(Mark::Waiting { since });
    set_mark(dir, m)
}

/// Each voice cut's id and whether it plays cut, as a mark keeps them: `v1+ v2-`.
fn states(cuts: &[Cut]) -> String {
    cuts.iter()
        .filter(|c| c.kind != CutKind::Manual)
        .map(|c| format!("{}{}", c.id, if c.applied { '+' } else { '-' }))
        .collect::<Vec<_>>()
        .join(" ")
}

/// A planned take's cuts as the owner page left them (the sweep's look, while it looks for
/// words): a press there since the last look — Put back, Cut again, Cut it too — is noted
/// (`v2 put back`) and kept in the mark. `None`: nothing new, or the record unreachable now.
pub fn pressed(s3: &Client, id: &str, dir: &Path, note: Note) -> Option<Mark> {
    let Some(Mark::Planned { at, cues, cuts }) = mark(dir) else { return None };
    let bytes = with_retry(|| s3.get(&keys::meta(id))).ok().flatten()?;
    let meta: Value = serde_json::from_slice(&bytes).ok()?;
    let saved: Saved = serde_json::from_value(meta.get("edits")?.clone()).ok()?;
    let now = states(&saved.list.cuts);
    if now == cuts {
        return None;
    }
    let read = |s: &str| s.split_whitespace().filter_map(|w| Some((w.get(..w.len() - 1)?.to_string(), w.ends_with('+')))).collect::<Vec<_>>();
    let was = read(&cuts);
    let changed: Vec<String> = read(&now)
        .into_iter()
        .filter(|(id, on)| was.iter().find(|(w, _)| w == id).map(|(_, o)| o) != Some(on))
        .map(|(id, on)| format!("{id} {}", if on { "cut" } else { "put back" }))
        .collect();
    if !changed.is_empty() {
        note(&format!("on the page since: {}", changed.join(", ")));
    }
    Some(set_mark(dir, Mark::Planned { at, cues, cuts: now }))
}

/// What one lane has read and been told, so a retry neither reads the take's sound again nor
/// asks the judgment twice: each question's cue, its candidate count and what was said.
#[derive(Default)]
struct Memo {
    sound: Option<Sound>,
    weighed: Option<Vec<(usize, usize, Weighed)>>,
    looks: u32,
    heard: bool,
}

/// The take's quiet and its loudness, as the planner reads them; why there are none.
#[derive(Clone)]
struct Sound {
    silences: Vec<Silence>,
    levels: Option<Levels>,
    unread: Option<String>,
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
/// A take planned now leaves two kinds of line: what was heard and landed, and one per cue
/// (where it was, what the judgment said, what was cut).
fn look(s3: &Client, id: &str, dir: &Path, now: u64, memo: &mut Memo, judge: Judge, note: Note) -> Option<Mark> {
    memo.looks += 1;
    let h = match whole_words(s3, id) {
        Ok(Some(h)) => h,
        Ok(None) => return None,
        Err(why) => return Some(Mark::Skipped { at: now, why: why.into() }),
    };
    if h.clock.is_some() {
        return Some(Mark::Skipped { at: now, why: "its words are on a clock a bake replaced".into() });
    }
    memo.heard = true;
    let sound = memo.sound.get_or_insert_with(|| sound(dir)).clone();
    let quiet = match &sound.unread {
        Some(why) => format!("its sound unread ({why})"),
        None => format!("{} quiet stretches{}", sound.silences.len(), if sound.levels.is_some() { "" } else { ", no loudness" }),
    };
    let mut t = Transcript::heard(h.duration, &h.words, sound.silences);
    if let Some(levels) = sound.levels {
        t = t.with_levels(levels);
    }
    let weighed = memo.weighed.get_or_insert_with(|| questions(&t, Options::default()).iter().map(|q| (q.cue, q.candidates.len(), judge(q))).collect()).clone();
    let answers: Vec<Answer> = weighed.iter().filter_map(|(cue, _, w)| Some(Answer { cue: *cue, scores: w.scores.clone()? })).collect();
    let began = Instant::now();
    match land(s3, id, &t, now, &answers) {
        Ok(Landed::Planned { at, cues, cuts, fresh }) => {
            if fresh {
                let found = find_cues(&t.words);
                note(&format!(
                    "heard {} words over {:.0} s after {} look{}; {quiet}; {}; landed {} cut{} in {} ms",
                    t.words.len(),
                    t.duration,
                    memo.looks,
                    if memo.looks == 1 { "" } else { "s" },
                    match found.len() {
                        0 => "no cue".to_string(),
                        1 => "1 cue".to_string(),
                        n => format!("{n} cues"),
                    },
                    cuts.iter().filter(|c| c.kind != CutKind::Manual).count(),
                    if cuts.len() == 1 { "" } else { "s" },
                    began.elapsed().as_millis()
                ));
                for (n, cue) in found.iter().enumerate() {
                    let judged = match weighed.iter().find(|(c, _, _)| *c == n) {
                        Some((_, k, Weighed { scores: Some(scores), ms, .. })) => format!(
                            "{k} starts weighed{} ({})",
                            ms.map_or(String::new(), |ms| format!(" in {ms} ms")),
                            scores.iter().map(|x| format!("{x:.2}")).collect::<Vec<_>>().join(" ")
                        ),
                        Some((_, k, w)) => format!("{k} starts, the code's own plan ({})", w.why.as_deref().unwrap_or("no answer")),
                        None => "nothing said after it".to_string(),
                    };
                    let made: Vec<String> = cuts
                        .iter()
                        .filter(|c| c.kind != CutKind::Manual && c.start <= cue.start + 0.05 && c.end >= cue.end - 0.05)
                        .map(|c| {
                            let kind = serde_json::to_value(c.kind).ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                            format!("{} {kind} {:.2}–{:.2} s {}", c.id, c.start, c.end, if c.applied { "cut" } else { "offered" })
                        })
                        .collect();
                    note(&format!("cue {} at {:.1} s: {judged} → {}", n + 1, cue.start, if made.is_empty() { "nothing cut".to_string() } else { made.join(", ") }));
                }
            } else {
                note(&format!("already planned ({cues} cue{})", if cues == 1 { "" } else { "s" }));
            }
            Some(Mark::Planned { at, cues, cuts: states(&cuts) })
        }
        Ok(Landed::Refused(why)) => Some(Mark::Skipped { at: now, why: why.into() }),
        Ok(Landed::NotLive) => None,
        Err(e) => {
            note(&format!("landing failed ({}); looks again", failure(&e)));
            None
        }
    }
}

/// A landing's failure in a word or two: a status, never an address.
fn failure(e: &UploadError) -> String {
    match e {
        UploadError::S3(S3Error::Status { status, .. }) => format!("status {status}"),
        UploadError::Incomplete(_) => "the record kept changing".into(),
        _ => "no answer".into(),
    }
}

/// The quiet and the loudness of the take's own sound (its single-file copy), or none known.
fn sound(dir: &Path) -> Sound {
    match shroom_encode::quiet::read(&dir.join(shroom_encode::files::PREVIEW)) {
        Ok(q) => Sound {
            silences: q.silences.into_iter().map(|(start, end)| Silence { start, end }).collect(),
            levels: (!q.db.is_empty()).then(|| Levels { step: shroom_encode::quiet::STEP, db: q.db }),
            unread: None,
        },
        Err(e) => Sound { silences: Vec::new(), levels: None, unread: Some(format!("{:?}", e.kind())) },
    }
}

/// What landing a take's cues came to.
#[derive(Debug, Clone, PartialEq)]
pub enum Landed {
    /// In the record: planned now (`fresh`), or planned already (`at`: the record's own
    /// mark); `cuts`: the record's voice cuts as they are now.
    Planned { at: u64, cues: u32, cuts: Vec<Cut>, fresh: bool },
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
        if let Some(s) = saved.as_ref().filter(|s| s.list.voice.is_some()) {
            let v = s.list.voice.expect("planned");
            return Ok(Landed::Planned { at: v.planned_at, cues: v.cues, cuts: s.list.cuts.clone(), fresh: false });
        }
        let length = meta.get("durationSec").and_then(Value::as_f64).filter(|d| *d > 0.0).unwrap_or(t.duration);
        let Some(next) = shroom_edit::land_with(saved, t, length, now_ms, answers) else {
            return Ok(Landed::Planned { at: now_ms, cues: 0, cuts: Vec::new(), fresh: true });
        };
        let cues = next.list.voice.map_or(0, |v| v.cues);
        let cuts = next.list.cuts.clone();
        let mut edits = original.unwrap_or_default();
        if let Value::Object(fresh) = serde_json::to_value(&next).expect("an edit list serializes") {
            edits.extend(fresh);
        }
        meta.insert("edits".into(), Value::Object(edits));
        let body = serde_json::to_vec(&Value::Object(meta)).expect("a record serializes");
        match with_retry(|| s3.put_if_match(&keys::meta(id), &body, "application/json", &etag)) {
            Ok(true) => return Ok(Landed::Planned { at: now_ms, cues, cuts, fresh: true }),
            Ok(false) => continue,
            Err(S3Error::Status { status: 404, .. }) => return Ok(Landed::Refused("the take has no record")),
            Err(e) => return Err(e.into()),
        }
    }
    Err(UploadError::Incomplete("the record kept changing under us".into()))
}
