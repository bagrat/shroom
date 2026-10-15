//! The init-segment readers against ffprobe on a real VideoToolbox init segment.
//! Needs ffmpeg on PATH: without it this test fails, never passes silently.

use std::process::Command;

#[test]
fn size_and_codec_match_ffprobe() {
    assert!(Command::new("ffmpeg").arg("-version").output().is_ok(), "ffmpeg isn't on PATH — this test needs it (brew install ffmpeg)");
    let dir = tempfile::tempdir().unwrap();
    let ok = Command::new("ffmpeg")
        .current_dir(dir.path())
        .args(["-hide_banner", "-loglevel", "error", "-f", "lavfi", "-i", "testsrc2=size=1280x720:rate=30", "-t", "1"])
        .args(["-c:v", "h264_videotoolbox", "-b:v", "2000k", "-pix_fmt", "yuv420p"])
        .args(["-f", "hls", "-hls_segment_type", "fmp4", "-hls_fmp4_init_filename", "init.mp4", "s.m3u8"])
        .status()
        .unwrap()
        .success();
    assert!(ok);
    let init = std::fs::read(dir.path().join("init.mp4")).unwrap();
    assert_eq!(shroom_upload::upload::probe_size(&init), Some((1280, 720)));

    let out = Command::new("ffprobe")
        .args(["-v", "error", "-select_streams", "v:0", "-show_entries", "stream=profile,level", "-of", "csv=p=0"])
        .arg(dir.path().join("s.m3u8"))
        .output()
        .unwrap();
    let probed = String::from_utf8_lossy(&out.stdout).trim().to_string(); // e.g. "High,31"
    let codec = shroom_upload::master::avc_codec(&init).expect("avcC present");
    let level = u32::from_str_radix(&codec[9..11], 16).unwrap();
    let profile_idc = u32::from_str_radix(&codec[5..7], 16).unwrap();
    let profile = match profile_idc { 66 => "Constrained Baseline", 77 => "Main", 100 => "High", _ => "?" };
    assert!(probed.starts_with(profile) || (profile_idc == 66 && probed.contains("Baseline")), "{codec} vs {probed}");
    assert!(probed.ends_with(&level.to_string()), "{codec} vs {probed}");
}
