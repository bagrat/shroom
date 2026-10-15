//! The master playlist: one variant now, more later without changing any link.

/// The `avc1.PPCCLL` codec string, read from the `avcC` box in an fMP4 init segment.
pub fn avc_codec(init: &[u8]) -> Option<String> {
    let at = init.windows(4).position(|w| w == b"avcC")?;
    // avcC body: configurationVersion, AVCProfileIndication, profile_compatibility, AVCLevelIndication.
    let body = init.get(at + 4..at + 8)?;
    (body[0] == 1).then(|| format!("avc1.{:02x}{:02x}{:02x}", body[1], body[2], body[3]))
}

/// Whether the init segment carries an AAC track.
pub fn has_aac(init: &[u8]) -> bool {
    init.windows(4).any(|w| w == b"mp4a")
}

pub struct Variant {
    /// Peak bits per second over any one segment.
    pub bandwidth: u64,
    pub width: u32,
    pub height: u32,
    pub codecs: String,
    pub uri: String,
}

pub fn render(variants: &[Variant]) -> String {
    let mut out = String::from("#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-INDEPENDENT-SEGMENTS\n");
    for v in variants {
        out.push_str(&format!(
            "#EXT-X-STREAM-INF:BANDWIDTH={},RESOLUTION={}x{},CODECS=\"{}\"\n{}\n",
            v.bandwidth, v.width, v.height, v.codecs, v.uri
        ));
    }
    out
}

/// Peak segment bitrate, from `(bytes, seconds)` per segment.
pub fn peak_bandwidth(segments: &[(u64, f64)]) -> u64 {
    segments
        .iter()
        .filter(|(_, s)| *s > 0.0)
        .map(|(b, s)| (*b as f64 * 8.0 / s).ceil() as u64)
        .max()
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_the_codec_from_avcc() {
        let mut init = b"....ftypiso6....avcC".to_vec();
        init.extend([1, 0x64, 0x00, 0x28, 0xff]);
        assert_eq!(avc_codec(&init).as_deref(), Some("avc1.640028"));
        assert_eq!(avc_codec(b"nothing here"), None);
    }

    #[test]
    fn renders_one_variant() {
        let m = render(&[Variant { bandwidth: 3_900_000, width: 1920, height: 1080, codecs: "avc1.640028".into(), uri: "v0/stream.m3u8".into() }]);
        assert_eq!(
            m,
            "#EXTM3U\n#EXT-X-VERSION:7\n#EXT-X-INDEPENDENT-SEGMENTS\n#EXT-X-STREAM-INF:BANDWIDTH=3900000,RESOLUTION=1920x1080,CODECS=\"avc1.640028\"\nv0/stream.m3u8\n"
        );
    }

    #[test]
    fn bandwidth_is_the_peak_segment() {
        assert_eq!(peak_bandwidth(&[(3_000_000, 6.0), (1_000_000, 1.0), (0, 0.0)]), 8_000_000);
    }
}
