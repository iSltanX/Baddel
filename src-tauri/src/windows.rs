//! The two webview windows.
//!
//! Both are created when asked for and destroyed when closed, so an idle Baddel
//! runs no webview at all. Anything that must survive a close belongs in
//! [`crate::settings`], not in the page.

use tauri::{AppHandle, Manager, WebviewUrl, WebviewWindowBuilder};

use crate::menu_text;
use crate::settings::AppState;

pub const SETTINGS: &str = "settings";
pub const ONBOARDING: &str = "onboarding";

/// Fixed by the spec: macOS settings windows do not resize horizontally.
const SETTINGS_WIDTH: f64 = 560.0;
/// A starting height; the page measures its pane and asks for the real one.
const SETTINGS_HEIGHT: f64 = 520.0;
const ONBOARDING_SIZE: (f64, f64) = (560.0, 600.0);

/// Debug aid for capturing the documentation screenshots: forces a window's
/// appearance so the dark screens can be taken without switching the whole Mac over.
fn forced_theme() -> Option<tauri::Theme> {
    #[cfg(debug_assertions)]
    match std::env::var("BADDEL_THEME").as_deref() {
        Ok("dark") => return Some(tauri::Theme::Dark),
        Ok("light") => return Some(tauri::Theme::Light),
        _ => {}
    }
    None
}

/// Opens the settings window, or brings it forward if it is already open.
pub fn open_settings(app: &AppHandle) -> tauri::Result<()> {
    open_settings_at(app, None)
}

/// As [`open_settings`], but starting on a named pane instead of the last one used.
pub fn open_settings_at(app: &AppHandle, pane: Option<&str>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(SETTINGS) {
        return focus(&window);
    }
    let language = app.state::<AppState>().get().language;
    let url = match pane {
        Some(pane) => format!("index.html?window=settings&pane={pane}"),
        None => "index.html?window=settings".into(),
    };
    let window = WebviewWindowBuilder::new(app, SETTINGS, WebviewUrl::App(url.into()))
        .title(menu_text::strings(language).settings.trim_end_matches('…'))
        .inner_size(SETTINGS_WIDTH, SETTINGS_HEIGHT)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        .visible(false)
        .theme(forced_theme())
        .build()?;
    focus(&window)
}

/// Opens the welcome window. Its title bar carries no title, so the page draws
/// right up to the top with room left for the traffic lights.
pub fn open_onboarding(app: &AppHandle) -> tauri::Result<()> {
    open_onboarding_at(app, None)
}

/// As [`open_onboarding`], but starting on a given step.
pub fn open_onboarding_at(app: &AppHandle, step: Option<&str>) -> tauri::Result<()> {
    if let Some(window) = app.get_webview_window(ONBOARDING) {
        return focus(&window);
    }
    let url = match step {
        Some(step) => format!("index.html?window=onboarding&step={step}"),
        None => "index.html?window=onboarding".into(),
    };
    let window = WebviewWindowBuilder::new(app, ONBOARDING, WebviewUrl::App(url.into()))
        .title("")
        .inner_size(ONBOARDING_SIZE.0, ONBOARDING_SIZE.1)
        .resizable(false)
        .maximizable(false)
        .minimizable(false)
        // The page runs under the title bar, so the window is one colour top to bottom and
        // exactly the size the design gives it. The strip the page leaves at the top is its
        // drag region, and the traffic lights float over it.
        .title_bar_style(tauri::TitleBarStyle::Overlay)
        .hidden_title(true)
        .visible(false)
        .theme(forced_theme())
        .build()?;
    focus(&window)
}

/// Windows are built hidden so the page can lay itself out before it is seen;
/// the page reveals its own window once it has rendered.
pub fn reveal(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    window.show()?;
    focus(window)
}

fn focus(window: &tauri::WebviewWindow) -> tauri::Result<()> {
    // A menu bar app is an accessory: it has to ask for activation explicitly,
    // or its window opens behind whatever the user was using.
    let _ = window.app_handle().set_activation_policy(tauri::ActivationPolicy::Regular);
    window.set_focus()
}

/// Debug aid for the documentation screenshots: opens a window at launch that the
/// user would normally reach through the menu.
/// `BADDEL_WINDOW=settings[:pane]` or `BADDEL_WINDOW=onboarding[:step]`.
#[cfg(debug_assertions)]
pub fn open_requested_window(app: &AppHandle) {
    let Ok(request) = std::env::var("BADDEL_WINDOW") else { return };
    let (window, at) = match request.split_once(':') {
        Some((window, at)) => (window, Some(at)),
        None => (request.as_str(), None),
    };
    let _ = match window {
        SETTINGS => open_settings_at(app, at),
        ONBOARDING => open_onboarding_at(app, at),
        "hud" => {
            show_notice_repeatedly(app, at.unwrap_or("success").to_string());
            Ok(())
        }
        _ => Ok(()),
    };
}

/// Closes every window after a delay, for the check that no WebKit process is left
/// behind once the settings window is gone.
#[cfg(debug_assertions)]
pub fn close_windows_after(app: &AppHandle) {
    let Ok(seconds) = std::env::var("BADDEL_CLOSE_AFTER") else { return };
    let Ok(seconds) = seconds.parse::<u64>() else { return };
    let app = app.clone();
    std::thread::spawn(move || {
        std::thread::sleep(std::time::Duration::from_secs(seconds));
        for window in app.webview_windows().values() {
            let _ = window.close();
        }
    });
}

/// Keeps one notice on screen for as long as the app runs, so it can be captured.
/// The real notice fades out after about a second by design.
#[cfg(debug_assertions)]
fn show_notice_repeatedly(app: &AppHandle, kind: String) {
    use crate::hud;
    let app = app.clone();
    // `BADDEL_HUD_DELAY=<seconds>` holds the first notice back, so the focus check can put
    // another app in front first: a notice at launch is not what a user ever sees.
    let delay = std::env::var("BADDEL_HUD_DELAY").ok().and_then(|s| s.parse::<u64>().ok());
    std::thread::spawn(move || loop {
        if let Some(seconds) = delay {
            static WAITED: std::sync::Once = std::sync::Once::new();
            WAITED.call_once(|| std::thread::sleep(std::time::Duration::from_secs(seconds)));
        }
        let language = app.state::<AppState>().get().language;
        let rtl = language == crate::settings::Language::Ar;
        let strings = crate::menu_text::strings(language);
        let (kind, text, badge) = match kind.as_str() {
            "undone" => (hud::Kind::Undone, strings.hud_undone.into(), None),
            "blocked" => (hud::Kind::Blocked, strings.hud_blocked.into(), None),
            "too-long" => (hud::Kind::TooLong, strings.hud_too_long.into(), None),
            "no-text" => (hud::Kind::NoText, strings.hud_no_text.into(), None),
            _ => (hud::Kind::Success, hud::conversion_line("اثممخ", "hello", rtl), Some("EN".to_string())),
        };
        hud::show_pinned(&app, hud::Notice { kind, text, badge });
        // The panel cannot be photographed from outside, so it draws itself to file.
        if let Ok(path) = std::env::var("BADDEL_HUD_CAPTURE") {
            std::thread::sleep(std::time::Duration::from_millis(300));
            hud::capture(&app, path);
            app.exit(0);
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(400));
    });
}

/// Back to accessory once no window is left, so no Dock icon lingers.
pub fn restore_activation_policy(app: &AppHandle) {
    let open = app.webview_windows().values().any(|w| w.is_visible().unwrap_or(false));
    if !open {
        let _ = app.set_activation_policy(tauri::ActivationPolicy::Accessory);
    }
}
