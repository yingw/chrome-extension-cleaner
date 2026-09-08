//! Cleanup planning and guarded execution.
//!
//! The frontend never sends paths. It sends candidate ids from the current
//! scan; this module resolves them against the stored snapshot, re-verifies
//! everything at delete time, and deletes exactly one directory per call.

use rand::RngCore;
use serde::Serialize;

use crate::chrome::user_data_dir;
use crate::env::{chrome_status, ChromeStatus};
use crate::prefs::load_secure_prefs;
use crate::scan::dir_size;
use crate::state::{AppState, PlanItem};
use crate::version::{split_dir_name, ChromeVersion};

/// A prepared plan: ids the UI shows, paths stay server-side.
#[derive(Debug, Serialize)]
pub struct PrepareResponse {
    pub plan_id: String,
    pub items: Vec<PlanItem>,
    pub total_bytes: u64,
}

/// Outcome of a single deletion.
#[derive(Debug, Serialize)]
pub struct DeleteResult {
    pub cid: String,
    pub ok: bool,
    pub freed_bytes: u64,
    pub session_freed_bytes: u64,
    /// Folders removed this session, including this one when it succeeded.
    pub session_cleaned_count: u64,
    pub chrome_status: ChromeStatus,
    pub error: Option<String>,
}

fn random_plan_id() -> String {
    let mut bytes = [0u8; 16];
    rand::thread_rng().fill_bytes(&mut bytes);
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// Freeze a set of candidate ids into a single-use plan.
/// Every id must come from the current scan's snapshot; anything else
/// (unknown, stale, duplicated) fails the whole call.
pub fn prepare_cleanup(
    state: &AppState,
    candidate_ids: Vec<String>,
) -> Result<PrepareResponse, String> {
    if candidate_ids.is_empty() {
        return Err("no candidates selected".to_string());
    }
    let (scan_id, snapshot) = state
        .snapshot()
        .ok_or_else(|| "scan first; no active snapshot".to_string())?;
    if candidate_ids.len() > snapshot.len() + 1 {
        return Err("candidate list larger than snapshot; refused".to_string());
    }
    let mut seen = std::collections::HashSet::new();
    let mut items = Vec::new();
    let mut total_bytes = 0u64;
    for cid in &candidate_ids {
        if !seen.insert(cid.clone()) {
            return Err(format!("duplicated candidate id: {cid}"));
        }
        let record = snapshot
            .get(cid)
            .ok_or_else(|| format!("unknown or expired candidate: {cid}"))?;
        total_bytes += record.size;
        items.push(PlanItem {
            cid: cid.clone(),
            profile: record.profile.clone(),
            ext_id: record.ext_id.clone(),
            name: record.name.clone(),
            names: record.names.clone(),
            active_version: record.active_version.clone(),
            version: record.version.clone(),
            dir: record.dir.clone(),
            size: record.size,
            reason: record.reason.clone(),
            path: record.path.clone(),
        });
    }
    let plan_id = random_plan_id();
    state.insert_plan(crate::state::CleanupPlan {
        id: plan_id.clone(),
        scan_id,
        items: items.clone(),
        used: std::collections::HashSet::new(),
    });
    Ok(PrepareResponse {
        plan_id,
        items,
        total_bytes,
    })
}

/// Resolve `<chrome>/<profile>/Extensions/<id>` for in-app reveal.
/// Strict shape checks only (32 lowercase letters id, known profile dir,
/// canonicalized containment). Returns a display path without the home dir.
pub fn resolve_extension_dir(
    chrome_dir: &std::path::Path,
    profile: &str,
    ext_id: &str,
) -> Result<(std::path::PathBuf, String), String> {
    if profile.is_empty()
        || profile.contains(['/', '\\', '.', '\0'])
        || ext_id.len() != 32
        || !ext_id.bytes().all(|b| b.is_ascii_lowercase())
    {
        return Err("illegal profile or extension id".to_string());
    }
    let idir = chrome_dir.join(profile).join("Extensions").join(ext_id);
    let canon = idir
        .canonicalize()
        .map_err(|_| "extension directory not found".to_string())?;
    if !canon.is_dir() {
        return Err("extension directory not found".to_string());
    }
    // Must still sit directly inside <chrome>/<profile>/Extensions.
    let parent = canon.parent().ok_or("unresolvable path".to_string())?;
    let expect = chrome_dir.join(profile).join("Extensions");
    let canon_expect = expect
        .canonicalize()
        .map_err(|_| "extensions root not found".to_string())?;
    if parent != canon_expect {
        return Err("path escapes the extensions root".to_string());
    }
    Ok((canon, format!("{profile}/Extensions/{ext_id}/")))
}

/// Canonicalize and prove `target` is exactly
/// `<chrome>/<profile>/Extensions/<id>/<version>_<n>`.
fn contained_version_dir(
    chrome_dir: &std::path::Path,
    profile: &str,
    ext_id: &str,
    target: &std::path::Path,
) -> Result<String, String> {
    let canon_target = target
        .canonicalize()
        .map_err(|e| format!("cannot resolve target: {e}"))?;
    let idir = chrome_dir.join(profile).join("Extensions").join(ext_id);
    let canon_base = idir
        .canonicalize()
        .map_err(|e| format!("cannot resolve extension dir: {e}"))?;
    let rel = canon_target
        .strip_prefix(&canon_base)
        .map_err(|_| "target escapes its extension directory".to_string())?;
    if rel.components().count() != 1 {
        return Err("target is not a direct version child".to_string());
    }
    let dirname = rel
        .file_name()
        .map(|n| n.to_string_lossy().into_owned())
        .unwrap_or_default();
    if split_dir_name(&dirname).is_none() {
        return Err("target is not a version directory".to_string());
    }
    Ok(dirname)
}

/// Delete exactly one planned candidate with full re-verification.
/// Chrome running (or unknown) is reported but does not block, per product
/// decision: the UI warns, the backend still refuses anything unsafe.
pub fn delete_candidate(
    state: &AppState,
    plan_id: &str,
    cid: &str,
) -> Result<DeleteResult, String> {
    let chrome_dir = match user_data_dir() {
        Ok(dir) => dir,
        Err(e) => {
            return Ok(DeleteResult {
                cid: cid.to_string(),
                ok: false,
                freed_bytes: 0,
                session_freed_bytes: state.session_stats().0,
                session_cleaned_count: state.session_stats().1,
                chrome_status: chrome_status(),
                error: Some(e),
            })
        }
    };
    delete_candidate_in(&chrome_dir, state, plan_id, cid)
}

fn delete_candidate_in(
    chrome_dir: &std::path::Path,
    state: &AppState,
    plan_id: &str,
    cid: &str,
) -> Result<DeleteResult, String> {
    let status = chrome_status();
    let fail = |error: String| DeleteResult {
        cid: cid.to_string(),
        ok: false,
        freed_bytes: 0,
        session_freed_bytes: state.add_session_freed(0),
        session_cleaned_count: state.session_stats().1,
        chrome_status: status,
        error: Some(error),
    };

    let item = match state.take_plan_item(plan_id, cid) {
        Ok(item) => item,
        Err(e) => return Ok(fail(e)),
    };

    // Path must still resolve inside its own extension directory.
    let dirname = match contained_version_dir(chrome_dir, &item.profile, &item.ext_id, &item.path)
    {
        Ok(d) => d,
        Err(e) => return Ok(fail(e)),
    };
    // Chrome's record, re-read right now.
    let prefs = load_secure_prefs(chrome_dir, &item.profile);
    let entry = match prefs.get(&item.ext_id) {
        Some(e) => e,
        None => return Ok(fail("extension vanished from Chrome prefs".to_string())),
    };
    let active = match entry.version.as_deref().and_then(ChromeVersion::parse) {
        Some(v) => v,
        None => return Ok(fail("active version unreadable; refused".to_string())),
    };
    let target_v = match split_dir_name(&dirname).map(|(v, _)| v) {
        Some(v) => v,
        None => return Ok(fail("target is not a version directory".to_string())),
    };
    // Plan-time facts must still hold: same active, target strictly older.
    let plan_active = item
        .active_version
        .as_deref()
        .and_then(ChromeVersion::parse);
    if plan_active != Some(active) {
        return Ok(fail(
            "active version changed since scan; rescan required".to_string(),
        ));
    }
    if target_v >= active {
        return Ok(fail("target is not older than active; refused".to_string()));
    }
    // Both manifests must agree with their own directories. The active
    // directory is located by version part, never by assuming a `_0` suffix.
    let idir = chrome_dir
        .join(&item.profile)
        .join("Extensions")
        .join(&item.ext_id);
    let siblings: Vec<String> = std::fs::read_dir(&idir)
        .map(|entries| {
            entries
                .flatten()
                .filter(|e| {
                    e.file_type().map(|t| t.is_dir()).unwrap_or(false)
                        && split_dir_name(&e.file_name().to_string_lossy()).is_some()
                })
                .map(|e| e.file_name().to_string_lossy().into_owned())
                .collect()
        })
        .unwrap_or_default();
    let active_dirname = siblings
        .iter()
        .find(|d| split_dir_name(d).map(|(v, _)| v) == Some(active))
        .cloned();
    let Some(active_dirname) = active_dirname else {
        return Ok(fail("active directory not found on disk".to_string()));
    };
    let manifest_version_of = |dir: &str| {
        std::fs::read_to_string(idir.join(dir).join("manifest.json"))
            .ok()
            .and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok())
            .and_then(|m| m.get("version").and_then(|v| v.as_str()).map(str::to_string))
            .and_then(|s| ChromeVersion::parse(&s))
    };
    if manifest_version_of(&active_dirname) != Some(active) {
        return Ok(fail("active manifest mismatch; refused".to_string()));
    }
    if manifest_version_of(&dirname) != Some(target_v) {
        return Ok(fail("target manifest mismatch; refused".to_string()));
    }
    // Keep at least one version directory per extension.
    if siblings.len() <= 1 || !siblings.contains(&dirname) {
        return Ok(fail("nothing safe to delete; skipped".to_string()));
    }

    let freed = dir_size(&item.path);
    match std::fs::remove_dir_all(&item.path) {
        Ok(()) => {
            let (session_freed, session_cleaned) = state.record_deletion(freed);
            Ok(DeleteResult {
                cid: cid.to_string(),
                ok: true,
                freed_bytes: freed,
                session_freed_bytes: session_freed,
                session_cleaned_count: session_cleaned,
                chrome_status: status,
                error: None,
            })
        }
        Err(e) => Ok(fail(format!("delete failed: {e}"))),
    }
}

#[cfg(test)]
mod fixture_tests {
    use super::*;
    use crate::scan::scan_root;
    use std::path::PathBuf;
    use std::sync::atomic::{AtomicU64, Ordering};

    static COUNTER: AtomicU64 = AtomicU64::new(0);

    fn write_manifest(dir: &std::path::Path, version: &str) {
        std::fs::create_dir_all(dir).unwrap();
        std::fs::write(
            dir.join("manifest.json"),
            format!(r#"{{"name": "t", "version": "{version}"}}"#),
        )
        .unwrap();
        std::fs::write(dir.join("payload.bin"), vec![7u8; 4096]).unwrap();
    }

    /// Fake Chrome root covering every classification branch.
    fn fixture_root() -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "cec-fixture-{}-{}",
            std::process::id(),
            COUNTER.fetch_add(1, Ordering::SeqCst)
        ));
        let _ = std::fs::remove_dir_all(&root);
        let ext_base = root.join("Default/Extensions");
        let mut settings = serde_json::Map::new();

        let mut ext = |id: &str,
                       vers: &[(&str, Option<&str>)],
                       prefs: Option<serde_json::Value>| {
            for (dir, manifest_ver) in vers {
                let d = ext_base.join(id).join(dir);
                match manifest_ver {
                    Some(v) => write_manifest(&d, v),
                    None => {
                        std::fs::create_dir_all(&d).unwrap();
                    }
                }
            }
            if let Some(p) = prefs {
                settings.insert(id.to_string(), p);
            }
        };
        let prefs_entry =
            |version: &str, disabled: bool, standard: bool| {
                serde_json::json!({
                    "manifest": {"name": "t", "version": version},
                    "path": format!("x/{version}_0"),
                    "disable_reasons": if disabled { vec![1] } else { Vec::<i32>::new() },
                    "from_webstore": standard,
                    "location": 1,
                })
            };

        // Normal: 1.0 is a candidate, 2.0 active.
        ext("aaaa", &[("1.0_0", Some("1.0")), ("2.0_0", Some("2.0"))],
            Some(prefs_entry("2.0", false, true)));
        // Staged newer: active 1.0, 2.0 must be protected (not deletable).
        ext("bbbb", &[("1.0_0", Some("1.0")), ("2.0_0", Some("2.0"))],
            Some(prefs_entry("1.0", false, true)));
        // Broken target manifest: 1.0_0 claims 9.9.
        ext("cccc", &[("1.0_0", Some("9.9")), ("2.0_0", Some("2.0"))],
            Some(prefs_entry("2.0", false, true)));
        // No prefs entry at all: unverified, display only.
        ext("dddd", &[("1.0_0", Some("1.0")), ("2.0_0", Some("2.0"))], None);
        // Disabled extension: still cleanable.
        ext("eeee", &[("1.0_0", Some("1.0")), ("3.0_0", Some("3.0"))],
            Some(prefs_entry("3.0", true, true)));
        // Non-standard install: protected.
        ext("ffff", &[("1.0_0", Some("1.0")), ("2.0_0", Some("2.0"))],
            Some(prefs_entry("2.0", false, false)));
        // Unknown layout alongside a real version.
        ext("gggg", &[("Temp", None), ("1.0_0", Some("1.0"))],
            Some(prefs_entry("1.0", false, true)));
        // Single version: nothing to delete.
        ext("hhhh", &[("1.0_0", Some("1.0"))],
            Some(prefs_entry("1.0", false, true)));

        std::fs::create_dir_all(root.join("Default")).unwrap();
        std::fs::write(
            root.join("Default/Secure Preferences"),
            serde_json::Value::Object({
                let mut m = serde_json::Map::new();
                m.insert("extensions".to_string(), serde_json::json!({"settings": settings}));
                m
            })
            .to_string(),
        )
        .unwrap();
        std::fs::write(
            root.join("Local State"),
            r#"{"profile":{"info_cache":{"Default":{"name":"Tester"}}}}"#,
        )
        .unwrap();
        std::fs::write(root.join("Last Version"), "152.0.0.0").unwrap();
        root
    }

    fn scan_ids(state: &AppState, root: &std::path::Path) -> ScanResponseShim {
        let resp = scan_root(root, state, "en").unwrap();
        ScanResponseShim { resp }
    }

    struct ScanResponseShim {
        resp: crate::scan::ScanResponse,
    }

    fn deletable_cids(resp: &crate::scan::ScanResponse) -> Vec<String> {
        resp.extensions
            .iter()
            .flat_map(|e| e.versions.iter())
            .filter(|v| v.deletable)
            .map(|v| v.cid.clone())
            .collect()
    }

    #[test]
    fn classification_branches() {
        let root = fixture_root();
        let state = AppState::default();
        let shim = scan_ids(&state, &root);
        let resp = &shim.resp;

        // Privacy: labels carry names, never e-mails.
        assert_eq!(
            resp.profile_labels.get("Default").map(String::as_str),
            Some("Tester")
        );
        let payload = serde_json::to_string(resp).unwrap();
        assert!(!payload.contains('@'));

        // Only the two genuinely stale versions are candidates.
        let mut got = deletable_cids(resp);
        got.sort();
        assert_eq!(got.len(), 2);
        let by_ext: std::collections::HashMap<_, _> = resp
            .extensions
            .iter()
            .map(|e| {
                (
                    e.id.as_str(),
                    e.versions
                        .iter()
                        .filter(|v| v.deletable)
                        .map(|v| v.version.as_str())
                        .collect::<Vec<_>>(),
                )
            })
            .collect();
        assert_eq!(by_ext["aaaa"], vec!["1.0"]);
        assert_eq!(by_ext["eeee"], vec!["1.0"]);
        assert!(by_ext["bbbb"].is_empty(), "staged 2.0 must be protected");
        assert!(by_ext["cccc"].is_empty(), "manifest mismatch must be protected");
        assert!(by_ext["dddd"].is_empty(), "unverified must be protected");
        assert!(by_ext["ffff"].is_empty(), "non-standard must be protected");
        assert!(by_ext["gggg"].is_empty());
        assert!(by_ext["hhhh"].is_empty());

        // Reasons are explicit for the interesting cases.
        let reasons: std::collections::HashMap<_, _> = resp
            .extensions
            .iter()
            .flat_map(|e| {
                e.versions
                    .iter()
                    .map(move |v| ((e.id.as_str(), v.dir.as_str()), v.reason.as_str()))
            })
            .collect();
        assert_eq!(reasons[&("bbbb", "2.0_0")], "protected:newer-than-active");
        assert_eq!(reasons[&("cccc", "1.0_0")], "protected:manifest-mismatch");
        assert_eq!(reasons[&("ffff", "1.0_0")], "protected:non-standard-install");
        assert_eq!(reasons[&("gggg", "Temp")], "protected:unknown-layout");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn prepare_guards_unknown_dup_and_empty() {
        let root = fixture_root();
        let state = AppState::default();
        let resp = scan_root(&root, &state, "en").unwrap();
        let cids = deletable_cids(&resp);

        assert!(prepare_cleanup(&state, vec![]).is_err());
        assert!(prepare_cleanup(&state, vec!["nope".to_string()]).is_err());
        assert!(prepare_cleanup(&state, vec![cids[0].clone(), cids[0].clone()]).is_err());

        // Happy path returns items without absolute paths on the wire.
        let plan = prepare_cleanup(&state, cids.clone()).unwrap();
        assert_eq!(plan.items.len(), 2);
        assert!(plan.total_bytes > 0);
        let wire = serde_json::to_string(&plan).unwrap();
        assert!(!wire.contains(root.to_string_lossy().as_ref()));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn delete_happy_path_then_replay_refused() {
        let root = fixture_root();
        let state = AppState::default();
        let resp = scan_root(&root, &state, "en").unwrap();
        let cids = deletable_cids(&resp);
        let plan = prepare_cleanup(&state, cids).unwrap();

        let target = plan.items[0].clone();
        let out = delete_candidate_in(&root, &state, &plan.plan_id, &target.cid).unwrap();
        assert!(out.ok, "unexpected: {:?}", out.error);
        assert!(out.freed_bytes > 0);
        assert_eq!(out.session_freed_bytes, out.freed_bytes);
        assert!(!target.path.exists(), "directory must be gone");
        // Active version untouched.
        assert!(root
            .join(format!("Default/Extensions/{}/2.0_0", target.ext_id))
            .exists()
            || root
                .join(format!("Default/Extensions/{}/3.0_0", target.ext_id))
                .exists());

        // Replay of the same candidate is refused.
        let again =
            delete_candidate_in(&root, &state, &plan.plan_id, &target.cid).unwrap();
        assert!(!again.ok);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rescan_invalidates_old_plans() {
        let root = fixture_root();
        let state = AppState::default();
        let resp = scan_root(&root, &state, "en").unwrap();
        let cids = deletable_cids(&resp);
        let plan = prepare_cleanup(&state, cids.clone()).unwrap();

        // A fresh scan kills every outstanding plan.
        let _ = scan_root(&root, &state, "en").unwrap();
        let out =
            delete_candidate_in(&root, &state, &plan.plan_id, &cids[0]).unwrap();
        assert!(!out.ok);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn active_change_between_prepare_and_delete_refuses() {
        let root = fixture_root();
        let state = AppState::default();
        let resp = scan_root(&root, &state, "en").unwrap();
        let cids = deletable_cids(&resp);
        let plan = prepare_cleanup(&state, cids.clone()).unwrap();

        // Simulate Chrome updating between prepare and execute.
        let prefs_path = root.join("Default/Secure Preferences");
        let mut prefs: serde_json::Value =
            serde_json::from_str(&std::fs::read_to_string(&prefs_path).unwrap()).unwrap();
        prefs["extensions"]["settings"]["aaaa"]["manifest"]["version"] = serde_json::json!("9.9");
        std::fs::write(&prefs_path, prefs.to_string()).unwrap();

        let target_cid = plan
            .items
            .iter()
            .find(|i| i.ext_id == "aaaa")
            .unwrap()
            .cid
            .clone();
        let out =
            delete_candidate_in(&root, &state, &plan.plan_id, &target_cid).unwrap();
        assert!(!out.ok, "must refuse when active moved");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn symlink_escape_refused() {
        let root = fixture_root();
        let state = AppState::default();
        let resp = scan_root(&root, &state, "en").unwrap();
        let cids = deletable_cids(&resp);
        let plan = prepare_cleanup(&state, cids).unwrap();
        let target = plan.items[0].clone();

        // Swap the planned directory for a symlink pointing outside.
        let outside = root.join("outside-secret");
        std::fs::create_dir_all(&outside).unwrap();
        std::fs::write(outside.join("x"), b"nope").unwrap();
        std::fs::remove_dir_all(&target.path).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(&outside, &target.path).unwrap();

        let out =
            delete_candidate_in(&root, &state, &plan.plan_id, &target.cid).unwrap();
        assert!(!out.ok, "symlink escape must be refused");
        assert!(outside.join("x").exists(), "outside tree must survive");

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn unknown_plan_and_foreign_paths_refused() {
        let root = fixture_root();
        let state = AppState::default();
        let _ = scan_root(&root, &state, "en").unwrap();
        assert!(state
            .take_plan_item("missing-plan", "s1c1")
            .is_err());
        let _ = std::fs::remove_dir_all(&root);
    }
}
