// The marker toast (N78): ⌥⌘M's answer, where the agent label sits. Its words come from the
// Scene; it takes no clicks (its panel passes them through) and has no keys. Each press —
// a new `pressed` — starts its fade again.
import { onScene, sceneGet } from "./lib/ipc";

type Toast = { display: number; line: string; hint: string; under_label: boolean; pressed: number };
const pill = document.getElementById("pill")!;
const line = document.getElementById("line")!;
const hint = document.getElementById("hint")!;
let pressed: number | null = null;
const show = (t: Toast | null) => {
  if (!t) return;
  line.textContent = t.line;
  hint.textContent = t.hint;
  pill.hidden = false;
  if (t.pressed !== pressed) {
    pressed = t.pressed;
    pill.classList.remove("on");
    void pill.offsetWidth; // the animation starts again
    pill.classList.add("on");
  }
};
sceneGet<Toast>("toast").then(show);
onScene<Toast>(show, "toast"); // its own slice only
