//! The screen as a frame source: ScreenCaptureKit through the MIT `scap-screencapturekit` crate.
//!
//! Frames are scaled to the output size by ScreenCaptureKit itself (on the GPU), forced
//! to BGRA (the default format isn't CPU-readable — RESEARCH, probe results), and copied
//! row by row into the hand-off, dropping any row padding.

use std::ffi::c_void;

use cidre::{arc, cv, ns, sc};
use scap_screencapturekit::{Capturer, Frame, StreamCfgBuilder};
use shroom_encode::quality::Quality;
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
    live: Option<Live>,
}

impl ScreenSource {
    /// The display `display_id`, captured at the size `quality` fits it into.
    pub fn new(display_id: u32, quality: Quality) -> Result<ScreenSource, String> {
        let (pw, ph) = pixel_size(display_id).ok_or("that display isn't available")?;
        let (width, height) = quality.output_size(pw, ph);
        Ok(ScreenSource { display_id, width, height, cursor: true, capture_fps: 60.0, excluded: Vec::new(), live: None })
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

            let row_bytes = width as usize * 4;
            // The stream stopping by itself (screen recording switched off, the display gone,
            // the system's own stop) is capture dying: the recorder's watchdog hears it.
            let dead = sink.clone();
            let capturer = Capturer::builder(filter, config)
                .with_output_sample_buf_cb(move |frame| {
                    let Frame::Screen(mut video) = frame else { return };
                    let Some(image) = video.sample_buf_mut().image_buf_mut() else { return };
                    copy_into(image, &sink, row_bytes, height as usize);
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

fn describe(error: &arc::R<ns::Error>) -> String {
    format!("{} ({})", error.localized_description(), error.code())
}
