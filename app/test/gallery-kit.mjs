// What the galleries share: app/dist served on loopback, the stubbed Tauri bridge (scene_get
// answers the entry's slice; every other call is recorded; __emit pushes a new slice as the
// runner's "scene" event would), the core's stand-in for typing (`echo`: each field() call
// answered as the core answers it, a little later), and the readers that pull said-lines
// and buttons out of a Scene slice.

import { execFileSync } from "node:child_process";
import { createServer } from "node:http";
import { existsSync, readFileSync } from "node:fs";
import { dirname, extname, join, normalize, resolve } from "node:path";
import { fileURLToPath } from "node:url";

const here = dirname(fileURLToPath(import.meta.url));
export const repo = resolve(here, "../..");
export const dist = resolve(here, "../dist");
export const out = join(here, "gallery-out");

/** `cargo run -p shroom-core --example gallery [-- which]`, parsed. */
export const gallery = (which) =>
  JSON.parse(
    execFileSync("cargo", ["run", "-q", "-p", "shroom-core", "--example", "gallery", ...(which ? ["--", which] : [])], {
      cwd: repo,
      encoding: "utf8",
      maxBuffer: 1 << 26,
    }),
  );

/** The page a window's webview gets: its WindowSpec in runtime/presenter.rs (read from there, so
 *  the galleries and the runner stay in step), less macOS's title bar. The spec's height is
 *  the window's frame, title bar included: on the hosted Mac a 520×600 setup window measured
 *  a 520×568 web area (dev/mac/scenarios/window-size.sh, 2026-10-17). */
export const TITLE_BAR = 32;
export function windowPage(label, fallback) {
  let size = fallback;
  try {
    const src = readFileSync(join(repo, "app/src-tauri/src/runtime/presenter.rs"), "utf8");
    const m = src.match(new RegExp(`label: ${label},[^}]*?width: ([\\d.]+), height: ([\\d.]+)`));
    if (m) size = { width: Number(m[1]), height: Number(m[2]) };
  } catch {}
  return { width: size.width, height: size.height - TITLE_BAR };
}

const MIME = { ".html": "text/html", ".js": "text/javascript", ".css": "text/css", ".svg": "image/svg+xml", ".png": "image/png" };

/** Serves app/dist on 127.0.0.1; resolves to { origin, close }. */
export async function serve() {
  const server = createServer((req, res) => {
    const path = normalize(decodeURIComponent(new URL(req.url, "http://x").pathname)).replace(/^\/+/, "");
    const file = join(dist, path || "index.html");
    if (!file.startsWith(dist) || !existsSync(file)) {
      res.writeHead(404).end();
      return;
    }
    res.writeHead(200, { "content-type": MIME[extname(file)] ?? "application/octet-stream" });
    res.end(readFileSync(file));
  });
  await new Promise((r) => server.listen(0, "127.0.0.1", r));
  return { origin: `http://127.0.0.1:${server.address().port}`, close: () => server.close() };
}

/** Runs in the page (addInitScript): the bridge stand-in. */
export const stub = (scene) => {
  window.__calls = [];
  const callbacks = {};
  let next = 0;
  let sceneHandler = null;
  window.__TAURI_INTERNALS__ = {
    transformCallback(f) {
      const id = ++next;
      callbacks[id] = f;
      return id;
    },
    unregisterCallback(id) {
      delete callbacks[id];
    },
    convertFileSrc: (p) => p,
    invoke(cmd, args) {
      if (cmd === "scene_get") return Promise.resolve(scene);
      if (cmd === "plugin:event|listen") {
        if (args.event === "scene") sceneHandler = { id: args.handler, target: args.target };
        return Promise.resolve(next);
      }
      if (cmd === "plugin:event|unlisten") return Promise.resolve();
      window.__calls.push({ cmd, args: JSON.parse(JSON.stringify(args ?? null)) });
      return Promise.resolve(null);
    },
  };
  window.__TAURI_EVENT_PLUGIN_INTERNALS__ = { unregisterListener() {} };
  window.__emit = (payload) => callbacks[sceneHandler.id]({ event: "scene", id: 1, payload });
  window.__listenTarget = () => sceneHandler?.target ?? null;
};

/**
 * Runs in the page after `stub` (a second addInitScript): the core's answer to typing.
 * Each field() call is an edit the stand-in takes in the order it was sent, the way the
 * core takes it — the link name lowercased and held to its characters and its 63, the
 * composer's name to its 80, anything to the runner's 256; an edit of a field that isn't
 * showing is dropped — counts it (`edits`), and answers with the Scene as it then stands,
 * `delay` ms after the call. `delay: null` holds the answers until `__deliver(n)` hands
 * over the next n (all of them by default), so a late answer is a fact, not a race.
 *
 * `__tick()` is a Scene nobody typed for (the core's loop turned for something else): the
 * Scene as the core holds it right now. `__say(field, value)` is the core changing a field
 * by itself (the composer emptied once its invite is made); `__forget(field)` is its
 * window's fields cleared (empty, counting from nothing again); `__lose()` drops the held
 * edits — the core never took them.
 */
export const echo = ({ scene, delay }) => {
  const core = JSON.parse(JSON.stringify(scene));
  const slot = (field) => {
    if (field === "library-search") return core.body.search ?? null;
    if (field === "title") return (core.body.rows ?? []).map((r) => r.rename).find(Boolean) ?? null;
    if (field === "invite-name") return core.body.pane?.name ?? null;
    return core.body.nest?.kind === "name" ? core.body.nest : null;
  };
  const cut = (v, n) => [...v].slice(0, n).join("");
  const rule = {
    name: (v) => cut([...v.toLowerCase()].filter((c) => /^[a-z0-9-]$/.test(c)).join(""), 63),
    "invite-name": (v) => cut(v, 80),
  };
  const answer = () => window.__emit(JSON.parse(JSON.stringify(core)));
  const take = (field, value) => () => {
    const s = slot(field);
    if (s) {
      s.value = (rule[field] ?? ((v) => v))(cut(value, 256));
      s.edits = (s.edits ?? 0) + 1;
    }
    answer();
  };
  const held = [];
  const invoke = window.__TAURI_INTERNALS__.invoke;
  window.__TAURI_INTERNALS__.invoke = (cmd, args) => {
    if (cmd === "field") {
      const t = take(args.field, args.value);
      if (delay === null) held.push(t);
      else setTimeout(t, delay);
    }
    return invoke(cmd, args);
  };
  window.__deliver = (n) => held.splice(0, n ?? held.length).forEach((t) => t());
  window.__lose = () => held.splice(0, held.length).length;
  window.__tick = answer;
  window.__say = (field, value) => {
    slot(field).value = value;
    answer();
  };
  window.__forget = (field) => {
    Object.assign(slot(field), { value: "", edits: 0 });
    answer();
  };
};

/** Every said-line under `node` whose key is in `keys` (actions skipped). */
export function texts(node, keys, acc = []) {
  if (Array.isArray(node)) node.forEach((n) => texts(n, keys, acc));
  else if (node && typeof node === "object") {
    for (const [k, v] of Object.entries(node)) {
      if (k === "action" || k === "gestures") continue;
      if (keys.has(k) && typeof v === "string") acc.push(v);
      else if (keys.has(k) && Array.isArray(v) && v.every((x) => typeof x === "string")) acc.push(...v);
      else texts(v, keys, acc);
    }
  }
  return acc;
}

export const isButton = (o) => o && typeof o === "object" && "action" in o && "style" in o && "label" in o;
export const isChoice = (o) => o && typeof o === "object" && "action" in o && "selected" in o;

export function collect(node, pred, acc = []) {
  if (Array.isArray(node)) node.forEach((n) => collect(n, pred, acc));
  else if (node && typeof node === "object") {
    if (pred(node)) acc.push(node);
    for (const [k, v] of Object.entries(node)) if (k !== "action" && k !== "gestures") collect(v, pred, acc);
  }
  return acc;
}

export const norm = (s) => s.replace(/\s+/g, " ").trim();
export const hex = (h) => {
  const n = parseInt(h.trim().slice(1), 16);
  return `rgb(${(n >> 16) & 255}, ${(n >> 8) & 255}, ${n & 255})`;
};
