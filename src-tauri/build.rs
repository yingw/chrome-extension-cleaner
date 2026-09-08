/// Embed an auto-incrementing build number so every artifact is traceable
/// without touching the semver stored in the package manifests.
///
/// Resolution order:
///   1. `CEC_BUILD` env var (set by CI, e.g. to the Actions run number) — highest
///      priority so a build can be pinned to an external counter.
///   2. `git rev-list --count HEAD` — stable and reproducible locally, so a
///      `npm run tauri dev` build shows the same number as CI would for that
///      commit. This is the agreed source of truth for the build number.
///   3. `"0"` — fallback when neither is available (e.g. building from a tarball
///      with no git history and no env override).
///
/// The number is consumed at runtime via `env!("CEC_BUILD")` (see lib.rs, the
/// About menu) and is intentionally NOT written into any packaging metadata —
/// semver build metadata is ignored by the updater priority rules and macOS/Windows
/// version fields reject the `+` separator.
fn main() {
    let build = std::env::var("CEC_BUILD")
        .ok()
        .filter(|s| !s.is_empty())
        .or_else(|| {
            std::process::Command::new("git")
                .args(["rev-list", "--count", "HEAD"])
                .output()
                .ok()
                .filter(|o| o.status.success())
                .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
        })
        .unwrap_or_else(|| "0".to_string());

    println!("cargo:rustc-env=CEC_BUILD={build}");
    // Re-run when the env override changes (CI path) or when this script itself
    // changes. Note: a plain new commit does not auto-refresh the number unless
    // a file under src-tauri changes (e.g. a version bump) — acceptable for a
    // dev tool; a clean rebuild always picks up the latest count.
    println!("cargo:rerun-if-env-changed=CEC_BUILD");
    println!("cargo:rerun-if-changed=build.rs");

    tauri_build::build();
}
