//! Quality presets: a box the capture is scaled to fit, and a bitrate (POC
//! `lib/quality.mjs`). ScreenCaptureKit scales on the GPU at capture time, so frames
//! reach the encoder already at the output size.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum Quality {
    #[default]
    Normal,
    #[serde(rename = "2k")]
    TwoK,
    #[serde(rename = "4k")]
    FourK,
}

impl Quality {
    /// (max width, max height) of the box the output fits inside.
    pub fn bounds(self) -> (u32, u32) {
        match self {
            Quality::Normal => (1920, 1080),
            Quality::TwoK => (2560, 1440),
            Quality::FourK => (3840, 2160),
        }
    }

    pub fn video_kbps(self) -> u32 {
        match self {
            Quality::Normal => 3500,
            Quality::TwoK => 6000,
            Quality::FourK => 12000,
        }
    }

    /// The output size for a source of `width`×`height` pixels: fit the box, keep the
    /// aspect ratio, never upscale, even dimensions (yuv420p needs them).
    pub fn output_size(self, width: u32, height: u32) -> (u32, u32) {
        let (max_w, max_h) = self.bounds();
        let scale = f64::min(1.0, f64::min(max_w as f64 / width as f64, max_h as f64 / height as f64));
        let even = |v: f64| ((v.floor() as u32) & !1).max(2);
        (even(width as f64 * scale), even(height as f64 * scale))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fits_a_4k_display_into_1080p() {
        assert_eq!(Quality::Normal.output_size(3840, 2160), (1920, 1080));
    }

    #[test]
    fn never_upscales() {
        assert_eq!(Quality::FourK.output_size(1920, 1080), (1920, 1080));
    }

    #[test]
    fn keeps_aspect_and_even_dimensions_for_odd_shapes() {
        // A 16:10 laptop panel at 2x: 3024x1964 → fits 1080 high.
        let (w, h) = Quality::Normal.output_size(3024, 1964);
        assert_eq!(h, 1080);
        assert_eq!(w % 2, 0);
        assert!((w as f64 / h as f64 - 3024.0 / 1964.0).abs() < 0.01);
    }
}
