//! The media-truth pattern (TESTING-PLAN §4).
//!
//! Every frame carries a **strip**: a 4×8 grid of square cells in the top-left corner,
//! each fully black (0) or fully white (1), encoding 32 bits row-major, most significant
//! first — a 24-bit counter followed by an 8-bit CRC of it, the whole word XORed with
//! [`MASK`] so that a blank frame (all black or all white) can never decode — without the
//! mask, all-black reads as counter 0 with a valid CRC (found 2026-10-02 when a capture
//! that excluded the pattern window came back black and still "read"). Cells are large (a twelfth
//! of the frame height) so the code survives scaling and H.264 at the lowest bitrate;
//! positions are fractions of the frame, so a strip drawn full-screen on any display can
//! be read back from video at any output size. A wrong CRC means "unreadable", never a
//! wrong number.
//!
//! The on-screen pattern window draws the identical layout (see `layout()`), so real
//! captures are read by the same decoder.

/// Rows and columns of the strip.
pub const ROWS: u32 = 4;
pub const COLS: u32 = 8;
/// Largest counter value (24 bits).
pub const MAX_COUNTER: u32 = (1 << 24) - 1;
/// XORed over the word: blank frames never decode.
pub const MASK: u32 = 0xA5C3_5A3C;

/// Where the strip sits in a `width`×`height` frame: (cell size, left, top), in pixels.
pub fn layout(_width: u32, height: u32) -> (u32, u32, u32) {
    let cell = (height / 12).max(4);
    (cell, cell, cell)
}

/// CRC-8 (polynomial 0x07) over the counter's three bytes.
pub fn crc8(counter: u32) -> u8 {
    let mut crc: u8 = 0;
    for byte in [(counter >> 16) as u8, (counter >> 8) as u8, counter as u8] {
        crc ^= byte;
        for _ in 0..8 {
            crc = if crc & 0x80 != 0 { (crc << 1) ^ 0x07 } else { crc << 1 };
        }
    }
    crc
}

/// The 32-bit word the strip shows for `counter`.
pub fn word(counter: u32) -> u32 {
    assert!(counter <= MAX_COUNTER, "counter exceeds 24 bits");
    ((counter << 8) | crc8(counter) as u32) ^ MASK
}

/// Draws the strip for `counter` into a tightly packed BGRA frame.
pub fn draw_bgra(frame: &mut [u8], width: u32, height: u32, counter: u32) {
    let w = word(counter);
    let (cell, left, top) = layout(width, height);
    // One cell's row of each colour, copied whole: a test source draws this every frame,
    // and pixel by pixel a debug build can't keep a 60 fps source's pace.
    let span = |v: u8| [v, v, v, 255].repeat(cell as usize);
    let (black, white) = (span(0), span(255));
    for bit in 0..ROWS * COLS {
        let on = (w >> (31 - bit)) & 1 == 1;
        let px = if on { &white } else { &black };
        let (r, c) = (bit / COLS, bit % COLS);
        for y in top + r * cell..top + (r + 1) * cell {
            let i = ((y * width + left + c * cell) * 4) as usize;
            frame[i..i + px.len()].copy_from_slice(px);
        }
    }
}

/// Reads the counter from an 8-bit grayscale frame; `None` if the CRC doesn't match.
pub fn read_gray(frame: &[u8], width: u32, height: u32) -> Option<u32> {
    let (cell, left, top) = layout(width, height);
    // Average the middle half of each cell: edges blur under scaling and compression.
    let inset = cell / 4;
    let mut w: u32 = 0;
    for bit in 0..ROWS * COLS {
        let (r, c) = (bit / COLS, bit % COLS);
        let (x0, y0) = (left + c * cell + inset, top + r * cell + inset);
        let (x1, y1) = (left + (c + 1) * cell - inset, top + (r + 1) * cell - inset);
        let mut sum: u64 = 0;
        let mut n: u64 = 0;
        for y in y0..y1 {
            let row = (y * width) as usize;
            for x in x0..x1 {
                sum += frame[row + x as usize] as u64;
                n += 1;
            }
        }
        let on = n > 0 && sum / n >= 128;
        w = (w << 1) | on as u32;
    }
    let w = w ^ MASK;
    let counter = w >> 8;
    (crc8(counter) as u32 == (w & 0xff)).then_some(counter)
}

/// Cells from the first strip's left edge to the second strip's right edge: the counter's
/// eight, a gap of one, and the pattern's own missed-refresh count (`app/src/pattern.ts`).
pub const STRIPS_COLS: u32 = COLS + 1 + COLS;
/// A pixel of the dark field at or above this isn't the pattern's black: a light scrim over
/// black reads about 80, the darkest face a window of shroom's has about 50, and black out
/// of H.264 stays under 10.
pub const LIT: u8 = 24;

/// Whether `(x, y)` is in the on-screen pattern's **dark field**: what the pattern window
/// draws black and nothing else — above the band its bar sweeps (which starts at six tenths
/// of the height), and clear of both strips and of the display's own edges by a quarter of
/// a cell (edges ring under scaling and compression, and a display can draw the window's
/// rim a shade off black — 25 at the very edge on a hosted Mac at 1×).
pub fn in_field(x: u32, y: u32, width: u32, height: u32) -> bool {
    let (cell, left, top) = layout(width, height);
    let m = cell / 4;
    if y >= height * 11 / 20 || y < m || x < m || x + m >= width {
        return false;
    }
    let in_rows = y + m >= top && y < top + ROWS * cell + m;
    let in_cols = x + m >= left && x < left + STRIPS_COLS * cell + m;
    !(in_rows && in_cols)
}

/// How much of the pattern's dark field isn't dark in an 8-bit grayscale frame, from 0 to
/// 1: 0 is the pattern alone; anything drawn over it — a window, a scrim — lights the part
/// of the field it covers. What "nothing but the pattern is in the picture" is read from.
pub fn field_lit(frame: &[u8], width: u32, height: u32) -> f64 {
    let (mut field, mut lit) = (0u64, 0u64);
    for y in 0..height {
        let row = (y * width) as usize;
        for x in 0..width {
            if in_field(x, y, width, height) {
                field += 1;
                lit += (frame[row + x as usize] >= LIT) as u64;
            }
        }
    }
    if field == 0 { 0.0 } else { lit as f64 / field as f64 }
}

/// A synthetic BGRA frame: a slow diagonal gradient (so the encoder has real work)
/// with the strip for `counter` on top.
pub fn synthetic_bgra(width: u32, height: u32, counter: u32) -> Vec<u8> {
    let mut frame = vec![0u8; (width * height * 4) as usize];
    synthetic_bgra_into(&mut frame, width, height, counter);
    frame
}

/// [`synthetic_bgra`] drawn into `frame` (tightly packed BGRA, `width`×`height`) — what a
/// test source offers straight into the recorder's buffer.
///
/// The gradient's pixel at (x, y) is shade `(x + y + 4·counter) mod 256`, so every row is a
/// window onto one strip of shades 0, 1, …, 255, 0, 1, …: each row is a single copy. A test
/// source draws this every frame; pixel by pixel, a debug build on a busy runner drew
/// slower than the 60 fps it promised (CI, 2026-10-14: 17 frames in 1.1 s).
pub fn synthetic_bgra_into(frame: &mut [u8], width: u32, height: u32, counter: u32) {
    let row_bytes = width as usize * 4;
    assert_eq!(frame.len(), row_bytes * height as usize, "a {width}×{height} BGRA frame");
    let strip: Vec<u8> = (0..width as usize + 256).flat_map(|k| {
        let g = (k % 256) as u8;
        [g, g / 2, 255 - g, 255]
    }).collect();
    let shift = (counter % 64) * 4; // 4·counter mod 256
    for (y, row) in frame.chunks_exact_mut(row_bytes).enumerate() {
        let start = (y + shift as usize) % 256 * 4;
        row.copy_from_slice(&strip[start..start + row_bytes]);
    }
    draw_bgra(frame, width, height, counter);
}

/// What a decoded sequence of counters says about the video.
#[derive(Debug, Clone, PartialEq)]
pub struct Summary {
    pub frames: usize,
    pub unreadable: usize,
    /// Frames whose counter repeats the previous one.
    pub repeated: usize,
    /// Counter values skipped between consecutive frames (a jump of 3 skips 2).
    pub skipped: u64,
    /// Times the counter went backwards.
    pub backwards: usize,
    pub first: Option<u32>,
    pub last: Option<u32>,
}

/// Summarizes counters read from consecutive decoded frames.
pub fn summarize(counters: &[Option<u32>]) -> Summary {
    let mut s = Summary { frames: counters.len(), unreadable: 0, repeated: 0, skipped: 0, backwards: 0, first: None, last: None };
    let mut prev: Option<u32> = None;
    for c in counters {
        let Some(c) = *c else {
            s.unreadable += 1;
            continue;
        };
        s.first.get_or_insert(c);
        s.last = Some(c);
        if let Some(p) = prev {
            if c == p {
                s.repeated += 1;
            } else if c < p {
                s.backwards += 1;
            } else {
                s.skipped += (c - p - 1) as u64;
            }
        }
        prev = Some(c);
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn to_gray(bgra: &[u8]) -> Vec<u8> {
        bgra.chunks(4).map(|p| ((p[0] as u32 + p[1] as u32 + p[2] as u32) / 3) as u8).collect()
    }

    /// The frame as first drawn, pixel by pixel: the golden signature (shroom-record's
    /// `signature.rs`) was taken over these bytes, so the row-copying painter must match them.
    fn drawn_pixel_by_pixel(width: u32, height: u32, counter: u32) -> Vec<u8> {
        let mut frame = vec![0u8; (width * height * 4) as usize];
        for y in 0..height {
            for x in 0..width {
                let i = ((y * width + x) * 4) as usize;
                let g = ((x + y + counter * 4) % 256) as u8;
                frame[i..i + 4].copy_from_slice(&[g, g / 2, 255 - g, 255]);
            }
        }
        let (cell, left, top) = layout(width, height);
        let w = word(counter);
        for bit in 0..ROWS * COLS {
            let v = if (w >> (31 - bit)) & 1 == 1 { 255 } else { 0 };
            let (r, c) = (bit / COLS, bit % COLS);
            for y in top + r * cell..top + (r + 1) * cell {
                for x in left + c * cell..left + (c + 1) * cell {
                    let i = ((y * width + x) * 4) as usize;
                    frame[i..i + 4].copy_from_slice(&[v, v, v, 255]);
                }
            }
        }
        frame
    }

    #[test]
    fn the_frame_is_the_same_bytes_as_drawn_pixel_by_pixel() {
        for (w, h) in [(640, 360), (1280, 720), (333, 97)] {
            for counter in [0, 1, 63, 64, 65, 299, 10_000, 65_535, MAX_COUNTER] {
                assert!(synthetic_bgra(w, h, counter) == drawn_pixel_by_pixel(w, h, counter), "{w}×{h}, counter {counter}");
            }
        }
    }

    #[test]
    fn round_trips_through_a_clean_frame() {
        for counter in [0, 1, 299, 65_535, MAX_COUNTER] {
            let f = synthetic_bgra(640, 360, counter);
            assert_eq!(read_gray(&to_gray(&f), 640, 360), Some(counter));
        }
    }

    #[test]
    fn a_damaged_cell_reads_as_unreadable_not_as_another_number() {
        let mut g = to_gray(&synthetic_bgra(640, 360, 1234));
        let (cell, left, top) = layout(640, 360);
        // Invert the last counter bit's cell (row 2, col 7).
        let (r, c) = (23 / COLS, 23 % COLS);
        for y in top + r * cell..top + (r + 1) * cell {
            for x in left + c * cell..left + (c + 1) * cell {
                let i = (y * 640 + x) as usize;
                g[i] = 255 - g[i];
            }
        }
        assert_eq!(read_gray(&g, 640, 360), None);
    }

    #[test]
    fn blank_frames_never_decode() {
        assert_eq!(read_gray(&vec![0u8; 640 * 360], 640, 360), None, "all black");
        assert_eq!(read_gray(&vec![255u8; 640 * 360], 640, 360), None, "all white");
    }

    /// The on-screen pattern as `app/src/pattern.ts` draws it, in gray: black, the bar in
    /// its band, both strips.
    fn on_screen(width: u32, height: u32, counter: u32) -> Vec<u8> {
        let mut bgra = vec![0u8; (width * height * 4) as usize];
        draw_bgra(&mut bgra, width, height, counter);
        let mut g = to_gray(&bgra);
        let (cell, left, top) = layout(width, height);
        // The second strip, nine cells to the right: every cell white is the brightest it gets.
        for y in top..top + ROWS * cell {
            for x in (left + (COLS + 1) * cell).min(width)..(left + STRIPS_COLS * cell).min(width) {
                g[(y * width + x) as usize] = 255;
            }
        }
        // The bar: 200 wide, from six tenths to nine tenths of the height.
        for y in height * 6 / 10..height * 9 / 10 {
            for x in 40..240.min(width) {
                g[(y * width + x) as usize] = 0x5a;
            }
        }
        g
    }

    /// `g` with a `w`×`h` box of `value` drawn at `(x0, y0)`, blended at `alpha`.
    fn over(mut g: Vec<u8>, width: u32, (x0, y0, w, h): (u32, u32, u32, u32), value: u8, alpha: f64) -> Vec<u8> {
        for y in y0..y0 + h {
            for x in x0..x0 + w {
                let i = (y * width + x) as usize;
                g[i] = (g[i] as f64 * (1.0 - alpha) + value as f64 * alpha).round() as u8;
            }
        }
        g
    }

    #[test]
    fn the_pattern_alone_lights_none_of_its_dark_field() {
        // A wide display, and the clean VM's 4:3 one (where the second strip runs off the edge).
        for (w, h) in [(1920, 1080), (1728, 1117), (1024, 768), (480, 360)] {
            assert_eq!(field_lit(&on_screen(w, h, 4242), w, h), 0.0, "{w}×{h}");
            // Black out of an encoder isn't exactly 0.
            let noisy: Vec<u8> = on_screen(w, h, 4242).iter().map(|v| v.saturating_add(9)).collect();
            assert_eq!(field_lit(&noisy, w, h), 0.0, "{w}×{h}, black as an encoder leaves it");
        }
    }

    #[test]
    fn the_displays_own_rim_is_not_the_field() {
        // A hosted Mac's 1024×768 display at 1× draws the pattern window with its outermost
        // ring of pixels at 25 and the next ring in at 5 — the window's edge, there before
        // the page has drawn (issue #4). Nothing shroom shows sits in those two rings alone.
        for (w, h) in [(1920u32, 1080u32), (1024, 768), (480, 360)] {
            let mut g = on_screen(w, h, 4242);
            for y in 0..h {
                for x in 0..w {
                    let ring = x.min(y).min(w - 1 - x).min(h - 1 - y);
                    if ring < 2 {
                        g[(y * w + x) as usize] = if ring == 0 { 25 } else { 5 };
                    }
                }
            }
            assert_eq!(field_lit(&g, w, h), 0.0, "{w}×{h}");
        }
    }

    #[test]
    fn anything_over_the_pattern_lights_the_field_it_covers() {
        for (w, h) in [(1920u32, 1080u32), (1024, 768)] {
            // The count's scrim: the whole display, light, a third opaque.
            let scrim = over(on_screen(w, h, 7), w, (0, 0, w, h), 236, 0.35);
            assert!(field_lit(&scrim, w, h) > 0.99, "{w}×{h}: the scrim lights all of it: {}", field_lit(&scrim, w, h));
            // The popover: 320×360 points hanging from the menu bar at the right — in its
            // darkest face, on a display whose points are its pixels.
            let popover = over(on_screen(w, h, 7), w, (w - 360, 24, 320, 360), 0x33, 1.0);
            assert!(field_lit(&popover, w, h) > 0.05, "{w}×{h}: the popover lights its part: {}", field_lit(&popover, w, h));
        }
        // And the strip still reads under the scrim: a readable counter alone never meant a clean picture.
        let scrim = over(on_screen(1024, 768, 7), 1024, (0, 0, 1024, 768), 236, 0.35);
        assert_eq!(read_gray(&scrim, 1024, 768), Some(7));
    }

    #[test]
    fn summary_counts_repeats_skips_and_unreadable_frames() {
        let s = summarize(&[Some(0), Some(1), Some(1), None, Some(4), Some(3)]);
        assert_eq!((s.frames, s.unreadable, s.repeated, s.skipped, s.backwards), (6, 1, 1, 2, 1));
        assert_eq!((s.first, s.last), (Some(0), Some(3)));
    }
}
