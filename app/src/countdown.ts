// The countdown scrim (N22): a light scrim on the chosen display, 3 · 2 · 1, one quiet
// line. Any click, or Esc, cancels. The digit and line come from the Scene.
import { act, key, onScene, sceneGet } from "./lib/ipc";

type Countdown = { display: number; digit: number; line: string };
const digit = document.getElementById("digit")!;
const line = document.getElementById("line")!;
const show = (c: Countdown | null) => {
  if (!c) return;
  digit.textContent = String(c.digit);
  line.textContent = c.line;
};
sceneGet<Countdown>("countdown").then(show);
onScene<Countdown>(show, "countdown"); // its own slice only
window.addEventListener("mousedown", () => act("countdown", { do: "cancel-countdown" }));
window.addEventListener("keydown", (e) => { if (e.key === "Escape") key("countdown", "esc"); });
