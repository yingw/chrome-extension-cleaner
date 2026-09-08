//! Extension inventory scan with three-state classification.
//!
//! Protection model (fail closed):
//!
//! ```text
//! protected = active version
//!           ∪ newer-than-active version dirs (possibly staged by Chrome)
//!           ∪ unknown layouts / unverified extensions
//!           ∪ non-standard installations
//! ```
//!
//! Only directories strictly older than the active version, from a verified
//! standard Web Store install whose own manifest agrees with its directory
//! name, become deletion candidates.

use std::collections::HashMap;
use std::path::Path;

use serde::Serialize;

use crate::chrome::{default_user_data_dir, list_profiles, profile_labels, user_data_dir};
use crate::env::{chrome_status, chrome_version, os_label, ChromeStatus};
use crate::prefs::{load_secure_prefs, InstallSource, PrefsEntry};
use crate::state::{AppState, CandidateRecord};
use crate::version::{split_dir_name, ChromeVersion};

/// One on-disk version directory, as seen by the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct VersionInfo {
    /// Opaque per-scan id. Never an absolute path.
    pub cid: String,
    pub dir: String,
    pub version: String,
    /// Size in BYTES. The backend never formats numbers: unit selection and
    /// precision are the frontend's call (see fmtSize in app.js).
    pub size: u64,
    pub is_active: bool,
    pub unknown: bool,
    /// Whether the UI may offer this version for deletion.
    pub deletable: bool,
    /// Machine-readable reason: `active`, `candidate`, `protected:<why>`.
    pub reason: String,
}

/// One installed extension, versions newest-first.
#[derive(Debug, Clone, Serialize)]
pub struct ExtensionInfo {
    pub profile: String,
    pub id: String,
    pub name: String,
    /// Display name in every UI language the app ships (keyed by Chrome
    /// `_locales` tag: en/zh_CN/zh_TW/ko/ja/de/fr/es/ru). Lets the frontend
    /// repaint names live on language switch without rescanning.
    pub names: HashMap<String, String>,
    pub active_version: Option<String>,
    pub verified: bool,
    pub enabled: bool,
    pub disable_reasons: Vec<i64>,
    pub versions: Vec<VersionInfo>,
    pub n_versions: usize,
    /// Bytes across every version dir of this extension.
    pub total_bytes: u64,
    /// Bytes held by deletable versions only.
    pub waste_bytes: u64,
}

/// Full scan payload returned to the frontend.
#[derive(Debug, Clone, Serialize)]
pub struct ScanResponse {
    pub scan_id: u64,
    pub platform: String,
    pub os: String,
    pub chrome_version: Option<String>,
    pub chrome_dir: String,
    pub chrome_status: ChromeStatus,
    /// False when no Chrome user-data directory exists at all. The UI shows a
    /// dedicated prerequisite page instead of an empty list or a clean bill.
    pub chrome_detected: bool,
    /// Where we looked for Chrome. Only meaningful when `chrome_detected` is
    /// false; shown inside the collapsed technical details, never as an error.
    pub expected_dir: String,
    pub profiles: Vec<String>,
    /// Display name per profile directory. No e-mails, by design.
    pub profile_labels: HashMap<String, String>,
    pub extensions: Vec<ExtensionInfo>,
    pub total_bytes: u64,
    pub waste_bytes: u64,
    pub baseline_total_bytes: u64,
    pub baseline_waste_bytes: u64,
    /// Running session totals, reported on every scan so the summary cards
    /// have one source of truth instead of relying on values left over in the
    /// DOM. `(bytes_freed, folders_removed)`.
    pub session_freed_bytes: u64,
    pub session_cleaned_count: u64,
}

/// Total bytes under `path`. Symlinks are not followed, errors ignored.
pub(crate) fn dir_size(path: &Path) -> u64 {
    fn walk(path: &Path, total: &mut u64) {
        let Ok(entries) = std::fs::read_dir(path) else {
            return;
        };
        for entry in entries.flatten() {
            let entry_path = entry.path();
            let Ok(meta) = std::fs::symlink_metadata(&entry_path) else {
                continue;
            };
            if meta.file_type().is_symlink() {
                continue;
            }
            if meta.is_dir() {
                walk(&entry_path, total);
            } else {
                *total += meta.len();
            }
        }
    }
    let mut total = 0;
    walk(path, &mut total);
    total
}

fn read_manifest(ver_dir: &Path) -> serde_json::Value {
    std::fs::read_to_string(ver_dir.join("manifest.json"))
        .ok()
        .and_then(|text| serde_json::from_str(&text).ok())
        .unwrap_or(serde_json::Value::Null)
}

/// Pick the most useful icon file (relative path) declared by a manifest.
/// Prefers `icons` (larger sizes win), then `action.default_icon` /
/// `browser_action.default_icon` (string or size map). Returns None when the
/// manifest declares nothing renderable.
fn icon_relative_path(manifest: &serde_json::Value) -> Option<String> {
    // 1. `icons`: { "16": "i16.png", "48": "i48.png", "128": "i128.png" }
    if let Some(icons) = manifest.get("icons").and_then(|v| v.as_object()) {
        let mut best: Option<(u32, String)> = None;
        for (k, v) in icons {
            if let Some(path) = v.as_str() {
                let size = k.parse::<u32>().unwrap_or(0);
                if best.as_ref().map(|(bs, _)| size > *bs).unwrap_or(true) {
                    best = Some((size, path.to_string()));
                }
            }
        }
        if let Some((_, p)) = best {
            return Some(p);
        }
    }
    // 2. action / browser_action default_icon (string or size map).
    let action = manifest
        .get("action")
        .or_else(|| manifest.get("browser_action"));
    if let Some(di) = action.and_then(|v| v.get("default_icon")) {
        if let Some(s) = di.as_str() {
            return Some(s.to_string());
        }
        if let Some(obj) = di.as_object() {
            let mut best: Option<(u32, String)> = None;
            for (k, v) in obj {
                if let Some(path) = v.as_str() {
                    let size = k.parse::<u32>().unwrap_or(0);
                    if best.as_ref().map(|(bs, _)| size > *bs).unwrap_or(true) {
                        best = Some((size, path.to_string()));
                    }
                }
            }
            if let Some((_, p)) = best {
                return Some(p);
            }
        }
    }
    None
}

/// Read an extension version's icon as a `data:` URI, or `""` when absent or
/// unreadable. `ver_dir` is the absolute path to `<ext_id>/<version>_0`.
/// Never leaks the path: the caller only gets encoded image bytes.
pub fn read_extension_icon(ver_dir: &Path) -> String {
    let manifest = read_manifest(ver_dir);
    match icon_relative_path(&manifest) {
        Some(rel) => {
            // Chrome manifests may declare icon paths with a leading "/" to mean
            // "relative to extension root". Rust's Path::join treats a "/"-prefixed
            // segment as absolute and discards `ver_dir`, so strip it first.
            let full = ver_dir.join(rel.trim_start_matches('/'));
            match std::fs::read(&full) {
                Ok(bytes) => format!(
                    "data:image/png;base64,{}",
                    base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes)
                ),
                Err(_) => String::new(),
            }
        }
        None => String::new(),
    }
}

/// Locale fallback chain for extension display names.
///
/// Order: requested locale (full form + base language, both hyphen and
/// underscore spellings) → English → the extension's own `default_locale`.
///
/// English is the universal fallback, so a non-zh user (ko/ja/ru/...) sees the
/// name in their own language when shipped, otherwise in English — *not* in the
/// extension's `default_locale` when that happens to be Chinese. `default_locale`
/// is only the final resort when even English is absent. Chinese is special-cased
/// (zh_CN → zh_TW → zh) because extensions commonly ship those directories.
fn locale_chain(requested: &str, default_locale: Option<&str>) -> Vec<String> {
    let mut chain: Vec<String> = Vec::new();
    let push = |chain: &mut Vec<String>, l: &str| {
        if !l.is_empty() && !chain.iter().any(|x| x == l) {
            chain.push(l.to_string());
        }
    };
    if requested.to_ascii_lowercase().starts_with("zh") {
        // Honor the specific variant: traditional locales (TW/HK/MO) prefer
        // zh_TW, everything else (CN/SG and bare zh) prefers zh_CN. Both then
        // fall back to bare zh, so a zh-CN UI never surfaces zh_TW names first.
        if requested.to_ascii_lowercase().contains("tw")
            || requested.to_ascii_lowercase().contains("hk")
            || requested.to_ascii_lowercase().contains("mo") {
            push(&mut chain, "zh_TW");
        } else {
            push(&mut chain, "zh_CN");
        }
        push(&mut chain, "zh");
    } else {
        let norm = requested.replace('-', "_");
        push(&mut chain, &norm);
        if let Some(base) = norm.split('_').next() {
            push(&mut chain, base);
        }
        push(&mut chain, requested); // original hyphen form (some extensions use it)
    }
    // English is the universal fallback. A non-Chinese UI (ko/ja/ru/...) must
    // see English when its own language is missing — NOT the extension's
    // `default_locale` (which is often `zh_CN` and would otherwise surface
    // Chinese names to every non-zh user). `default_locale` is only the final
    // resort when even English is absent.
    push(&mut chain, "en");
    if let Some(d) = default_locale {
        push(&mut chain, d);
    }
    chain
}

/// Substitute every `__MSG_key__` placeholder using one locale's messages.
/// Returns None when any key is missing, so the caller tries the next locale.
fn substitute_placeholders(s: &str, messages: &serde_json::Value) -> Option<String> {
    if !s.contains("__MSG_") {
        return Some(s.to_string());
    }
    let mut out = String::with_capacity(s.len());
    let mut rest = s;
    while let Some(start) = rest.find("__MSG_") {
        out.push_str(&rest[..start]);
        let after = &rest[start + 6..];
        let Some(end) = after.find("__") else {
            return None; // malformed; let the caller fall back
        };
        // Chrome normalizes placeholder keys to lowercase before lookup
        // (manifest says `__MSG_APP_NAME__` but messages.json uses `app_name`).
        // A case-sensitive lookup would never match and break every extension
        // that mixes cases. Try the lowercase key first (Chrome's rule), then
        // fall back to the original casing for non-conformant extensions.
        let raw_key = &after[..end];
        let lower_key = raw_key.to_ascii_lowercase();
        let message = messages
            .get(&lower_key)
            .or_else(|| messages.get(raw_key))
            .and_then(|m| m.get("message"))
            .and_then(|m| m.as_str())?;
        if message.is_empty() || message.starts_with("__") {
            return None;
        }
        out.push_str(message);
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    Some(out)
}

/// Localize a manifest string (`name`, `default_title`, ...) for `lang`.
fn localize_string(s: &str, ver_dir: &Path, chain: &[String]) -> Option<String> {
    for locale in chain {
        let Ok(text) =
            std::fs::read_to_string(ver_dir.join("_locales").join(locale).join("messages.json"))
        else {
            continue;
        };
        let Ok(messages) = serde_json::from_str::<serde_json::Value>(&text) else {
            continue;
        };
        if !messages.is_object() {
            continue;
        }
        if let Some(localized) = substitute_placeholders(s, &messages) {
            if !localized.is_empty() {
                return Some(localized);
            }
        }
    }
    None
}

fn resolve_display_name(
    ext_id: &str,
    ver_dir: &Path,
    manifest: &serde_json::Value,
    pref_name: Option<&str>,
    lang: &str,
) -> String {
    let default_locale = manifest.get("default_locale").and_then(|v| v.as_str());
    let chain = locale_chain(lang, default_locale);
    // 1. Manifest name in the requested language (English fallback built in).
    if let Some(raw) = manifest.get("name").and_then(|v| v.as_str()) {
        if !raw.is_empty() {
            if let Some(localized) = localize_string(raw, ver_dir, &chain) {
                return localized;
            }
        }
    }
    // 2. Action title, same treatment.
    if let Some(title) = manifest
        .get("action")
        .and_then(|v| v.get("default_title"))
        .and_then(|v| v.as_str())
    {
        if !title.is_empty() {
            if let Some(localized) = localize_string(title, ver_dir, &chain) {
                return localized;
            }
        }
    }
    // 3. Whatever Chrome itself resolved (its own UI language), then raw, then id.
    if let Some(name) = pref_name {
        if !name.is_empty() && !name.starts_with("__MSG") {
            return name.to_string();
        }
    }
    manifest
        .get("name")
        .and_then(|v| v.as_str())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .unwrap_or_else(|| ext_id.to_string())
}

#[allow(clippy::too_many_arguments)]
fn classify_extension(
    scan_id: u64,
    profile: &str,
    ext_id: &str,
    idir: &Path,
    pref: Option<&PrefsEntry>,
    cid_counter: &mut u64,
    snapshot: &mut HashMap<String, CandidateRecord>,
    lang: &str,
) -> Option<ExtensionInfo> {
    let mut known: Vec<(String, ChromeVersion)> = Vec::new();
    let mut unknown: Vec<String> = Vec::new();
    let Ok(children) = std::fs::read_dir(idir) else {
        return None;
    };
    let mut children: Vec<_> = children.flatten().collect();
    children.sort_by_key(|e| e.file_name());
    for child in children {
        let Ok(file_type) = child.file_type() else {
            continue;
        };
        if !file_type.is_dir() {
            continue;
        }
        let dirname = child.file_name().to_string_lossy().into_owned();
        match split_dir_name(&dirname) {
            Some((version, _)) => known.push((dirname, version)),
            None => {
                if !dirname.starts_with('.') {
                    unknown.push(dirname);
                }
            }
        }
    }
    if known.is_empty() && unknown.is_empty() {
        return None;
    }

    // Active version: whatever Chrome recorded, parsed strictly.
    let active_version: Option<ChromeVersion> = pref
        .and_then(|p| p.version.as_deref())
        .and_then(ChromeVersion::parse);
    let mut active_dir: Option<String> = known
        .iter()
        .find(|(_, v)| Some(*v) == active_version)
        .map(|(d, _)| d.clone());
    let mut fallback = false;
    if active_dir.is_none() {
        if let Some((dirname, _)) = known.iter().max_by(|a, b| a.1.cmp(&b.1)) {
            active_dir = Some(dirname.clone());
            fallback = true;
        }
    }
    let active_dir = active_dir?;

    // Cross-checks: own manifest agrees, and prefs `path` (when present)
    // points at the same directory.
    let active_path = idir.join(&active_dir);
    let active_manifest = read_manifest(&active_path);
    let manifest_version = active_manifest
        .get("version")
        .and_then(|v| v.as_str())
        .and_then(ChromeVersion::parse);
    let manifest_agrees = manifest_version == active_version && !fallback;
    let path_agrees = pref
        .and_then(|p| p.disk_path.as_deref())
        .map(|p| {
            Path::new(p)
                .file_name()
                .map(|n| n.to_string_lossy() == active_dir)
                .unwrap_or(false)
        })
        .unwrap_or(true);
    let verified = manifest_agrees && path_agrees && !fallback;

    let standard = pref.map(|p| p.install_source) == Some(InstallSource::WebStoreStandard);
    let active_v = active_version;

    let mut versions: Vec<VersionInfo> = Vec::new();
    let mut ext_total = 0u64;
    for (dirname, version) in &known {
        let full = idir.join(dirname);
        let size = dir_size(&full);
        ext_total += size;
        let is_active = *dirname == active_dir;
        let (deletable, reason) = if is_active {
            (false, "active".to_string())
        } else if !verified {
            (false, "protected:unverified".to_string())
        } else if !standard {
            (false, "protected:non-standard-install".to_string())
        } else if Some(*version) > active_v {
            // Newer than active: possibly staged by Chrome. Never touch.
            (false, "protected:newer-than-active".to_string())
        } else {
            // Strictly older: verify the candidate's own manifest too.
            let own = read_manifest(&full)
                .get("version")
                .and_then(|v| v.as_str())
                .and_then(ChromeVersion::parse);
            if own == Some(*version) {
                (true, "candidate".to_string())
            } else {
                (false, "protected:manifest-mismatch".to_string())
            }
        };
        *cid_counter += 1;
        let cid = format!("s{scan_id}c{cid_counter}");
        versions.push(VersionInfo {
            cid,
            dir: dirname.clone(),
            version: dirname
                .rsplit_once('_')
                .map(|(v, _)| v.to_string())
                .unwrap_or_else(|| dirname.clone()),
            size,
            is_active,
            unknown: false,
            deletable,
            reason,
        });
    }
    for dirname in &unknown {
        let full = idir.join(dirname);
        let size = dir_size(&full);
        ext_total += size;
        *cid_counter += 1;
        versions.push(VersionInfo {
            cid: format!("s{scan_id}c{cid_counter}"),
            dir: dirname.clone(),
            version: dirname.clone(),
            size,
            is_active: true, // Unknown layout: displayed as protected.
            unknown: true,
            deletable: false,
            reason: "protected:unknown-layout".to_string(),
        });
    }
    // Newest version first.
    versions.sort_by(|a, b| {
        let ka = split_dir_name(&a.dir).map(|(v, _)| v);
        let kb = split_dir_name(&b.dir).map(|(v, _)| v);
        kb.cmp(&ka)
    });
    let waste: u64 = versions
        .iter()
        .filter(|v| v.deletable)
        .map(|v| v.size)
        .sum();

    let pref_name = pref.and_then(|p| p.name.as_deref());
    // `name` is resolved in the caller's requested locale (the machine's OS
    // language) and stays the universal fallback. `names` carries the display
    // name in every UI language the app ships, so the frontend can repaint
    // extension names live when the user switches language — no rescan.
    let name = resolve_display_name(ext_id, &active_path, &active_manifest, pref_name, lang);
    // Keys use BCP-47 (hyphen) form to match the frontend's LANGS codes, so
    // x.names[LANG] resolves directly. locale_chain still lowercases and splits
    // on "zh" for resolution, so "zh-CN" resolves the zh_CN _locales dir fine.
    const UI_LOCALES: [&str; 10] = ["en", "zh-CN", "zh-TW", "ko", "ja", "de", "fr", "es", "pt-BR", "ru"];
    let mut names: HashMap<String, String> = HashMap::new();
    for l in UI_LOCALES.iter() {
        names.insert((*l).to_string(),
            resolve_display_name(ext_id, &active_path, &active_manifest, pref_name, l));
    }
    let info = ExtensionInfo {
        profile: profile.to_string(),
        id: ext_id.to_string(),
        name,
        names,
        active_version: active_v
            .map(|_| {
                active_dir
                    .rsplit_once('_')
                    .map(|(v, _)| v.to_string())
                    .unwrap_or(active_dir.clone())
            }),
        verified,
        enabled: pref.map(|p| p.is_enabled()).unwrap_or(true),
        disable_reasons: pref.map(|p| p.disable_reasons.clone()).unwrap_or_default(),
        n_versions: known.len() + unknown.len(),
        total_bytes: ext_total,
        waste_bytes: waste,
        versions,
    };
    // Bind deletable versions into this scan's snapshot (name known now).
    for v in &info.versions {
        if v.deletable {
            snapshot.insert(
                v.cid.clone(),
                CandidateRecord {
                    path: idir.join(&v.dir),
                    size: v.size,
                    profile: info.profile.clone(),
                    ext_id: info.id.clone(),
                    name: info.name.clone(),
                    names: info.names.clone(),
                    active_version: info.active_version.clone(),
                    version: v.version.clone(),
                    dir: v.dir.clone(),
                    reason: v.reason.clone(),
                },
            );
        }
    }
    Some(info)
}

/// Read-only inventory of every profile. Stores a candidate snapshot for
/// later `prepare_cleanup` calls and captures the process baseline once.
pub fn scan(state: &AppState, lang: &str) -> Result<ScanResponse, String> {
    match user_data_dir() {
        Ok(dir) => {
            let mut resp = scan_root(&dir, state, lang)?;
            resp.chrome_detected = true;
            Ok(resp)
        }
        Err(_) => Ok(ScanResponse {
            scan_id: state.next_scan_id(),
            platform: std::env::consts::OS.to_string(),
            os: os_label(),
            chrome_version: None,
            chrome_dir: String::new(),
            chrome_status: ChromeStatus::Unknown,
            chrome_detected: false,
            expected_dir: default_user_data_dir().to_string_lossy().into_owned(),
            profiles: Vec::new(),
            profile_labels: HashMap::new(),
            extensions: Vec::new(),
            total_bytes: 0,
            waste_bytes: 0,
            baseline_total_bytes: 0,
            baseline_waste_bytes: 0,
            session_freed_bytes: state.session_stats().0,
            session_cleaned_count: state.session_stats().1,
        }),
    }
}

/// Same as [`scan`], but against an explicit root (used by fixture tests).
pub fn scan_root(
    user_data_dir: &Path,
    state: &AppState,
    lang: &str,
) -> Result<ScanResponse, String> {
    let profiles = list_profiles(user_data_dir);
    let labels = profile_labels(user_data_dir);
    let status = chrome_status();

    let scan_id = state.next_scan_id();
    let mut cid_counter = 0u64;
    let mut snapshot: HashMap<String, CandidateRecord> = HashMap::new();
    let mut extensions = Vec::new();
    let mut total_all = 0u64;
    let mut total_waste = 0u64;

    for profile in &profiles {
        let prefs = load_secure_prefs(user_data_dir, profile);
        let base = user_data_dir.join(profile).join("Extensions");
        let Ok(entries) = std::fs::read_dir(&base) else {
            continue;
        };
        let mut ids: Vec<String> = entries
            .flatten()
            .filter(|e| {
                e.file_type().map(|t| t.is_dir()).unwrap_or(false)
                    && !e.file_name().to_string_lossy().starts_with('.')
            })
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .collect();
        ids.sort();
        for ext_id in ids {
            let idir = base.join(&ext_id);
            // Chrome components (location 5 / 10) are hidden from the UI — only
            // reached when a real prefs record exists; a missing record is left
            // visible on purpose (fail-safe), never silently hidden.
            if prefs.get(&ext_id).is_some_and(|p| p.is_component()) {
                continue;
            }
            if let Some(info) = classify_extension(
                scan_id,
                profile,
                &ext_id,
                &idir,
                prefs.get(&ext_id),
                &mut cid_counter,
                &mut snapshot,
                lang,
            ) {
                total_all += info
                    .versions
                    .iter()
                    .map(|v| v.size)
                    .sum::<u64>();
                total_waste += info
                    .versions
                    .iter()
                    .filter(|v| v.deletable)
                    .map(|v| v.size)
                    .sum::<u64>();
                extensions.push(info);
            }
        }
    }
    extensions.sort_by_key(|e| std::cmp::Reverse(e.waste_bytes));

    state.store_snapshot(scan_id, snapshot);
    let (baseline_total_bytes, baseline_waste_bytes) =
        state.baseline_or_set(total_all, total_waste);

    Ok(ScanResponse {
        scan_id,
        platform: std::env::consts::OS.to_string(),
        os: os_label(),
        chrome_version: chrome_version(user_data_dir),
        chrome_dir: user_data_dir.to_string_lossy().into_owned(),
        chrome_status: status,
        chrome_detected: true,
        expected_dir: String::new(),
        profiles,
        profile_labels: labels,
        extensions,
        total_bytes: total_all,
        waste_bytes: total_waste,
        baseline_total_bytes,
        baseline_waste_bytes,
        session_freed_bytes: state.session_stats().0,
        session_cleaned_count: state.session_stats().1,
    })
}


#[cfg(test)]
mod real_profile_tests {
    use super::*;

    /// Runs against the developer's real Chrome profile when present.
    /// Verifies structural invariants and prints totals for comparison
    /// with the Python reference implementation.
    ///
    /// Ignored by default: the assertions below encode one specific machine
    /// (a known AdGuard version in the Default profile), so this can never
    /// pass on CI or on another developer's machine. Run it locally with:
    /// `cargo test --manifest-path src-tauri/Cargo.toml -- --ignored --nocapture`
    #[test]
    #[ignore = "needs the developer's real Chrome profile"]
    fn real_scan_invariants() {
        let state = AppState::default();
        let response = match scan(&state, "en") {
            Ok(r) => r,
            Err(e) => {
                println!("SKIPPED (no Chrome profile): {e}");
                return;
            }
        };
        // Same situation as the Err branch above, on a machine where Chrome
        // exists but carries no profile with an Extensions folder (a CI
        // runner, or a fresh install): skip instead of failing an assert.
        if response.profiles.is_empty() {
            println!("SKIPPED (no Chrome profile with extensions)");
            return;
        }
        println!(
            "profiles={} extensions={} total_bytes={} waste_bytes={} chrome={:?} status={:?}",
            response.profiles.len(),
            response.extensions.len(),
            response.total_bytes,
            response.waste_bytes,
            response.chrome_version,
            response.chrome_status,
        );
        assert!(!response.extensions.is_empty());
        assert!(response.total_bytes > 0);

        let json = serde_json::to_string(&response).unwrap();
        // Privacy: no e-mail may leave the backend.
        assert!(!json.contains('@'), "e-mail leaked into scan payload");

        for ext in &response.extensions {
            for v in &ext.versions {
                // Frontend never receives absolute paths.
                assert!(!v.cid.contains('/'), "cid leaks a path: {}", v.cid);
                assert!(!v.cid.contains('\\'), "cid leaks a path: {}", v.cid);
                if v.unknown {
                    assert!(!v.deletable);
                    continue;
                }
                if v.deletable {
                    // Candidates are strictly older than the active version.
                    let active = ext
                        .active_version
                        .as_deref()
                        .and_then(ChromeVersion::parse)
                        .expect("deletable requires a parsed active version");
                    let own = ChromeVersion::parse(&v.version).unwrap();
                    assert!(own < active, "{} {} not older than active", ext.id, v.dir);
                    assert!(!v.is_active);
                }
                if v.is_active {
                    assert!(!v.deletable);
                }
            }
        }

        // Spot check the known worst offender from the reference data.
        if let Some(adguard) = response.extensions.iter().find(|e| {
            e.profile == "Default" && e.id == "bgnkhhnnamicmpeenaelnjfhikgbkllg"
        }) {
            println!(
                "AdGuard: active={:?} verified={} waste_bytes={} n_versions={}",
                adguard.active_version,
                adguard.verified,
                adguard.waste_bytes,
                adguard.n_versions
            );
            assert!(adguard.verified);
            assert_eq!(adguard.active_version.as_deref(), Some("5.5.2.13"));
        }
    }
}

#[cfg(test)]
mod icon_tests {
    use super::*;

    // Minimal 1x1 transparent PNG, base64-encoded (decoded in tests to bytes).
    const PNG_B64: &str = "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mNkYPhfDwAChwGA60e6kgAAAABJRU5ErkJggg==";

    fn png_bytes() -> Vec<u8> {
        base64::Engine::decode(&base64::engine::general_purpose::STANDARD, PNG_B64).unwrap()
    }

    fn mk_ver(manifest: &str, icon_file: Option<&str>) -> std::path::PathBuf {
        let ver = std::env::temp_dir().join(format!("cec_icon_{}_{}", std::process::id(), uuid()));
        std::fs::create_dir_all(&ver).unwrap();
        std::fs::write(ver.join("manifest.json"), manifest).unwrap();
        if let Some(name) = icon_file {
            std::fs::write(ver.join(name), png_bytes()).unwrap();
        }
        ver
    }

    fn uuid() -> u64 {
        use std::sync::atomic::{AtomicU64, Ordering};
        static N: AtomicU64 = AtomicU64::new(0);
        N.fetch_add(1, Ordering::SeqCst)
    }

    #[test]
    fn picks_largest_icon_and_round_trips() {
        let ver = mk_ver(
            r#"{"name":"t","version":"1.0","icons":{"16":"i16.png","48":"i48.png","128":"i128.png"}}"#,
            Some("i128.png"),
        );
        let uri = read_extension_icon(&ver);
        assert!(uri.starts_with("data:image/png;base64,"), "got: {uri}");
        let decoded = base64::Engine::decode(
            &base64::engine::general_purpose::STANDARD,
            uri.trim_start_matches("data:image/png;base64,"),
        )
        .unwrap();
        assert_eq!(decoded, png_bytes());
        let _ = std::fs::remove_dir_all(&ver);
    }

    #[test]
    fn falls_back_to_action_default_icon() {
        let ver = mk_ver(
            r#"{"name":"t","version":"1.0","action":{"default_icon":"action.png"}}"#,
            Some("action.png"),
        );
        let uri = read_extension_icon(&ver);
        assert!(uri.starts_with("data:image/png;base64,"));
        let _ = std::fs::remove_dir_all(&ver);
    }

    #[test]
    fn missing_icon_returns_empty() {
        let ver = mk_ver(r#"{"name":"t","version":"1.0"}"#, None);
        assert_eq!(read_extension_icon(&ver), "");
        let _ = std::fs::remove_dir_all(&ver);
    }

    #[test]
    fn declared_but_absent_icon_file_returns_empty() {
        let ver = mk_ver(
            r#"{"name":"t","version":"1.0","icons":{"128":"gone.png"}}"#,
            None,
        );
        assert_eq!(read_extension_icon(&ver), "");
        let _ = std::fs::remove_dir_all(&ver);
    }

    #[test]
    fn leading_slash_icon_path_is_extension_relative() {
        // Chrome allows "/images/icon-128.png" to mean "relative to extension
        // root". A leading "/" must not be treated as an OS absolute path, or
        // Path::join discards `ver_dir` and the read fails.
        let ver = std::env::temp_dir().join(format!("cec_icon_{}_{}", std::process::id(), uuid()));
        std::fs::create_dir_all(&ver).unwrap();
        std::fs::write(
            ver.join("manifest.json"),
            r#"{"name":"t","version":"1.0","icons":{"128":"/images/icon-128.png"}}"#,
        )
        .unwrap();
        let img_dir = ver.join("images");
        std::fs::create_dir_all(&img_dir).unwrap();
        std::fs::write(img_dir.join("icon-128.png"), png_bytes()).unwrap();
        let uri = read_extension_icon(&ver);
        assert!(uri.starts_with("data:image/png;base64,"), "got: {uri}");
        let _ = std::fs::remove_dir_all(&ver);
    }
}

#[cfg(test)]
mod locale_chain_tests {
    use super::*;

    // Regression: a non-zh UI (ko/ja/ru/...) whose language the extension does
    // not ship must fall back to English, NOT the extension's `default_locale`
    // when that happens to be Chinese. Named after a real-world case where
    // an extension with a Chinese name exposed the bad fallback.
    #[test]
    fn non_zh_falls_back_to_en_not_default_zh() {
        let chain = locale_chain("ko", Some("zh_CN"));
        let en_pos = chain.iter().position(|l| l == "en").unwrap();
        let zh_pos = chain.iter().position(|l| l == "zh_CN").unwrap();
        assert!(en_pos < zh_pos, "en must precede default_locale zh_CN: {chain:?}");
    }

    // A zh UI still prefers Chinese first.
    #[test]
    fn zh_user_still_prefers_chinese() {
        let chain = locale_chain("zh", Some("zh_CN"));
        assert_eq!(chain.first().unwrap(), "zh_CN");
    }

    // Traditional-Chinese UI prefers zh_TW, not zh_CN — the 9-language split
    // (zh-CN vs zh-TW) must resolve the two variants independently.
    #[test]
    fn zh_tw_user_prefers_zh_tw() {
        let chain = locale_chain("zh_TW", Some("zh_CN"));
        assert_eq!(chain.first().unwrap(), "zh_TW");
        let tw = chain.iter().position(|l| l == "zh_TW").unwrap();
        let cn = chain.iter().position(|l| l == "zh_CN").unwrap();
        assert!(tw < cn, "zh_TW must precede zh_CN for a traditional UI: {chain:?}");
    }

    // Portuguese (Brazil) UI: the BCP-47 tag "pt-BR" must normalize to Chrome's
    // `_locales/pt_BR` directory form, so extension names ship in Brazilian
    // Portuguese when the extension provides that locale.
    #[test]
    fn ptbr_user_resolves_ptbr_dir() {
        let chain = locale_chain("pt-BR", Some("en"));
        assert_eq!(chain.first().unwrap(), "pt_BR");
        assert!(chain.iter().any(|l| l == "pt_BR"), "chain must include Chrome's pt_BR dir form: {chain:?}");
    }

    // Even with a non-en default_locale, English must precede it.
    #[test]
    fn en_is_universal_fallback_before_default() {
        let chain = locale_chain("ru", Some("ja"));
        let en_pos = chain.iter().position(|l| l == "en").unwrap();
        let ja_pos = chain.iter().position(|l| l == "ja").unwrap();
        assert!(en_pos < ja_pos, "en must precede default_locale: {chain:?}");
    }

    // Chrome normalizes `__MSG_APP_NAME__` to a lowercase key `app_name` when
    // looking it up in messages.json. A case-sensitive lookup (the old bug)
    // failed to match and fell back to Chrome's pref_name (Chinese for this
    // locale), so e.g. the "Chrome Web Store Payments" extension showed a
    // Chinese name under every non-zh UI.
    #[test]
    fn placeholder_key_is_case_insensitive() {
        let messages = serde_json::json!({"app_name": {"message": "Chrome Web Store Payments"}});
        let got = substitute_placeholders("__MSG_APP_NAME__", &messages);
        assert_eq!(got.as_deref(), Some("Chrome Web Store Payments"));
    }

    #[test]
    fn placeholder_keeps_original_case_when_present() {
        // When the exact-case key exists, it must still be used.
        let messages = serde_json::json!({"APP_NAME": {"message": "ExactCase"}});
        let got = substitute_placeholders("__MSG_APP_NAME__", &messages);
        assert_eq!(got.as_deref(), Some("ExactCase"));
    }
}

/// End-to-end coverage for the Linux prefs layout.
///
/// On Linux, Chrome keeps `extensions.settings` inline in `Preferences` and
/// leaves `Secure Preferences` as a stub (verified on 152.0.7977.8x). Before the
/// fallback existed the registry came back empty, so every extension was
/// `protected:unverified` and nothing could ever be cleaned. This fixture pins
/// the whole chain — prefs -> active version -> deletable candidate.
#[cfg(test)]
mod linux_layout_tests {
    use super::*;

    const EXT_ID: &str = "mcbpblocgmgfnpjjppndjkmgjaogfceg";
    const ACTIVE: &str = "2.1.4.18";

    /// Linux-style `Preferences`: settings inline, no segregation.
    fn prefs_body() -> String {
        format!(
            r#"{{"extensions":{{"settings":{{"{EXT_ID}":{{
                "manifest":{{"name":"FireShot","version":"{ACTIVE}"}},
                "path":"{EXT_ID}/{ACTIVE}_0",
                "from_webstore":true,"location":1}}}}}}}}"#
        )
    }

    fn scaffold() -> std::path::PathBuf {
        let root = std::env::temp_dir().join("cec-scan-linux-layout");
        let _ = std::fs::remove_dir_all(&root);
        let def = root.join("Default");
        std::fs::create_dir_all(def.join("Extensions")).unwrap();
        // The stub Linux Chrome actually writes.
        std::fs::write(
            def.join("Secure Preferences"),
            r#"{"protection":{"super_mac":"DEADBEEF"}}"#,
        )
        .unwrap();
        std::fs::write(def.join("Preferences"), prefs_body()).unwrap();
        root
    }

    fn add_version(root: &Path, version: &str) {
        let dir = root
            .join("Default")
            .join("Extensions")
            .join(EXT_ID)
            .join(format!("{version}_0"));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("manifest.json"),
            format!(
                r#"{{"name":"FireShot","version":"{version}","manifest_version":3,"default_locale":"en"}}"#
            ),
        )
        .unwrap();
    }

    #[test]
    fn older_versions_are_deletable_on_linux_layout() {
        let root = scaffold();
        add_version(&root, ACTIVE);
        add_version(&root, "2.1.4.17");

        let state = crate::state::AppState::default();
        let resp = scan_root(&root, &state, "en").unwrap();

        let ext = resp
            .extensions
            .iter()
            .find(|e| e.id == EXT_ID)
            .unwrap_or_else(|| panic!("extension missing: {:?}", resp.extensions.len()));
        assert_eq!(ext.active_version.as_deref(), Some(ACTIVE));
        assert!(ext.verified, "prefs must resolve on the Linux layout");

        let old = ext
            .versions
            .iter()
            .find(|v| v.version == "2.1.4.17")
            .expect("older version dir");
        assert!(old.deletable, "older version must be cleanable: {old:?}");
        assert_eq!(old.reason, "candidate");

        let active = ext
            .versions
            .iter()
            .find(|v| v.version == ACTIVE)
            .expect("active version dir");
        assert!(!active.deletable && active.is_active);

        let _ = std::fs::remove_dir_all(&root);
    }

    /// No prefs at all (neither file) must stay fail-closed.
    #[test]
    fn missing_prefs_keeps_everything_protected() {
        let root = std::env::temp_dir().join("cec-scan-no-prefs");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("Default").join("Extensions")).unwrap();
        add_version(&root, ACTIVE);
        add_version(&root, "2.1.4.17");

        let state = crate::state::AppState::default();
        let resp = scan_root(&root, &state, "en").unwrap();
        let ext = resp.extensions.iter().find(|e| e.id == EXT_ID).unwrap();
        assert!(!ext.verified);
        assert!(
            ext.versions.iter().all(|v| !v.deletable),
            "without prefs nothing may be deletable: {:?}",
            ext.versions
        );

        let _ = std::fs::remove_dir_all(&root);
    }

    /// Chrome components (location 10, e.g. the Web Store payments extension) must
    /// be skipped entirely — not shown as a protected row. Only applies when a
    /// real prefs record exists; a missing record is left visible (fail-safe).
    #[test]
    fn component_extensions_are_hidden() {
        let root = std::env::temp_dir().join("cec-scan-hidden-component");
        let _ = std::fs::remove_dir_all(&root);
        let def = root.join("Default");
        std::fs::create_dir_all(def.join("Extensions")).unwrap();
        std::fs::write(
            def.join("Secure Preferences"),
            r#"{"protection":{"super_mac":"DEADBEEF"}}"#,
        )
        .unwrap();

        // A normal store extension (location 1) plus a component (location 10),
        // both with a real on-disk version directory.
        let prefs = r#"{"extensions":{"settings":{
            "mcbpblocgmgfnpjjppndjkmgjaogfceg":{"manifest":{"name":"FireShot","version":"2.1.4.18"},
                "path":"mcbpblocgmgfnpjjppndjkmgjaogfceg/2.1.4.18_0","from_webstore":true,"location":1},
            "nmmhkkegccagdldgiimedpiccmgmieda":{"manifest":{"name":"Payments","version":"1.0.0.6"},
                "path":"nmmhkkegccagdldgiimedpiccmgmieda/1.0.0.6_0","from_webstore":true,"location":10}
        }}}"#;
        std::fs::write(def.join("Preferences"), prefs).unwrap();

        for id in [
            "mcbpblocgmgfnpjjppndjkmgjaogfceg",
            "nmmhkkegccagdldgiimedpiccmgmieda",
        ] {
            let ver = if id.starts_with("mcbp") {
                "2.1.4.18"
            } else {
                "1.0.0.6"
            };
            let d = def.join("Extensions").join(id).join(format!("{ver}_0"));
            std::fs::create_dir_all(&d).unwrap();
            std::fs::write(
                d.join("manifest.json"),
                format!(r#"{{"name":"x","version":"{ver}","manifest_version":3}}"#),
            )
            .unwrap();
        }

        let state = crate::state::AppState::default();
        let resp = scan_root(&root, &state, "en").unwrap();
        assert!(
            resp.extensions
                .iter()
                .any(|e| e.id == "mcbpblocgmgfnpjjppndjkmgjaogfceg"),
            "normal store extension must be listed"
        );
        assert!(
            !resp
                .extensions
                .iter()
                .any(|e| e.id == "nmmhkkegccagdldgiimedpiccmgmieda"),
            "component extension must be hidden, not shown as protected"
        );

        let _ = std::fs::remove_dir_all(&root);
    }
}
