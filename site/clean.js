// What a report may carry from shroom's log, cleaned (dev/proposals/report-a-problem.md,
// DECISIONS B180). The log never holds a secret (law 13), but it does hold values: a
// space's host, paths in the home folder, and error text from macOS and the network. Every
// line passes `cleanLine` before anyone sees it, the same rules wherever a report is made:
// the feedback page (this file, served as /clean.js), the Worker (once more, in case a
// sender skipped them) and the app (crates/shroom-core, the same cases:
// site/test/clean-cases.json).

/** The lines a report keeps: the last ones, the newest last. */
export const LOG_LINES = 200;
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
// A key-shaped run: 24 or more of base64's characters, holding a letter and a digit.
const RUN_RE = /[A-Za-z0-9+/_-]{24,}={0,2}/g;

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
  const webview = s.match(/^(\s*\d*\s*webview [a-z-]+: )(.*)$/i);
  if (webview && webview[2].length > WEBVIEW_CHARS) s = webview[1] + webview[2].slice(0, WEBVIEW_CHARS) + "…";
  s = s.replace(URL_RE, (u) => {
    const host = hostOf(u);
    if (/\.workers\.dev$/i.test(host)) return "<your space>";
    return PUBLIC_HOSTS.test(host) ? `https://${host}/…` : "<a web address>";
  });
  s = s.replace(EMAIL_RE, "<an email>");
  s = s.replace(HOME_RE, "~");
  s = s.replace(HOST_RE, (h) => {
    if (PUBLIC_HOSTS.test(h)) return h;
    return /\.workers\.dev$/i.test(h) ? "<your space>" : "<a host>";
  });
  s = s.replace(HEX_RE, "<an id>");
  s = s.replace(RUN_RE, (r) => (/[A-Za-z]/.test(r) && /\d/.test(r) ? "<an id>" : r));
  return s.length > LINE_CHARS ? s.slice(0, LINE_CHARS) + "…" : s;
}

/** A log, cleaned: its last `LOG_LINES` lines that hold anything, each cleaned.
 * @param {string} text */
export function cleanLog(text) {
  const lines = String(text).split(/\r?\n/).filter((l) => l.trim() !== "");
  return lines.slice(-LOG_LINES).map(cleanLine).join("\n");
}
