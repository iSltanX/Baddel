//! Everything the settings and welcome windows can ask for.
//!
//! The pages hold no state and no logic: they render what these commands return
//! and call back when the user changes something.

use baddel_core::{Layer, LayoutProvider};
use serde::Serialize;
use tauri::{AppHandle, Manager, WebviewWindow};
use tauri_plugin_dialog::DialogExt;

use crate::controller;
use crate::settings::{self, AppState, Binding, Settings};
use crate::sys::app_icons::AppInfo;
use crate::sys::{app_icons, chrome, input_source, on_main, permissions};
use crate::{hud, shortcuts, sync, windows};

/// The three letter rows of a Mac keyboard, by virtual keycode. The keyboard is a
/// real-world object, so this row order is never mirrored for a right-to-left UI.
const MAP_ROWS: [&[u16]; 3] = [
    &[12, 13, 14, 15, 17, 16, 32, 34, 31, 35, 33, 30],
    &[0, 1, 2, 3, 5, 4, 38, 40, 37, 41, 39],
    &[6, 7, 8, 9, 11, 45, 46, 43, 47, 44],
];

// ── Preferences ──────────────────────────────────────────────────────────────

#[tauri::command]
pub fn get_settings(state: tauri::State<'_, AppState>) -> Settings {
    state.get()
}

/// Merges `patch` — an object with any subset of the settings fields — into the
/// current preferences and applies the result.
#[tauri::command]
pub fn set_settings(app: AppHandle, patch: serde_json::Value) -> Result<Settings, String> {
    let previous = app.state::<AppState>().get();
    let mut merged = serde_json::to_value(&previous).map_err(|e| e.to_string())?;
    match (merged.as_object_mut(), patch.as_object()) {
        (Some(base), Some(patch)) => base.extend(patch.clone()),
        _ => return Err("expected an object of settings fields".into()),
    }
    let next: Settings = serde_json::from_value(merged).map_err(|e| e.to_string())?;
    let next = settings::update(&app, |current| *current = next);
    sync::apply(&app, &previous, &next);
    Ok(next)
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutResult {
    pub settings: Settings,
    /// True when macOS would not give us the shortcut: another app already holds it.
    pub conflict: bool,
}

/// Tries to bind `accelerator`, keeping the old one if it is refused. An empty
/// accelerator clears the binding.
#[tauri::command]
pub fn set_shortcut(app: AppHandle, binding: Binding, accelerator: String) -> ShortcutResult {
    let previous = app.state::<AppState>().get();
    if !accelerator.is_empty() && !shortcuts::is_valid(&accelerator) {
        return ShortcutResult { settings: previous, conflict: true };
    }
    let next = settings::update(&app, |s| s.set_binding(binding, accelerator));
    let conflict = sync::apply(&app, &previous, &next).contains(&binding);
    if conflict {
        // Put the working shortcut back rather than leaving the user with none.
        let restored = settings::update(&app, |s| s.set_binding(binding, previous.binding(binding).to_string()));
        sync::apply(&app, &next, &restored);
        return ShortcutResult { settings: restored, conflict: true };
    }
    ShortcutResult { settings: next, conflict: false }
}

// ── Permission ───────────────────────────────────────────────────────────────

#[tauri::command]
pub fn permission_granted() -> bool {
    permissions::is_trusted(false)
}

/// Asks macOS for its own permission dialog, then opens the pane it points to.
#[tauri::command]
pub fn request_permission() {
    permissions::is_trusted(true);
    permissions::open_settings();
}

#[tauri::command]
pub fn open_keyboard_settings() {
    permissions::open_keyboard_settings();
}

// ── Layouts ──────────────────────────────────────────────────────────────────

#[tauri::command]
pub fn list_layouts(app: AppHandle) -> Vec<input_source::Entry> {
    on_main(&app, input_source::list).unwrap_or_default()
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Key {
    pub latin: String,
    pub arabic: String,
    /// One key, more than one Arabic letter — "لا" on Arabic – PC.
    pub multi: bool,
}

#[derive(Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct KeyboardMap {
    /// Empty when no Arabic layout is enabled; the pane then shows its empty state.
    pub rows: Vec<Vec<Key>>,
    pub arabic_name: Option<String>,
    pub latin_name: Option<String>,
    /// Whether this pair has a multi-letter key at all. The note about it, and the
    /// highlight on that key, appear only when it does — the stock macOS "Arabic"
    /// layout has none.
    pub has_multi_char: bool,
}

/// The key map as the two live layouts actually define it. Nothing here is
/// hard-coded: a wrong hand-written table is exactly the bug this avoids.
#[tauri::command]
pub fn keyboard_map(app: AppHandle) -> KeyboardMap {
    let settings = app.state::<AppState>().get();
    let (arabic, latin) = (settings.arabic_layout.clone(), settings.latin_layout.clone());
    let Some((snapshot, names)) =
        on_main(&app, move || (input_source::snapshot(&arabic, &latin), input_source::list()))
    else {
        return KeyboardMap::default();
    };
    let (Some(arabic), Some(latin)) = (&snapshot.arabic, &snapshot.latin) else {
        return KeyboardMap::default();
    };
    let name_of = |id: &str| names.iter().find(|e| e.id == id).map(|e| e.name.clone());

    let mut rows = Vec::with_capacity(MAP_ROWS.len());
    let mut has_multi_char = false;
    for row in MAP_ROWS {
        rows.push(
            row.iter()
                .map(|&keycode| {
                    let letters = arabic.output(keycode, Layer::Base).unwrap_or_default();
                    let multi = letters.chars().count() > 1;
                    has_multi_char |= multi;
                    Key {
                        latin: latin.output(keycode, Layer::Base).unwrap_or_default(),
                        arabic: letters,
                        multi,
                    }
                })
                .collect(),
        );
    }
    KeyboardMap { rows, arabic_name: name_of(&arabic.id), latin_name: name_of(&latin.id), has_multi_char }
}

/// Converts `text` the way the shortcut would, without touching any other app.
/// This is what the practice field in the welcome window uses.
#[tauri::command]
pub fn convert_text(app: AppHandle, text: String) -> Option<String> {
    let settings = app.state::<AppState>().get();
    let (arabic, latin) = (settings.arabic_layout.clone(), settings.latin_layout.clone());
    let snapshot = on_main(&app, move || input_source::snapshot(&arabic, &latin))?;
    controller::build_map(&snapshot).convert(&text).map(|converted| converted.text)
}

// ── Exceptions ───────────────────────────────────────────────────────────────

#[tauri::command]
pub fn excluded_apps(app: AppHandle) -> Vec<AppInfo> {
    let ids = app.state::<AppState>().get().excluded_apps;
    // A default entry for an app that is not installed stays in force (the app may be installed
    // later) but is not listed: a dozen bare identifiers would bury the user's own entries.
    on_main(&app, move || {
        ids.iter()
            .map(|id| app_icons::info(id))
            .filter(|info| info.icon.is_some() || !settings::is_default_excluded(&info.id))
            .collect()
    })
    .unwrap_or_default()
}

#[tauri::command]
pub async fn add_excluded_app(app: AppHandle) -> Result<Vec<AppInfo>, String> {
    let (tx, rx) = std::sync::mpsc::channel();
    app.dialog()
        .file()
        .set_directory("/Applications")
        .add_filter("Applications", &["app"])
        .pick_file(move |picked| {
            let _ = tx.send(picked);
        });
    // The panel answers on the main thread, so this wait cannot happen there.
    let picked = tauri::async_runtime::spawn_blocking(move || rx.recv().ok().flatten())
        .await
        .map_err(|e| e.to_string())?;
    let Some(path) = picked.and_then(|p| p.into_path().ok()) else { return Ok(excluded_apps(app)) };

    let path = path.to_string_lossy().into_owned();
    let Some(info) = on_main(&app, move || app_icons::info_at(&path)).flatten() else {
        return Err("that folder is not an application bundle".into());
    };
    let previous = app.state::<AppState>().get();
    if !previous.excluded_apps.contains(&info.id) {
        let next = settings::update(&app, |s| s.excluded_apps.push(info.id.clone()));
        sync::apply(&app, &previous, &next);
    }
    Ok(excluded_apps(app))
}

#[tauri::command]
pub fn remove_excluded_app(app: AppHandle, id: String) -> Vec<AppInfo> {
    let previous = app.state::<AppState>().get();
    let next = settings::update(&app, |s| s.excluded_apps.retain(|existing| *existing != id));
    sync::apply(&app, &previous, &next);
    excluded_apps(app)
}

// ── Windows and odds and ends ────────────────────────────────────────────────

#[tauri::command]
pub fn app_version(app: AppHandle) -> String {
    app.package_info().version.to_string()
}

/// Lets the user see the notice without converting anything.
#[tauri::command]
pub fn preview_hud(app: AppHandle) {
    let rtl = app.state::<AppState>().get().language == settings::Language::Ar;
    hud::show(
        &app,
        hud::Notice {
            kind: hud::Kind::Success,
            text: hud::conversion_line("اثممخ", "hello", rtl),
            badge: Some("EN".into()),
        },
    );
}

#[tauri::command]
pub fn open_onboarding(app: AppHandle) -> Result<(), String> {
    windows::open_onboarding(&app).map_err(|e| e.to_string())
}

/// Marks the welcome window as done, so it does not come back on the next launch.
#[tauri::command]
pub fn finish_onboarding(app: AppHandle, window: WebviewWindow) {
    let previous = app.state::<AppState>().get();
    let next = settings::update(&app, |s| s.welcomed = true);
    sync::apply(&app, &previous, &next);
    let _ = window.close();
}

/// Called once the page has rendered, so no window is ever shown mid-layout.
#[tauri::command]
pub fn reveal_window(window: WebviewWindow) -> Result<(), String> {
    windows::reveal(&window).map_err(|e| e.to_string())
}

/// macOS settings windows are as tall as the pane they show, so the page reports
/// the height it needs whenever the pane changes.
#[tauri::command]
pub fn set_content_height(app: AppHandle, window: WebviewWindow, height: f64) -> Result<(), String> {
    let mut size = window.inner_size().map_err(|e| e.to_string())?;
    let scale = window.scale_factor().map_err(|e| e.to_string())?;
    // The size Tauri reports and accepts here covers the title bar as well, so the
    // page's own height is not the whole story.
    let ns_window = window.ns_window().map_err(|e| e.to_string())? as usize;
    let chrome = on_main(&app, move || chrome::title_bar_height(ns_window as *mut _)).unwrap_or(0.0);
    let wanted = ((height + chrome) * scale).round() as u32;
    if size.height == wanted {
        return Ok(());
    }
    size.height = wanted;
    // A window built non-resizable carries fixed size constraints, and macOS holds
    // the app to them too. Lift them for the moment it takes to resize the window.
    let _ = window.set_resizable(true);
    let result = window.set_size(size).map_err(|e| e.to_string());
    let _ = window.set_resizable(false);
    result
}

/// The window title follows the selected pane (HIG · Settings).
#[tauri::command]
pub fn set_window_title(window: WebviewWindow, title: String) -> Result<(), String> {
    window.set_title(&title).map_err(|e| e.to_string())
}

/// Opens a link in the user's browser. The app itself never makes a network request.
#[tauri::command]
pub fn open_external(url: String) -> Result<(), String> {
    // Only ever called with the links built into the About pane.
    if !url.starts_with("https://") {
        return Err("only https links can be opened".into());
    }
    std::process::Command::new("/usr/bin/open").arg(url).spawn().map(|_| ()).map_err(|e| e.to_string())
}
