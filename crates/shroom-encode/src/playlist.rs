//! Joining takes into the one playlist that goes live (POC `lib/finalize.mjs`).
//!
//! One shared `EXT-X-MAP` (the init segment is byte-identical across takes), an
//! `EXT-X-DISCONTINUITY` before every take after the first (a pause is a timestamp
//! jump), one `ENDLIST`. A take whose picture differs (another size) plays under its own
//! init segment, named after its discontinuity (`assemble_mapped`).

use std::fmt::Write;

use crate::files;

#[derive(Debug, Clone, PartialEq)]
pub struct Segment {
    pub file: String,
    pub seconds: f64,
}

/// The `(seconds, file)` entries of one take's playlist, in order.
pub fn take_segments(playlist: &str) -> Vec<Segment> {
    let mut out = Vec::new();
    let mut seconds = 0.0;
    for line in playlist.lines().map(str::trim) {
        if let Some(rest) = line.strip_prefix("#EXTINF:") {
            seconds = rest.trim_end_matches(',').split(',').next().and_then(|s| s.parse().ok()).unwrap_or(0.0);
        } else if files::segment_index(line).is_some() {
            out.push(Segment { file: line.to_string(), seconds });
        }
    }
    out
}

#[derive(Debug, Clone, PartialEq)]
pub struct Assembled {
    pub text: String,
    pub seconds: f64,
    pub segments: Vec<String>,
    /// Every init segment the playlist names, in order of first use.
    pub inits: Vec<String>,
}

/// The init segments a playlist names (`EXT-X-MAP` URIs), in order of first use.
pub fn maps(playlist: &str) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    for line in playlist.lines().map(str::trim) {
        if let Some(uri) = line.strip_prefix("#EXT-X-MAP:URI=\"").and_then(|r| r.split('"').next()) {
            if !out.iter().any(|u| u == uri) {
                out.push(uri.to_string());
            }
        }
    }
    out
}

/// The master VOD playlist for takes given in order (empty takes are skipped), all under
/// the shared init segment.
pub fn assemble(takes: &[Vec<Segment>]) -> Assembled {
    let mapped: Vec<(String, Vec<Segment>)> = takes.iter().map(|t| (files::INIT_SEGMENT.to_string(), t.clone())).collect();
    assemble_mapped(&mapped)
}

/// The master VOD playlist for takes given in order, each with the init segment it plays
/// under: the map is named at the top and again after a discontinuity wherever it changes.
pub fn assemble_mapped(takes: &[(String, Vec<Segment>)]) -> Assembled {
    let all: Vec<&Segment> = takes.iter().flat_map(|(_, t)| t).collect();
    let target = all.iter().map(|s| s.seconds).fold(0.0, f64::max).ceil().max(1.0) as u64;

    let mut text = String::new();
    let _ = writeln!(text, "#EXTM3U");
    let _ = writeln!(text, "#EXT-X-VERSION:7");
    let _ = writeln!(text, "#EXT-X-TARGETDURATION:{target}");
    let _ = writeln!(text, "#EXT-X-MEDIA-SEQUENCE:0");
    let _ = writeln!(text, "#EXT-X-PLAYLIST-TYPE:VOD");
    let mut current: Option<&str> = None;
    let mut inits: Vec<String> = Vec::new();
    for (init, take) in takes.iter().filter(|(_, t)| !t.is_empty()) {
        if current.is_some() {
            let _ = writeln!(text, "#EXT-X-DISCONTINUITY");
        }
        if current != Some(init.as_str()) {
            let _ = writeln!(text, "#EXT-X-MAP:URI=\"{init}\"");
            current = Some(init);
            if !inits.contains(init) {
                inits.push(init.clone());
            }
        }
        for s in take {
            let _ = writeln!(text, "#EXTINF:{:.6},", s.seconds);
            let _ = writeln!(text, "{}", s.file);
        }
    }
    let _ = writeln!(text, "#EXT-X-ENDLIST");

    Assembled {
        text,
        seconds: all.iter().map(|s| s.seconds).sum(),
        segments: all.iter().map(|s| s.file.clone()).collect(),
        inits,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TAKE_0: &str = "#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-TARGETDURATION:6\n#EXT-X-MEDIA-SEQUENCE:0\n\
        #EXT-X-PLAYLIST-TYPE:VOD\n#EXT-X-MAP:URI=\"init.mp4\"\n#EXTINF:6.000000,\nseg_00000.m4s\n\
        #EXTINF:2.500000,\nseg_00001.m4s\n#EXT-X-ENDLIST\n";
    const TAKE_1: &str = "#EXTM3U\n#EXT-X-MAP:URI=\"init.mp4\"\n#EXTINF:4.000000,\nseg_00002.m4s\n#EXT-X-ENDLIST\n";

    #[test]
    fn reads_a_takes_segments_in_order() {
        assert_eq!(
            take_segments(TAKE_0),
            vec![
                Segment { file: "seg_00000.m4s".into(), seconds: 6.0 },
                Segment { file: "seg_00001.m4s".into(), seconds: 2.5 },
            ]
        );
    }

    #[test]
    fn joins_takes_with_one_map_and_a_discontinuity_per_pause() {
        let a = assemble(&[take_segments(TAKE_0), take_segments(TAKE_1)]);
        assert_eq!(a.text.matches("#EXT-X-MAP").count(), 1);
        assert_eq!(a.text.matches("#EXT-X-DISCONTINUITY").count(), 1);
        assert!(a.text.ends_with("#EXT-X-ENDLIST\n"));
        assert_eq!(a.seconds, 12.5);
        assert_eq!(a.segments, vec!["seg_00000.m4s", "seg_00001.m4s", "seg_00002.m4s"]);
        let disc = a.text.find("#EXT-X-DISCONTINUITY").unwrap();
        assert!(disc > a.text.find("seg_00001.m4s").unwrap() && disc < a.text.find("seg_00002.m4s").unwrap());
    }

    #[test]
    fn an_empty_take_adds_no_discontinuity() {
        let a = assemble(&[take_segments(TAKE_0), vec![], take_segments(TAKE_1)]);
        assert_eq!(a.text.matches("#EXT-X-DISCONTINUITY").count(), 1);
    }

    #[test]
    fn a_take_under_its_own_init_is_mapped_after_its_pause() {
        let take_2 = "#EXTM3U\n#EXTINF:3.000000,\nseg_00003.m4s\n#EXT-X-ENDLIST\n";
        let a = assemble_mapped(&[
            ("init.mp4".into(), take_segments(TAKE_0)),
            ("init_1.mp4".into(), take_segments(TAKE_1)),
            ("init.mp4".into(), take_segments(take_2)),
        ]);
        let lines: Vec<&str> = a.text.lines().collect();
        let at = |l: &str| lines.iter().position(|x| *x == l).unwrap();
        assert_eq!(a.text.matches("#EXT-X-MAP").count(), 3);
        assert_eq!(lines[at("seg_00002.m4s") - 3..at("seg_00002.m4s") - 1], ["#EXT-X-DISCONTINUITY", "#EXT-X-MAP:URI=\"init_1.mp4\""]);
        assert_eq!(lines[at("seg_00003.m4s") - 3..at("seg_00003.m4s") - 1], ["#EXT-X-DISCONTINUITY", "#EXT-X-MAP:URI=\"init.mp4\""]);
        assert_eq!(a.inits, ["init.mp4", "init_1.mp4"]);
        assert_eq!(maps(&a.text), ["init.mp4", "init_1.mp4"]);
        // Every take under the shared init: one map, as ever.
        let b = assemble(&[take_segments(TAKE_0), take_segments(TAKE_1)]);
        assert_eq!(maps(&b.text), ["init.mp4"]);
        assert_eq!(b.text.matches("#EXT-X-MAP").count(), 1);
    }

    #[test]
    fn target_duration_rounds_up_the_longest_segment() {
        let a = assemble(&[vec![Segment { file: "seg_00000.m4s".into(), seconds: 6.006 }]]);
        assert!(a.text.contains("#EXT-X-TARGETDURATION:7\n"));
    }
}
