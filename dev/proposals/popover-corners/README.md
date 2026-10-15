# The popover's square backing (B169)

Taken on the hosted Mac (macOS 26, a silver desktop so the shadow shows), the dev build
before (left, 0.1.15's code) and after (right) the fix:

- `corner-before-after.png` — the popover's bottom-left corner, 12× (40×30 px of screen).
  Before: the page's CSS shadow cut off at the panel's square edge, a grey square past
  the rounded card. After: macOS's shadow, following the curve.
- `popover-before-after.png` — the whole popover, actual size.

Library and Settings (and the setup window, the same kind of window) are macOS's own
titled windows: their corners and shadows were already right on the same Mac.
