//! Screen capture: ScreenCaptureKit through the MIT `scap-screencapturekit` and cidre.

/// The drives' microphone: the test pattern's beep, never the room.
pub mod beep;

#[cfg(target_os = "macos")]
mod liveness;

#[cfg(target_os = "macos")]
pub use liveness::{grab_gray, liveness_probe};
#[cfg(target_os = "macos")]
pub use scap_screencapturekit::{has_permission, request_permission};

/// What a short capture proved.
///
/// Only `frames_lockable` is an honest liveness signal: permission plus a started stream
/// can still deliver frames whose pixels can't be read (SCK's default pixel format isn't
/// CPU-readable — history/RESEARCH.md, "Hands-on probe results").
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct LivenessReport {
    pub permission: bool,
    pub frames_delivered: usize,
    pub frames_lockable: usize,
    pub width: usize,
    pub height: usize,
    pub elapsed_ms: u128,
    pub error: Option<String>,
}

#[cfg(target_os = "macos")]
mod devices;
#[cfg(target_os = "macos")]
pub use devices::{Access, Camera, camera_access, cameras, mic_access, request_camera, request_mic};

#[cfg(target_os = "macos")]
mod screen;
#[cfg(target_os = "macos")]
pub use screen::{ScreenSource, main_display_id, not_shareable};

/// Which of a camera's formats to record from.
pub mod format;

/// When a camera picture has stopped, and its camera wants a restart.
pub mod watchdog;

#[cfg(target_os = "macos")]
mod camera;
#[cfg(target_os = "macos")]
pub use camera::CameraSource;

#[cfg(target_os = "macos")]
pub mod mic;

/// A virtual loopback as a microphone that hears only shroom's own signal (DECISIONS B59 e).
#[cfg(target_os = "macos")]
pub mod loopback;

#[cfg(target_os = "macos")]
mod preview;
#[cfg(target_os = "macos")]
pub use preview::CameraPreview;
