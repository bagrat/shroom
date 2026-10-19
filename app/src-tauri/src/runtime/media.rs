//! The editor's media (N63): shroom's own `shroom-media:` scheme, through which an editor
//! window's webview plays the video it is on and reads its waveform. The editor's lane
//! registers, for each window (N73), the video it has found or fetched (a take's
//! `preview.mp4`, or `edits/<rec>/source.mp4`) and the peaks it read from it
//! (`edits/<rec>/peaks.u8`); the scheme serves those two files and nothing else, for that
//! recording only, to that window's webview only (`main.rs`):
//!
//!   shroom-media://localhost/<rec>/video   the video, by byte ranges (WebKit's player asks
//!                                          for them): 206 with `Content-Range`, `video/mp4`
//!   shroom-media://localhost/<rec>/peaks   the peaks, whole: `application/octet-stream`
//!
//! Anything else — another recording, another name, a path that isn't an id — is a 404. No
//! path from a request ever reaches the disk: the request names a recording and a kind, and
//! the registration says which file that is. A range is answered a piece at a time (at most
//! [`PIECE`]), so a long recording is never read into memory whole.

use std::fs::File;
use std::io::{Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};
use std::sync::Mutex;

use shroom_core::world::EditorId;

/// The scheme the editor's webview reads its media from (`shroom-media://localhost/…`).
pub const SCHEME: &str = "shroom-media";

/// The most one range answer carries: WebKit asks again for the rest.
pub const PIECE: u64 = 2 * 1024 * 1024;

/// The video an editor window is on, and its waveform: served to that window (N73).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Registered {
    pub window: EditorId,
    pub rec: String,
    pub video: PathBuf,
    pub peaks: PathBuf,
}

/// What the scheme serves: each editor window's video and peaks — one window per video (N73).
#[derive(Default)]
pub struct Registry(Mutex<Vec<Registered>>);

impl Registry {
    pub const fn new() -> Registry {
        Registry(Mutex::new(Vec::new()))
    }

    /// Serves `r.rec`'s video and peaks to its window from now on — in place of whatever
    /// was registered for that video.
    pub fn register(&self, r: Registered) {
        let mut g = self.0.lock().unwrap();
        g.retain(|x| x.rec != r.rec);
        g.push(r);
    }

    /// Stops serving `rec` (its window closed while it opened, or closed since).
    pub fn forget(&self, rec: &str) {
        self.0.lock().unwrap().retain(|r| r.rec != rec);
    }

    /// Stops serving anything (Disconnect, Leave).
    pub fn forget_all(&self) {
        self.0.lock().unwrap().clear();
    }

    /// The file for `rec`'s `kind`, while it's registered.
    pub fn file(&self, rec: &str, kind: Kind) -> Option<PathBuf> {
        self.served(rec, kind, |_| true)
    }

    /// …served to editor window `window` only: another window's video is none of its own.
    pub fn file_for(&self, window: EditorId, rec: &str, kind: Kind) -> Option<PathBuf> {
        self.served(rec, kind, |r| r.window == window)
    }

    fn served(&self, rec: &str, kind: Kind, to: impl Fn(&Registered) -> bool) -> Option<PathBuf> {
        let g = self.0.lock().unwrap();
        let r = g.iter().find(|r| r.rec == rec && to(r))?;
        Some(match kind {
            Kind::Video => r.video.clone(),
            Kind::Peaks => r.peaks.clone(),
        })
    }

    /// The video registered for `rec` (the copy's source, when the lane already found it).
    pub fn video(&self, rec: &str) -> Option<PathBuf> {
        self.file(rec, Kind::Video)
    }
}

/// The one registry the scheme and the editor's lane share.
pub static MEDIA: Registry = Registry::new();

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Video,
    Peaks,
}

/// `/<rec>/video` or `/<rec>/peaks`, the id checked as posters' are; anything else `None`.
pub fn route(path: &str) -> Option<(&str, Kind)> {
    let mut parts = path.strip_prefix('/')?.split('/');
    let (rec, kind) = (parts.next()?, parts.next()?);
    if parts.next().is_some() || !super::posters::valid_id(rec) {
        return None;
    }
    match kind {
        "video" => Some((rec, Kind::Video)),
        "peaks" => Some((rec, Kind::Peaks)),
        _ => None,
    }
}

/// A `Range` header against a file of `len` bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Range {
    /// No usable range: the whole file (a header that isn't one byte range is ignored, as
    /// RFC 9110 lets a server do).
    Whole,
    /// Bytes `start..=end`.
    Part(u64, u64),
    /// A range that starts past the end: 416.
    Unsatisfiable,
}

/// Reads one byte range (`bytes=a-b`, `bytes=a-`, `bytes=-n`); several, or anything else, is
/// the whole file.
pub fn range(header: Option<&str>, len: u64) -> Range {
    let Some(spec) = header.map(str::trim).and_then(|h| h.strip_prefix("bytes=")) else { return Range::Whole };
    if spec.contains(',') {
        return Range::Whole;
    }
    let Some((a, b)) = spec.trim().split_once('-') else { return Range::Whole };
    let num = |s: &str| if s.trim().is_empty() { Ok(None) } else { s.trim().parse::<u64>().map(Some).map_err(|_| ()) };
    let (Ok(a), Ok(b)) = (num(a), num(b)) else { return Range::Whole };
    match (a, b) {
        (None, None) => Range::Whole,
        // The last n bytes.
        (None, Some(n)) => {
            if n == 0 || len == 0 {
                Range::Unsatisfiable
            } else {
                Range::Part(len.saturating_sub(n), len - 1)
            }
        }
        (Some(start), end) => {
            if start >= len {
                return Range::Unsatisfiable;
            }
            match end {
                Some(e) if e < start => Range::Whole,
                Some(e) => Range::Part(start, e.min(len - 1)),
                None => Range::Part(start, len - 1),
            }
        }
    }
}

/// An answer, ready for the scheme's responder (`main.rs` makes it an HTTP response).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Answer {
    pub status: u16,
    pub headers: Vec<(&'static str, String)>,
    pub body: Vec<u8>,
}

impl Answer {
    fn bare(status: u16) -> Answer {
        Answer { status, headers: base_headers(), body: Vec::new() }
    }
}

/// Every answer's: never cached (the file under a name changes with each edit opened), and
/// readable by the editor's page — whose origin isn't the scheme's (`tauri://localhost` on
/// macOS). Nothing but shroom's own pages ever runs in its webviews.
fn base_headers() -> Vec<(&'static str, String)> {
    vec![("Cache-Control", "no-store".into()), ("Access-Control-Allow-Origin", "*".into())]
}

/// The answer to `method path` (with its `Range` header), asked by editor window `window`,
/// from what `registry` serves it.
pub fn answer(registry: &Registry, window: EditorId, method: &str, path: &str, range_header: Option<&str>) -> Answer {
    let head = method.eq_ignore_ascii_case("HEAD");
    if !head && !method.eq_ignore_ascii_case("GET") {
        return Answer::bare(405);
    }
    let Some((rec, kind)) = route(path) else { return Answer::bare(404) };
    let Some(file) = registry.file_for(window, rec, kind) else { return Answer::bare(404) };
    let served = match kind {
        Kind::Video => video(&file, head, range_header),
        Kind::Peaks => peaks(&file, head),
    };
    served.unwrap_or_else(|_| Answer::bare(404))
}

fn peaks(file: &Path, head: bool) -> std::io::Result<Answer> {
    let bytes = if head { Vec::new() } else { std::fs::read(file)? };
    let len = if head { std::fs::metadata(file)?.len() } else { bytes.len() as u64 };
    let mut headers = base_headers();
    headers.push(("Content-Type", "application/octet-stream".into()));
    headers.push(("Content-Length", len.to_string()));
    Ok(Answer { status: 200, headers, body: bytes })
}

fn video(file: &Path, head: bool, range_header: Option<&str>) -> std::io::Result<Answer> {
    let mut f = File::open(file)?;
    let len = f.metadata()?.len();
    let mut headers = base_headers();
    headers.push(("Content-Type", "video/mp4".into()));
    headers.push(("Accept-Ranges", "bytes".into()));
    let (status, start, end) = match range(range_header, len) {
        Range::Unsatisfiable => {
            headers.push(("Content-Range", format!("bytes */{len}")));
            return Ok(Answer { status: 416, headers, body: Vec::new() });
        }
        // A HEAD, or a GET with no range: the length, and (GET) the first piece as a range —
        // a player always asks by range, and a whole long recording is never read at once.
        Range::Whole if head || len == 0 => {
            headers.push(("Content-Length", len.to_string()));
            return Ok(Answer { status: 200, headers, body: Vec::new() });
        }
        Range::Whole if len <= PIECE => (200, 0, len - 1),
        Range::Whole => (206, 0, PIECE - 1),
        Range::Part(s, e) => (206, s, e.min(s + PIECE - 1)),
    };
    let n = end - start + 1;
    if status == 206 {
        headers.push(("Content-Range", format!("bytes {start}-{end}/{len}")));
    }
    headers.push(("Content-Length", n.to_string()));
    if head {
        return Ok(Answer { status, headers, body: Vec::new() });
    }
    f.seek(SeekFrom::Start(start))?;
    let mut body = vec![0u8; n as usize];
    f.read_exact(&mut body)?;
    Ok(Answer { status, headers, body })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_range_header_reads_as_one_byte_range() {
        assert_eq!(range(None, 1000), Range::Whole);
        assert_eq!(range(Some("bytes=0-1"), 1000), Range::Part(0, 1));
        assert_eq!(range(Some("bytes=0-"), 1000), Range::Part(0, 999));
        assert_eq!(range(Some("bytes=500-"), 1000), Range::Part(500, 999));
        assert_eq!(range(Some("bytes=10-5000"), 1000), Range::Part(10, 999), "an end past the file is the file's end");
        assert_eq!(range(Some("bytes=-100"), 1000), Range::Part(900, 999), "the last 100 bytes");
        assert_eq!(range(Some("bytes=-5000"), 1000), Range::Part(0, 999));
        assert_eq!(range(Some(" bytes=7-7 "), 1000), Range::Part(7, 7));
        assert_eq!(range(Some("bytes=1000-"), 1000), Range::Unsatisfiable);
        assert_eq!(range(Some("bytes=-0"), 1000), Range::Unsatisfiable);
        assert_eq!(range(Some("bytes=0-"), 0), Range::Unsatisfiable);
        for ignored in ["bytes=0-1,5-6", "items=0-1", "bytes=a-b", "bytes=5-2", "bytes=-", "bytes", ""] {
            assert_eq!(range(Some(ignored), 1000), Range::Whole, "{ignored}");
        }
    }

    #[test]
    fn only_a_recording_id_and_a_kind_route() {
        assert_eq!(route("/AbCdEfGhIjKlMnOp/video"), Some(("AbCdEfGhIjKlMnOp", Kind::Video)));
        assert_eq!(route("/AbCd_-9/peaks"), Some(("AbCd_-9", Kind::Peaks)));
        for bad in ["/AbCd", "/AbCd/", "/AbCd/poster", "/AbCd/video/x", "/../video", "/a.b/video", "AbCd/video", "/%2e%2e/video", "//video", "/"] {
            assert_eq!(route(bad), None, "{bad}");
        }
    }

    /// Each editor window's video, served to it (N73): registering one keeps the others, and
    /// a window reads only its own.
    #[test]
    fn the_registry_holds_each_windows_video_and_forgets_them() {
        let r = Registry::new();
        let (one, two) = (EditorId(1), EditorId(2));
        assert_eq!(r.video("A"), None);
        r.register(Registered { window: one, rec: "A".into(), video: "/a.mp4".into(), peaks: "/a.u8".into() });
        assert_eq!(r.file("A", Kind::Peaks), Some(PathBuf::from("/a.u8")));
        assert_eq!(r.file("B", Kind::Video), None, "only what's registered");
        r.register(Registered { window: two, rec: "B".into(), video: "/b.mp4".into(), peaks: "/b.u8".into() });
        assert_eq!(r.video("A"), Some(PathBuf::from("/a.mp4")), "another window's video keeps the first");
        assert_eq!((r.file_for(one, "A", Kind::Video), r.file_for(one, "B", Kind::Video)), (Some(PathBuf::from("/a.mp4")), None), "window 1 reads its own only");
        assert_eq!(r.file_for(two, "B", Kind::Peaks), Some(PathBuf::from("/b.u8")));
        r.register(Registered { window: EditorId(3), rec: "A".into(), video: "/a2.mp4".into(), peaks: "/a2.u8".into() });
        assert_eq!((r.file_for(one, "A", Kind::Video), r.file_for(EditorId(3), "A", Kind::Video)), (None, Some(PathBuf::from("/a2.mp4"))), "the video's next window takes its place");
        r.forget("A");
        assert_eq!(r.video("B"), Some(PathBuf::from("/b.mp4")), "forgetting another changes nothing");
        r.forget("B");
        assert_eq!(r.video("B"), None);
        r.register(Registered { window: one, rec: "C".into(), video: "/c.mp4".into(), peaks: "/c.u8".into() });
        r.forget_all();
        assert_eq!(r.video("C"), None);
    }

    fn header<'a>(a: &'a Answer, name: &str) -> Option<&'a str> {
        a.headers.iter().find(|(k, _)| k.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }

    #[test]
    fn the_scheme_serves_the_registered_video_by_ranges_and_nothing_else() {
        let tmp = tempfile::tempdir().unwrap();
        let video = tmp.path().join("preview.mp4");
        let bytes: Vec<u8> = (0..(PIECE + 4096)).map(|i| (i % 251) as u8).collect();
        std::fs::write(&video, &bytes).unwrap();
        let peaks = tmp.path().join("peaks.u8");
        std::fs::write(&peaks, [0u8, 9, 255]).unwrap();
        std::fs::write(tmp.path().join("token.json"), b"secret").unwrap();
        let r = Registry::new();
        r.register(Registered { window: EditorId(1), rec: "AbCdEfGhIjKlMnOp".into(), video: video.clone(), peaks });
        let len = bytes.len() as u64;

        let a = answer(&r, EditorId(1), "GET", "/AbCdEfGhIjKlMnOp/video", Some("bytes=0-1"));
        assert_eq!((a.status, a.body.as_slice()), (206, &bytes[0..2]));
        assert_eq!(header(&a, "Content-Range"), Some(format!("bytes 0-1/{len}").as_str()));
        assert_eq!((header(&a, "Content-Length"), header(&a, "Accept-Ranges"), header(&a, "Content-Type")), (Some("2"), Some("bytes"), Some("video/mp4")));
        assert_eq!(header(&a, "Access-Control-Allow-Origin"), Some("*"));

        // A long ask is answered a piece at a time.
        let a = answer(&r, EditorId(1), "GET", "/AbCdEfGhIjKlMnOp/video", Some("bytes=100-"));
        assert_eq!((a.status, a.body.len() as u64), (206, PIECE));
        assert_eq!(a.body.as_slice(), &bytes[100..(100 + PIECE as usize)]);
        assert_eq!(header(&a, "Content-Range"), Some(format!("bytes 100-{}/{len}", 100 + PIECE - 1).as_str()));
        let a = answer(&r, EditorId(1), "GET", "/AbCdEfGhIjKlMnOp/video", Some("bytes=-10"));
        assert_eq!((a.status, a.body.as_slice()), (206, &bytes[bytes.len() - 10..]));
        let a = answer(&r, EditorId(1), "GET", "/AbCdEfGhIjKlMnOp/video", Some(&format!("bytes={len}-")));
        assert_eq!((a.status, header(&a, "Content-Range")), (416, Some(format!("bytes */{len}").as_str())));

        // HEAD: the length, no body.
        let a = answer(&r, EditorId(1), "HEAD", "/AbCdEfGhIjKlMnOp/video", None);
        assert_eq!((a.status, header(&a, "Content-Length"), a.body.len()), (200, Some(len.to_string().as_str()), 0));
        let a = answer(&r, EditorId(1), "HEAD", "/AbCdEfGhIjKlMnOp/video", Some("bytes=0-1"));
        assert_eq!((a.status, header(&a, "Content-Length"), a.body.len()), (206, Some("2"), 0));
        // A GET with no range on a long file: its first piece, as a range.
        let a = answer(&r, EditorId(1), "GET", "/AbCdEfGhIjKlMnOp/video", None);
        assert_eq!((a.status, a.body.len() as u64), (206, PIECE));

        let a = answer(&r, EditorId(1), "GET", "/AbCdEfGhIjKlMnOp/peaks", None);
        assert_eq!((a.status, a.body.as_slice(), header(&a, "Content-Type")), (200, &[0u8, 9, 255][..], Some("application/octet-stream")));

        for (method, path) in [("GET", "/OtherRecording00/video"), ("GET", "/AbCdEfGhIjKlMnOp/poster"), ("GET", "/../token.json"), ("GET", "/AbCdEfGhIjKlMnOp/../../token.json")] {
            assert_eq!(answer(&r, EditorId(1), method, path, None).status, 404, "{path}");
        }
        assert_eq!(answer(&r, EditorId(2), "GET", "/AbCdEfGhIjKlMnOp/video", Some("bytes=0-1")).status, 404, "another editor window's video (N73)");
        assert_eq!(answer(&r, EditorId(1), "POST", "/AbCdEfGhIjKlMnOp/video", None).status, 405);
        r.forget("AbCdEfGhIjKlMnOp");
        assert_eq!(answer(&r, EditorId(1), "GET", "/AbCdEfGhIjKlMnOp/video", Some("bytes=0-1")).status, 404, "forgotten: gone");

        // A small file with no range comes whole.
        let small = tmp.path().join("small.mp4");
        std::fs::write(&small, b"0123456789").unwrap();
        r.register(Registered { window: EditorId(1), rec: "S".into(), video: small, peaks: tmp.path().join("none.u8") });
        let a = answer(&r, EditorId(1), "GET", "/S/video", None);
        assert_eq!((a.status, a.body.as_slice(), header(&a, "Content-Range")), (200, &b"0123456789"[..], None));
        assert_eq!(answer(&r, EditorId(1), "GET", "/S/peaks", None).status, 404, "a file that isn't there");
    }
}
