//! A take's voice cues, landed in its saved edit (N65).

use crate::{Answer, EditList, Options, Saved, Transcript, Voice, find_cues, plan_with};

/// The take's voice cues planned and landed in what the editor saved (N63's `meta.edits`),
/// or in a list of its own when nothing was saved: the planner's cuts beside the editor's
/// (hand cuts and the trim kept), named `v1`… in the order of their cues, and the plan's
/// mark set, so a take is planned once. `None` when it was planned already: a cut put back
/// stays put back. The revision moves on only when what plays changes, so a copy cut from an
/// edit the cues leave as it was stays current. `source_duration`: the recording's own
/// length, for a list made here.
pub fn land(
    saved: Option<Saved>,
    t: &Transcript,
    source_duration: f64,
    now_ms: u64,
) -> Option<Saved> {
    land_with(saved, t, source_duration, now_ms, &[])
}

/// [`land`], the plan taking the judgment's answers ([`crate::plan_with`]).
pub fn land_with(
    saved: Option<Saved>,
    t: &Transcript,
    source_duration: f64,
    now_ms: u64,
    answers: &[Answer],
) -> Option<Saved> {
    if saved.as_ref().is_some_and(|s| s.list.voice.is_some()) {
        return None;
    }
    let mut s = saved.unwrap_or_else(|| Saved {
        list: EditList::new(source_duration),
        rev: 0,
        copy: None,
    });
    let plays = s.list.clone().checked().keep();
    // A list written by hand might hold voice ids already: never name two cuts alike.
    let mut n = s
        .list
        .cuts
        .iter()
        .filter_map(|c| c.id.strip_prefix('v')?.parse::<u32>().ok())
        .max()
        .unwrap_or(0);
    for mut cut in plan_with(t, Options::default(), answers).cuts {
        n += 1;
        cut.id = format!("v{n}");
        s.list.cuts.push(cut);
    }
    s.list.voice = Some(Voice {
        planned_at: now_ms,
        cues: find_cues(&t.words).len() as u32,
    });
    s.list = s.list.checked();
    if s.list.keep() != plays {
        s.rev += 1;
    }
    Some(s)
}
