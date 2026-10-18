//! Getting a recording into the user's bucket (ARCHITECTURE §5, §9.1).
//!
//! Keys (DECISIONS B13): `meta/<id>.json` is the record and the tombstone;
//! `rec/<id>/master.m3u8` points at the one variant `rec/<id>/v0/` (init, segments,
//! stream.m3u8 — local names kept, so file → key is one-to-one); the single-file copy is
//! `rec/<id>/video.mp4`. Order: claim the meta create-only, media, the variant playlist
//! *last* (that's the go-live), the master, then the meta flips to live.

pub mod keys;
pub mod library;
pub mod live;
pub mod master;
pub mod meta;
pub mod upload;
pub mod voice;

/// A new recording id: 12 random bytes, base64url without padding — 16 characters,
/// 96 bits, no structure. The id is the link's only secret (RESEARCH R10.2, P1).
pub fn new_id() -> String {
    use base64::Engine;
    let mut bytes = [0u8; 12];
    getrandom::fill(&mut bytes).expect("the OS random source works");
    base64::engine::general_purpose::URL_SAFE_NO_PAD.encode(bytes)
}

#[cfg(test)]
mod tests {
    #[test]
    fn ids_are_sixteen_url_safe_characters_and_distinct() {
        let a = super::new_id();
        let b = super::new_id();
        assert_eq!(a.len(), 16);
        assert!(a.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_'));
        assert_ne!(a, b);
    }
}
