//! Updates: the one place Baddel talks to the network.
//!
//! Releases are signed (the public key is in `tauri.conf.json`), so a download that was
//! not produced by our release script is refused before anything is installed.
//!
//! - **Automatic** (when enabled): a check shortly after launch, then daily. It never
//!   interrupts: a found update becomes a menu item and a line in Settings.
//! - **From the menu**: a native dialog answers — install, up to date, or failed.
//! - **From Settings**: the row itself shows the state, with a button to install.

use std::sync::Mutex;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::settings::{self, AppState};
use crate::sys::{chrome, on_main};
use crate::{menu_text, tray, windows};

/// The first automatic check waits for the app to settle after login.
const FIRST_CHECK_DELAY: Duration = Duration::from_secs(20);
/// How often the automatic check wakes up to see whether a check is due.
const WAKE_INTERVAL: Duration = Duration::from_secs(60 * 60);
/// Automatic checks are at least this far apart, across relaunches too.
const CHECK_INTERVAL_MS: u64 = 24 * 60 * 60 * 1000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trigger {
    Automatic,
    Menu,
    Settings,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Phase {
    #[default]
    Idle,
    Checking,
    UpToDate,
    Available,
    Installing,
    Failed,
}

/// What Settings shows. Sent with the `update` event and by `update_status`.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub phase: Phase,
    /// The version on offer, while `phase` is `Available` or `Installing`.
    pub version: Option<String>,
    /// Milliseconds since the Unix epoch.
    pub last_checked: Option<u64>,
}

#[derive(Default)]
pub struct UpdateState {
    status: Mutex<Status>,
    /// The update found by the last check, ready to install.
    pending: Mutex<Option<Update>>,
}

pub fn status(app: &AppHandle) -> Status {
    let mut status = app.state::<UpdateState>().status.lock().unwrap().clone();
    status.last_checked = status.last_checked.or(app.state::<AppState>().get().last_update_check);
    status
}

/// The version on offer, for the menu.
pub fn available_version(app: &AppHandle) -> Option<String> {
    let status = app.state::<UpdateState>().status.lock().unwrap().clone();
    (status.phase == Phase::Available).then_some(status.version).flatten()
}

/// Starts the automatic checks. They only run while the preference is on.
pub fn start(app: &AppHandle) {
    let app = app.clone();
    let _ = thread::Builder::new().name("baddel-updater".into()).spawn(move || {
        thread::sleep(FIRST_CHECK_DELAY);
        loop {
            let settings = app.state::<AppState>().get();
            let due = settings.last_update_check.is_none_or(|last| now_ms().saturating_sub(last) >= CHECK_INTERVAL_MS);
            if settings.auto_update && due {
                let app = app.clone();
                tauri::async_runtime::spawn(async move { check(&app, Trigger::Automatic).await });
            }
            thread::sleep(WAKE_INTERVAL);
        }
    });
}

/// Checks now, on behalf of `trigger`, without waiting for the answer.
pub fn check_in_background(app: &AppHandle, trigger: Trigger) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move { check(&app, trigger).await });
}

async fn check(app: &AppHandle, trigger: Trigger) {
    {
        let status = app.state::<UpdateState>().status.lock().unwrap().clone();
        if matches!(status.phase, Phase::Checking | Phase::Installing) {
            return;
        }
    }
    set_status(app, Phase::Checking, None);

    let result = match app.updater() {
        Ok(updater) => updater.check().await,
        Err(error) => Err(error),
    };
    let checked = now_ms();
    settings::update(app, |s| s.last_update_check = Some(checked));

    match result {
        Ok(Some(update)) => {
            let version = update.version.clone();
            let current = update.current_version.clone();
            *app.state::<UpdateState>().pending.lock().unwrap() = Some(update);
            set_status(app, Phase::Available, Some(version.clone()));
            if trigger == Trigger::Menu {
                ask_to_install(app, &version, &current);
            }
        }
        Ok(None) => {
            set_status(app, Phase::UpToDate, None);
            if trigger == Trigger::Menu {
                let text = strings(app);
                let current = app.package_info().version.to_string();
                inform(app, MessageDialogKind::Info, text.up_to_date_title, &text.up_to_date_body.replace("{current}", &current));
            }
        }
        Err(_error) => {
            #[cfg(debug_assertions)]
            eprintln!("[baddel] update check failed: {_error}");
            set_status(app, Phase::Failed, None);
            if trigger == Trigger::Menu {
                let text = strings(app);
                inform(app, MessageDialogKind::Warning, text.check_failed_title, text.update_failed_body);
            }
        }
    }
}

/// Downloads and installs the update found by the last check, then relaunches.
pub fn install_in_background(app: &AppHandle) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move { install(&app).await });
}

async fn install(app: &AppHandle) {
    let Some(update) = app.state::<UpdateState>().pending.lock().unwrap().take() else { return };
    set_status(app, Phase::Installing, Some(update.version.clone()));
    match update.download_and_install(|_, _| {}, || {}).await {
        // macOS: the new bundle is in place; it runs from the next launch.
        Ok(()) => app.restart(),
        Err(_error) => {
            #[cfg(debug_assertions)]
            eprintln!("[baddel] update install failed: {_error}");
            let version = update.version.clone();
            *app.state::<UpdateState>().pending.lock().unwrap() = Some(update);
            set_status(app, Phase::Available, Some(version));
            let text = strings(app);
            inform(app, MessageDialogKind::Error, text.install_failed_title, text.update_failed_body);
        }
    }
}

fn ask_to_install(app: &AppHandle, version: &str, current: &str) {
    let text = strings(app);
    let body = text.update_available_body.replace("{version}", version).replace("{current}", current);
    on_main(app, chrome::activate);
    let handle = app.clone();
    app.dialog()
        .message(body)
        .title(text.update_available_title)
        .kind(MessageDialogKind::Info)
        .buttons(MessageDialogButtons::OkCancelCustom(text.install_and_restart.into(), text.later.into()))
        .show(move |install| {
            if install {
                install_in_background(&handle);
            } else {
                step_back(&handle);
            }
        });
}

fn inform(app: &AppHandle, kind: MessageDialogKind, title: &str, body: &str) {
    on_main(app, chrome::activate);
    let handle = app.clone();
    app.dialog().message(body).title(title).kind(kind).show(move |_| step_back(&handle));
}

/// After a dialog: with no window of our own open, hand activation back to the app the
/// user was in.
fn step_back(app: &AppHandle) {
    let open = app.webview_windows().values().any(|w| w.is_visible().unwrap_or(false));
    if !open {
        on_main(app, chrome::resign_activation);
        windows::restore_activation_policy(app);
    }
}

fn set_status(app: &AppHandle, phase: Phase, version: Option<String>) {
    let last_checked = app.state::<AppState>().get().last_update_check;
    {
        let state = app.state::<UpdateState>();
        let mut status = state.status.lock().unwrap();
        status.phase = phase;
        status.version = version;
        status.last_checked = last_checked;
    }
    tray::rebuild(app);
    let _ = app.emit("update", status(app));
}

fn strings(app: &AppHandle) -> &'static menu_text::Strings {
    menu_text::strings(app.state::<AppState>().get().language)
}

fn now_ms() -> u64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |d| d.as_millis() as u64)
}
