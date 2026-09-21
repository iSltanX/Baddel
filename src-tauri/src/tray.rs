//! The menu bar icon and its menu.
//!
//! A native `NSMenu`, not a webview: menu bar extras open a menu (HIG · The menu
//! bar), and a popover here would keep a WebKit process alive for nothing. Native
//! menus take no custom views, so the status header and the hint are plain
//! disabled items.

use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::Mutex;

use tauri::image::Image;
use tauri::menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::{TrayIcon, TrayIconBuilder};
use tauri::{AppHandle, Manager, Wry};

use crate::controller::{Command, Commands};
use crate::menu_text;
use crate::settings::AppState;
use crate::sys::permissions;
use crate::{settings, sync, windows};

const TRAY_ID: &str = "main";
const TRAY_ICON: &[u8] = include_bytes!("../icons/tray.png");
const TRAY_ICON_PAUSED: &[u8] = include_bytes!("../icons/tray-paused.png");
const TRAY_ICON_NEEDS_PERMISSION: &[u8] = include_bytes!("../icons/tray-needs-permission.png");

/// Which drawing the menu bar icon shows, so the state reads without opening the menu.
/// All three are template images: black and alpha only, tinted by the system.
#[derive(Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
enum Glyph {
    Ready = 0,
    Paused = 1,
    NeedsPermission = 2,
}

impl Glyph {
    /// The same precedence as the menu's header: a missing permission outranks a pause.
    fn of(trusted: bool, paused: bool) -> Self {
        match (trusted, paused) {
            (false, _) => Self::NeedsPermission,
            (_, true) => Self::Paused,
            _ => Self::Ready,
        }
    }

    fn bytes(self) -> &'static [u8] {
        match self {
            Self::Ready => TRAY_ICON,
            Self::Paused => TRAY_ICON_PAUSED,
            Self::NeedsPermission => TRAY_ICON_NEEDS_PERMISSION,
        }
    }
}

/// What the menu shows beyond the preferences: the permission and the last conversion.
#[derive(Default)]
pub struct TrayState {
    trusted: AtomicBool,
    /// The glyph currently on the menu bar, so the icon is only replaced when it changes.
    glyph: AtomicU8,
    /// The last conversion, for the "اثممخ ← hello · تراجع" item. Held in memory only,
    /// never written anywhere, and cleared as soon as undo is no longer offered.
    last: Mutex<Option<(String, String)>>,
}

pub fn build(app: &AppHandle) -> tauri::Result<TrayIcon> {
    let glyph = current_glyph(app);
    app.state::<TrayState>().glyph.store(glyph as u8, Ordering::SeqCst);
    let tray = TrayIconBuilder::with_id(TRAY_ID)
        .icon(Image::from_bytes(glyph.bytes())?)
        .icon_as_template(true)
        .tooltip(menu_text::strings(app.state::<AppState>().get().language).name)
        .menu(&menu(app)?)
        .on_menu_event(on_event)
        .build(app)?;
    Ok(tray)
}

/// Records the permission state and redraws the menu if it changed.
pub fn set_trusted(app: &AppHandle, trusted: bool) {
    let state = app.state::<TrayState>();
    if state.trusted.swap(trusted, Ordering::SeqCst) != trusted {
        rebuild(app);
    }
}

/// Remembers the last conversion for the menu. `None` clears it.
pub fn set_last_conversion(app: &AppHandle, last: Option<(String, String)>) {
    *app.state::<TrayState>().last.lock().unwrap() = last;
    rebuild(app);
}

/// Rebuilds the menu after anything it displays has changed. Safe to call from any
/// thread: the menu itself is only ever touched on the main one.
pub fn rebuild(app: &AppHandle) {
    let handle = app.clone();
    let _ = app.run_on_main_thread(move || {
        let Some(tray) = handle.tray_by_id(TRAY_ID) else { return };
        if let Ok(menu) = menu(&handle) {
            let _ = tray.set_menu(Some(menu));
        }
        let glyph = current_glyph(&handle);
        if handle.state::<TrayState>().glyph.swap(glyph as u8, Ordering::SeqCst) != glyph as u8 {
            if let Ok(image) = Image::from_bytes(glyph.bytes()) {
                let _ = tray.set_icon(Some(image));
                // Replacing the image drops the template flag, and with it the system tint.
                let _ = tray.set_icon_as_template(true);
            }
        }
        let _ = tray.set_visible(handle.state::<AppState>().get().show_tray_icon);
    });
}

fn current_glyph(app: &AppHandle) -> Glyph {
    let trusted = app.state::<TrayState>().trusted.load(Ordering::SeqCst);
    Glyph::of(trusted, app.state::<AppState>().get().paused)
}

fn menu(app: &AppHandle) -> tauri::Result<Menu<Wry>> {
    let settings = app.state::<AppState>().get();
    let state = app.state::<TrayState>();
    let text = menu_text::strings(settings.language);
    let trusted = state.trusted.load(Ordering::SeqCst);

    let status = match (trusted, settings.paused) {
        (false, _) => text.needs_permission,
        (_, true) => text.paused,
        _ => text.ready,
    };
    // Everything below the header needs both the permission and an unpaused app.
    let live = trusted && !settings.paused;

    let header = item(app, "header", &format!("{} — {status}", text.name), false)?;
    let menu = Menu::new(app)?;
    menu.append(&header)?;
    if trusted {
        menu.append(&item(app, "hint", &hint(app, text.hint), false)?)?;
    } else {
        menu.append(&item(app, "grant", text.grant_permission, true)?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;

    if let Some((from, to)) = state.last.lock().unwrap().clone() {
        let arrow = if settings.language == settings::Language::Ar { '←' } else { '→' };
        menu.append(&item(app, "last", &format!("{from} {arrow} {to}"), false)?)?;
        menu.append(&item(app, "undo", text.undo, live)?)?;
        menu.append(&PredefinedMenuItem::separator(app)?)?;
    }

    let switch = CheckMenuItem::with_id(
        app,
        "switch",
        text.switch_layout,
        live,
        settings.switch_input_source,
        None::<&str>,
    )?;
    menu.append(&switch)?;
    let pause_label = if settings.paused { text.resume } else { text.pause };
    menu.append(&item(app, "pause", pause_label, trusted)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;

    menu.append(&MenuItem::with_id(app, "settings", text.settings, true, Some("Cmd+,"))?)?;
    // Wired up in phase 5, together with the updater plugin and its signing key.
    menu.append(&item(app, "updates", text.check_updates, false)?)?;
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&MenuItem::with_id(app, "quit", text.quit, true, Some("Cmd+Q"))?)?;
    Ok(menu)
}

/// The hint line, with the shortcut the user actually has bound.
fn hint(app: &AppHandle, text: &str) -> String {
    let settings = app.state::<AppState>().get();
    let Some((_, rest)) = text.split_once(" — ") else { return text.to_string() };
    match crate::shortcuts::glyphs(&settings.shortcut_convert) {
        Some(glyphs) => format!("{glyphs} — {rest}"),
        None => rest.to_string(),
    }
}

fn item(app: &AppHandle, id: &str, text: &str, enabled: bool) -> tauri::Result<MenuItem<Wry>> {
    MenuItem::with_id(app, id, text, enabled, None::<&str>)
}

fn on_event(app: &AppHandle, event: tauri::menu::MenuEvent) {
    match event.id().as_ref() {
        "quit" => app.exit(0),
        "undo" => app.state::<Commands>().send(Command::Undo),
        "grant" => {
            permissions::is_trusted(true);
            permissions::open_settings();
        }
        "switch" => {
            let previous = app.state::<AppState>().get();
            let next = settings::update(app, |s| s.switch_input_source = !s.switch_input_source);
            sync::apply(app, &previous, &next);
        }
        "pause" => sync::toggle_pause(app),
        "settings" => {
            let _ = windows::open_settings(app);
        }
        _ => {}
    }
}
