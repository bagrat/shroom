// An in-memory stand-in for the R2 binding, shaped as R2 answers (DECISIONS B35's probes):
//   get (with Range) · head · put · delete (one key or many) · list.
//   put's onlyIf takes both forms R2 takes — an R2Conditional ({ etagMatches,
//   etagDoesNotMatch }) or a Headers ("if-match", "if-none-match") — and a failed
//   precondition answers null, as the binding does (P1a, P2). `etagDoesNotMatch: "*"` and
//   `if-none-match: *` are create-only. customMetadata is kept and returned by get/head,
//   and by list only when asked (`include: ["customMetadata"]`, P3).
//
// What tests inject (WORKER-INTERFACE §7.1, "the fake bucket first gains"):
//   opts.pageSize         a short list page: at most this many objects, `truncated: true`
//                         while more remain — so no code can lean on the count (P3)
//   opts.preconditionThrows  a failed precondition throws instead of answering null
//   refuse(key, n)        the next n writes to key throw, as R2's per-key rate limit
//                         would (C5)
//   hooks.afterGet(key)   runs after a get/head has read its object (racing a writer in)
//   hooks.beforePut(key, value, options)  runs before a put checks its precondition
//   ops                   every call, in order: { op, key, options }
export function fakeBucket(objects = {}) {
  const enc = (v) => (typeof v === "string" ? new TextEncoder().encode(v) : v);
  const store = new Map(Object.entries(objects).map(([k, v]) => [k, enc(v)]));
  const etags = new Map([...store.keys()].map((k) => [k, "e0"]));
  const custom = new Map();
  const refusals = new Map();
  const opts = { pageSize: Infinity, preconditionThrows: false };
  const hooks = { afterGet: null, beforePut: null };
  const ops = [];
  let writes = 0;
  const meta = (key, bytes, range) => ({
    key, size: bytes.length, etag: etags.get(key), httpEtag: `"${etags.get(key)}"`, uploaded: new Date(0),
    customMetadata: { ...(custom.get(key) ?? {}) }, range,
  });

  /** @returns {boolean} whether the precondition holds */
  function holds(key, onlyIf) {
    if (!onlyIf) return true;
    const exists = store.has(key);
    const tag = etags.get(key);
    let match, noneMatch;
    if (typeof onlyIf.get === "function") {
      match = onlyIf.get("if-match") ?? undefined;
      noneMatch = onlyIf.get("if-none-match") ?? undefined;
      if (match) match = match.replace(/^"|"$/g, "");
      if (noneMatch && noneMatch !== "*") noneMatch = noneMatch.replace(/^"|"$/g, "");
    } else {
      match = onlyIf.etagMatches;
      noneMatch = onlyIf.etagDoesNotMatch;
    }
    if (match !== undefined && (!exists || (match !== "*" && tag !== match))) return false;
    if (noneMatch !== undefined && exists && (noneMatch === "*" || tag === noneMatch)) return false;
    return true;
  }

  const bucket = {
    store,
    opts,
    hooks,
    ops,
    /** Rewrites a key as another writer would (a new etag), between a read and a write. */
    touch(key, value, customMetadata) {
      store.set(key, enc(value));
      etags.set(key, `e${++writes}`);
      if (customMetadata) custom.set(key, { ...customMetadata });
    },
    /** The next `n` writes to `key` throw, as R2 refusing more than one write a second. */
    refuse(key, n = 1) { refusals.set(key, n); },
    /** The stored JSON at `key`, parsed (null when absent). */
    json(key) { const b = store.get(key); return b ? JSON.parse(new TextDecoder().decode(b)) : null; },
    text(key) { const b = store.get(key); return b ? new TextDecoder().decode(b) : null; },
    customMetadata(key) { return custom.has(key) ? { ...custom.get(key) } : null; },
    /** Keys written (put or delete), in order. */
    written() { return ops.filter((o) => o.op === "put" || o.op === "delete").map((o) => o.key); },
    /** Keys read (get or head), in order. */
    read() { return ops.filter((o) => o.op === "get" || o.op === "head").map((o) => o.key); },
    async put(key, value, options) {
      ops.push({ op: "put", key, options });
      if (hooks.beforePut) await hooks.beforePut(key, value, options);
      const left = refusals.get(key) ?? 0;
      if (left > 0) {
        refusals.set(key, left - 1);
        throw new Error("put: We encountered an internal error. Please try again. (10058) — too many writes to this key");
      }
      if (!holds(key, options?.onlyIf)) {
        if (opts.preconditionThrows) throw new Error("put: precondition failed (10031)");
        return null;
      }
      const bytes = typeof value === "string" ? enc(value) : value instanceof Uint8Array ? value : enc(String(value));
      store.set(key, bytes);
      etags.set(key, `e${++writes}`);
      if (options?.customMetadata) custom.set(key, { ...options.customMetadata });
      else custom.delete(key);
      return meta(key, bytes);
    },
    async delete(keys) {
      for (const key of Array.isArray(keys) ? keys : [keys]) {
        ops.push({ op: "delete", key });
        store.delete(key); etags.delete(key); custom.delete(key);
      }
    },
    async head(key) {
      ops.push({ op: "head", key });
      const b = store.get(key);
      const out = b ? meta(key, b) : null;
      if (hooks.afterGet) await hooks.afterGet(key);
      return out;
    },
    async get(key, options) {
      ops.push({ op: "get", key, options });
      const b = store.get(key);
      if (!b) { if (hooks.afterGet) await hooks.afterGet(key); return null; }
      let slice = b, range;
      const header = options?.range?.get?.("range");
      if (header) {
        const m = header.match(/bytes=(\d*)-(\d*)/);
        if (m[1] === "") { const suffix = Number(m[2]); slice = b.slice(b.length - suffix); range = { suffix }; }
        else { const offset = Number(m[1]); const end = m[2] === "" ? b.length - 1 : Number(m[2]); slice = b.slice(offset, end + 1); range = { offset, length: slice.length }; }
      }
      const out = { ...meta(key, b, range), body: new Blob([slice]).stream(), text: async () => new TextDecoder().decode(slice), json: async () => JSON.parse(new TextDecoder().decode(slice)) };
      if (hooks.afterGet) await hooks.afterGet(key);
      return out;
    },
    async list(options = {}) {
      const { prefix = "", startAfter, cursor, limit = 1000, include = [] } = options;
      ops.push({ op: "list", key: prefix, options });
      let keys = [...store.keys()].filter((k) => k.startsWith(prefix)).sort();
      if (startAfter !== undefined) keys = keys.filter((k) => k > startAfter);
      if (cursor) {
        let after;
        try { after = atob(cursor); } catch { throw new Error("list: invalid cursor"); }
        keys = keys.filter((k) => k > after);
      }
      const page = keys.slice(0, Math.min(limit, 1000, opts.pageSize));
      const truncated = keys.length > page.length;
      const objects = page.map((k) => {
        const o = { key: k, size: store.get(k).length, etag: etags.get(k), httpEtag: `"${etags.get(k)}"`, uploaded: new Date(0) };
        if (include.includes("customMetadata")) o.customMetadata = { ...(custom.get(k) ?? {}) };
        return o;
      });
      return truncated
        ? { objects, truncated: true, cursor: btoa(page[page.length - 1] ?? startAfter ?? ""), delimitedPrefixes: [] }
        : { objects, truncated: false, delimitedPrefixes: [] };
    },
  };
  return bucket;
}
