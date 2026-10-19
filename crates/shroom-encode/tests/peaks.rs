//! The editor's waveform from committed media (`tests/fixtures`, made by `make.sh` there):
//! a tone, then silence, read from a faststart MP4 and from one fragmented the way a take is
//! written; a picture with no sound reads as silence for the picture's length. Nothing here
//! encodes, so it runs on every machine.

use std::path::{Path, PathBuf};

use shroom_encode::peaks::{self, PER_SECOND, PeaksError};

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures").join(name)
}

/// tone.mp4 and tone-frag.mp4: 2 s of a tone at 0.8, then 2 s of silence, 4 s of picture.
fn holds_the_tone_then_the_silence(name: &str) {
    let p = peaks::read(&fixture(name)).unwrap_or_else(|e| panic!("{name}: {e}"));
    assert_eq!(p.per_second, PER_SECOND);
    assert!((p.duration - 4.0).abs() < 0.02, "{name}: {} s", p.duration);
    let expected = (p.duration * f64::from(PER_SECOND)).round() as i64;
    assert!((p.values.len() as i64 - expected).abs() <= 1, "{name}: {} buckets for {} s", p.values.len(), p.duration);
    assert!((p.values.len() as i64 - 200).abs() <= 1, "{name}: {} buckets", p.values.len());
    // Clear of the edges (the encoder's priming at the start, its ringing at the cut).
    let tone = &p.values[5..95];
    let quiet = &p.values[110..195];
    assert!(tone.iter().all(|v| *v >= 180), "{name}: the tone is loud: {tone:?}");
    assert!(quiet.iter().all(|v| *v <= 3), "{name}: the silence is quiet: {quiet:?}");
}

#[test]
fn a_faststart_mp4_reads_as_its_sound() {
    holds_the_tone_then_the_silence("tone.mp4");
}

#[test]
fn a_fragmented_take_reads_as_its_sound() {
    holds_the_tone_then_the_silence("tone-frag.mp4");
}

#[test]
fn both_shapes_of_the_same_recording_read_alike() {
    let a = peaks::read(&fixture("tone.mp4")).unwrap();
    let b = peaks::read(&fixture("tone-frag.mp4")).unwrap();
    let n = a.values.len().min(b.values.len());
    let differ = (0..n).filter(|i| a.values[*i].abs_diff(b.values[*i]) > 2).count();
    assert_eq!(differ, 0, "{:?}\n{:?}", a.values, b.values);
}

#[test]
fn a_picture_with_no_sound_reads_as_silence_for_its_length() {
    let p = peaks::read(&fixture("video-only.mp4")).unwrap();
    assert!((p.duration - 3.0).abs() < 0.02, "{} s", p.duration);
    assert!((p.values.len() as i64 - 150).abs() <= 1, "{} buckets", p.values.len());
    assert!(p.values.iter().all(|v| *v == 0));
}

#[test]
fn a_take_cut_short_reads_up_to_where_it_ends() {
    // A crash mid-write leaves a fragmented take truncated: every whole fragment (a second
    // each) still reads.
    let bytes = std::fs::read(fixture("tone-frag.mp4")).unwrap();
    let dir = tempfile::tempdir().unwrap();
    let cut = dir.path().join("preview_0.mp4");
    std::fs::write(&cut, &bytes[..bytes.len() * 2 / 3]).unwrap();
    match peaks::read(&cut) {
        Ok(p) => {
            assert!(p.duration >= 1.0 && p.duration < 4.0, "{} s", p.duration);
            assert!(p.values[5..40].iter().all(|v| *v >= 180), "{:?}", p.values);
        }
        Err(PeaksError::Unreadable(why)) => panic!("a truncated take should read as far as it goes: {why}"),
        Err(e) => panic!("{e}"),
    }
}
