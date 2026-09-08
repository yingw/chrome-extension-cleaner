//! Host environment: OS label, installed Chrome version, Chrome liveness.
//!
//! Liveness is tri-state on purpose: a failed check must surface as
//! `Unknown`, never as "not running".

use std::process::Command;

use serde::Serialize;

/// Chrome process state. `Unknown` means the check itself failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum ChromeStatus {
    Running,
    Stopped,
    Unknown,
}

/// Short human-readable OS label, e.g. `macOS 15.7.2`.
pub fn os_label() -> String {
    #[cfg(target_os = "macos")]
    {
        // `sw_vers -productVersion` is the reliable source on macOS.
        let release = Command::new("sw_vers")
            .arg("-productVersion")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "macOS".to_string());
        if release == "macOS" {
            release
        } else {
            format!("macOS {release}")
        }
    }
    #[cfg(target_os = "windows")]
    {
        // `wmic` is deprecated; release number is enough for a label.
        format!("Windows {}", std::env::consts::OS)
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        let release = std::process::Command::new("uname")
            .arg("-r")
            .output()
            .ok()
            .and_then(|o| String::from_utf8(o.stdout).ok())
            .map(|s| s.trim().to_string())
            .unwrap_or_default();
        format!("{} {}", std::env::consts::OS, release).trim().to_string()
    }
}

/// Installed Chrome version from the `Last Version` sidecar file.
pub fn chrome_version(user_data_dir: &std::path::Path) -> Option<String> {
    for filename in ["Last Version", "RunningChromeVersion"] {
        if let Ok(text) = std::fs::read_to_string(user_data_dir.join(filename)) {
            let version = text.trim().to_string();
            if !version.is_empty() {
                return Some(version);
            }
        }
    }
    None
}

fn process_exists(name: &str) -> Option<bool> {
    #[cfg(target_os = "windows")]
    {
        // CREATE_NO_WINDOW: without it every status probe pops a visible
        // console window running tasklist.exe (reported: flashes on Rescan).
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        let output = Command::new("tasklist")
            .args(["/FI", "IMAGENAME eq chrome.exe", "/NH"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            .ok()?;
        let _ = name;
        Some(
            output
                .stdout
                .windows(10)
                .any(|w| w.eq_ignore_ascii_case(b"chrome.exe")),
        )
    }
    #[cfg(not(target_os = "windows"))]
    {
        let output = Command::new("pgrep").arg("-x").arg(name).output().ok()?;
        Some(output.status.success())
    }
}

/// Best-effort liveness check. Never fails closed into `Stopped`.
pub fn chrome_status() -> ChromeStatus {
    #[cfg(target_os = "macos")]
    let candidates = ["Google Chrome", "chrome"];
    #[cfg(target_os = "windows")]
    let candidates = ["chrome"];
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let candidates = ["chrome", "chromium", "google-chrome"];

    let mut saw_error = false;
    for name in candidates {
        match process_exists(name) {
            Some(true) => return ChromeStatus::Running,
            Some(false) => {}
            None => saw_error = true,
        }
    }
    if saw_error {
        ChromeStatus::Unknown
    } else {
        ChromeStatus::Stopped
    }
}
