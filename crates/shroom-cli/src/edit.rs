//! `shroom edit` (spec N72, DECISIONS B225): what a video's edit is, and one change a call —
//! the editor's own steps (`shroom_edit::edit::check`), refused for the editor's own reasons, and
//! saved as the editor's Done saves (B195), through the space's page under this Mac's device
//! key, as a rename is (B215). The link plays the change from its next load; nothing is
//! re-encoded first. The final video is the app's to make: after a change that leaves
//! something out, the running app is asked (`edit.copy`), and before any change it's asked
//! whether the video is open in one of its editor windows (`edit.check`), so that window's Done never silently undoes
//! the command's change. Undo and Redo stay the editor's: a window's history, never a record's.
//!
//!   shroom edit <video>                      what it is
//!   shroom edit <video> cut 1:02 1:10        trim <in> <out> · put-back <cut> · apply <cut>
//!                                            resize <cut> <start> <end> · revert · finish

use serde_json::{Value, json};
use shroom_edit::{CutKind, EditList, Op, Refusal, Saved};
use shroom_upload::meta::Meta;

use crate::out::{Done, Fail, at};
use crate::{Args, app, say};

/// One change, as the command line names it: a cut by its id (`m1`, `v2`…), or by its place
/// (`3`) in a list written before ids.
enum Change {
    Trim(f64, f64),
    Cut(f64, f64),
    PutBack(String),
    Apply(String),
    Resize(String, f64, f64),
    Revert,
}

enum Ask {
    Show,
    Change(Change),
    Finish,
}

pub fn edit(a: &Args, rest: &[String]) -> Result<Done, Fail> {
    let [which, what @ ..] = rest else { return Err(Fail::Usage(say::edit_usage())) };
    let ask = ask(what)?;
    let (h, d, v) = crate::videos(a)?;
    let hit = crate::find(&v, which, &d.identity.host, true)?;
    let id = hit.row.id.clone();
    let (s3, meta) = d.record(&id)?;
    let saved = meta.edits.clone();
    let list = list_of(&meta);
    let mut j = json!({"id": id, "title": hit.row.title, "link": hit.link, "rev": saved.as_ref().map_or(0, |s| s.rev)});
    match ask {
        Ask::Show => {
            j["edit"] = summary(&list, saved.as_ref());
            Ok(Done::new(j.clone(), said(&hit.row.title, &list, saved.as_ref())))
        }
        Ask::Finish => {
            let (state, text) = match saved.as_ref() {
                Some(s) if !list.is_identity() && s.current_copy().is_some() => ("made", say::final_ready()),
                // `finish` is that very ask: the app not there is its failure.
                Some(s) if !list.is_identity() => {
                    copy(a, &h, &id, s.rev)?;
                    ("asked", say::final_asked())
                }
                _ => ("not_needed", say::final_not_needed()),
            };
            j["edit"] = summary(&list, saved.as_ref());
            j["final"] = state.into();
            Ok(Done::new(j, text))
        }
        Ask::Change(change) => {
            let op = op_of(&list, change)?;
            let next = shroom_edit::edit::check(&list, &op).map_err(refused)?;
            let (plays, now, whole) = (at(next.edited_duration()), at(list.edited_duration()), at(next.source_duration));
            if a.dry_run {
                j["edit"] = summary(&next, None);
                j["saved"] = false.into();
                j["dry_run"] = true.into();
                return Ok(Done::new(j, say::edit_would(&plays, &now)));
            }
            crate::quiet(&h)?;
            // The video is open in an editor window here: its Done is the edit (N72). A Mac whose app
            // isn't running has no editor open.
            if let Err(f @ Fail::CantEdit { .. }) = app::ask(&h.control_socket(), "edit.check", &by_id(&id, None), None) {
                return Err(f);
            }
            let rev = d.save_edit(&s3, &id, &next, &meta)?;
            j["rev"] = rev.into();
            j["saved"] = true.into();
            let mut text = if next.is_identity() { say::edit_whole_again() } else { say::edit_saved(&plays, &whole) };
            let state = if next.is_identity() {
                "not_needed"
            } else if copy(a, &h, &id, rev).is_ok() {
                text = format!("{text}\n{}", say::final_asked());
                "asked"
            } else {
                text = format!("{text}\n{}", say::final_later(&id));
                "later"
            };
            let after = Saved::new(next.clone(), rev);
            j["edit"] = summary(&next, Some(&after));
            j["final"] = state.into();
            Ok(Done::new(j, text))
        }
    }
}

/// Asks the running app (opened unless `--no-launch`) to make the final video of `rev`.
fn copy(a: &Args, h: &crate::home::Home, id: &str, rev: u64) -> Result<Value, Fail> {
    let launch = (!a.no_launch).then_some(h.app_id.as_str());
    app::ask(&h.control_socket(), "edit.copy", &by_id(id, Some(rev)), launch)
}

fn by_id(id: &str, rev: Option<u64>) -> serde_json::Map<String, Value> {
    let mut how = serde_json::Map::new();
    how.insert("id".into(), id.into());
    if let Some(rev) = rev {
        how.insert("rev".into(), rev.into());
    }
    how
}

/// The edit the record holds, made safe against the record's own length (as the page reads
/// it); the whole recording when it holds none.
fn list_of(meta: &Meta) -> EditList {
    let whole = meta.duration_sec.filter(|d| d.is_finite() && *d > 0.0);
    let mut list = meta.edits.as_ref().map(|s| s.list.clone()).unwrap_or_else(|| EditList::new(whole.unwrap_or(0.0)));
    if let Some(d) = whole {
        list.source_duration = d;
    }
    list.checked()
}

fn ask(what: &[String]) -> Result<Ask, Fail> {
    let usage = || Fail::Usage(say::edit_usage());
    let t = |s: &String| secs(s).ok_or_else(|| Fail::Usage(say::not_a_time(s)));
    Ok(match what {
        [] => Ask::Show,
        [w] if w == "revert" => Ask::Change(Change::Revert),
        [w] if w == "finish" => Ask::Finish,
        [w, a, b] if w == "trim" => Ask::Change(Change::Trim(t(a)?, t(b)?)),
        [w, a, b] if w == "cut" => Ask::Change(Change::Cut(t(a)?, t(b)?)),
        [w, c] if w == "put-back" => Ask::Change(Change::PutBack(c.clone())),
        [w, c] if w == "apply" => Ask::Change(Change::Apply(c.clone())),
        [w, c, a, b] if w == "resize" => Ask::Change(Change::Resize(c.clone(), t(a)?, t(b)?)),
        _ => return Err(usage()),
    })
}

/// A time on the recording: seconds (`12.5`), or minutes and seconds (`1:02`, `1:02.5`),
/// or hours too (`1:02:03`).
pub fn secs(s: &str) -> Option<f64> {
    let parts: Vec<&str> = s.split(':').collect();
    if parts.len() > 3 || parts.iter().any(|p| p.is_empty()) {
        return None;
    }
    let (last, whole) = parts.split_last()?;
    let mut total = 0.0;
    for p in whole {
        let n: u32 = p.parse().ok()?;
        total = total * 60.0 + f64::from(n);
    }
    let tail: f64 = last.parse().ok().filter(|x: &f64| x.is_finite() && *x >= 0.0)?;
    if !whole.is_empty() && tail >= 60.0 {
        return None;
    }
    Some(total * 60.0 + tail)
}

fn op_of(list: &EditList, change: Change) -> Result<Op, Fail> {
    let index = |name: &str| place(list, name).ok_or_else(|| refused(Refusal::NoSuchCut));
    Ok(match change {
        Change::Trim(in_, out) => Op::Trim { in_, out },
        Change::Cut(start, end) => Op::Cut { start, end },
        Change::PutBack(c) => Op::PutBack { index: index(&c)? },
        Change::Apply(c) => Op::Apply { index: index(&c)? },
        Change::Resize(c, start, end) => Op::Resize { index: index(&c)?, start, end },
        Change::Revert => Op::Revert,
    })
}

/// A cut's place in the list, by its id — or by its number in a list written before ids.
fn place(list: &EditList, name: &str) -> Option<usize> {
    list.cuts.iter().position(|c| c.id == name).or_else(|| {
        let n: usize = name.parse().ok()?;
        (n >= 1 && list.cuts.get(n - 1).is_some_and(|c| c.id.is_empty())).then(|| n - 1)
    })
}

/// A cut's name, as the command shows it: its id, or its number when it has none.
fn name(i: usize, c: &shroom_edit::Cut) -> String {
    if c.id.is_empty() { (i + 1).to_string() } else { c.id.clone() }
}

fn refused(r: Refusal) -> Fail {
    let reason = match r {
        Refusal::OutOfRange => "out_of_range",
        Refusal::LeavesTooLittle => "leaves_too_little",
        Refusal::TooShort => "too_short",
        Refusal::NothingChanges => "nothing_changes",
        Refusal::NoSuchCut => "no_such_cut",
    };
    Fail::CantEdit { reason, message: say::cant_edit(reason) }
}

/// Seconds as an answer gives them: to the millisecond.
fn sec(x: f64) -> f64 {
    (x * 1000.0).round() / 1000.0
}

fn final_video(list: &EditList, saved: Option<&Saved>) -> &'static str {
    match saved {
        _ if list.is_identity() => "none",
        Some(s) if s.current_copy().is_some() => "made",
        _ => "not_yet",
    }
}

/// The edit, for an agent.
fn summary(list: &EditList, saved: Option<&Saved>) -> Value {
    let cuts: Vec<Value> = list
        .cuts
        .iter()
        .enumerate()
        .map(|(i, c)| json!({"id": name(i, c), "start": sec(c.start), "end": sec(c.end), "applied": c.applied, "by": if c.kind == CutKind::Manual { "hand" } else { "voice" }}))
        .collect();
    json!({
        "plays_sec": sec(list.edited_duration()),
        "recording_sec": sec(list.source_duration),
        "edited": !list.is_identity(),
        "trim": {"in": sec(list.in_), "out": sec(list.out)},
        "cuts": cuts,
        "final_video": final_video(list, saved),
        "original_kept_until": saved.and_then(|s| s.keep_until.clone()),
    })
}

/// The edit, for a person.
fn said(title: &str, list: &EditList, saved: Option<&Saved>) -> String {
    let trimmed = list.in_ > 0.0 || list.out < list.source_duration;
    let ends = trimmed.then(|| (at(list.in_), at(list.out)));
    let cuts: Vec<(String, String, String, bool)> = list.cuts.iter().enumerate().map(|(i, c)| (name(i, c), at(c.start), at(c.end), c.applied)).collect();
    say::edit_is(title, &at(list.edited_duration()), &at(list.source_duration), ends, &cuts, final_video(list, saved))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times_read_as_seconds_or_minutes_and_seconds() {
        assert_eq!(secs("12.5"), Some(12.5));
        assert_eq!(secs("1:02"), Some(62.0));
        assert_eq!(secs("1:02.5"), Some(62.5));
        assert_eq!(secs("1:02:03"), Some(3723.0));
        assert_eq!(secs("0:20"), Some(20.0));
        for bad in ["", "ten", "1:", ":5", "1:75", "-3", "1:2:3:4", "NaN", "inf"] {
            assert_eq!(secs(bad), None, "{bad}");
        }
    }

    #[test]
    fn a_cut_is_named_by_its_id_or_by_its_number_before_ids() {
        let mut list = EditList::new(100.0);
        list.cuts = vec![shroom_edit::Cut::manual(10.0, 20.0), shroom_edit::Cut { id: "m2".into(), ..shroom_edit::Cut::manual(30.0, 40.0) }];
        assert_eq!(place(&list, "1"), Some(0), "no id: its number");
        assert_eq!(place(&list, "m2"), Some(1));
        assert_eq!(place(&list, "2"), None, "a cut with an id goes by its id");
        assert_eq!(place(&list, "m9"), None);
        assert_eq!((name(0, &list.cuts[0]), name(1, &list.cuts[1])), ("1".to_string(), "m2".to_string()));
    }
}
