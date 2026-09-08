//! `Secure Preferences` parsing: the same record Chrome itself uses.
//!
//! Per extension id we extract the active version, the enablement state and
//! the install origin. Missing or conflicting fields fail closed (Protected).

use std::collections::HashMap;
use std::path::Path;

/// Install origin of an extension, for gating automatic cleanup.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum InstallSource {
    /// `from_webstore == true` and standard installed location.
    WebStoreStandard,
    /// Anything else, including missing fields. Fails closed.
    #[default]
    Other,
}

/// What Chrome recorded for one extension.
#[derive(Debug, Clone, Default)]
pub struct PrefsEntry {
    pub name: Option<String>,
    pub version: Option<String>,
    /// Raw `Extensions/<id>/<version>_0`-style relative path, when present.
    pub disk_path: Option<String>,
    pub disable_reasons: Vec<i64>,
    /// Raw install location from Chrome (`extensions.settings.<id>.location`).
    /// `None` when the record is malformed or missing — we never guess.
    pub location: Option<i64>,
    pub install_source: InstallSource,
}

impl PrefsEntry {
    /// Enabled when no disable reason is recorded.
    pub fn is_enabled(&self) -> bool {
        self.disable_reasons.is_empty()
    }

    /// Chrome components (`kComponent` = 5, `kExternalComponent` = 10) are
    /// internal implementation details — Chrome hides them from its own
    /// management UI, so we skip them entirely rather than show a row that can
    /// never be cleaned. Only consulted when a prefs record actually exists; a
    /// missing record falls through to displaying (fail-safe), never hiding.
    pub fn is_component(&self) -> bool {
        matches!(self.location, Some(5) | Some(10))
    }
}

/// Map extension id -> prefs entry. Empty map when neither file holds settings.
///
/// Chrome keeps protected preferences in a different file per platform, at the
/// same version (verified on 152.0.7977.8x). On macOS, `extensions.settings` is
/// segregated into `Secure Preferences` and `Preferences` holds none. On Linux
/// the same settings stay inline in `Preferences`, while `Secure Preferences` is
/// a stub carrying only a `super_mac`.
///
/// Reading the authoritative file first and falling back to the other covers
/// both with one code path. When neither holds settings we return empty and the
/// caller fails closed (every extension Protected) — never a guess.
pub fn load_secure_prefs(user_data_dir: &Path, profile: &str) -> HashMap<String, PrefsEntry> {
    let dir = user_data_dir.join(profile);
    for file in ["Secure Preferences", "Preferences"] {
        let Ok(text) = std::fs::read_to_string(dir.join(file)) else {
            continue;
        };
        let Ok(root) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        let Some(settings) = root
            .pointer("/extensions/settings")
            .and_then(|v| v.as_object())
        else {
            continue;
        };
        let out = parse_entries(settings);
        if !out.is_empty() {
            return out;
        }
    }
    HashMap::new()
}

/// Turn an `extensions.settings` object into prefs entries.
fn parse_entries(
    settings: &serde_json::Map<String, serde_json::Value>,
) -> HashMap<String, PrefsEntry> {
    let mut out = HashMap::new();
    for (ext_id, entry) in settings {
        let manifest = entry.get("manifest");
        let version = manifest
            .and_then(|m| m.get("version"))
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let name = manifest
            .and_then(|m| m.get("name"))
            .and_then(|v| v.as_str())
            .map(str::to_string);
        let disable_reasons = entry
            .get("disable_reasons")
            .and_then(|v| v.as_array())
            .map(|arr| arr.iter().filter_map(|v| v.as_i64()).collect())
            .unwrap_or_default();
        // Standard installs observed in the wild: from_webstore=true, location=1.
        // Anything missing or different is `Other` (fail closed, display only).
        let from_webstore = entry
            .get("from_webstore")
            .and_then(|v| v.as_bool())
            .unwrap_or(false);
        let location = entry.get("location").and_then(|v| v.as_i64());
        let install_source = if from_webstore && location == Some(1) {
            InstallSource::WebStoreStandard
        } else {
            InstallSource::Other
        };
        out.insert(
            ext_id.clone(),
            PrefsEntry {
                name,
                version,
                disk_path: entry
                    .get("path")
                    .and_then(|v| v.as_str())
                    .map(str::to_string),
                disable_reasons,
                location,
                install_source,
            },
        );
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    /// A profile dir with only `Secure Preferences` populated (macOS layout).
    const SECURE_BODY: &str = r#"{"protection":{"super_mac":"AA"},"extensions":{"settings":{
        "aaaa":{"manifest":{"name":"From Secure","version":"1.0"},
                "path":"aaaa/1.0_0","from_webstore":true,"location":1}}}}"#;
    /// A profile dir with the settings inline (Linux layout).
    const PLAIN_BODY: &str = r#"{"extensions":{"settings":{
        "bbbb":{"manifest":{"name":"From Plain","version":"2.0"},
                "path":"bbbb/2.0_0","from_webstore":true,"location":1}}}}"#;
    /// What Linux Chrome actually writes to `Secure Preferences`: a bare stub.
    const STUB_BODY: &str = r#"{"protection":{"super_mac":"DEADBEEF"}}"#;

    fn scaffold(tag: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("cec-prefs-{tag}"));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(dir.join("Default")).unwrap();
        dir
    }

    fn write(dir: &Path, file: &str, body: &str) {
        fs::write(dir.join("Default").join(file), body).unwrap();
    }

    #[test]
    fn macos_layout_reads_secure_preferences() {
        let dir = scaffold("macos");
        write(&dir, "Secure Preferences", SECURE_BODY);
        write(&dir, "Preferences", r#"{"extensions":{"settings":{}}}"#);

        let map = load_secure_prefs(&dir, "Default");
        assert_eq!(map.len(), 1, "expected the secure entry: {map:?}");
        let e = &map["aaaa"];
        assert_eq!(e.name.as_deref(), Some("From Secure"));
        assert_eq!(e.version.as_deref(), Some("1.0"));
        assert_eq!(e.disk_path.as_deref(), Some("aaaa/1.0_0"));
        assert_eq!(e.install_source, InstallSource::WebStoreStandard);

        let _ = fs::remove_dir_all(&dir);
    }

    /// The Linux regression: `Secure Preferences` is a stub, so every extension
    /// used to look unverified and was refused deletion.
    #[test]
    fn linux_layout_falls_back_to_plain_preferences() {
        let dir = scaffold("linux");
        write(&dir, "Secure Preferences", STUB_BODY);
        write(&dir, "Preferences", PLAIN_BODY);

        let map = load_secure_prefs(&dir, "Default");
        assert_eq!(map.len(), 1, "must fall back to Preferences: {map:?}");
        let e = &map["bbbb"];
        assert_eq!(e.name.as_deref(), Some("From Plain"));
        assert_eq!(e.version.as_deref(), Some("2.0"));
        assert_eq!(e.install_source, InstallSource::WebStoreStandard);

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn neither_file_holds_settings_fails_closed() {
        let dir = scaffold("empty");
        write(&dir, "Secure Preferences", STUB_BODY);
        write(&dir, "Preferences", "{}");

        assert!(
            load_secure_prefs(&dir, "Default").is_empty(),
            "no settings anywhere must yield an empty map (fail closed)"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn secure_preferences_wins_when_both_populated() {
        let dir = scaffold("both");
        write(&dir, "Secure Preferences", SECURE_BODY);
        write(&dir, "Preferences", PLAIN_BODY);

        let map = load_secure_prefs(&dir, "Default");
        assert_eq!(map.len(), 1);
        assert!(
            map.contains_key("aaaa"),
            "the authoritative file must win: {map:?}"
        );

        let _ = fs::remove_dir_all(&dir);
    }

    #[test]
    fn component_locations_are_flagged_hideable() {
        let at = |loc: Option<i64>| PrefsEntry {
            location: loc,
            ..Default::default()
        };
        assert!(at(Some(10)).is_component(), "kExternalComponent (10) must hide");
        assert!(at(Some(5)).is_component(), "kComponent (5) must hide");
        assert!(!at(Some(1)).is_component(), "kInternal (1) must stay visible");
        assert!(!at(Some(4)).is_component(), "kUnpacked (4) must stay visible");
        assert!(
            !at(None).is_component(),
            "missing location must stay visible (fail-safe)"
        );
    }
}
