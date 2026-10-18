//! The run log's lines for one read of the videos suggested to clean up (N68): what a
//! problem report needs to see of it, in two lines a day of use leaves room for. Counts,
//! sizes, the Worker's verdict on each candidate by a short id and a code, and how counting
//! fares in the space — never a title, a link or a whole id (`report::clean_log` keeps them
//! as they are).

use crate::member::{CleanupItem, ViewsHealth};

/// One line, at most, once a report has cleaned it.
const MAX_CHARS: usize = 380;

/// Everything one read of the suggestions saw, every page of it.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct Read {
    pub items: Vec<CleanupItem>,
    pub counting: bool,
    pub views: ViewsHealth,
    pub pages: usize,
    pub ms: u128,
    /// How long ago the newest counted play was, in hours (`None`: none yet).
    pub newest_play_hours: Option<u64>,
}

/// The read's summary, then (when there are any) the candidates, highest score first.
pub fn lines(r: &Read) -> Vec<String> {
    let mut picked: Vec<&CleanupItem> = r.items.iter().filter(|i| i.candidate).collect();
    picked.sort_by(|a, b| b.score.total_cmp(&a.score));
    let total: u64 = r.items.iter().map(|i| i.bytes).sum();
    let freed: u64 = picked.iter().map(|i| i.bytes).sum();
    let newest = match r.newest_play_hours {
        None => "none".to_string(),
        Some(h) if h < 48 => format!("{h}h ago"),
        Some(h) => format!("{}d ago", h / 24),
    };
    let mut out = vec![format!(
        "library suggestions: {} of {} videos, {} of {}, counting {}; plays read {}, unreadable {}, newest play {newest}; {} page{}, {} ms",
        picked.len(),
        r.items.len(),
        gb(freed),
        gb(total),
        if r.counting { "on" } else { "off" },
        r.views.records,
        r.views.unreadable,
        r.pages,
        if r.pages == 1 { "" } else { "s" },
        r.ms,
    )];
    if picked.is_empty() {
        return out;
    }
    let mut line = String::from("library suggested:");
    for (n, i) in picked.iter().enumerate() {
        let entry = format!(" {}", entry(i));
        let more = format!(" +{} more", picked.len() - n);
        if line.chars().count() + entry.chars().count() + more.chars().count() > MAX_CHARS {
            line.push_str(&more);
            break;
        }
        line.push_str(&entry);
    }
    out.push(line);
    out
}

/// `Cdri… not-lately s0.92 3.2GB p4 q300d m35%`: who, why, the score and the facts behind it.
fn entry(i: &CleanupItem) -> String {
    let head: String = i.id.chars().take(4).collect();
    let code = i.reason.as_ref().map(|r| r.code.as_str()).unwrap_or("?");
    let quiet = i.quiet_days.map(|d| format!(" q{}d", d.max(0.0) as u64)).unwrap_or_default();
    let watched = i.median_watched.map(|m| format!(" m{}%", (m * 100.0).round() as u64)).unwrap_or_default();
    format!("{head}… {code} s{:.2} {} p{}{quiet}{watched}", i.score, gb(i.bytes).replace(' ', ""), i.plays)
}

fn gb(bytes: u64) -> String {
    format!("{:.1} GB", bytes as f64 / 1e9)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::member::CleanupReason;

    fn item(id: &str, bytes: u64, candidate: bool, score: f64, code: &str, plays: u32, quiet: Option<f64>, watched: Option<f64>) -> CleanupItem {
        CleanupItem { id: id.into(), bytes, plays, quiet_days: quiet, median_watched: watched, candidate, score, reason: Some(CleanupReason { code: code.into() }) }
    }

    fn read() -> Read {
        Read {
            items: vec![
                item("Cdrive00000recap", 1_800_000_000, true, 0.54, "never-played", 0, Some(120.0), None),
                item("Cdrive000billing", 3_200_000_000, true, 0.92, "not-lately", 4, Some(300.4), Some(0.35)),
                item("Cdrive000000t502", 4_200_000_000, false, 0.0, "watched-lately", 1, Some(0.0), Some(0.95)),
            ],
            counting: true,
            views: ViewsHealth { records: 2, unreadable: 0, newest_play_at: None },
            pages: 1,
            ms: 82,
            newest_play_hours: Some(3),
        }
    }

    #[test]
    fn a_read_says_what_it_picked_and_how_counting_fares() {
        assert_eq!(
            lines(&read()),
            [
                "library suggestions: 2 of 3 videos, 5.0 GB of 9.2 GB, counting on; plays read 2, unreadable 0, newest play 3h ago; 1 page, 82 ms",
                "library suggested: Cdri… not-lately s0.92 3.2GB p4 q300d m35% Cdri… never-played s0.54 1.8GB p0 q120d",
            ]
        );
    }

    #[test]
    fn a_read_with_nothing_to_suggest_is_one_line() {
        let r = Read { items: vec![], counting: false, newest_play_hours: None, pages: 2, ..read() };
        assert_eq!(lines(&r), ["library suggestions: 0 of 0 videos, 0.0 GB of 0.0 GB, counting off; plays read 2, unreadable 0, newest play none; 2 pages, 82 ms"]);
        let old = Read { newest_play_hours: Some(24 * 9 + 5), ..read() };
        assert!(lines(&old)[0].contains("newest play 9d ago"));
    }

    /// A space with many candidates still fits a line: the rest are counted, not listed.
    #[test]
    fn many_candidates_fit_one_line() {
        let items = (0..60).map(|n| item(&format!("Vid{n:02}xxxxxxxxxxx"), 2_000_000_000, true, 1.0 - n as f64 / 100.0, "never-played", 0, Some(200.0), None)).collect();
        let l = lines(&Read { items, ..read() });
        assert!(l[1].chars().count() <= MAX_CHARS, "{}", l[1].len());
        assert!(l[1].ends_with(" more"), "{}", l[1]);
        assert!(l[1].starts_with("library suggested: Vid0… never-played s1.00"));
    }

    /// What a report sends is what was logged: nothing in these lines is cleaned away.
    #[test]
    fn a_report_keeps_the_lines_as_they_are() {
        for l in lines(&read()) {
            assert_eq!(shroom_core::report::clean_line(&l), l);
        }
    }
}
