//! Reads the counter strip from every frame of a video and reports what it shows.
//!
//!   pattern-check <video or playlist> [ticks-per-second]
//!
//! With a tick rate (the on-screen pattern counts 60 per second), it also reports how
//! the counter's progress compares with the video's own length — a ratio of 1.0 means
//! the video's duration is the wall-clock duration.

use std::collections::BTreeMap;
use std::io::Read;
use std::process::{Command, Stdio};

use shroom_testpattern::{read_gray, summarize};

fn probe_size(input: &str) -> (u32, u32) {
    let out = Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=width,height", "-of", "csv=p=0:s=x", input])
        .output()
        .expect("ffprobe runs");
    let text = String::from_utf8_lossy(&out.stdout);
    let mut it = text.trim().lines().next().unwrap_or("").split('x').map(|v| v.parse::<u32>().unwrap_or(0));
    (it.next().unwrap_or(0), it.next().unwrap_or(0))
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let input = args.get(1).expect("usage: pattern-check <video> [ticks-per-second]");
    let ticks: Option<f64> = args.get(2).and_then(|t| t.parse().ok());
    let (w, h) = probe_size(input);
    assert!(w > 0 && h > 0, "couldn't read the video's size");

    let mut child = Command::new("ffmpeg")
        .args(["-hide_banner", "-loglevel", "error", "-i", input, "-f", "rawvideo", "-pix_fmt", "gray", "-"])
        .stdout(Stdio::piped())
        .spawn()
        .expect("ffmpeg runs");
    let mut out = child.stdout.take().unwrap();
    let mut frame = vec![0u8; (w * h) as usize];
    let mut counters = Vec::new();
    while out.read_exact(&mut frame).is_ok() {
        counters.push(read_gray(&frame, w, h));
    }
    let _ = child.wait();

    let s = summarize(&counters);
    let mut deltas: BTreeMap<i64, usize> = BTreeMap::new();
    let mut prev: Option<u32> = None;
    for c in counters.iter().flatten() {
        if let Some(p) = prev {
            *deltas.entry(*c as i64 - p as i64).or_default() += 1;
        }
        prev = Some(*c);
    }
    let video_seconds = s.frames as f64 / 30.0;
    let pace = match (ticks, s.first, s.last) {
        (Some(t), Some(a), Some(b)) if s.frames > 1 => Some(((b - a) as f64 / t) / ((s.frames - 1) as f64 / 30.0)),
        _ => None,
    };
    let deltas: Vec<String> = deltas.iter().map(|(d, n)| format!("\"{d}\":{n}")).collect();
    println!(
        "{{\"input\":{input:?},\"width\":{w},\"height\":{h},\"frames\":{},\"unreadable\":{},\"repeated\":{},\"skipped\":{},\"backwards\":{},\"first\":{},\"last\":{},\"video_seconds\":{video_seconds:.3},\"pace\":{},\"deltas\":{{{}}}}}",
        s.frames,
        s.unreadable,
        s.repeated,
        s.skipped,
        s.backwards,
        opt(s.first),
        opt(s.last),
        pace.map(|p| format!("{p:.4}")).unwrap_or("null".into()),
        deltas.join(",")
    );
}

fn opt(v: Option<u32>) -> String {
    v.map(|n| n.to_string()).unwrap_or_else(|| "null".into())
}
