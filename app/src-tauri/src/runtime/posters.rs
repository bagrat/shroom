//! The Library's posters (N35: a row is “title, date · length, the poster”).
//!
//! A take this Mac holds shows its own `takes/<dir>/poster.jpg`. A record of the space is
//! fetched once — the file its record names (`poster.jpg`, or a baked edit's
//! `edit-<rev>-poster.jpg`, B197) over the token's S3 on a token Mac, from the space's page
//! (`/m/<id>/<file>`, public like the page itself) on any other — and cached under the home
//! (`posters/<id>.jpg`, with the file it came from beside it, `<id>.from`: another file
//! named is fetched again; folder 700, files 600; Disconnect's wipe removes it). The
//! Library window reads them through shroom's own `shroom-poster:` scheme, which serves
//! only those two kinds of file, by strictly checked names — never another path, never the
//! network (the fetch is the Library lane's, after a refresh).

use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;

use shroom_core::world::SpaceIdentity;
use shroom_storage::s3::Client;
use shroom_upload::library::Cache;

use super::{Runtime, TEST_PREFIX};

/// The scheme the Library window loads posters from (`shroom-poster://localhost/<key>`).
pub const SCHEME: &str = "shroom-poster";

/// At most this many new posters per refresh (a first open of a big space fills in over a
/// few refreshes, never one long wait).
const PER_REFRESH: usize = 40;
/// A poster is a 1200-wide JPEG; anything much bigger isn't one.
const MAX_BYTES: usize = 4 * 1024 * 1024;

/// Ids tried this run that had no poster to fetch: not asked again until the next launch.
static MISSED: Mutex<Option<HashSet<String>>> = Mutex::new(None);

fn dir(rt: &Runtime) -> PathBuf {
    rt.home.root.join("posters")
}

/// A recording id as the space makes them (`[A-Za-z0-9_-]`, short).
pub fn valid_id(id: &str) -> bool {
    !id.is_empty() && id.len() <= 64 && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// A take folder's name as `takes::new_dir_name` makes them (`yyyymmdd-hhmmss-xxxx`).
pub fn valid_dir(d: &str) -> bool {
    !d.is_empty() && d.len() <= 64 && d.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-') && !d.starts_with('-')
}

/// This record's poster is on this Mac.
pub fn cached(rt: &Runtime, id: &str) -> bool {
    valid_id(id) && dir(rt).join(format!("{id}.jpg")).is_file()
}

/// Which of the recording's files the cached poster came from (one cached before the
/// record could name another: `poster.jpg`).
fn cached_from(rt: &Runtime, id: &str) -> String {
    std::fs::read_to_string(dir(rt).join(format!("{id}.from"))).map(|s| s.trim().to_string()).unwrap_or_else(|_| shroom_encode::files::POSTER.to_string())
}

/// A poster to fetch: the record names one beside it, and this Mac's isn't that one.
fn wanted(rt: &Runtime, id: &str, file: Option<&str>) -> bool {
    valid_id(id) && file.is_some_and(|f| !cached(rt, id) || cached_from(rt, id) != f)
}

/// The bytes for a `shroom-poster:` path — `/<rec>` or `/<rec>/<take dir>` — or `None`.
/// Only `takes/<dir>/poster.jpg` and `posters/<rec>.jpg` are ever read.
pub fn serve(rt: &Runtime, path: &str) -> Option<Vec<u8>> {
    let path = path.trim_start_matches('/');
    let mut parts = path.split('/');
    let rec = parts.next()?;
    let take = parts.next();
    if parts.next().is_some() || !valid_id(rec) || take.is_some_and(|d| !valid_dir(d)) {
        return None;
    }
    if let Some(d) = take {
        if let Ok(b) = std::fs::read(rt.home.takes().join(d).join(shroom_encode::files::POSTER)) {
            return Some(b);
        }
    }
    std::fs::read(dir(rt).join(format!("{rec}.jpg"))).ok()
}

/// Where a space's page serves a record's poster file, as this build may reach it: the page
/// in a release build; a dev build's local stand-in, a `shroom-test-…` page, or the e2e run's
/// own — nothing else.
fn page_url(rt: &Runtime, host: &str, id: &str, file: &str) -> Option<String> {
    reachable(rt, host, &format!("/m/{id}/{file}"))
}

/// `path` on a space's page, as this build may reach it (as [`page_url`]).
pub(super) fn reachable(rt: &Runtime, host: &str, path: &str) -> Option<String> {
    if !rt.dev {
        return Some(format!("https://{host}{path}"));
    }
    if let Some(base) = &rt.space_base {
        return Some(format!("{}/{host}{path}", base.trim_end_matches('/')));
    }
    if rt.e2e() {
        return super::e2e_host(host).then(|| format!("https://{host}{path}"));
    }
    (rt.cloud_fake.is_none() && host.starts_with(TEST_PREFIX)).then(|| format!("https://{host}{path}"))
}

pub(super) fn get(url: &str) -> Option<Vec<u8>> {
    let agent: ureq::Agent = ureq::Agent::config_builder()
        .http_status_as_error(false)
        .timeout_global(Some(Duration::from_secs(5)))
        .tls_config(ureq::tls::TlsConfig::builder().root_certs(ureq::tls::RootCerts::PlatformVerifier).build())
        .build()
        .into();
    let mut res = agent.get(url).call().ok()?;
    if res.status() != 200 {
        return None;
    }
    res.body_mut().with_config().limit(MAX_BYTES as u64).read_to_vec().ok()
}

/// Fetches the posters the cache's records name and this Mac doesn't have yet (a few per
/// refresh, each id once per run): `s3` on a token Mac, else the page. `true` when any
/// landed (the rows are worth posting again).
pub fn fetch_missing(rt: &Runtime, space: &SpaceIdentity, s3: Option<&Client>, cache: &Cache) -> bool {
    let wanted: Vec<(String, String)> = {
        let mut missed = MISSED.lock().unwrap();
        let missed = missed.get_or_insert_with(HashSet::new);
        cache
            .visible()
            .into_iter()
            .filter(|r| r.poster.is_some() && wanted(rt, &r.id, r.poster_file.as_deref()))
            .filter_map(|r| Some((r.id.clone(), r.poster_file.clone()?)))
            .filter(|(id, file)| !missed.contains(&format!("{id}/{file}")))
            .take(PER_REFRESH)
            .collect()
    };
    if wanted.is_empty() {
        return false;
    }
    let mut landed = 0usize;
    for (id, file) in &wanted {
        let bytes = match s3 {
            Some(s3) => s3.get(&format!("{}{file}", shroom_upload::keys::prefix(id))).ok().flatten(),
            None => page_url(rt, &space.host, id, file).and_then(|u| get(&u)),
        };
        match bytes.filter(|b| !b.is_empty() && b.len() <= MAX_BYTES && b.starts_with(&[0xFF, 0xD8])) {
            Some(b) if write(rt, id, file, &b).is_ok() => landed += 1,
            _ => {
                MISSED.lock().unwrap().get_or_insert_with(HashSet::new).insert(format!("{id}/{file}"));
            }
        }
    }
    rt.log(&format!("library: {landed} of {} poster(s) fetched", wanted.len()));
    landed > 0
}

/// `posters/<id>.jpg`, atomically, mode 600 in a folder of mode 700, and the file it came
/// from beside it.
fn write(rt: &Runtime, id: &str, from: &str, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    let d = dir(rt);
    if !d.is_dir() {
        std::fs::DirBuilder::new().recursive(true).mode(0o700).create(&d)?;
    }
    let tmp = d.join(format!(".{id}.tmp"));
    {
        let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&tmp)?;
        f.write_all(bytes)?;
    }
    std::fs::rename(&tmp, d.join(format!("{id}.jpg")))?;
    let tmp = d.join(format!(".{id}.from.tmp"));
    {
        let mut f = std::fs::OpenOptions::new().write(true).create(true).truncate(true).mode(0o600).open(&tmp)?;
        f.write_all(from.as_bytes())?;
    }
    std::fs::rename(&tmp, d.join(format!("{id}.from")))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_scheme_serves_only_posters_by_checked_names() {
        let tmp = tempfile::tempdir().unwrap();
        let (rt, _rx) = Runtime::for_test(tmp.path().join("h"), None, None);
        let take = rt.home.takes().join("20261002-120000-ab12");
        std::fs::create_dir_all(&take).unwrap();
        std::fs::write(take.join("poster.jpg"), b"\xFF\xD8local").unwrap();
        write(&rt, "AbCdEfGhIjKlMnOp", "poster.jpg", b"\xFF\xD8cached").unwrap();
        std::fs::write(rt.home.root.join("token.json"), b"secret").unwrap();

        assert_eq!(serve(&rt, "/AbCdEfGhIjKlMnOp/20261002-120000-ab12").as_deref(), Some(&b"\xFF\xD8local"[..]), "this Mac's own first");
        assert_eq!(serve(&rt, "/AbCdEfGhIjKlMnOp/20261002-999999-zz99").as_deref(), Some(&b"\xFF\xD8cached"[..]), "else the space's, cached");
        assert_eq!(serve(&rt, "/AbCdEfGhIjKlMnOp").as_deref(), Some(&b"\xFF\xD8cached"[..]));
        for bad in ["/..", "/../token.json", "/x/../../token.json", "/AbCd/..", "/AbCd%2F..", "/a/b/c", "/", "/token.json", "/AbCd/.hidden"] {
            assert_eq!(serve(&rt, bad), None, "{bad}");
        }
        assert!(cached(&rt, "AbCdEfGhIjKlMnOp") && !cached(&rt, "../token"));
        use std::os::unix::fs::PermissionsExt;
        let mode = |p: PathBuf| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
        assert_eq!((mode(dir(&rt)), mode(dir(&rt).join("AbCdEfGhIjKlMnOp.jpg"))), (0o700, 0o600));
    }

    /// A baked video's record names its edit's poster (B197): the one cached from before is
    /// fetched again; one already from that file isn't.
    #[test]
    fn a_poster_is_fetched_again_when_the_record_names_another_file() {
        let tmp = tempfile::tempdir().unwrap();
        let (rt, _rx) = Runtime::for_test(tmp.path().join("h"), None, None);
        assert!(wanted(&rt, "AbCd", Some("poster.jpg")), "none here");
        assert!(!wanted(&rt, "AbCd", None), "the record names none beside it");
        write(&rt, "AbCd", "poster.jpg", b"\xFF\xD8a").unwrap();
        assert!(!wanted(&rt, "AbCd", Some("poster.jpg")));
        assert!(wanted(&rt, "AbCd", Some("edit-2-poster.jpg")), "baked: the edit's poster");
        write(&rt, "AbCd", "edit-2-poster.jpg", b"\xFF\xD8b").unwrap();
        assert!(!wanted(&rt, "AbCd", Some("edit-2-poster.jpg")));
        std::fs::remove_file(dir(&rt).join("AbCd.from")).unwrap();
        assert!(!wanted(&rt, "AbCd", Some("poster.jpg")) && wanted(&rt, "AbCd", Some("edit-2-poster.jpg")), "one cached before the file was kept: the original's");
        assert!(!wanted(&rt, "../x", Some("poster.jpg")));
    }

    #[test]
    fn a_dev_build_fetches_posters_only_inside_its_fence() {
        let tmp = tempfile::tempdir().unwrap();
        let (rt, _rx) = Runtime::for_test(tmp.path().join("a"), Some(("http://127.0.0.1:1/client/v4".into(), "http://127.0.0.1:2".into())), None);
        assert_eq!(page_url(&rt, "v.myshroom.workers.dev", "AbCd", "poster.jpg"), None, "never a real space's page");
        let (rt, _rx) = Runtime::for_test(tmp.path().join("b"), None, None);
        assert_eq!(page_url(&rt, "shroom-test-x.a.workers.dev", "AbCd", "poster.jpg").as_deref(), Some("https://shroom-test-x.a.workers.dev/m/AbCd/poster.jpg"));
        let (rt, _rx) = Runtime::for_test(tmp.path().join("c"), None, Some("http://127.0.0.1:8787".into()));
        assert_eq!(page_url(&rt, "v.a.workers.dev", "AbCd", "edit-3-poster.jpg").as_deref(), Some("http://127.0.0.1:8787/v.a.workers.dev/m/AbCd/edit-3-poster.jpg"));
        let (rt, _rx) = Runtime::for_test(tmp.path().join("d"), None, None);
        assert_eq!(page_url(&rt, "shroom.myshroom.workers.dev", "AbCd", "poster.jpg"), None, "a plain dev run: never the real space");
    }
}
