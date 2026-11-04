// What a report may carry from shroom's log, cleaned (dev/proposals/report-a-problem.md,
// DECISIONS B180). The log never holds a secret (law 13), but it does hold values: a
// space's host, paths in the home folder, and error text from macOS and the network. Every
// line passes `cleanLine` before anyone sees it, the same rules wherever a report is made:
// the feedback page (this file, served as /clean.js), the Worker (once more, in case a
// sender skipped them) and the app (crates/shroom-core, the same cases:
// site/test/clean-cases.json).

/** The lines a report keeps: the last ones, the newest last. */
export const LOG_LINES = 1000;
/** One line, at most, once cleaned. */
export const LINE_CHARS = 400;
/** A window's own error (`webview <surface>: <message>`): its message is cut to this. */
export const WEBVIEW_CHARS = 160;

// Hosts that name no one: shroom's own and the services every Mac running it talks to.
const PUBLIC_HOSTS = /^(?:shroom\.am|github\.com|api\.github\.com|objects\.githubusercontent\.com|dash\.cloudflare\.com|api\.cloudflare\.com|www\.cloudflare\.com|cloudflare\.com)$/i;
// The endings a bare host in the log can have; a file name (`run.log`, `worker.js`) has none
// of them.
const TLDS = "com|net|org|dev|app|io|am|co|me|ai|sh|xyz|site|cloud|tech|page|link|eu|uk|de|fr|nl|us|ca|au";
const URL_RE = /\b[a-z][a-z0-9+.-]*:\/\/[^\s"'<>)\]]+/gi;
const EMAIL_RE = /[a-z0-9._%+-]+@[a-z0-9.-]+\.[a-z]{2,}/gi;
const HOME_RE = /\/(?:Users|home)\/[^/\s"']+/g;
const HOST_RE = new RegExp(`(?<![\\w./~-])(?:[a-z0-9](?:[a-z0-9-]*[a-z0-9])?\\.)+(?:${TLDS})\\b`, "gi");
const HEX_RE = /\b[0-9a-f]{16,}\b/gi;
// What follows a link's code (`/join/`, `?k=`, `&k=`) or a credential's scheme, up to a space.
const AFTER_RE = /(\/join\/|[?&]k=|Bearer |ShroomParent )\S+/g;
// A run of base64's characters long enough to be a key (`idLike` says which).
const RUN_RE = /[A-Za-z0-9+/_-]{16,}={0,2}/g;

/** A part of a run between `+ / _ -` that names a thing, not a key: short, a number, a
 * word with a number after it (`sha256`), or a number with a letter or two (`26b`, `18T17`).
 * @param {string} p */
const shaped = (p) => p.length <= 5 || /^[0-9]+$/.test(p) || /^[A-Za-z]+[0-9]*$/.test(p) || /^[0-9]+[A-Za-z]?[0-9]*[A-Za-z]?$/.test(p);

/** A run's letters read as words: one case, or words of three letters or more (one shorter
 * allowed), at least three letters a word (`ScreenCaptureKit`, `NSURLErrorDomain`).
 * @param {string} r */
function words(r) {
  if (!/[a-z]/.test(r) || !/[A-Z]/.test(r)) return true;
  const pieces = r.match(/[A-Z]+(?![a-z])|[A-Z]?[a-z]+/g) ?? [];
  const letters = pieces.join("").length;
  const short = pieces.filter((p) => /[a-z]/.test(p) && p.length < 3).length;
  return short <= 1 && pieces.length * 3 <= letters;
}

/** Whether a run (its `=` apart) is key-shaped: 24 or more holding a letter and a digit; 32
 * or more holding both cases; 16 or more holding a digit or both cases, unless it reads as
 * names (each part `shaped`, its letters `words`).
 * @param {string} r */
function idLike(r) {
  const digit = /\d/.test(r);
  const mixed = /[a-z]/.test(r) && /[A-Z]/.test(r);
  if (r.length >= 24 && digit && /[A-Za-z]/.test(r)) return true;
  if (r.length >= 32 && mixed) return true;
  if (r.length < 16 || (!digit && !mixed)) return false;
  return !(r.split(/[+/_-]+/).filter(Boolean).every(shaped) && words(r));
}

/** The host of a URL, or "" when it has none. @param {string} url */
function hostOf(url) {
  try {
    return new URL(url).hostname;
  } catch {
    return "";
  }
}

/** One log line, cleaned. @param {string} line */
export function cleanLine(line) {
  let s = String(line).replace(/[\u0000-\u0008\u000b-\u001f\u007f]/g, "");
  // A window's error is whatever its script said: cut it short first.
  const webview = s.match(/^((?:\d{4}-\d\d-\d\dT\d\d:\d\d:\d\dZ )?\s*\d*\s*webview [a-z-]+: )(.*)$/i);
  if (webview && webview[2].length > WEBVIEW_CHARS) s = webview[1] + webview[2].slice(0, WEBVIEW_CHARS) + "…";
  s = s.replace(URL_RE, (u) => {
    const host = hostOf(u);
    if (/\.workers\.dev$/i.test(host)) return "<your space>";
    return PUBLIC_HOSTS.test(host) ? `https://${host}/…` : "<a web address>";
  });
  s = s.replace(AFTER_RE, "$1<an id>");
  s = s.replace(EMAIL_RE, "<an email>");
  s = s.replace(HOME_RE, "~");
  s = s.replace(HOST_RE, (h) => {
    if (PUBLIC_HOSTS.test(h)) return h;
    return /\.workers\.dev$/i.test(h) ? "<your space>" : "<a host>";
  });
  s = s.replace(HEX_RE, "<an id>");
  s = s.replace(RUN_RE, (r) => (idLike(r.replace(/=+$/, "")) ? "<an id>" : r));
  return s.length > LINE_CHARS ? s.slice(0, LINE_CHARS) + "…" : s;
}

/** A log, cleaned: its last `LOG_LINES` lines that hold anything, each cleaned.
 * @param {string} text */
export function cleanLog(text) {
  const lines = String(text).split(/\r?\n/).filter((l) => l.trim() !== "");
  return lines.slice(-LOG_LINES).map(cleanLine).join("\n");
}
