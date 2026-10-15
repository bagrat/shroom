//! Which of a camera's formats to record from (DECISIONS B33, B120). Plain data, so the
//! choice is tested on any platform; `camera.rs` reads the formats from AVFoundation.

use shroom_encode::quality::Quality;

/// One format a camera offers: its size and the frame-rate ranges it runs at.
#[derive(Debug, Clone, PartialEq)]
pub struct Offer {
    pub width: u32,
    pub height: u32,
    /// `(min, max)` frames per second, as AVFoundation lists them.
    pub rates: Vec<(f64, f64)>,
}

impl Offer {
    /// It can be held to `fps`: one of its ranges includes it.
    pub fn holds(&self, fps: f64) -> bool {
        self.rates.iter().any(|&(min, max)| min <= fps + 0.5 && max >= fps - 0.5)
    }

    /// It runs at `fps` or faster — the frame clock samples a faster camera at `fps`.
    pub fn reaches(&self, fps: f64) -> bool {
        self.rates.iter().any(|&(_, max)| max >= fps - 0.5)
    }
}

/// The format to record from: among those that can be held to `fps`, the largest that fits
/// the quality's bounds, failing that the smallest over them. A camera with none that can
/// be held there (OBS's virtual camera runs only at 60) is chosen among those that run
/// faster, the same way. `None` when nothing reaches `fps`.
pub fn choose(offers: &[Offer], quality: Quality, fps: f64) -> Option<usize> {
    let usable = |held: bool| -> Vec<usize> {
        (0..offers.len())
            .filter(|&i| {
                let o = &offers[i];
                o.width > 0 && o.height > 0 && if held { o.holds(fps) } else { o.reaches(fps) }
            })
            .collect()
    };
    let held = usable(true);
    let candidates = if held.is_empty() { usable(false) } else { held };
    let area = |i: &usize| offers[*i].width as u64 * offers[*i].height as u64;
    let (fits, over): (Vec<usize>, Vec<usize>) = candidates.into_iter().partition(|&i| {
        let o = &offers[i];
        quality.output_size(o.width, o.height) == (o.width, o.height)
    });
    fits.into_iter().max_by_key(area).or_else(|| over.into_iter().min_by_key(area))
}

/// The format of `size` to open again at start: one that can be held to `fps` if the camera
/// has it, else one that runs faster.
pub fn of_size(offers: &[Offer], size: (u32, u32), fps: f64) -> Option<usize> {
    let sized = |i: &usize| (offers[*i].width, offers[*i].height) == size;
    let mut same = (0..offers.len()).filter(sized);
    let held = (0..offers.len()).filter(sized).find(|&i| offers[i].holds(fps));
    held.or_else(|| same.find(|&i| offers[i].reaches(fps)))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn offer(width: u32, height: u32, rates: &[(f64, f64)]) -> Offer {
        Offer { width, height, rates: rates.to_vec() }
    }

    #[test]
    fn a_webcam_keeps_the_largest_format_that_fits_and_holds_thirty() {
        let offers = [
            offer(640, 480, &[(1.0, 30.0)]),
            offer(1920, 1080, &[(1.0, 30.0)]),
            offer(1280, 720, &[(1.0, 60.0)]),
            offer(3840, 2160, &[(1.0, 15.0)]),
        ];
        assert_eq!(choose(&offers, Quality::Normal, 30.0), Some(1));
    }

    #[test]
    fn a_format_that_holds_thirty_wins_over_a_larger_faster_only_one() {
        let offers = [offer(1280, 720, &[(15.0, 30.0)]), offer(1920, 1080, &[(60.0, 60.0)])];
        assert_eq!(choose(&offers, Quality::Normal, 30.0), Some(0));
    }

    #[test]
    fn a_camera_that_only_runs_faster_is_still_a_camera() {
        // OBS Virtual Camera 32.2.2, as AVFoundation lists it (held Mac, 2026-10-13).
        let offers = [offer(1920, 1080, &[(60.0, 60.0)])];
        assert_eq!(choose(&offers, Quality::Normal, 30.0), Some(0));
        assert!(!offers[0].holds(30.0));
        assert!(offers[0].reaches(30.0));
    }

    #[test]
    fn over_the_bounds_the_smallest_is_taken() {
        let offers = [offer(3840, 2160, &[(1.0, 30.0)]), offer(2560, 1440, &[(1.0, 30.0)])];
        assert_eq!(choose(&offers, Quality::Normal, 30.0), Some(1));
    }

    #[test]
    fn a_camera_slower_than_thirty_offers_nothing() {
        let offers = [offer(3840, 2160, &[(1.0, 15.0)]), offer(0, 0, &[(1.0, 30.0)])];
        assert_eq!(choose(&offers, Quality::Normal, 30.0), None);
    }

    #[test]
    fn at_start_the_same_size_is_found_again_preferring_one_held_to_thirty() {
        let offers = [offer(1920, 1080, &[(60.0, 60.0)]), offer(1920, 1080, &[(1.0, 30.0)]), offer(1280, 720, &[(1.0, 30.0)])];
        assert_eq!(of_size(&offers, (1920, 1080), 30.0), Some(1));
        assert_eq!(of_size(&offers[..1], (1920, 1080), 30.0), Some(0));
        assert_eq!(of_size(&offers, (640, 480), 30.0), None);
    }
}
