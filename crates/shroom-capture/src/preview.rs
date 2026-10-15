//! A live camera picture for shroom's own windows — the Show yourself bubble (mirrored)
//! and Camera-as-the-source's preview at `config` (as recorded) — through
//! AVCaptureVideoPreviewLayer, on the exact camera the core picked by its id (N39: never
//! an iPhone by itself; WebKit's `getUserMedia` can only start from the system default).
//! The layer is handed to the shell, which puts it in a window.
//!
//! The camera starts and stops on shroom's camera thread, never the caller's:
//! `startRunning` blocks until the camera delivers — a second or more — and on the main
//! thread it held everything else back with it, the popover too.

use std::ffi::c_void;
use std::sync::{OnceLock, mpsc};

use cidre::{arc, av, ns};

pub struct CameraPreview {
    session: arc::R<av::capture::Session>,
    layer: arc::R<av::CaptureVideoPreviewLayer>,
    _input: arc::R<av::CaptureDeviceInput>,
    uid: String,
}
// The session and its layer are thread-safe Objective-C objects; the shell only places the
// layer (main thread) and drops the preview.
unsafe impl Send for CameraPreview {}

/// A session handed to the camera thread to start or stop.
struct Session(arc::R<av::capture::Session>);
// SAFETY: AVCaptureSession is thread-safe; Apple's own advice is to start and stop it off
// the main thread.
unsafe impl Send for Session {}

impl Session {
    /// Blocks until the camera delivers (or gives up); whether it's running.
    fn start(&mut self) -> bool {
        self.0.start_running();
        self.0.is_running()
    }

    fn stop(&mut self) {
        self.0.stop_running();
    }
}

/// Runs `job` on shroom's camera thread: one thread, so a session's start and stop keep
/// their order.
fn on_camera_thread(job: impl FnOnce() + Send + 'static) {
    type Job = Box<dyn FnOnce() + Send>;
    static JOBS: OnceLock<mpsc::Sender<Job>> = OnceLock::new();
    let jobs = JOBS.get_or_init(|| {
        let (tx, rx) = mpsc::channel::<Job>();
        std::thread::Builder::new()
            .name("shroom-camera".into())
            .spawn(move || {
                for job in rx {
                    cidre::objc::ar_pool(job);
                }
            })
            .expect("the camera thread");
        tx
    });
    let _ = jobs.send(Box::new(job));
}

impl CameraPreview {
    /// Opens the camera `uid` into a preview layer, `mirrored` like a mirror (the bubble)
    /// or as it records (Camera as the source). The layer shows nothing until `run`.
    pub fn open(uid: &str, mirrored: bool) -> Result<CameraPreview, String> {
        let device = av::CaptureDevice::with_unique_id(&ns::String::with_str(uid)).ok_or("that camera isn't available")?;
        let input = av::capture::DeviceInput::with_device(&device).map_err(|e| e.localized_description().to_string())?;
        let mut session = av::capture::Session::new();
        let mut added = false;
        session.configure(|s| {
            // The camera's format is the take's, never the preview's: a preview that opens
            // (or restarts) beside a take recording the same camera must not reset the format
            // and frame rate the recorder locked (Camera only, Bagrat's 0.1.10 checkpoint).
            let keep = av::CaptureSessionPreset::input_priority();
            if s.can_set_session_preset(keep) {
                let _ = s.set_session_preset(keep);
            }
            if s.can_add_input(&input) {
                s.add_input(&input);
                added = true;
            }
        });
        if !added {
            return Err("the camera couldn't be opened".into());
        }
        let mut layer = av::CaptureVideoPreviewLayer::with_session(&session);
        layer.set_video_gravity(av::LayerVideoGravity::resize_aspect_fill());
        if let Some(conn) = layer.connection() {
            let mut conn = conn.retained();
            if conn.is_video_mirroring_supported() {
                conn.set_automatically_adjusts_video_mirroring(false);
                conn.set_video_mirrored(mirrored);
            }
        }
        Ok(CameraPreview { session, layer, _input: input, uid: uid.to_string() })
    }

    /// Starts the camera, on the camera thread; `started` runs there once it's up, with
    /// whether it's delivering.
    pub fn run(&self, started: impl FnOnce(bool) + Send + 'static) {
        let mut s = Session(self.session.retained());
        on_camera_thread(move || started(s.start()));
    }

    /// The `CALayer` to put in a view (an `AVCaptureVideoPreviewLayer`).
    pub fn layer(&self) -> *mut c_void {
        let l: &av::CaptureVideoPreviewLayer = &self.layer;
        l as *const av::CaptureVideoPreviewLayer as *mut c_void
    }

    pub fn uid(&self) -> &str {
        &self.uid
    }

    /// Whether the camera is delivering into the layer.
    pub fn running(&self) -> bool {
        self.session.is_running()
    }
}

impl Drop for CameraPreview {
    fn drop(&mut self) {
        // Stopping blocks too; the camera thread lets the session go once it has.
        let mut s = Session(self.session.retained());
        on_camera_thread(move || s.stop());
    }
}
