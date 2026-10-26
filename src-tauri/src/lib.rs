#![deny(unsafe_code)]

mod commands;
mod controller;
mod diagnostics;
mod hud;
mod instance;
mod maintenance;
mod menu_text;
mod report;
mod settings;
mod shortcuts;
mod startup;
mod sync;
// All `unsafe` lives in `sys`, behind safe functions; the compiler holds the rest to that.
#[allow(unsafe_code)]
mod sys;
mod tray;
mod updater;
mod upgrade;
mod windows;

use std::path::PathBuf;
use std::thread;
use std::time::{Duration, Instant};

use tauri::{Emitter, Manager, RunEvent, WindowEvent};

use instance::{Claim, Request};
use settings::AppState;
use sys::permissions;
use tray::TrayState;

/// How often we look for a change in the Accessibility permission. The user grants
/// it in System Settings, where nothing notifies us.
const PERMISSION_POLL: Duration = Duration::from_secs(2);

/// After a launch at login, how long a request to open Baddel is taken for the same login (a
/// second login mechanism, or an older one not yet moved over) rather than for the user.
const LOGIN_QUIET: Duration = Duration::from_secs(10);

/// How this run of Baddel began, for telling a request to show Settings from a second launch at
/// login that only happens to look like one.
pub struct Launch {
    /// Until when requests to open Baddel are taken for that same login and show nothing.
    quiet_until: Option<Instant>,
    /// Why this run has no single-instance lock, when it has none (see `run`).
    pub unguarded: Option<String>,
}

impl Launch {
    fn quiet(&self) -> bool {
        self.quiet_until.is_some_and(|until| Instant::now() < until)
    }
}

/// What a request to open Baddel again shows: Settings, unless it is part of a launch at login.
fn reopened(app: &tauri::AppHandle, request: Request) {
    if request == Request::Reopen && !app.state::<Launch>().quiet() {
        let _ = windows::open_settings(app);
    }
}

fn home() -> Option<PathBuf> {
    std::env::var_os("HOME").filter(|home| !home.is_empty()).map(PathBuf::from)
}

pub fn run() {
    let context = tauri::generate_context!();
    // First of all, before the menu bar icon, the shortcuts, the login item or any setting is
    // touched: one Baddel per user. A second launch hands its request over and ends here.
    let identifier = context.config().identifier.clone();
    let (owner, unguarded) =
        match home().map(|home| instance::claim(&instance::folder(&home, &identifier), Request::of_this_launch())) {
            Some(Ok(Claim::Running { delivered })) => {
                #[cfg(debug_assertions)]
                eprintln!("[baddel] another Baddel runs; request handed over: {delivered}");
                let _ = delivered;
                return;
            }
            Some(Ok(Claim::Owner(owner))) => (Some(owner), None),
            // No home folder, or no way to the lock in it: run unguarded, as before 2.1, and say so
            // in maintenance.json rather than not at all.
            Some(Err(error)) => (None, Some(error.to_string())),
            None => (None, Some("no home folder".into())),
        };

    let mut app = tauri::Builder::default()
        .plugin(tauri_plugin_global_shortcut::Builder::new().build())
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
            commands::startup_status,
            commands::set_launch_at_login,
            commands::open_login_items,
            commands::restore_default_shortcuts,
            commands::set_shortcut,
            commands::suspend_shortcuts,
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
        .setup(move |app| {
            let handle = app.handle().clone();
            // Started at login by an older LaunchAgent, or by macOS as a login item: a request to
            // open Baddel in the next few seconds is that same login, not the user.
            let at_login = Request::of_this_launch() == Request::Background || sys::system::launched_as_login_item();
            app.manage(Launch { quiet_until: at_login.then(|| Instant::now() + LOGIN_QUIET), unguarded });

            let (mut stored, source) = settings::load(&handle);
            // A timed pause that ran out while Baddel was not running is over.
            if stored.pause_expired(sys::clock::now_ms()) {
                stored.set_paused(false, None);
                settings::save(&handle, &stored);
            }
            *app.state::<AppState>().settings.lock().unwrap() = stored.clone();

            // Menu bar utility: no Dock icon and no app menu, windows or not (the policy itself is
            // set before the event loop starts, below). tao asks for activation as the app finishes
            // launching; with no window to show, give it back to the app the user is in. The
            // welcome window, opening now, keeps it: the user just opened Baddel for it.
            #[cfg(target_os = "macos")]
            if stored.welcomed && !windows::opens_at_launch() {
                sys::chrome::resign_activation();
            }

            app.manage(controller::spawn(handle.clone()));
            tray::build(&handle)?;
            hud::prime(&handle);
            // Checked without asking: the welcome window explains the permission and asks for it
            // when the user gets there, and the menu offers it after that (GEMINI-006).
            let mut trusted = permissions::is_trusted(false);
            tray::set_trusted(&handle, trusted);
            shortcuts::apply(&handle, &stored);

            app.manage(startup::Startup::new(startup_system(), home(), identifier.clone()));
            // The login item and what an older Baddel or an update left behind: off the main thread,
            // since they wait on launchd, ServiceManagement and the disk.
            let cleaner = handle.clone();
            thread::Builder::new().name("baddel-maintenance".into()).spawn(move || {
                maintenance::at_launch(&cleaner, stored.launch_at_login, source);
            })?;

            updater::start(&handle);
            if !stored.welcomed {
                windows::open_onboarding(&handle)?;
            }
            #[cfg(debug_assertions)]
            {
                windows::open_requested_window(&handle);
                windows::close_windows_after(&handle);
                acceptance(&handle);
            }

            if let Some(owner) = owner {
                let reopen = handle.clone();
                app.manage(owner.serve(move |request| {
                    let app = reopen.clone();
                    let _ = reopen.run_on_main_thread(move || reopened(&app, request));
                }));
            }

            let watcher = handle.clone();
            thread::Builder::new().name("baddel-permission".into()).spawn(move || loop {
                thread::sleep(PERMISSION_POLL);
                // The end of a timed pause rides on the same clock: nothing notifies us of it either.
                sync::resume_if_expired(&watcher);
                let now = permissions::is_trusted(false);
                if now != trusted {
                    trusted = now;
                    tray::set_trusted(&watcher, now);
                    let _ = watcher.emit("permission", now);
                }
            })?;

            Ok(())
        })
        .build(context)
        .expect("failed to build Baddel");
    // Set before the event loop starts, so the app is an accessory from its first moment: tao would
    // otherwise make it a regular app as it finishes launching, with a Dock icon until setup ran.
    #[cfg(target_os = "macos")]
    app.set_activation_policy(tauri::ActivationPolicy::Accessory);

    app.run(|app, event| match event {
        // The app lives in the menu bar: closing the last window must not quit it.
        // `code` is only set for explicit exits (the Quit item).
        RunEvent::ExitRequested { api, code, .. } if code.is_none() => api.prevent_exit(),
        // Opening Baddel again from Applications (or Spotlight) while it runs is the way back
        // to its settings when the menu bar icon is hidden. With a window already up, macOS
        // brings that one forward on its own.
        #[cfg(target_os = "macos")]
        RunEvent::Reopen { has_visible_windows: false, .. } => reopened(app, Request::Reopen),
        RunEvent::Exit => {
            if let Some(held) = app.try_state::<instance::Held>() {
                held.release();
            }
        }
        // Windows are destroyed on close, so nothing of theirs survives; once the last one is
        // gone, the keyboard goes back to the app the user was in.
        RunEvent::WindowEvent { label, event: WindowEvent::Destroyed, .. } => {
            // A report not sent is dropped with its window: image, preview and all.
            if label == windows::REPORT {
                app.state::<report::ReportState>().0.lock().unwrap().clear();
            }
            // Closed while the shortcut recorder was listening: bind the shortcuts again.
            if label == windows::SETTINGS {
                shortcuts::suspend(app, false);
            }
            windows::step_back(app);
        }
        _ => {}
    });
}

/// Debug builds only, for `audit/remediation-2026-10/acceptance/`: a step a person would take in
/// Settings, taken at launch instead, through the same code. `BADDEL_ACCEPT=login-on` or
/// `login-off` flips the launch-at-login switch; `update` checks for an update and installs it.
#[cfg(debug_assertions)]
fn acceptance(app: &tauri::AppHandle) {
    let Ok(step) = std::env::var("BADDEL_ACCEPT") else { return };
    let app = app.clone();
    thread::spawn(move || {
        // After the launch's own work on the login item (the maintenance thread).
        thread::sleep(Duration::from_secs(2));
        match step.as_str() {
            "login-on" | "login-off" => {
                let status = commands::set_launch_at_login(app.clone(), step == "login-on");
                eprintln!("[baddel] acceptance {step}: {status:?}");
            }
            "update" => updater::accept_update(&app),
            _ => eprintln!("[baddel] acceptance: unknown step {step}"),
        }
    });
}

/// macOS, or in a debug build `BADDEL_LOGIN_ITEM_FAKE` standing in for it (see [`startup::fake`]).
fn startup_system() -> Box<dyn startup::System> {
    #[cfg(debug_assertions)]
    if let Some(file) = std::env::var_os("BADDEL_LOGIN_ITEM_FAKE") {
        return Box::new(startup::fake::FileBacked(file.into()));
    }
    Box::new(startup::MacOs)
}
