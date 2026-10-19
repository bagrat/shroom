// The agent label (N62, B216): top right of the display a take the shroom command started
// records, while it records. Its line comes from the Scene; it takes no clicks (its panel
// passes them through) and has no keys.
import { onScene, sceneGet } from "./lib/ipc";

type Label = { display: number; line: string };
const pill = document.getElementById("pill")!;
const line = document.getElementById("line")!;
const show = (l: Label | null) => {
  if (!l) return;
  line.textContent = l.line;
  pill.hidden = false;
};
sceneGet<Label>("label").then(show);
onScene<Label>(show, "label"); // its own slice only
