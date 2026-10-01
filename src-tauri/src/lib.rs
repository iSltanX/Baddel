mod commands;
mod controller;
mod diagnostics;
mod hud;
mod menu_text;
mod report;
mod settings;
mod shortcuts;
mod sync;
mod sys;
mod tray;
mod updater;
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
        .plugin(tauri_plugin_updater::Builder::new().build())
        .manage(AppState::default())
        .manage(TrayState::default())
        .manage(updater::UpdateState::default())
        .manage(diagnostics::History::default())
        .manage(report::ReportState::default())
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
            commands::update_status,
            commands::check_for_updates,
            commands::install_update,
            commands::preview_hud,
            commands::open_onboarding,
            commands::finish_onboarding,
            commands::app_needs_move,
            commands::reveal_app_in_finder,
            commands::reveal_window,
            commands::set_content_height,
            commands::set_window_title,
            commands::open_external,
            commands::copy_diagnostics,
            commands::open_report,
            commands::report_pick_image,
            commands::report_paste_image,
            commands::report_clear_image,
            commands::report_preview,
            commands::report_send,
            commands::report_copy,
            commands::report_copy_number,
        ])
        .setup(|app| {
            // Menu bar utility: no Dock icon, no app menu, until a window opens.
            #[cfg(target_os = "macos")]
            app.set_activation_policy(tauri::ActivationPolicy::Accessory);
            #[cfg(target_os = "macos")]
            sys::chrome::resign_activation();

            let handle = app.handle().clone();
            let stored = settings::load(&handle);
            *app.state::<AppState>().settings.lock().unwrap() = stored.clone();

            app.manage(controller::spawn(handle.clone()));
            tray::build(&handle)?;
            hud::prime(&handle);
            // Asks macOS to show its permission dialog on first run.
            let mut trusted = permissions::is_trusted(true);
            tray::set_trusted(&handle, trusted);
            shortcuts::apply(&handle, &stored);
            if stored.launch_at_login {
                use tauri_plugin_autostart::ManagerExt;
                let _ = handle.autolaunch().enable();
            }
            updater::start(&handle);
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
        // Opening Baddel again from Applications (or Spotlight) while it runs is the way back
        // to its settings when the menu bar icon is hidden. With a window already up (the Dock
        // icon is only there while one is), macOS brings that one forward on its own.
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { has_visible_windows: false, .. } => {
            let _ = windows::open_settings(app);
        }
        // Windows are destroyed on close, so nothing of theirs survives; drop the
        // Dock icon again once the last one is gone.
        RunEvent::WindowEvent { label, event: WindowEvent::Destroyed, .. } => {
            // A report not sent is dropped with its window: image, preview and all.
            if label == windows::REPORT {
                app.state::<report::ReportState>().0.lock().unwrap().clear();
            }
            windows::restore_activation_policy(app);
        }
        _ => {}
    });
}
