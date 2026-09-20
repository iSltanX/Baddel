mod controller;
mod state;
mod sys;
mod tray;

use std::thread;
use std::time::Duration;

use tauri::{Emitter, RunEvent};
use tauri_plugin_autostart::MacosLauncher;
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};

use controller::Command;
use state::AppState;
use sys::permissions;

const DEFAULT_SHORTCUT: &str = "Alt+Shift+Space";
/// How often we look for a change in the Accessibility permission.
const PERMISSION_POLL: Duration = Duration::from_secs(2);

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None::<Vec<&str>>))
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .manage(AppState::default())
        .setup(|app| {
            // Menu bar utility: no Dock icon, no app menu.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let commands = controller::spawn(app.handle().clone());
            let tray = tray::build(app, commands.clone())?;
            // Asks macOS to show its permission dialog on first run.
            let mut trusted = permissions::is_trusted(true);
            tray.show_permission(trusted);

            let handle = app.handle().clone();
            thread::Builder::new().name("baddel-permission".into()).spawn(move || loop {
                thread::sleep(PERMISSION_POLL);
                let now = permissions::is_trusted(false);
                if now != trusted {
                    trusted = now;
                    tray.show_permission(now);
                    let _ = handle.emit("permission", now);
                }
            })?;

            // Act on release: by then the hotkey's own keys are on their way up and cannot
            // combine with the keys we synthesize.
            app.global_shortcut().on_shortcut(DEFAULT_SHORTCUT, move |_app, _shortcut, event| {
                if event.state() == ShortcutState::Released {
                    let _ = commands.send(Command::Convert);
                }
            })?;

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
