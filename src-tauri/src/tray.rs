use std::sync::mpsc::Sender;
use std::sync::Mutex;

use tauri::image::Image;
use tauri::menu::{Menu, MenuItem, PredefinedMenuItem};
use tauri::tray::TrayIconBuilder;
use tauri::{App, Wry};

use crate::controller::Command;
use crate::sys::permissions;

const TRAY_ICON: &[u8] = include_bytes!("../icons/tray.png");

pub struct TrayItems {
    pub status: MenuItem<Wry>,
    pub grant: MenuItem<Wry>,
}

impl TrayItems {
    pub fn show_permission(&self, trusted: bool) {
        let _ = self.status.set_text(if trusted { "بدّل — جاهز" } else { "بدّل — تحتاج صلاحية" });
        let _ = self.grant.set_enabled(!trusted);
    }
}

pub fn build(app: &App, commands: Sender<Command>) -> tauri::Result<TrayItems> {
    // `Sender` is not `Sync`; the menu handler must be.
    let commands = Mutex::new(commands);
    let status = MenuItem::with_id(app, "status", "بدّل", false, None::<&str>)?;
    let hint = MenuItem::with_id(app, "hint", "⌥⇧Space — حوّل التحديد أو آخر كلمة", false, None::<&str>)?;
    let undo = MenuItem::with_id(app, "undo", "تراجع عن آخر تحويل", true, None::<&str>)?;
    let grant = MenuItem::with_id(app, "grant", "امنح الصلاحية…", true, None::<&str>)?;
    let quit = MenuItem::with_id(app, "quit", "إنهاء بدّل", true, Some("Cmd+Q"))?;
    let menu = Menu::with_items(
        app,
        &[&status, &hint, &PredefinedMenuItem::separator(app)?, &undo, &grant, &PredefinedMenuItem::separator(app)?, &quit],
    )?;

    TrayIconBuilder::with_id("main")
        .icon(Image::from_bytes(TRAY_ICON)?)
        .icon_as_template(true)
        .tooltip("بدّل")
        .menu(&menu)
        .on_menu_event(move |app, event| match event.id().as_ref() {
            "quit" => app.exit(0),
            "undo" => {
                let _ = commands.lock().unwrap().send(Command::Undo);
            }
            "grant" => {
                permissions::is_trusted(true);
                permissions::open_settings();
            }
            _ => {}
        })
        .build(app)?;

    Ok(TrayItems { status, grant })
}
