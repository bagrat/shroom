# Copy as a thumbnail: making a shroom link look embedded where it can't embed

*Proposal and probe, 2026-10-15. No code. The words on the boards are placeholders; the copy is
Bagrat's.*

> **Decided (Bagrat, 2026-10-15; DECISIONS B164, spec N46).**
> - **Where:** "A both": the owner's page, as a second button beside Copy link, and the
>   Library row's menu, under Copy link. In the build, only the owner's page has Copy link,
>   so a visitor's page has neither.
> - **The label:** "b copy thumbnail" → **“Copy thumbnail”**. Asked whether it still acts as a
>   link: yes. The picture is wrapped in the link, and the plain link goes on the clipboard
>   as text too.
> - **The picture** (B preferred, if it could be kept current, then the loop added too):
>   **B + loop, kept current**.
>   - The title band and the length sit over a 3 s silent loop from the start of the video.
>   - A new picture is made for each title, by whoever sets the title. It's made in the
>     background after the title saves, and stored beside the video under a name taken from
>     its contents.
>   - B's still stands in until the teaser exists. An untitled video gets the loop and the
>     length, with no band.
> - **The viewer's page:** the buttons go on the viewer's page as well, with Download kept
>   there. Copy link and Copy thumbnail sit beside Download.
>   A viewer stores nothing, so their copy wraps the newest picture, or the first frame when
>   none exists yet.
> - **Old videos:** "Your suggestion is good". Their picture is made on the first copy.
> - **The new power:** "yes to both, build it". A page that may rename may also store that one
>   picture beside the video, with a title save (WORKER-INTERFACE §3.5, §4.17).
> - **Built:** the boards "Copy as a thumbnail — designs", "B kept current" and "B + loop,
>   kept current". The probes for those boards (the Worker-side render, drawing at publish,
>   the loop's cost) are recorded in the B164 row.

**Boards** on the [shroom UI canvas](https://claude.ai/artifact/FQRbQUHrumYGXtvTm6UQsg), in the
page column under the existing boards:

- **"The page and the Library — copy as a thumbnail"**: the second copy button on the page,
  its "Copied" moment, what it copies, and the same item in a Library row's menu.
- **"A thumbnail, pasted"**: the result in a Basecamp message and in an email.

## Why

Basecamp turns a Loom link into a player, but a shroom link stays a plain link. Basecamp embeds
only from a fixed list of services: YouTube, Vimeo, Loom, Spotify, CleanShot, X, Flickr, Imgur
and SoundCloud. Everything else stays a standard link
([Basecamp help](https://5.basecamp-help.com/article/1091-formatting-text-in-basecamp)).
shroom can't get on that list, because every space has its own domain. The oEmbed endpoint and
og:/twitter: tags (`worker.js` `unfurl`, WORKER-INTERFACE §4.17) already cover Notion,
Iframely-based tools and the chat unfurlers. For the rest, especially email and Basecamp, Loom's
answer is the one to copy: a clickable picture that looks like a player.

## What Loom does

- **Where it lives:** Share → Embed tab. The tab offers "Copy embed code", "Copy GIF thumbnail"
  and "Copy thumbnail code", beside Copy link. Loom's docs say the GIF thumbnail "works with all
  email clients" (*documented*:
  [Loom: embed in email](https://support.atlassian.com/loom/docs/embed-video-into-your-email/);
  the tab layout is *reported* from a screenshot in
  [Instantly's help](https://help.instantly.ai/en/articles/6191433-how-to-add-loom-videos)).
- **What goes on the clipboard:** HTML. A linked title line, then a linked hosted image
  (*reported*, verbatim from the screenshot above):
  ```html
  <a href="https://www.loom.com/share/<id>"><p><title> - Watch Video</p></a>
  <a href="https://www.loom.com/share/<id>"><img style="max-width:300px;"
     src="https://cdn.loom.com/sessions/thumbnails/<id>-<hash>-full-play.gif#t=0.1"></a>
  ```
  The image is **hosted**, not inline. *Measured* by fetching it: `-full-play.gif` is
  640×324, 22 frames at 200 ms, looping. The play circle is drawn into the frames. There is no
  title or length on the picture. It is cached `public, max-age=31536000`.
- **Deleted or private videos:** Loom serves a "private thumbnail" for restricted videos
  (*documented*). Whether the CDN GIF stops serving after a delete is unknown. With a one-year
  cache it likely lingers (*inferred*).

## What pastes well where

The clipboard tested holds `text/html` (a linked title line plus `<a href><img src=https://…></a>`)
and `text/plain` (the bare URL). It holds **no** image file.

| Target | Result | Basis |
|---|---|---|
| **Basecamp (Trix 2.1.15)** | The picture becomes a remote-image attachment whose `href` is the page. It stays clickable and is saved as `<figure><a href><img src></a></figure>`. The title line stays a link | **Measured** (Playwright, a synthetic `insertFromPaste`). Basecamp's own server-side save is **not tested**: it could re-host or drop a remote image |
| **Basecamp, with a PNG also on the clipboard** | Trix takes the **file**: it uploads the PNG and drops the HTML, so the link is gone | **Measured** |
| Gmail web, Apple Mail, Outlook web | The linked picture is kept. This is the mechanism Loom relies on. Apple Mail may need "load remote content" turned on. Gmail fetches through its image proxy when the message is opened | *Reported* (Loom docs, mail-tool blogs). Gmail proxy: *reported* |
| A plain rich editor (contenteditable, Quill 2) | `<a><img></a>` is kept as is | **Measured** |
| ProseMirror/Tiptap, default schema (Linear and others are built on it) | The picture is kept, but the link around it is dropped. The title line keeps its link | **Measured** |
| GitHub comments | The bare URL. paste-markdown converts only text links, and the plain text is the URL | **Measured** (`@github/paste-markdown` 1.5.3) |
| Slack | Slack's composer has no inline pictures, so it keeps the link and unfurls it from the og tags we already serve | *Inferred* plus Slack's documented unfurler |
| Notion | An image block; the link is probably lost. The bare URL already embeds through oEmbed | *Inferred* |
| Google Docs, Teams | The picture is re-hosted; the link is uncertain | *Inferred / weak reports* |

What this means:

1. **HTML plus plain text, never an image file.** A PNG on the clipboard makes Trix, and
   reportedly Slack and Outlook Classic, upload a detached picture with no link.
2. **Put a linked text line beside the picture**, as Loom does. Wherever an editor drops the
   link on an image (ProseMirror, possibly Notion and Docs), the line still carries it.
3. **Keep it a separate action, not what Copy link does.** If Copy link also wrote HTML, a
   plain link pasted into Gmail, Docs or Notion would turn into a large picture. Loom keeps the
   two apart too.

Still to check by hand on the hosted Mac before building: a real paste into Basecamp (does its save keep
the remote image?), into Gmail, Apple Mail and Outlook, and into Slack. That's a 5-minute
matrix, one clipboard, about seven apps.

## Where the picture comes from

**Recommended: the app makes `thumb.jpg` at go-live, beside the poster, and uploads it with the
take.**

- It lives at `rec/<id>/thumb.jpg`, so `/m/<id>/thumb.jpg` serves it through the existing media
  route (`jpg` is allowed; D:B125). **The Worker doesn't change.** A deleted or sealed video
  answers 404 for it exactly as for the poster.
- The record gains `media.thumb` (a key, like `poster`), so the page knows it exists.
- What it shows: the first frame (the poster's) at 1200 wide, with a play mark in the centre.
  Optionally the length in a corner, which never changes. **The title is not drawn in**, because
  it can be renamed at any time (N26, R4). The title stays live text in the copied line.
- Cost: the bundled ffmpeg (`tools/build-ffmpeg.sh`) has neither `overlay` nor `drawtext` and
  no PNG decoder, and the build rules out external libraries, so drawtext/freetype is out.
  Two ways in:
  - Add the internal `overlay` filter, and pipe the mark as a small Rust-drawn BGRA frame
    through the `rawvideo` demuxer that's already enabled.
  - Draw the mark (and the digits for the length) into the poster frame in Rust before the
    `mjpeg` encode.

  Either is a small, in-frame change.
- **Before it's ready:** the poster and the thumb exist from go-live (ARCHITECTURE §9.1 step 5),
  the moment the page can play. The action is drawn only once the video plays; before that it
  simply isn't there. Nothing ever points at an
  image that 404s.
- **Videos from before this ships** have no `thumb.jpg`. They fall back to `poster.jpg` with no
  play mark. (Alternatively, the button doesn't show for them.)

**Privacy.** The image URL carries the same id as the link, so whoever has the picture can reach
the video. It grants nothing the link doesn't. A delete or seal stops the frame at the Worker at
once. Copies already fetched linger:
- the route's `max-age=3600`;
- Gmail's proxy, which by reports caches for hours, including a 404;
- anything an app re-hosted on paste (Docs, Teams, maybe Basecamp).

Loom has the same exposure and a one-year cache. If Bagrat wants a removed video to show
something kinder than a broken picture in old emails, a "removed" placeholder (200) for
`thumb.jpg` is a later Worker option. It is not recommended now: a 404 is the honest answer,
and it is the media route's rule.

### Rejected

- **The existing `poster.jpg` alone.** Free today, but with no play mark it reads as a
  screenshot, not a video. Kept only as the fallback for old videos, or as a first step if
  Bagrat wants this before the app change.
- **The Worker draws the thumbnail** (`/m/<id>/thumb.jpg` rendered on request). Workers have no
  image library. A WASM JPEG decode, composite and encode of a 1200-wide frame risks the free
  plan's CPU budget (*inferred*). Cloudflare's Images binding would add a binding the setup
  wizard must wire and a monthly quota on the owner's account. Either way it breaks the
  Worker's "exact and cheap" posture for a picture that never changes.
- **An animated GIF like Loom's.** It needs gif and palette filters in the build and is several
  times the size. Outlook desktop shows only its first frame and can hijack the click. Worth
  revisiting later; a still frame with a play mark carries most of the effect.
- **An inline (base64) image.** Gmail and most editors strip `data:` images on paste, and an
  inline picture can't be revoked.
- **A PNG on the clipboard as well.** It turns the paste into a detached upload (measured in
  Trix, above).
- **Copy link writes the HTML too.** This changes what the one ratified copy does, and turns
  plain-link pastes into pictures. See point 3 above.

## Where it lives in the UI

The spec has **no auto-copy at stop**: N1 says "Never auto-copied. Copying is explicit". The
copy places are the page's [ Copy link ], "the page's one copy door" (N26), and the Library
row's Copy link (N35). (The offline popover's Copy link is the third, only when offline.)
Nothing here touches the stop.

Recommended, as drawn on the first board:

- **The page:** a second icon button beside Copy link, a picture with a play mark. It shows only
  once the video plays, for everyone (like Copy link), with the same "Copied" moment. The copy
  runs in the browser: `navigator.clipboard.write` with a `ClipboardItem` of
  `text/html` + `text/plain`, falling back to a selection copy.
- **The Library:** the same item in a row's menu, right under Copy link. The always-visible
  row button stays Copy link alone. The app writes `public.html` + `public.utf8-plain-text`
  natively.

This is a spec change. N26 calls Copy link "the page's one copy door", and N35's row menu gains
an item. It needs Bagrat's yes, then a `machine.json` PR before any code.

## For Bagrat to decide

1. **Do it at all, and where:** the page and the Library (recommended), the Library only (no
   change to the page), or the page only.
2. **The words:**
   - the button's and menu item's label (drawn as `[COPY AS A THUMBNAIL]`);
   - the linked line's form (drawn as "‹title› · ‹length›"; Loom uses "‹title› - Watch
     Video").
3. **The picture:** a play mark only, or a play mark and the length (drawn with the length).
4. **Old videos:** fall back to the plain poster, or no button.

Once he says yes: the hand-test matrix above on the hosted Mac, then the spec PR (N26 and N35
amended), then the build. The build means the app-side `thumb.jpg` and `media.thumb`, the page
button and the Library menu item.

## After the build (2026-10-15)

- **Built:** [PR #104](https://github.com/bagrat/shroom-archive/pull/104), B164 and spec N46.
- **The paste, on a real Mac:** `dev/mac/scenarios/paste-thumbnail.sh`
  ([run 38753129546](https://github.com/bagrat/shroom-archive/actions/runs/38753129546)).
  - It puts the app's own pasteboard (`public.html` with the linked picture, plus the plain
    link) on macOS's general pasteboard and pastes with ⌘V.
  - **Safari** (WebKit, Apple Mail's compose engine) gets a linked picture that loads.
  - **TextEdit** shows an attachment placeholder; it doesn't fetch remote pictures. It isn't
    a target.
  - **Gmail, Apple Mail, Slack and Basecamp** need signed-in accounts the runner can't hold.
    They are Bagrat's to try by hand on 0.1.15.
- **Dropped: the Library making the picture on a first copy.** Bagrat ruled on 2026-10-15
  that older videos need no backward compatibility. An older video copied only from
  the Library keeps getting its first frame (linked) until its page is opened once. From
  0.1.15 every new video's page opens at stop and makes the picture itself.
  - What it would have cost: the runtime asking the Library's webview to draw, a new IPC
    command for the bytes, and the manage credential carried out of the library lane, all in
    the macOS-only crate.
  - The cheaper variant that was offered is dropped with it: opening the owner page in the
    background on such a copy.
- **After 0.1.15: Copy thumbnail waits until its picture is ready** (B166, N46 amended).
  Bagrat ruled on 2026-10-15: disabled until ready, the wording
  as drawn, and the tooltips given room from their buttons.
  - On the owner page it is greyed until the loop for the title is stored. Its tooltip says
    "Preparing the thumbnail…". It wakes with the still when no loop can be made.
  - On the viewer's page it is hidden until a picture for the title exists.
  - The Library keeps it enabled.
  - Every tooltip is drawn 8px below its button. The board is
    [`thumb-wait-board.png`](thumbnail-share/thumb-wait-board.png)
    (source `ThumbWait.dc.html`).
