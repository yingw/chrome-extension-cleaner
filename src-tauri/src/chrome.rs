//! Chrome user-data discovery: profiles and display labels.
//!
//! Only the default Chrome Stable user-data directory is supported.
//! Custom `--user-data-dir`, portable builds, Beta/Dev/Canary and
//! other Chromium browsers are out of scope for v0.2.
//!
//! Privacy: profile labels expose the display name only. Account e-mails
//! from `Local State` are never read into memory beyond this module and
//! never leave the backend.

use std::collections::HashMap;
use std::path::PathBuf;

/// Locate the Chrome Stable user-data directory for this OS.
pub fn user_data_dir() -> Result<PathBuf, String> {
    let dir = default_user_data_dir();
    if dir.is_dir() {
        Ok(dir)
    } else {
        Err(format!("Chrome user-data directory not found: {}", dir.display()))
    }
}

/// The default Chrome Stable user-data directory for this OS, without
/// checking whether it exists. Used to tell the user where we looked when
/// Chrome is not detected.
pub fn default_user_data_dir() -> PathBuf {
    #[cfg(target_os = "macos")]
    let dir = dirs_home().join("Library/Application Support/Google/Chrome");
    #[cfg(target_os = "windows")]
    let dir = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .map(|base| base.join("Google/Chrome/User Data"))
        .unwrap_or_default();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let dir = dirs_home().join(".config/google-chrome");
    dir
}

/// Home directory on the platforms that resolve Chrome through `$HOME`.
/// Windows is excluded on purpose: it uses `%LOCALAPPDATA%` above and would
/// otherwise compile this function as dead code.
#[cfg(not(target_os = "windows"))]
fn dirs_home() -> PathBuf {
    std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default()
}

/// Profile directory names containing an `Extensions` folder, `Default` first.
pub fn list_profiles(user_data_dir: &std::path::Path) -> Vec<String> {
    let mut profiles = Vec::new();
    let Ok(entries) = std::fs::read_dir(user_data_dir) else {
        return profiles;
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if entry.path().join("Extensions").is_dir() {
            profiles.push(name);
        }
    }
    profiles.sort_by_key(|name| (name != "Default", name.clone()));
    profiles
}

/// Display name per profile directory. E-mail addresses are dropped here.
pub fn profile_labels(user_data_dir: &std::path::Path) -> HashMap<String, String> {
    let mut labels = HashMap::new();
    let Ok(text) = std::fs::read_to_string(user_data_dir.join("Local State")) else {
        return labels;
    };
    let Ok(root) = serde_json::from_str::<serde_json::Value>(&text) else {
        return labels;
    };
    if let Some(cache) = root
        .pointer("/profile/info_cache")
        .and_then(|v| v.as_object())
    {
        for (dir, info) in cache {
            let name = info
                .get("name")
                .and_then(|v| v.as_str())
                .filter(|s| !s.is_empty())
                .unwrap_or(dir);
            // NOTE: `user_name` (the account e-mail) is intentionally ignored.
            labels.insert(dir.clone(), name.to_string());
        }
    }
    labels
}
