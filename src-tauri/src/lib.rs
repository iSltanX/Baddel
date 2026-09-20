use tauri::{
    image::Image,
    menu::{Menu, MenuItem},
    tray::TrayIconBuilder,
    RunEvent,
};
use tauri_plugin_autostart::MacosLauncher;

const TRAY_ICON: &[u8] = include_bytes!("../icons/tray.png");

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(
            MacosLauncher::LaunchAgent,
            None::<Vec<&str>>,
        ))
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .setup(|app| {
            // Menu bar utility: no Dock icon, no app menu.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let quit = MenuItem::with_id(app, "quit", "إنهاء بدّل", true, Some("Cmd+Q"))?;
            let menu = Menu::with_items(app, &[&quit])?;

            TrayIconBuilder::with_id("main")
                .icon(Image::from_bytes(TRAY_ICON)?)
                .icon_as_template(true)
                .tooltip("بدّل")
                .menu(&menu)
                .on_menu_event(|app, event| {
                    if event.id() == "quit" {
                        app.exit(0);
                    }
                })
                .build(app)?;

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to build Baddel");

    app.run(|_app, event| {
        // The app lives in the menu bar: closing the last window must not quit it.
        // `code` is only set for explicit exits (the Quit item).
        if let RunEvent::ExitRequested { api, code, .. } = event {
            if code.is_none() {
                api.prevent_exit();
            }
        }
    });
}
