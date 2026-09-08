//! Tauri entry point: minimal commands, no broad plugin permissions.

mod chrome;
mod clean;
mod env;
mod prefs;
mod scan;
mod state;
mod version;

use tauri::State;
// Native-menu types are macOS-only (the menu block in setup is cfg-gated);
// an unconditional import fails `-D warnings` on Windows/Linux.
#[cfg(target_os = "macos")]
use tauri::menu::{AboutMetadataBuilder, MenuBuilder, PredefinedMenuItem, SubmenuBuilder};

use crate::clean::{DeleteResult, PrepareResponse};
use crate::env::ChromeStatus;
use crate::scan::{read_extension_icon, ScanResponse};
use crate::state::AppState;

/// Read-only inventory. `locale` is the OS language (any BCP-47 tag such as
/// "en", "zh-CN", "ja-JP"); it selects the language used to resolve extension
/// extension display names; anything else falls back to English.
#[tauri::command]
fn scan(state: State<AppState>, locale: Option<String>) -> Result<ScanResponse, String> {
    scan::scan(&state, locale.as_deref().unwrap_or("en"))
}

/// Freeze candidate ids into a single-use plan (no deletion yet).
#[tauri::command]
fn prepare_cleanup(
    state: State<AppState>,
    candidate_ids: Vec<String>,
) -> Result<PrepareResponse, String> {
    clean::prepare_cleanup(&state, candidate_ids)
}

/// Delete exactly one planned candidate with full re-verification.
#[tauri::command]
fn delete_candidate(
    state: State<AppState>,
    plan_id: String,
    cid: String,
) -> Result<DeleteResult, String> {
    clean::delete_candidate(&state, &plan_id, &cid)
}

/// Lightweight Chrome liveness probe (no rescan).
#[tauri::command]
fn chrome_status() -> ChromeStatus {
    env::chrome_status()
}

/// Reveal one extension folder in the system file manager.
/// Path never comes from the frontend: profile + id are validated and
/// re-resolved server-side; only a home-dir-free display path is returned.
#[tauri::command]
fn reveal_candidate_dir(
    app: tauri::AppHandle,
    profile: String,
    ext_id: String,
) -> Result<String, String> {
    use tauri_plugin_opener::OpenerExt;
    let chrome_dir = crate::chrome::user_data_dir()?;
    let (path, display) = clean::resolve_extension_dir(&chrome_dir, &profile, &ext_id)?;
    app.opener()
        .reveal_item_in_dir(path)
        .map_err(|e| format!("cannot reveal folder: {e}"))?;
    Ok(display)
}

/// Read one extension version's icon as a `data:` URI for display in the list.
/// Mirrors `reveal_candidate_dir`'s safety model: the frontend supplies only
/// `profile` + `ext_id` (validated by `resolve_extension_dir`) and a `version_dir`
/// that is a plain directory name produced during scan. The real path is
/// resolved server-side and never returned; a malformed `version_dir` or a
/// missing/unreadable icon yields an empty string (caller shows a placeholder).
#[tauri::command]
fn extension_icon(profile: String, ext_id: String, version_dir: String) -> String {
    if version_dir.contains("..") || version_dir.contains('/') || version_dir.contains('\\') {
        return String::new();
    }
    let chrome_dir = match crate::chrome::user_data_dir() {
        Ok(d) => d,
        Err(_) => return String::new(),
    };
    let (ext_dir, _) = match clean::resolve_extension_dir(&chrome_dir, &profile, &ext_id) {
        Ok(p) => p,
        Err(_) => return String::new(),
    };
    let ver_dir = ext_dir.join(&version_dir);
    if !ver_dir.is_dir() {
        return String::new();
    }
    read_extension_icon(&ver_dir)
}

// 2026-09-08: the `app_meta` command was removed. It existed only to feed the
// build number to the footer version chip; the footer is gone and the header
// badge shows a plain `vX.Y.Z` (no build number by design). CEC_BUILD is still
// injected by build.rs because the macOS About menu shows it.

/// (Re)build the native macOS app menu from frontend-resolved labels.
/// Mirrors RetroPlay's useMenubarI18n: the web i18n owns every language, Rust
/// only renders. Called once at startup and again whenever the in-app
/// language changes, so the menu tracks LANG instead of freezing on the OS
/// language. Off macOS this is a no-op (those platforms show version/build
/// in the footer). Labels arrive pre-localized; the English literals below
/// are unreachable fallbacks, kept English-only by the public-release rule.
fn apply_native_menu<R: tauri::Runtime>(
    app: &tauri::AppHandle<R>,
    about_label: &str,
    quit_label: &str,
    hide_label: &str,
) -> tauri::Result<()> {
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (app, about_label, quit_label, hide_label);
        Ok(())
    }
    #[cfg(target_os = "macos")]
    {
        // The About entry carries the version + build number. On macOS the
        // FIRST top-level item must be a submenu (the app menu); bare
        // About/Quit items make macOS drop the whole app menu. So wrap them
        // in an app-named submenu.
        let about = PredefinedMenuItem::about(
            app,
            Some(about_label),
            Some(
                AboutMetadataBuilder::new()
                    .name(Some("Chrome Extension Cleaner"))
                    .version(Some(app.package_info().version.to_string()))
                    .short_version(Some(format!("build {}", env!("CEC_BUILD"))))
                    // In `tauri dev` there is no .app bundle, so macOS's
                    // applicationIconImage falls back to the default folder
                    // icon. Pin it explicitly from the compile-time embedded
                    // default window icon so dev and packaged builds both
                    // show our icon. `AboutMetadataBuilder::icon` takes
                    // `tauri::image::Image`, which `default_window_icon()`
                    // returns (cloned). Mirrors RetroPlay keeping the About
                    // panel correct via the embedded icon.
                    .icon(app.default_window_icon().cloned())
                    .build(),
            ),
        )?;
        // Pass the full app name explicitly: `SubmenuBuilder::quit()` /
        // `hide()` emit macOS's default text, which appends the process
        // name ("Quit chrome-extension-cleaner"). RetroPlay fixes this the same
        // way — `PredefinedMenuItem::quit(app, Some("Quit RetroPlay"))`.
        let quit = PredefinedMenuItem::quit(app, Some(quit_label))?;
        let base = SubmenuBuilder::new(app, "Chrome Extension Cleaner")
            .item(&about)
            .separator()
            .item(&quit);
        let sub = {
            let hide = PredefinedMenuItem::hide(app, Some(hide_label))?;
            let hide_others = PredefinedMenuItem::hide_others(app, None)?;
            let show_all = PredefinedMenuItem::show_all(app, None)?;
            let services = PredefinedMenuItem::services(app, None)?;
            base.separator()
                .item(&hide)
                .item(&hide_others)
                .separator()
                .item(&show_all)
                .item(&services)
        };
        let app_menu = sub.build()?;
        let menu = MenuBuilder::new(app).items(&[&app_menu]).build()?;
        menu.set_as_app_menu()?;
        Ok(())
    }
}

/// Frontend menu-label sync hook: rebuild the native menu so it tracks the
/// in-app language (labels resolved via web i18n). Off macOS this is a no-op.
#[tauri::command]
fn set_native_menu_labels(
    app: tauri::AppHandle,
    about: String,
    quit: String,
    hide: String,
) -> Result<(), String> {
    apply_native_menu(&app, &about, &quit, &hide).map_err(|e| e.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let builder = tauri::Builder::default()
        .manage(AppState::default())
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            scan,
            prepare_cleanup,
            delete_candidate,
            reveal_candidate_dir,
            extension_icon,
            chrome_status,
            set_native_menu_labels
        ])
        .setup(|app| {
            // Native menu — macOS only. On macOS this becomes the app menu
            // (top of screen). On Windows/Linux we deliberately do NOT attach a
            // native menu: Tauri renders it as an extra in-window GTK/Win32 menu
            // bar that clashes with the app theme (and on Linux it is an
            // unstyleable grey strip). Those platforms show the version in the
            // header badge instead (#verBadge, no build number — the footer chip
            // that used to carry it was deleted on 2026-09-08). The build number
            // is still compiled in (`env!("CEC_BUILD")` from build.rs, i.e. the
            // git commit count) and shown by the macOS About menu.
            // Menu language: English defaults at boot; the frontend re-syncs
            // to the in-app language on boot and on every switch
            // (set_native_menu_labels), so it never freezes on the OS language.
            apply_native_menu(
                app.handle(),
                "About Chrome Extension Cleaner",
                "Quit Chrome Extension Cleaner",
                "Hide Chrome Extension Cleaner",
            )?;
            // Window-chrome fusion (Phase M): conf sets "create": false, so the
            // main window is built here from that same conf. Decorations are fixed
            // at birth (no flash) and the OS marker (`data-os`, consumed by CSS)
            // is injected before first paint via an initialization script —
            // no UA sniffing, no frontend async. macOS keeps the conf Overlay
            // recipe; other platforms drop decorations (Phase W reuses this spot).
            {
                use tauri::WebviewWindowBuilder;
                // Main window by label, not by index: conf order must never
                // decide which window boots (review M2).
                let win_cfg = app
                    .config()
                    .app
                    .windows
                    .iter()
                    .find(|w| w.label == "main")
                    .ok_or("main window missing in tauri.conf.json")?;
                let mut builder = WebviewWindowBuilder::from_config(app.handle(), win_cfg)?;
                #[cfg(not(target_os = "macos"))]
                {
                    builder = builder.decorations(false);
                }
                // OS marker for the CSS `[data-os]` gates. It must be written
                // to `window` only: initialization scripts run at document
                // start, and on Windows (WebView2
                // AddScriptToExecuteOnDocumentCreated) <html> does not exist
                // yet — the previous one-liner set
                // `document.documentElement.dataset.os`, threw a TypeError on
                // Windows that the runtime swallowed, and the self-drawn
                // caption buttons never appeared. index.html copies the value
                // onto <html> (see the inline script in <head>).
                builder = builder.initialization_script(format!(
                    "window.__CEC_OS__='{}';",
                    std::env::consts::OS
                ));
                builder.build()?;
            }
            Ok(())
        });
    // AI testing bridge (DOM inspection, clicks, screenshots for local
    // testing). Compiled in only when the `dev-mcp` feature is enabled AND
    // this is a debug build, so plain `cargo build` and every release
    // artifact never link the plugin. Transport is the plugin default
    // (per-instance IPC socket, see TAURI_MCP_IPC_PATH) — no fixed TCP port.
    // Local usage: `npm run dev:mcp`
    #[cfg(all(debug_assertions, feature = "dev-mcp"))]
    let builder = builder.plugin(tauri_plugin_mcp::init_with_config(
        tauri_plugin_mcp::PluginConfig::new("Chrome Extension Cleaner".to_string()),
    ));
    builder
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
