// The page renderer (SPEC §6): a PURE function from one template + per-video
// metadata + URLs → a static HTML string. Deterministic and side-effect-free, so
// every page is re-derivable from the metadata record. All interpolated text is
// escaped — titles/tldr/chapter labels are agent- or user-authored, so they must
// never be able to break out of an attribute, the markup, or the JSON island.

const ESC = { '&': '&amp;', '<': '&lt;', '>': '&gt;', '"': '&quot;', "'": '&#39;' };
const htmlEscape = (s) => String(s ?? '').replace(/[&<>"']/g, (c) => ESC[c]);

// JSON destined for an inline <script> block: neutralize `<` / `>` / `&` and the
// U+2028/U+2029 line separators so the string can't terminate the script element
// or trip a parser. (Escapes written as \uXXXX patterns to keep this source ASCII.)
function jsonForScript(obj) {
  return JSON.stringify(obj)
    .replace(/</g, '\\u003c')
    .replace(/>/g, '\\u003e')
    .replace(/&/g, '\\u0026')
    .replace(/\u2028/g, '\\u2028')
    .replace(/\u2029/g, '\\u2029');
}

// Seconds → "m:ss" or "h:mm:ss" (display + the duration chip).
export function formatDuration(totalSec) {
  const s = Math.max(0, Math.round(Number(totalSec) || 0));
  const h = Math.floor(s / 3600);
  const m = Math.floor((s % 3600) / 60);
  const sec = s % 60;
  const pad = (n) => String(n).padStart(2, '0');
  return h > 0 ? `${h}:${pad(m)}:${pad(sec)}` : `${m}:${pad(sec)}`;
}

function chaptersHtml(chapters) {
  if (!Array.isArray(chapters) || chapters.length === 0) return '';
  const items = chapters
    .filter((c) => c && c.label != null)
    .map((c) => {
      const t = Math.max(0, Number(c.t) || 0);
      return (
        `    <li><button type="button" data-seek="${htmlEscape(t)}">` +
        `<span class="t">${htmlEscape(formatDuration(t))}</span>` +
        `<span class="label">${htmlEscape(c.label)}</span></button></li>`
      );
    })
    .join('\n');
  return `<ul class="chapters">\n${items}\n  </ul>`;
}

function tldrHtml(tldr) {
  const t = String(tldr ?? '').trim();
  return t ? `<p class="tldr">${htmlEscape(t)}</p>` : '';
}

// The Download button — only when the record opted in (`mp4: true`) AND a public
// download URL resolved. The `download` attr asks the browser to save rather than
// navigate. Empty string otherwise, so the template token always substitutes.
function downloadHtml(mp4, downloadUrl) {
  if (!mp4 || !downloadUrl) return '';
  return `<a class="download" href="${htmlEscape(downloadUrl)}" download>Download MP4</a>`;
}

// Render one page. `meta` = the metadata record's frontmatter; `urls` = the
// resolved public links. Returns the final HTML string.
export function renderPage({ template, meta = {}, urls = {} }) {
  const title = String(meta.title ?? '').trim() || 'Untitled recording';
  const tldr = meta.tldr ?? '';
  const durationSec = Math.max(0, Math.round(Number(meta.durationSec) || 0));
  const chapters = Array.isArray(meta.chapters) ? meta.chapters : [];

  const data = {
    hlsUrl: urls.hlsUrl ?? '',
    hlsJsUrl: urls.hlsJsUrl ?? '/hls.min.js',
    durationSec,
    // Public page URL (for Copy link / Copy embed) and the chapter list, so the
    // client can build the seekable timeline + share snippets. Both are escaped
    // into the JSON island by jsonForScript.
    pageUrl: urls.pageUrl ?? '',
    chapters: chapters
      .filter((c) => c && c.label != null)
      .map((c) => ({ t: Math.max(0, Number(c.t) || 0), label: String(c.label) })),
  };

  const tokens = {
    TITLE: htmlEscape(title),
    DESCRIPTION: htmlEscape(String(tldr).trim() || title),
    PAGE_URL: htmlEscape(urls.pageUrl ?? ''),
    POSTER_URL: htmlEscape(urls.posterUrl ?? ''),
    DURATION_SEC: String(durationSec),
    DURATION_LABEL: htmlEscape(formatDuration(durationSec)),
    TLDR_HTML: tldrHtml(tldr),
    CHAPTERS_HTML: chaptersHtml(chapters),
    DOWNLOAD_HTML: downloadHtml(meta.mp4, urls.downloadUrl),
    DATA_JSON: jsonForScript(data),
  };

  return template.replace(/\{\{([A-Z_]+)\}\}/g, (whole, key) =>
    key in tokens ? tokens[key] : whole,
  );
}
