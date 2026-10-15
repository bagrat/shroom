use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use cidre::{cv, ns, sc};
use scap_screencapturekit::{Capturer, Frame, StreamCfgBuilder, has_permission};

use crate::LivenessReport;

#[link(name = "CoreVideo", kind = "framework")]
unsafe extern "C" {
    fn CVPixelBufferGetBaseAddress(pixel_buffer: &cv::PixelBuf) -> *const u8;
    fn CVPixelBufferGetBytesPerRow(pixel_buffer: &cv::PixelBuf) -> usize;
}

/// Captures the main display for `duration` at `fps` and counts the frames that arrive
/// and the ones whose pixels can actually be read.
pub fn liveness_probe(duration: Duration, fps: f32) -> LivenessReport {
    let started = Instant::now();
    let delivered = Arc::new(AtomicUsize::new(0));
    let lockable = Arc::new(AtomicUsize::new(0));

    let mut report = LivenessReport { permission: has_permission(), ..Default::default() };

    let outcome = futures::executor::block_on(capture(
        duration,
        fps,
        delivered.clone(),
        lockable.clone(),
        &mut report,
    ));
    if let Err(message) = outcome {
        report.error = Some(message);
    }

    report.frames_delivered = delivered.load(Ordering::SeqCst);
    report.frames_lockable = lockable.load(Ordering::SeqCst);
    report.elapsed_ms = started.elapsed().as_millis();
    report
}

async fn capture(
    duration: Duration,
    fps: f32,
    delivered: Arc<AtomicUsize>,
    lockable: Arc<AtomicUsize>,
    report: &mut LivenessReport,
) -> Result<(), String> {
    let content = sc::ShareableContent::current().await.map_err(describe)?;
    let displays = content.displays();
    let display = displays.first().ok_or("no display to capture")?;
    report.width = display.width() as usize;
    report.height = display.height() as usize;

    let filter = sc::ContentFilter::with_display_excluding_windows(display, &ns::Array::new());
    let mut config = StreamCfgBuilder::default()
        .with_width(report.width)
        .with_height(report.height)
        .with_fps(fps)
        .build();
    config.set_pixel_format(cv::PixelFormat::_32_BGRA);

    let capturer = Capturer::builder(filter, config)
        .with_output_sample_buf_cb(move |frame| {
            let Frame::Screen(mut video) = frame else { return };
            delivered.fetch_add(1, Ordering::SeqCst);
            if let Some(image) = video.sample_buf_mut().image_buf_mut() {
                if is_readable(image) {
                    lockable.fetch_add(1, Ordering::SeqCst);
                }
            }
        })
        .build()
        .map_err(describe)?;

    capturer.start().await.map_err(describe)?;
    std::thread::sleep(duration);
    capturer.stop().await.map_err(describe)?;
    Ok(())
}

fn is_readable(image: &mut cv::PixelBuf) -> bool {
    let flags = cv::pixel_buffer::LockFlags::READ_ONLY;
    unsafe {
        if image.lock_base_addr(flags).is_err() {
            return false;
        }
        let readable = !CVPixelBufferGetBaseAddress(image).is_null();
        let _ = image.unlock_lock_base_addr(flags);
        readable
    }
}

fn describe(error: impl AsRef<ns::Error>) -> String {
    let error = error.as_ref();
    format!("{} ({})", error.localized_description(), error.code())
}

/// One frame of display `display_id` at `width`×`height`, as 8-bit gray — what the test
/// harness reads to prove the pattern covers the display before it records anything.
pub fn grab_gray(display_id: u32, width: u32, height: u32) -> Result<Vec<u8>, String> {
    let slot: Arc<std::sync::Mutex<Option<Vec<u8>>>> = Arc::default();
    let out = slot.clone();
    let waiting = slot.clone();
    futures::executor::block_on(async move {
        let content = sc::ShareableContent::current().await.map_err(describe)?;
        let displays = content.displays();
        let display = displays.iter().find(|d| d.display_id().0 == display_id).ok_or("no such display")?;
        let filter = sc::ContentFilter::with_display_excluding_windows(display, &ns::Array::new());
        let mut config = StreamCfgBuilder::default().with_width(width as usize).with_height(height as usize).with_fps(30.0).with_shows_cursor(false).build();
        config.set_pixel_format(cv::PixelFormat::_32_BGRA);
        let (w, h) = (width as usize, height as usize);
        let capturer = Capturer::builder(filter, config)
            .with_output_sample_buf_cb(move |frame| {
                let Frame::Screen(mut video) = frame else { return };
                let Some(image) = video.sample_buf_mut().image_buf_mut() else { return };
                if out.lock().unwrap().is_some() || image.width() < w || image.height() < h {
                    return;
                }
                let flags = cv::pixel_buffer::LockFlags::READ_ONLY;
                unsafe {
                    if image.lock_base_addr(flags).is_err() {
                        return;
                    }
                    let base = CVPixelBufferGetBaseAddress(image);
                    let stride = CVPixelBufferGetBytesPerRow(image);
                    if !base.is_null() {
                        let mut gray = vec![0u8; w * h];
                        for y in 0..h {
                            let row = std::slice::from_raw_parts(base.add(y * stride), w * 4);
                            for x in 0..w {
                                let p = &row[x * 4..x * 4 + 3];
                                gray[y * w + x] = ((p[0] as u16 + p[1] as u16 + p[2] as u16) / 3) as u8;
                            }
                        }
                        *out.lock().unwrap() = Some(gray);
                    }
                    let _ = image.unlock_lock_base_addr(flags);
                }
            })
            .build()
            .map_err(describe)?;
        capturer.start().await.map_err(describe)?;
        for _ in 0..100 {
            if waiting.lock().unwrap().is_some() {
                break;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        capturer.stop().await.map_err(describe)?;
        Ok::<(), String>(())
    })?;
    let frame = slot.lock().unwrap().take();
    frame.ok_or_else(|| "no frame arrived".into())
}
