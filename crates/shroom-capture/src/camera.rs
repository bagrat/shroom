//! A camera as a frame source (camera-as-source, ARCHITECTURE §4.4): AVFoundation
//! through the MIT `cap-camera-avfoundation` delegate, the session set up the way
//! the MIT `cap-camera` crate does. AVFoundation converts and scales to BGRA at the output
//! size, so frames enter the same hand-off → frame clock → encoder path as the screen —
//! encode, pause and upload stay one path.

use cap_camera_avfoundation::{CallbackOutputDelegate, CallbackOutputDelegateInner};
use cidre::{arc, av, cm, cv, dispatch, ns};
use crate::format::{self, Offer};
use shroom_encode::quality::Quality;
use shroom_record::{FrameSink, FrameSource};

#[link(name = "CoreVideo", kind = "framework")]
unsafe extern "C" {
    fn CVPixelBufferGetBaseAddress(pixel_buffer: &cv::PixelBuf) -> *const u8;
    fn CVPixelBufferGetBytesPerRow(pixel_buffer: &cv::PixelBuf) -> usize;
}

/// The rate the frame clock samples at; the camera is asked for no faster.
const FPS: f64 = 30.0;

struct Live {
    session: arc::R<av::capture::Session>,
    output: arc::R<av::CaptureVideoDataOutput>,
    input: arc::R<av::CaptureDeviceInput>,
    _delegate: arc::R<CallbackOutputDelegate>,
    _device: arc::R<av::CaptureDevice>,
}
// AVCaptureSession and friends are thread-safe Objective-C objects; this owner only
// starts and stops them.
unsafe impl Send for Live {}

pub struct CameraSource {
    uid: String,
    format: (u32, u32),
    width: u32,
    height: u32,
    live: Option<Live>,
}

impl CameraSource {
    /// The camera `uid`, at the size `quality` fits the format `format::choose` picks into.
    pub fn new(uid: &str, quality: Quality) -> Result<CameraSource, String> {
        let device = av::CaptureDevice::with_unique_id(&ns::String::with_str(uid)).ok_or("that camera isn't available")?;
        let format = best_format(&device, quality).ok_or("that camera offers no usable format")?;
        let dims = format.format_desc().dims();
        let (width, height) = quality.output_size(dims.width as u32, dims.height as u32);
        Ok(CameraSource { uid: uid.to_string(), format: (dims.width as u32, dims.height as u32), width, height, live: None })
    }

    /// The camera's own format size (before scaling to the output size).
    pub fn native_size(&self) -> (u32, u32) {
        self.format
    }
}

/// The format `format::choose` picks from what the camera offers.
fn best_format(device: &av::CaptureDevice, quality: Quality) -> Option<arc::R<av::CaptureDeviceFormat>> {
    let formats: Vec<_> = device.formats().iter().map(|f| f.retained()).collect();
    let offers: Vec<_> = formats.iter().map(|f| offer(f)).collect();
    format::choose(&offers, quality, FPS).map(|i| formats[i].clone())
}

fn offer(format: &av::CaptureDeviceFormat) -> Offer {
    let d = format.format_desc().dims();
    let rates = format.video_supported_frame_rate_ranges().iter().map(|r| (r.min_frame_rate(), r.max_frame_rate())).collect();
    Offer { width: d.width.max(0) as u32, height: d.height.max(0) as u32, rates }
}

impl FrameSource for CameraSource {
    fn size(&self) -> (u32, u32) {
        (self.width, self.height)
    }

    /// A camera delivers every frame, a still room included: its silence is death.
    fn continuous(&self) -> bool {
        true
    }

    fn start(&mut self, sink: FrameSink) -> Result<(), String> {
        let mut device = av::CaptureDevice::with_unique_id(&ns::String::with_str(&self.uid)).ok_or("that camera isn't available")?;
        let format = {
            let formats: Vec<_> = device.formats().iter().map(|f| f.retained()).collect();
            let offers: Vec<_> = formats.iter().map(|f| offer(f)).collect();
            format::of_size(&offers, self.format, FPS).map(|i| formats[i].clone()).ok_or("the camera's format went away")?
        };
        let held = offer(&format).holds(FPS);
        let input = av::capture::DeviceInput::with_device(&device).map_err(|e| e.localized_description().to_string())?;
        let (width, height) = (self.width, self.height);
        let row_bytes = width as usize * 4;
        let delegate = CallbackOutputDelegate::with(CallbackOutputDelegateInner::new(Box::new(move |data| {
            let Some(image) = data.sample_buf.image_buf() else { return };
            copy_into(image, &sink, row_bytes, height as usize);
        })));
        let mut output = av::capture::VideoDataOutput::new();
        let mut session = av::capture::Session::new();
        let mut added = false;
        session.configure(|s| {
            if s.can_add_input(&input) {
                s.add_input(&input);
                s.add_output(&output);
                added = true;
            }
        });
        if !added {
            return Err("the camera couldn't be opened".into());
        }
        // BGRA at the output size: AVFoundation converts and scales, the hand-off copies rows.
        let settings = ns::Dictionary::with_keys_values(
            &[cv::pixel_buffer_keys::pixel_format().as_ns(), cv::pixel_buffer_keys::width().as_ns(), cv::pixel_buffer_keys::height().as_ns()],
            &[ns::Number::with_u32(cv::PixelFormat::_32_BGRA.0).as_id_ref(), ns::Number::with_u32(width).as_id_ref(), ns::Number::with_u32(height).as_id_ref()],
        );
        output.set_video_settings(Some(settings.as_ref())).map_err(|e| format!("the camera refused BGRA output: {e:?}"))?;
        output.set_always_discard_late_video_frames(true);
        let queue = dispatch::Queue::new();
        output.set_sample_buf_delegate(Some(delegate.as_ref()), Some(&queue));
        {
            // The device stays locked while running starts, or starting resets the format;
            // the frame duration is capped at 30 fps where the format allows it. One that
            // runs only faster (OBS's virtual camera: 60) throws at the cap and then sends
            // nothing, so it runs at its own rate and the frame clock samples it at 30.
            let mut lock = device.config_lock().map_err(|e| e.localized_description().to_string())?;
            lock.set_active_format(&format);
            if held {
                let _ = lock.set_active_video_min_frame_duration(cm::Time::new(1, FPS as i32));
            }
            session.start_running();
        }
        self.live = Some(Live { session, output, input, _delegate: delegate, _device: device });
        Ok(())
    }

    fn stop(&mut self) -> Result<(), String> {
        if let Some(mut live) = self.live.take() {
            live.session.stop_running();
            live.output.set_sample_buf_delegate::<CallbackOutputDelegate>(None, None);
            let (output, input) = (live.output.clone(), live.input.clone());
            live.session.configure(move |s| {
                s.remove_output(&output);
                s.remove_input(&input);
            });
        }
        Ok(())
    }
}

impl Drop for CameraSource {
    fn drop(&mut self) {
        let _ = self.stop();
    }
}

/// Copies a BGRA pixel buffer into the sink, row by row (rows may be padded).
fn copy_into(image: &cv::PixelBuf, sink: &FrameSink, row_bytes: usize, rows: usize) {
    let flags = cv::pixel_buffer::LockFlags::READ_ONLY;
    if image.width() * 4 < row_bytes || image.height() < rows || image.pixel_format() != cv::PixelFormat::_32_BGRA {
        return;
    }
    // SAFETY: the buffer is locked read-only for the duration of the copy; rows are read
    // within `stride` × `rows`, which the size check above guarantees.
    unsafe {
        let image = image as *const cv::PixelBuf as *mut cv::PixelBuf;
        if (*image).lock_base_addr(flags).is_err() {
            return;
        }
        let base = CVPixelBufferGetBaseAddress(&*image);
        let stride = CVPixelBufferGetBytesPerRow(&*image);
        if !base.is_null() {
            sink.offer(|buf| {
                for y in 0..rows {
                    let src = std::slice::from_raw_parts(base.add(y * stride), row_bytes);
                    buf[y * row_bytes..(y + 1) * row_bytes].copy_from_slice(src);
                }
            });
        }
        let _ = (*image).unlock_lock_base_addr(flags);
    }
}
