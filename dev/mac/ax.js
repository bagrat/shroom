// What an app shows, read the way VoiceOver reads it (System Events' accessibility tree), so a
// scenario can drive a build with no harness — a signed release — by the words on its
// buttons, as a person does. One line per element that says something, tab-separated:
//
//   <window index> <role> <label> <centre x,y> <frame x,y,w,h> <value> <enabled>
//
// and one per window: `W <index> <title> <frame x,y,w,h>`.
//
//   osascript -l JavaScript dev/mac/ax.js <process name>
//
// The webviews' text comes through as AXStaticText; buttons, switches and fields by their
// accessible names. A label is cut to 160 characters and its tabs and newlines flattened.
function run(argv) {
  const se = Application("System Events");
  const proc = se.processes.byName(argv[0] || "shroom");
  const out = [];
  const flat = (s) => String(s).replace(/[\t\n\r]+/g, " ").slice(0, 160);
  const get = (el, prop) => { try { return el[prop](); } catch (e) { return null; } };
  function walk(el, w, depth) {
    if (depth > 30) return;
    let kids = [];
    try { kids = el.uiElements(); } catch (e) { return; }
    for (const k of kids) {
      const role = get(k, "role") || "";
      const name = get(k, "name") || "";
      const desc = get(k, "description") || "";
      let value = get(k, "value");
      const pos = get(k, "position");
      const size = get(k, "size");
      const enabled = get(k, "enabled");
      const label = name || desc || (typeof value === "string" ? value : "");
      if (typeof value !== "string" && typeof value !== "number" && typeof value !== "boolean") value = "";
      if (label && pos && size) {
        out.push([w, role, flat(label), `${pos[0] + Math.round(size[0] / 2)},${pos[1] + Math.round(size[1] / 2)}`, `${pos[0]},${pos[1]},${size[0]},${size[1]}`, flat(value), enabled === false ? "off" : "on"].join("\t"));
      }
      walk(k, w, depth + 1);
    }
  }
  let windows = [];
  try { windows = proc.windows(); } catch (e) { return ""; }
  windows.forEach((win, i) => {
    const pos = get(win, "position") || [0, 0];
    const size = get(win, "size") || [0, 0];
    out.push(["W", i, flat(get(win, "name") || ""), `${pos[0]},${pos[1]},${size[0]},${size[1]}`].join("\t"));
    walk(win, i, 0);
  });
  return out.join("\n");
}
