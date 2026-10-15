// The pattern's dark field (crates/shroom-testpattern `in_field` and `field_lit`, ported):
// what the on-screen pattern draws black and nothing else — above the band its bar sweeps
// (which starts at six tenths of the height), and clear of both strips and of the display's
// own edges by a quarter of a cell. `fieldLit` is how much of it isn't dark in a one-channel
// (gray) frame, from 0 to 1: 0 is the pattern alone; anything drawn over it — a window, a
// scrim — lights the part of the field it covers. A readable counter alone never meant a
// clean picture: the strip still reads under the count's scrim.

// A pixel at or above this isn't the pattern's black: a light scrim over black reads about
// 80, the darkest face a window of shroom's has about 50, black out of H.264 under 10.
export const LIT = 24;
// The counter's eight cells, a gap of one, and the pattern's own missed-refresh count.
const STRIPS_COLS = 17;
const ROWS = 4;

export function fieldLit(gray, width, height) {
  const cell = Math.max(4, Math.floor(height / 12));
  const m = Math.floor(cell / 4);
  const below = Math.floor((height * 11) / 20);
  let field = 0, lit = 0;
  for (let y = m; y < below; y++) {
    const inRows = y + m >= cell && y < cell + ROWS * cell + m;
    for (let x = m; x + m < width; x++) {
      if (inRows && x + m >= cell && x < cell + STRIPS_COLS * cell + m) continue;
      field++;
      if (gray[y * width + x] >= LIT) lit++;
    }
  }
  return field ? lit / field : 0;
}
