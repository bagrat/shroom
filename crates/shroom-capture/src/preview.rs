//! A live camera picture for shroom's own windows — the camera bubble (mirrored)
//! and Camera-as-the-source's preview at `config` (as recorded) — on the exact camera the
//! core picked by its id (N39: never an iPhone by itself; WebKit's `getUserMedia` can only
//! start from the system default). The layer is handed to the shell, which puts it in a
//! window.
//!
//! shroom draws the picture itself: each frame the camera delivers (an
//! `AVCaptureVideoDataOutput`) becomes the layer's contents on the main thread. It used to
//! be an `AVCaptureVideoPreviewLayer`, whose picture comes from inside AVFoundation: in the
//! 2026-10-17 rehearsal it froze on a switch into a full-screen Space and never moved again,
//! the camera's light still on, and nothing in shroom could tell. Now each frame drawn is
//! counted, and a picture that stops is a camera restarted (`watchdog`).
//!
//! The camera starts and stops on shroom's camera thread, never the caller's:
//! `startRunning` blocks until the camera delivers — a second or more — and on the main
//! thread it held everything else back with it, the popover too.

use std::ffi::c_void;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, OnceLock, mpsc};
use std::time::{Duration, Instant};

use cap_camera_avfoundation::{CallbackOutputDelegate, CallbackOutputDelegateInner};
use cidre::{arc, av, ca, cg, cv, dispatch, ns};
use objc2::runtime::{AnyClass, AnyObject, Bool};
use objc2::msg_send;

use crate::watchdog::Watchdog;

#[link(name = "CoreVideo", kind = "framework")]
unsafe extern "C" {
    fn CVPixelBufferGetIOSurface(pixel_buffer: &cv::PixelBuf) -> *mut c_void;
}

/// How often the watchdog reads the frame count.
const WATCH_EVERY: Duration = Duration::from_millis(250);

pub struct CameraPreview {
    session: arc::R<av::capture::Session>,
    output: arc::R<av::CaptureVideoDataOutput>,
    delegate: arc::R<CallbackOutputDelegate>,
    _input: arc::R<av::CaptureDeviceInput>,
    shared: Arc<Shared>,
    uid: String,
}
// The session and its output are thread-safe Objective-C objects; the layer is only touched
// on the main thread (the shell places it; `show` draws into it).
unsafe impl Send for CameraPreview {}

/// What the camera's queue, the main thread and the watchdog share.
struct Shared {
    layer: arc::R<ca::Layer>,
    /// The newest frame not drawn yet; a draw is on its way to the main thread while it's set.
    next: Mutex<Option<Frame>>,
    /// The frame the layer shows, kept so the camera doesn't reuse its surface meanwhile.
    shown: Mutex<Option<Frame>>,
    drawn: AtomicU64,
    closed: AtomicBool,
}
// SAFETY: the layer is only messaged on the main thread (`show`); the rest is locked or atomic.
unsafe impl Send for Shared {}
unsafe impl Sync for Shared {}

/// One camera frame, held.
struct Frame(arc::R<cv::PixelBuf>);
// SAFETY: a CVPixelBuffer is a thread-safe CF object; it's only read (its surface) once handed on.
unsafe impl Send for Frame {}

/// A session (and what goes with it) handed to the camera thread.
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

/// The output and its delegate, let go of on the camera thread once the session has stopped.
struct Ending(arc::R<av::CaptureVideoDataOutput>, arc::R<CallbackOutputDelegate>);
// SAFETY: as `Session`; the delegate is only dropped there, after its output let it go.
unsafe impl Send for Ending {}

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

/// Runs `f` on the main queue.
fn on_main(f: impl FnOnce() + Send + 'static) {
    unsafe extern "C" {
        static _dispatch_main_q: c_void;
        fn dispatch_async_f(queue: *const c_void, context: *mut c_void, work: extern "C" fn(*mut c_void));
    }
    extern "C" fn trampoline(ctx: *mut c_void) {
        // SAFETY: ctx is the Box made below, run exactly once.
        let f: Box<Box<dyn FnOnce()>> = unsafe { Box::from_raw(ctx as *mut Box<dyn FnOnce()>) };
        cidre::objc::ar_pool(f);
    }
    let boxed: Box<Box<dyn FnOnce()>> = Box::new(Box::new(f));
    // SAFETY: the main queue outlives the process; the context is consumed by the trampoline.
    unsafe { dispatch_async_f(&raw const _dispatch_main_q, Box::into_raw(boxed) as *mut c_void, trampoline) };
}

/// Draws the newest frame into the layer (main thread).
fn show(shared: &Shared) {
    let Some(frame) = shared.next.lock().unwrap().take() else { return };
    if shared.closed.load(Ordering::SeqCst) {
        return;
    }
    // SAFETY: the surface belongs to the frame, which `shown` keeps until the next one is
    // drawn; the layer is messaged on the main thread, in a transaction with no animation
    // (a layer's contents otherwise cross-fade on every change).
    unsafe {
        let surface = CVPixelBufferGetIOSurface(&frame.0);
        if surface.is_null() {
            return;
        }
        let layer = &*(&*shared.layer as *const ca::Layer as *const AnyObject);
        let transaction = AnyClass::get(c"CATransaction").expect("QuartzCore's CATransaction");
        let _: () = msg_send![transaction, begin];
        let _: () = msg_send![transaction, setDisableActions: Bool::YES];
        let _: () = msg_send![layer, setContents: surface as *mut AnyObject];
        let _: () = msg_send![transaction, commit];
    }
    *shared.shown.lock().unwrap() = Some(frame);
    shared.drawn.fetch_add(1, Ordering::SeqCst);
}

impl CameraPreview {
    /// Opens the camera `uid` into a layer, `mirrored` like a mirror (the bubble) or as it
    /// records (Camera as the source). The layer shows nothing until `run`.
    pub fn open(uid: &str, mirrored: bool) -> Result<CameraPreview, String> {
        let device = av::CaptureDevice::with_unique_id(&ns::String::with_str(uid)).ok_or("that camera isn't available")?;
        let input = av::capture::DeviceInput::with_device(&device).map_err(|e| e.localized_description().to_string())?;
        let output = av::capture::VideoDataOutput::new();
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
            if s.can_add_input(&input) && s.can_add_output(&output) {
                s.add_input(&input);
                s.add_output(&output);
                added = true;
            }
        });
        if !added {
            return Err("the camera couldn't be opened".into());
        }
        let mut output = output;
        // BGRA at the camera's own size: what a layer draws as is.
        let settings = ns::Dictionary::with_keys_values(&[cv::pixel_buffer_keys::pixel_format().as_ns()], &[ns::Number::with_u32(cv::PixelFormat::_32_BGRA.0).as_id_ref()]);
        output.set_video_settings(Some(settings.as_ref())).map_err(|e| format!("the camera refused BGRA output: {e:?}"))?;
        output.set_always_discard_late_video_frames(true);

        let layer = ca::Layer::new();
        // Filled edge to edge, cropped to the window's shape (`kCAGravityResizeAspectFill`).
        let fill = ns::String::with_str("resizeAspectFill");
        // SAFETY: a plain QuartzCore message to a layer nothing shows yet.
        unsafe {
            let l = &*(&*layer as *const ca::Layer as *const AnyObject);
            let _: () = msg_send![l, setContentsGravity: &*fill as *const ns::String as *mut AnyObject];
        }
        if mirrored {
            layer.set_affine_transform(cg::AffineTransform::new_scale(-1.0, 1.0));
        }
        let shared = Arc::new(Shared { layer, next: Mutex::default(), shown: Mutex::default(), drawn: AtomicU64::new(0), closed: AtomicBool::new(false) });
        let to_draw = shared.clone();
        let delegate = CallbackOutputDelegate::with(CallbackOutputDelegateInner::new(Box::new(move |data| {
            let Some(image) = data.sample_buf.image_buf() else { return };
            // Only the newest frame waits; a draw is already on its way when one did.
            let waiting = to_draw.next.lock().unwrap().replace(Frame(image.retained())).is_some();
            if !waiting {
                let shared = to_draw.clone();
                on_main(move || show(&shared));
            }
        })));
        let queue = dispatch::Queue::new();
        output.set_sample_buf_delegate(Some(delegate.as_ref()), Some(&queue));
        Ok(CameraPreview { session, output, delegate, _input: input, shared, uid: uid.to_string() })
    }

    /// Starts the camera, on the camera thread; `started` runs there once it's up, with
    /// whether it's delivering. From then on a picture that stops restarts the camera, and
    /// `restarted` hears of each restart (with how many so far).
    pub fn run(&self, started: impl FnOnce(bool) + Send + 'static, restarted: impl Fn(u32) + Send + 'static) {
        let mut s = Session(self.session.retained());
        let shared = self.shared.clone();
        on_camera_thread(move || {
            started(s.start());
            // The watchdog's own thread: it ends when the preview closes.
            let _ = std::thread::Builder::new().name("shroom-camera-watch".into()).spawn(move || {
                let epoch = Instant::now();
                let mut dog = Watchdog::new(Duration::ZERO);
                while !shared.closed.load(Ordering::SeqCst) {
                    std::thread::sleep(WATCH_EVERY);
                    if dog.check(epoch.elapsed(), shared.drawn.load(Ordering::SeqCst)) && !shared.closed.load(Ordering::SeqCst) {
                        restarted(dog.restarts());
                        let mut again = Session(s.0.retained());
                        on_camera_thread(move || {
                            again.stop();
                            again.start();
                        });
                    }
                }
            });
        });
    }

    /// The `CALayer` to put in a view.
    pub fn layer(&self) -> *mut c_void {
        let l: &ca::Layer = &self.shared.layer;
        l as *const ca::Layer as *mut c_void
    }

    pub fn uid(&self) -> &str {
        &self.uid
    }

    /// Whether the camera is running.
    pub fn running(&self) -> bool {
        self.session.is_running()
    }

    /// The frames drawn so far.
    pub fn frames(&self) -> u64 {
        self.shared.drawn.load(Ordering::SeqCst)
    }
}

impl Drop for CameraPreview {
    fn drop(&mut self) {
        self.shared.closed.store(true, Ordering::SeqCst);
        // Stopping blocks too; the camera thread lets the session go once it has, then the
        // output lets its delegate go.
        let mut s = Session(self.session.retained());
        let ending = Ending(self.output.retained(), self.delegate.retained());
        on_camera_thread(move || {
            s.stop();
            let Ending(mut output, delegate) = ending;
            output.set_sample_buf_delegate::<CallbackOutputDelegate>(None, None);
            drop(delegate);
        });
    }
}
