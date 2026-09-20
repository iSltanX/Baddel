mod commands;
mod controller;
mod hud;
mod menu_text;
mod settings;
mod shortcuts;
mod sync;
mod sys;
mod tray;
mod windows;

use std::thread;
use std::time::Duration;

use tauri::{Emitter, Manager, RunEvent, WindowEvent};
use tauri_plugin_autostart::MacosLauncher;

use settings::AppState;
use sys::permissions;
use tray::TrayState;

/// How often we look for a change in the Accessibility permission. The user grants
/// it in System Settings, where nothing notifies us.
const PERMISSION_POLL: Duration = Duration::from_secs(2);

pub fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
        .plugin(tauri_plugin_autostart::init(MacosLauncher::LaunchAgent, None::<Vec<&str>>))
        .plugin(tauri_plugin_store::Builder::new().build())
        .plugin(tauri_plugin_process::init())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .manage(TrayState::default())
        .invoke_handler(tauri::generate_handler![
            commands::get_settings,
            commands::set_settings,
            commands::set_shortcut,
            commands::permission_granted,
            commands::request_permission,
            commands::open_keyboard_settings,
            commands::list_layouts,
            commands::keyboard_map,
            commands::convert_text,
            commands::excluded_apps,
            commands::add_excluded_app,
            commands::remove_excluded_app,
            commands::app_version,
            commands::preview_hud,
            commands::open_onboarding,
            commands::finish_onboarding,
            commands::reveal_window,
            commands::set_content_height,
            commands::set_window_title,
            commands::open_external,
        ])
        .setup(|app| {
            // Menu bar utility: no Dock icon, no app menu, until a window opens.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);

            let handle = app.handle().clone();
            let stored = settings::load(&handle);
            *app.state::<AppState>().settings.lock().unwrap() = stored.clone();

            app.manage(controller::spawn(handle.clone()));
            tray::build(&handle)?;
            // Asks macOS to show its permission dialog on first run.
            let mut trusted = permissions::is_trusted(true);
            tray::set_trusted(&handle, trusted);
            shortcuts::apply(&handle, &stored);
            if stored.launch_at_login {
                use tauri_plugin_autostart::ManagerExt;
                let _ = handle.autolaunch().enable();
            }
            if !stored.welcomed {
                windows::open_onboarding(&handle)?;
            }
            #[cfg(debug_assertions)]
            {
                windows::open_requested_window(&handle);
                windows::close_windows_after(&handle);
            }

            let watcher = handle.clone();
            thread::Builder::new().name("baddel-permission".into()).spawn(move || loop {
                thread::sleep(PERMISSION_POLL);
                let now = permissions::is_trusted(false);
                if now != trusted {
                    trusted = now;
                    tray::set_trusted(&watcher, now);
                    let _ = watcher.emit("permission", now);
                }
            })?;

            Ok(())
        })
        .build(tauri::generate_context!())
        .expect("failed to build Baddel");

    app.run(|app, event| match event {
        // The app lives in the menu bar: closing the last window must not quit it.
        // `code` is only set for explicit exits (the Quit item).
        RunEvent::ExitRequested { api, code, .. } if code.is_none() => api.prevent_exit(),
        // Windows are destroyed on close, so nothing of theirs survives; drop the
        // Dock icon again once the last one is gone.
        RunEvent::WindowEvent { event: WindowEvent::Destroyed, .. } => {
            windows::restore_activation_policy(app);
        }
        _ => {}
    });
}
