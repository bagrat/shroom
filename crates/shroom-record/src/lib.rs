//! The recording machine's mechanism: frames from a source, through the frame clock,
//! into one encode per take; pause ends a take, resume starts the next; finalize joins
//! them (POC §2 semantics, ScreenCaptureKit instead of avfoundation).
//!
//! Sources are a trait so the whole path runs in tests without a screen.

pub mod audio;
pub mod handoff;
pub mod probe;
pub mod recording;
pub mod repair;
pub mod source;

pub use audio::{AudioSink, AudioSource, AudioStats, Silence};
pub use handoff::FrameSink;
pub use recording::{Finalized, Recording, RecordError, Stall, TakeStats};
pub use source::FrameSource;
