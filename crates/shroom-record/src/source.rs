use crate::handoff::FrameSink;

/// Anything that delivers BGRA frames of a fixed size: the screen, the camera, a test
/// pattern. Delivery is push-based and irregular — the screen sends a frame only when
/// it changes; the frame clock downstream makes the rate constant.
pub trait FrameSource: Send {
    /// The size of every frame this source delivers, in pixels.
    fn size(&self) -> (u32, u32);
    /// Starts delivering frames into `sink` (from any thread).
    fn start(&mut self, sink: FrameSink) -> Result<(), String>;
    /// Stops delivery; no frame reaches the sink after this returns.
    fn stop(&mut self) -> Result<(), String>;
    /// Whether frames keep coming while it's alive, even of a still picture — a camera does;
    /// the screen sends a frame only when something changes. Only a continuous source's
    /// silence means it died (the recorder's watchdog).
    fn continuous(&self) -> bool {
        false
    }
}
