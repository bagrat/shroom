//! Records the synthetic counter pattern into a directory, then dies — the L3 kill tests'
//! subject (TESTING-PLAN §5).
//!
//!   shroom-crashtest <dir> <seconds> app      abort mid-take (no destructors): ffmpeg loses
//!                                             its input and finishes on its own
//!   shroom-crashtest <dir> <seconds> power    kill -9 the encoder too, then abort — what a
//!                                             power loss leaves

use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use shroom_record::{FrameSink, FrameSource, Recording};

struct Pattern(Arc<AtomicBool>);

impl FrameSource for Pattern {
    fn size(&self) -> (u32, u32) {
        (640, 360)
    }
    fn start(&mut self, sink: FrameSink) -> Result<(), String> {
        let stop = self.0.clone();
        std::thread::spawn(move || {
            let mut c = 0;
            while !stop.load(Ordering::SeqCst) {
                let f = shroom_testpattern::synthetic_bgra(640, 360, c);
                sink.offer(|b| b.copy_from_slice(&f));
                c += 1;
                std::thread::sleep(Duration::from_millis(33));
            }
        });
        Ok(())
    }
    fn stop(&mut self) -> Result<(), String> {
        self.0.store(true, Ordering::SeqCst);
        Ok(())
    }
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let dir = PathBuf::from(&args[1]);
    let seconds: f64 = args[2].parse().unwrap();
    let mode = args[3].clone();
    // The encoder under test: SHROOM_FFMPEG (the release's sidecar, B82), else the one on PATH.
    let ffmpeg = std::env::var_os("SHROOM_FFMPEG").filter(|p| !p.is_empty()).map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(String::from_utf8(std::process::Command::new("sh").args(["-c", "command -v ffmpeg"]).output().unwrap().stdout).unwrap().trim())
    });
    let mut rec = Recording::create(&dir, &ffmpeg, 1500).unwrap();
    rec.start_take(Box::new(Pattern(Arc::default())), None).unwrap();
    std::thread::sleep(Duration::from_secs_f64(seconds));
    if mode == "power" {
        // Our ffmpeg children run in their own process groups: kill them hard.
        let me = std::process::id().to_string();
        let out = std::process::Command::new("pgrep").args(["-P", &me]).output().unwrap();
        for pid in String::from_utf8_lossy(&out.stdout).split_whitespace() {
            if let Ok(pid) = pid.parse::<i32>() {
                unsafe { libc::kill(-pid, libc::SIGKILL) };
                unsafe { libc::kill(pid, libc::SIGKILL) };
            }
        }
    }
    std::process::abort();
}
