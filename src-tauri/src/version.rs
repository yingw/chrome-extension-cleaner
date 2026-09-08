//! Chrome extension version type.
//!
//! Chrome versions are 1-4 dot-separated integers, each in 0..=65535.
//! Anything else is rejected (fail closed). On-disk directory names append
//! a numeric suffix: `<version>_<n>` (usually `_0`).

use std::cmp::Ordering;

/// A validated Chrome-style version: up to 4 numeric segments.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChromeVersion {
    parts: [u16; 4],
}

impl ChromeVersion {
    /// Strict parse. Returns `None` for wrong segment counts,
    /// non-digit segments, or values above 65535.
    pub fn parse(s: &str) -> Option<Self> {
        let segments: Vec<&str> = s.split('.').collect();
        if segments.is_empty() || segments.len() > 4 {
            return None;
        }
        let mut parts = [0u16; 4];
        for (i, seg) in segments.iter().enumerate() {
            if seg.is_empty() || !seg.bytes().all(|b| b.is_ascii_digit()) {
                return None;
            }
            // Reject leading zeros ("01") to keep ordering unambiguous.
            if seg.len() > 1 && seg.starts_with('0') {
                return None;
            }
            let value: u32 = seg.parse().ok()?;
            if value > 65535 {
                return None;
            }
            parts[i] = value as u16;
        }
        Some(Self { parts })
    }
}

impl PartialOrd for ChromeVersion {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for ChromeVersion {
    fn cmp(&self, other: &Self) -> Ordering {
        self.parts.cmp(&other.parts)
    }
}

/// Split `<version>_<n>` into its version and suffix.
/// Returns `None` for anything that does not strictly match.
pub fn split_dir_name(dirname: &str) -> Option<(ChromeVersion, u32)> {
    let (version_part, suffix_part) = dirname.rsplit_once('_')?;
    let version = ChromeVersion::parse(version_part)?;
    if suffix_part.is_empty()
        || !suffix_part.bytes().all(|b| b.is_ascii_digit())
    {
        return None;
    }
    let suffix: u32 = suffix_part.parse().ok()?;
    Some((version, suffix))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_valid_versions() {
        assert!(ChromeVersion::parse("1").is_some());
        assert!(ChromeVersion::parse("5.5.2.13").is_some());
        assert!(ChromeVersion::parse("0.0.0.0").is_some());
        assert!(ChromeVersion::parse("65535.65535.65535.65535").is_some());
    }

    #[test]
    fn rejects_invalid_versions() {
        for bad in [
            "",
            "1.2.3.4.5",  // too many segments
            "1..2",       // empty segment
            "1.a.2",      // non-digit
            "65536",      // out of range
            "01.2",       // leading zero
            " 1.2",       // whitespace
            "-1",
        ] {
            assert!(ChromeVersion::parse(bad).is_none(), "input: {bad:?}");
        }
    }

    #[test]
    fn orders_numerically_not_lexically() {
        let v = |s: &str| ChromeVersion::parse(s).unwrap();
        assert!(v("5.5.2.9") < v("5.5.2.13"));
        assert!(v("1.99.2") > v("1.99"));
        assert!(v("14.1326.0") > v("14.1325.0"));
        assert_eq!(v("1.0"), v("1.0.0"));
    }

    #[test]
    fn splits_dir_names() {
        assert!(split_dir_name("5.5.2.13_0").is_some());
        assert!(split_dir_name("1.99_12").is_some());
        assert!(split_dir_name("Temp").is_none());
        assert!(split_dir_name(".DS_Store").is_none());
        assert!(split_dir_name("5.5.2.13").is_none()); // suffix required
        assert!(split_dir_name("5.5.2.13_").is_none());
        assert!(split_dir_name("abc_0").is_none());
    }
}
