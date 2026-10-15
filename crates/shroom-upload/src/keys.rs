//! Where things live in the bucket (DECISIONS B13). Keys are permanent once links exist.

/// The variant directory for v1's single rendition. Named by position, not resolution,
/// so Normal, 2K and 4K all fit and a v2 ladder adds `v1/`, `v2/` beside it.
pub const VARIANT: &str = "v0";

/// Where the deletion records live (worker.js `GONE`, DECISIONS B125): out of every
/// recording credential's reach, so a sealed id stays deleted whatever its record says.
pub const GONE: &str = ".shroom/gone/";

pub fn meta(id: &str) -> String {
    format!("meta/{id}.json")
}

pub fn prefix(id: &str) -> String {
    format!("rec/{id}/")
}

pub fn master(id: &str) -> String {
    format!("rec/{id}/master.m3u8")
}

/// A file of the variant: `init.mp4`, `seg_00000.m4s`, `stream.m3u8`.
pub fn variant_file(id: &str, file: &str) -> String {
    format!("rec/{id}/{VARIANT}/{file}")
}

/// The single-file copy. Title-independent: the Worker names downloads from the live
/// title, so a rename fixes download names retroactively.
pub fn mp4(id: &str) -> String {
    format!("rec/{id}/video.mp4")
}

/// The unfurl poster (og:image) — title-independent, so renames never break posted cards.
pub fn poster(id: &str) -> String {
    format!("rec/{id}/poster.jpg")
}

/// The unfurl teaser (og:video) — DECISIONS B27.
pub fn teaser(id: &str) -> String {
    format!("rec/{id}/teaser.mp4")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_media_key_sits_under_the_recordings_prefix() {
        let id = "AbCdEfGhIjKlMnOp";
        for k in [master(id), variant_file(id, "init.mp4"), mp4(id)] {
            assert!(k.starts_with(&prefix(id)), "{k}");
        }
        assert_eq!(meta(id), "meta/AbCdEfGhIjKlMnOp.json");
    }
}
