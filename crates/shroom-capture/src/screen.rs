//! The screen as a frame source: ScreenCaptureKit through the MIT `scap-screencapturekit` crate.
//!
//! Frames are scaled to the output size by ScreenCaptureKit itself (on the GPU), forced
//! to BGRA (the default format isn't CPU-readable — RESEARCH, probe results), and copied
//! row by row into the hand-off, dropping any row padding.

use std::ffi::c_void;

use cidre::{arc, cm, cv, ns, sc};
use scap_screencapturekit::{Capturer, Frame, StreamCfgBuilder};
use shroom_encode::quality::Quality;
use shroom_record::mix::{SYSTEM_RATE, SystemAudio};
use shroom_record::{FrameSink, FrameSource};

#[link(name = "CoreGraphics", kind = "framework")]
unsafe extern "C" {
    fn CGMainDisplayID() -> u32;
    fn CGDisplayCopyDisplayMode(display: u32) -> *mut c_void;
    fn CGDisplayModeGetPixelWidth(mode: *mut c_void) -> usize;
    fn CGDisplayModeGetPixelHeight(mode: *mut c_void) -> usize;
    fn CGDisplayModeRelease(mode: *mut c_void);
}

#[link(name = "CoreVideo", kind = "framework")]
unsafe extern "C" {
    fn CVPixelBufferGetBaseAddress(pixel_buffer: &cv::PixelBuf) -> *const u8;
    fn CVPixelBufferGetBytesPerRow(pixel_buffer: &cv::PixelBuf) -> usize;
}

pub fn main_display_id() -> u32 {
    unsafe { CGMainDisplayID() }
}

/// A display's size in real pixels (not points).
fn pixel_size(display_id: u32) -> Option<(u32, u32)> {
    unsafe {
        let mode = CGDisplayCopyDisplayMode(display_id);
        if mode.is_null() {
            return None;
        }
        let size = (CGDisplayModeGetPixelWidth(mode) as u32, CGDisplayModeGetPixelHeight(mode) as u32);
        CGDisplayModeRelease(mode);
        Some(size)
    }
}

struct Live(Capturer);
// ScreenCaptureKit's stream and its dispatch queue are thread-safe Objective-C objects;
// the capturer is only started and stopped, never shared mid-call.
unsafe impl Send for Live {}

pub struct ScreenSource {
    display_id: u32,
    width: u32,
    height: u32,
    cursor: bool,
    capture_fps: f32,
    excluded: Vec<u32>,
    system: Option<SystemAudio>,
    live: Option<Live>,
}

impl ScreenSource {
    /// The display `display_id`, captured at the size `quality` fits it into.
    pub fn new(display_id: u32, quality: Quality) -> Result<ScreenSource, String> {
        let (pw, ph) = pixel_size(display_id).ok_or("that display isn't available")?;
        let (width, height) = quality.output_size(pw, ph);
        Ok(ScreenSource { display_id, width, height, cursor: true, capture_fps: 60.0, excluded: Vec::new(), system: None, live: None })
    }

    /// How often ScreenCaptureKit may deliver. Twice the encode rate (the default) means
    /// a fresh frame is waiting at nearly every tick of the 30 fps frame clock; at 30 the
    /// two clocks drift in and out of phase and ~5 % of frames repeat (measured).
    pub fn with_capture_fps(mut self, fps: f32) -> Self {
        self.capture_fps = fps;
        self
    }

    /// Windows (by window number) left out of every frame — shroom's own popovers and
    /// countdown scrim, so its UI can never land in a take whatever the timing. The camera
    /// bubble is deliberately *not* excluded: it's meant to be recorded.
    pub fn excluding(mut self, window_numbers: Vec<u32>) -> Self {
        self.excluded = window_numbers;
        self
    }

    /// The Mac's own sound, on the same stream as the picture (spec N47, DECISIONS B172):
    /// mono at [`SYSTEM_RATE`], with shroom's own sounds left out, fed to `system`.
    pub fn with_system_audio(mut self, system: SystemAudio) -> Self {
        self.system = Some(system);
        self
    }

    /// Whether the pointer is drawn into frames (on by default).
    pub fn with_cursor(mut self, cursor: bool) -> Self {
        self.cursor = cursor;
        self
    }

    pub fn display_id(&self) -> u32 {
        self.display_id
    }
}

impl FrameSource for ScreenSource {
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    fn start(&mut self, sink: FrameSink) -> Result<(), String> {
        let (width, height, display_id, cursor, fps) = (self.width, self.height, self.display_id, self.cursor, self.capture_fps);
        let excluded = self.excluded.clone();
        let system = self.system.clone();
        let capturer = futures::executor::block_on(async move {
            let content = sc::ShareableContent::current().await.map_err(|e| describe(&e))?;
            let displays = content.displays();
            let display = displays
                .iter()
                .find(|d| d.display_id().0 == display_id)
                .ok_or_else(|| "that display isn't shareable".to_string())?;
            let windows = content.windows();
            let skip: Vec<&sc::Window> = windows.iter().filter(|w| excluded.contains(&w.id())).collect();
            if skip.len() != excluded.len() {
                let ours: Vec<u32> = windows
                    .iter()
                    .filter(|w| w.owning_app().map(|a| a.process_id() == std::process::id() as i32).unwrap_or(false))
                    .map(|w| w.id())
                    .collect();
                return Err(format!("windows to exclude weren't all found: wanted {excluded:?}, this app's shareable windows are {ours:?}"));
            }
            let filter = sc::ContentFilter::with_display_excluding_windows(display, &ns::Array::from_slice(&skip));
            let mut config = StreamCfgBuilder::default()
                .with_width(width as usize)
                .with_height(height as usize)
                .with_fps(fps)
                .with_shows_cursor(cursor)
                .build();
            config.set_pixel_format(cv::PixelFormat::_32_BGRA);
            if system.is_some() {
                // macOS 13+; the app's floor is 14.
                unsafe {
                    config.set_captures_audio(true);
                    config.set_excludes_current_process_audio(true);
                    config.set_sample_rate(SYSTEM_RATE as i64);
                    config.set_channel_count(1);
                }
            }

            let row_bytes = width as usize * 4;
            // The stream stopping by itself (screen recording switched off, the display gone,
            // the system's own stop) is capture dying: the recorder's watchdog hears it.
            let dead = sink.clone();
            let capturer = Capturer::builder(filter, config)
                .with_output_sample_buf_cb(move |frame| match frame {
                    Frame::Screen(mut video) => {
                        let Some(image) = video.sample_buf_mut().image_buf_mut() else { return };
                        copy_into(image, &sink, row_bytes, height as usize);
                    }
                    Frame::Audio(audio) => {
                        if let Some(system) = &system {
                            feed_audio(audio.sample_buf(), system);
                        }
                    }
                    Frame::Mic(_) => {}
                })
                .with_stop_with_err_cb(move |_stream, _error| dead.fail())
                .build()
                .map_err(|e| describe(&e))?;
            capturer.start().await.map_err(|e| describe(&e))?;
            Ok::<_, String>(capturer)
        })?;
        self.live = Some(Live(capturer));
        Ok(())
    }

    fn stop(&mut self) -> Result<(), String> {
        if let Some(Live(capturer)) = self.live.take() {
            futures::executor::block_on(capturer.stop()).map_err(|e| describe(&e))?;
        }
        Ok(())
    }
}

impl Drop for ScreenSource {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// Which of `excluded` ScreenCaptureKit's list of shareable windows doesn't have: the
/// question [`ScreenSource`]'s `start` asks before it records, where one window missing
/// refuses the take. Asked on its own it captures nothing. Only where recording is already
/// allowed — the listing itself would raise macOS's question anywhere else.
pub fn not_shareable(excluded: &[u32]) -> Result<Vec<u32>, String> {
    if !scap_screencapturekit::has_permission() {
        return Err("screen recording isn't allowed here".into());
    }
    let excluded = excluded.to_vec();
    futures::executor::block_on(async move {
        let content = sc::ShareableContent::current().await.map_err(|e| describe(&e))?;
        let found: Vec<u32> = content.windows().iter().map(|w| w.id()).collect();
        Ok(excluded.into_iter().filter(|n| !found.contains(n)).collect())
    })
}

/// Copies a BGRA pixel buffer into the sink, row by row (the buffer may pad its rows).
fn copy_into(image: &mut cv::PixelBuf, sink: &FrameSink, row_bytes: usize, rows: usize) {
    let flags = cv::pixel_buffer::LockFlags::READ_ONLY;
    if image.width() * 4 < row_bytes || image.height() < rows {
        return; // a frame of another size (mid display change) — skip it
    }
    unsafe {
        if image.lock_base_addr(flags).is_err() {
            return;
        }
        let base = CVPixelBufferGetBaseAddress(image);
        let stride = CVPixelBufferGetBytesPerRow(image);
        if !base.is_null() {
            sink.offer(|buf| {
                for y in 0..rows {
                    let src = std::slice::from_raw_parts(base.add(y * stride), row_bytes);
                    buf[y * row_bytes..(y + 1) * row_bytes].copy_from_slice(src);
                }
            });
        }
        let _ = image.unlock_lock_base_addr(flags);
    }
}

/// CoreAudio's description of a stream (`AudioStreamBasicDescription`).
#[repr(C)]
#[allow(dead_code)]
struct Asbd {
    sample_rate: f64,
    format_id: u32,
    format_flags: u32,
    bytes_per_packet: u32,
    frames_per_packet: u32,
    bytes_per_frame: u32,
    channels_per_frame: u32,
    bits_per_channel: u32,
    reserved: u32,
}

const FLAG_FLOAT: u32 = 1;
const FLAG_BIG_ENDIAN: u32 = 2;
const FLAG_NON_INTERLEAVED: u32 = 32;

#[link(name = "CoreMedia", kind = "framework")]
unsafe extern "C" {
    fn CMSampleBufferGetFormatDescription(sbuf: *const c_void) -> *const c_void;
    fn CMAudioFormatDescriptionGetStreamBasicDescription(desc: *const c_void) -> *const Asbd;
    fn CMSampleBufferGetPresentationTimeStamp(sbuf: *const c_void) -> CmTime;
    fn CMClockGetHostTimeClock() -> *const c_void;
    fn CMClockGetTime(clock: *const c_void) -> CmTime;
}

#[repr(C)]
#[derive(Clone, Copy)]
struct CmTime {
    value: i64,
    timescale: i32,
    flags: u32,
    epoch: i64,
}

impl CmTime {
    fn seconds(self) -> Option<f64> {
        (self.flags & 1 != 0 && self.timescale > 0).then(|| self.value as f64 / self.timescale as f64)
    }
}

/// The Mac's sound from one of ScreenCaptureKit's audio buffers, read as its own format
/// says (32-bit float or 16-bit integer, interleaved or one plane per channel), averaged
/// to mono. Asked for mono 32-bit float at 48 kHz; anything else is still read right.
fn feed_audio(buf: &cm::SampleBuf, system: &SystemAudio) {
    let asbd = unsafe {
        let desc = CMSampleBufferGetFormatDescription((buf as *const cm::SampleBuf).cast());
        if desc.is_null() { None } else { CMAudioFormatDescriptionGetStreamBasicDescription(desc).as_ref() }
    };
    let Some(asbd) = asbd else { return system.note_unreadable() };
    // When its first sample was heard: its presentation time against now, both on the host
    // clock ScreenCaptureKit stamps with — the mix places it by that (B172).
    let (pts, now) = unsafe { (CMSampleBufferGetPresentationTimeStamp((buf as *const cm::SampleBuf).cast()), CMClockGetTime(CMClockGetHostTimeClock())) };
    let arrived = std::time::Instant::now();
    let heard = match (pts.seconds(), now.seconds()) {
        (Some(pts), Some(now)) if now >= pts && now - pts < 5.0 => arrived.checked_sub(std::time::Duration::from_secs_f64(now - pts)).unwrap_or(arrived),
        _ => arrived,
    };
    let (float, bits, channels) = (asbd.format_flags & FLAG_FLOAT != 0, asbd.bits_per_channel as usize, asbd.channels_per_frame.max(1) as usize);
    let planar = asbd.format_flags & FLAG_NON_INTERLEAVED != 0;
    system.note_format(|| {
        format!(
            "{} Hz, {} ch, {} bit {}{}, flags {:#x}, {} bytes/frame",
            asbd.sample_rate,
            channels,
            bits,
            if float { "float" } else { "int" },
            if planar { " planar" } else { " interleaved" },
            asbd.format_flags,
            asbd.bytes_per_frame
        )
    });
    let frames = buf.num_samples() as usize;
    let Some(bytes) = buf.data_buf().and_then(|b| b.as_slice().ok()) else { return system.note_unreadable() };
    let width = bits / 8;
    let readable = asbd.format_flags & FLAG_BIG_ENDIAN == 0 && ((float && bits == 32) || (!float && bits == 16));
    if !readable || frames == 0 || bytes.len() < frames * channels * width {
        return system.note_unreadable();
    }
    let sample = |i: usize| -> f32 {
        let b = &bytes[i * width..(i + 1) * width];
        if float { f32::from_le_bytes([b[0], b[1], b[2], b[3]]) } else { i16::from_le_bytes([b[0], b[1]]) as f32 / 32_768.0 }
    };
    // Interleaved: frame f's channel c is sample f·channels + c; planar: plane c holds the
    // channel's samples in a row, the planes one after another (each maybe padded).
    let plane = bytes.len() / width / channels;
    let at = |f: usize, c: usize| if planar { c * plane + f } else { f * channels + c };
    let mono: Vec<f32> = (0..frames).map(|f| (0..channels).map(|c| sample(at(f, c))).sum::<f32>() / channels as f32).collect();
    if (asbd.sample_rate as u32) != SYSTEM_RATE {
        return system.note_unreadable();
    }
    system.feed_heard(&mono, heard);
}

fn describe(error: &arc::R<ns::Error>) -> String {
    format!("{} ({})", error.localized_description(), error.code())
}
