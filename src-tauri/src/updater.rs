//! Updates: the one place Baddel talks to the network.
//!
//! Releases are signed (the public key is in `tauri.conf.json`), so a download that was
//! not produced by our release script is refused before anything is installed.
//!
//! - **Automatic** (when enabled): a check shortly after launch, then daily. It never
//!   interrupts: a found update becomes a menu item and a line in Settings.
//! - **From the menu**: a native dialog answers — install, up to date, or failed. Asked while
//!   another check runs, that check's answer is the one the dialog gives (GEMINI-007).
//! - **From Settings**: the row itself shows the state, with a button to install.
//!
//! Installing goes through [`crate::upgrade`]: a copy of the running version is kept until the new
//! one has started, and put back if the swap fails. A failed install says so — in the row, and in
//! a dialog when the menu started it — and the update stays on offer to try again.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager};
use tauri_plugin_dialog::{DialogExt, MessageDialogButtons, MessageDialogKind};
use tauri_plugin_updater::{Update, UpdaterExt};

use crate::settings::{self, AppState};
use crate::sys::{chrome, on_main, system};
use crate::{menu_text, tray, upgrade, windows};

/// The first automatic check waits for the app to settle after login.
const FIRST_CHECK_DELAY: Duration = Duration::from_secs(20);
/// How often the automatic check wakes up to see whether a check is due.
const WAKE_INTERVAL: Duration = Duration::from_secs(60 * 60);
/// Automatic checks are at least this far apart, across relaunches too.
const CHECK_INTERVAL_MS: u64 = 24 * 60 * 60 * 1000;
/// A check that has not answered by then has failed: a stalled connection must not hold the
/// "checking" state, which turns every later check away. (The download has no such limit.)
const CHECK_TIMEOUT: Duration = Duration::from_secs(60);

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

/// What failed, with `Phase::Failed`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Failure {
    /// The check got no answer.
    Check,
    /// The update could not be downloaded, or its signature did not hold. Nothing was changed.
    Download,
    /// The app could not be replaced; the version running is still the one installed.
    Install,
    /// Neither version could be put in place; a copy of the previous one is kept in Baddel's folder.
    Restore,
}

/// What Settings shows. Sent with the `update` event and by `update_status`.
#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub phase: Phase,
    /// The version on offer, while `phase` is `Available` or `Installing` — and with `Failed`, the
    /// version that did not install.
    pub version: Option<String>,
    /// With `Failed`: what failed.
    pub failure: Option<Failure>,
    /// Milliseconds since the Unix epoch.
    pub last_checked: Option<u64>,
}

#[derive(Default)]
pub struct UpdateState {
    status: Mutex<Status>,
    /// The update found by the last check, ready to install.
    pending: Mutex<Option<Update>>,
    /// "Check for Updates…" was chosen in the menu while a check or an install ran: that one's
    /// outcome is told in a dialog when it ends.
    menu_waiting: AtomicBool,
}

pub fn status(app: &AppHandle) -> Status {
    let mut status = app.state::<UpdateState>().status.lock().unwrap().clone();
    status.last_checked = status.last_checked.or(app.state::<AppState>().get().last_update_check);
    status
}

/// The version on offer, for the menu: found by a check, or found and not yet installed.
pub fn available_version(app: &AppHandle) -> Option<String> {
    let state = app.state::<UpdateState>();
    let status = state.status.lock().unwrap().clone();
    let on_offer = status.phase == Phase::Available
        || (status.phase == Phase::Failed && state.pending.lock().unwrap().is_some());
    on_offer.then_some(status.version).flatten()
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
    let state = app.state::<UpdateState>();
    if !claim(&mut state.status.lock().unwrap(), trigger, &state.menu_waiting) {
        return;
    }
    set_status(app, Phase::Checking, None);

    let result = match app.updater_builder().timeout(CHECK_TIMEOUT).build() {
        Ok(updater) => updater.check().await,
        Err(error) => Err(error),
    };
    // Only a check that got an answer counts: a failed one is retried at the next wake-up,
    // not a day later.
    if result.is_ok() {
        let checked = now_ms();
        settings::update(app, |s| s.last_update_check = Some(checked));
    }
    // Asked from the menu, now or while this check ran: the menu's dialog answers.
    let tell = (trigger == Trigger::Menu) | app.state::<UpdateState>().menu_waiting.swap(false, Ordering::SeqCst);

    match result {
        Ok(Some(update)) => {
            let version = update.version.clone();
            let current = update.current_version.clone();
            *app.state::<UpdateState>().pending.lock().unwrap() = Some(update);
            set_status(app, Phase::Available, Some(version.clone()));
            if tell {
                ask_to_install(app, &version, &current);
            }
        }
        Ok(None) => {
            set_status(app, Phase::UpToDate, None);
            if tell {
                let text = strings(app);
                let current = app.package_info().version.to_string();
                inform(app, MessageDialogKind::Info, text.up_to_date_title, &text.up_to_date_body.replace("{current}", &current));
            }
        }
        Err(_error) => {
            #[cfg(debug_assertions)]
            eprintln!("[baddel] update check failed: {_error}");
            set_failed(app, None, Failure::Check);
            if tell {
                let text = strings(app);
                inform(app, MessageDialogKind::Warning, text.check_failed_title, text.update_failed_body);
            }
        }
    }
}

/// Downloads and installs the update found by the last check, then relaunches. `trigger` says
/// where it was asked for: from the menu (or its dialog), a failure is told in a dialog too.
pub fn install_in_background(app: &AppHandle, trigger: Trigger) {
    let app = app.clone();
    tauri::async_runtime::spawn(async move { install(&app, trigger).await });
}

async fn install(app: &AppHandle, trigger: Trigger) {
    let Some(update) = app.state::<UpdateState>().pending.lock().unwrap().take() else { return };
    let version = update.version.clone();
    set_status(app, Phase::Installing, Some(version.clone()));
    match replace(app, &update).await {
        // macOS: the new bundle is in place and checked; it runs from the next launch. The new
        // process starts before this one has gone, so this one lets it be Baddel first.
        Ok(()) => {
            if let Some(held) = app.try_state::<crate::instance::Held>() {
                held.step_aside();
            }
            app.restart()
        }
        Err(failure) => {
            // Still on offer: the previous version runs as before, and the install can be tried again.
            if failure != Failure::Restore {
                *app.state::<UpdateState>().pending.lock().unwrap() = Some(update);
            }
            set_failed(app, Some(version), failure);
            let tell = (trigger == Trigger::Menu) | app.state::<UpdateState>().menu_waiting.swap(false, Ordering::SeqCst);
            if tell {
                let text = strings(app);
                let body = match failure {
                    Failure::Check | Failure::Download => text.update_failed_body,
                    Failure::Install => text.install_failed_body,
                    Failure::Restore => text.restore_failed_body,
                };
                inform(app, MessageDialogKind::Error, text.install_failed_title, body);
            }
        }
    }
}

/// The swap itself, between a copy kept of the running version and a check of the new one.
async fn replace(app: &AppHandle, update: &Update) -> Result<(), Failure> {
    let bytes = update.download(|_, _| {}, || {}).await.map_err(|_error| {
        #[cfg(debug_assertions)]
        eprintln!("[baddel] update download failed: {_error}");
        Failure::Download
    })?;
    let bundle = system::bundle_path()
        .filter(|bundle| !system::is_temporary_location(bundle, &system::mounts()))
        .ok_or(Failure::Install)?;
    let paths = upgrade::Paths::new(&app.path().app_data_dir().map_err(|_| Failure::Install)?);
    let identifier = app.config().identifier.clone();
    let from = app.package_info().version.to_string();
    let _busy = upgrade::exclusive();
    upgrade::prepare(&paths, &bundle, &identifier, &from, &update.version).map_err(|_| Failure::Install)?;
    let _installed = update.install(&bytes);
    #[cfg(debug_assertions)]
    if let Err(error) = &_installed {
        eprintln!("[baddel] update install failed: {error}");
    }
    match upgrade::finish(&paths, &identifier) {
        upgrade::Finished::Installed => Ok(()),
        upgrade::Finished::Failed => Err(Failure::Install),
        upgrade::Finished::Unrecoverable { .. } => Err(Failure::Restore),
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
                install_in_background(&handle, Trigger::Menu);
            } else {
                step_back(&handle);
            }
        });
}

fn inform(app: &AppHandle, kind: MessageDialogKind, title: &str, body: &str) {
    on_main(app, chrome::activate);
    let handle = app.clone();
    // The default button is an English "OK" whatever the interface language.
    let ok = strings(app).ok;
    app.dialog()
        .message(body)
        .title(title)
        .kind(kind)
        .buttons(MessageDialogButtons::OkCustom(ok.into()))
        .show(move |_| step_back(&handle));
}

/// After a dialog: with no window of our own open, hand activation back to the app the
/// user was in.
fn step_back(app: &AppHandle) {
    let handle = app.clone();
    on_main(app, move || windows::step_back(&handle));
}

/// Takes the check for the caller, unless one is running or an update is being installed. The
/// test and the claim are one step under the lock: two checks started together run once. A menu
/// check turned away is not lost: the one running answers it ([`UpdateState::menu_waiting`]).
fn claim(status: &mut Status, trigger: Trigger, menu_waiting: &AtomicBool) -> bool {
    if matches!(status.phase, Phase::Checking | Phase::Installing) {
        if trigger == Trigger::Menu {
            menu_waiting.store(true, Ordering::SeqCst);
        }
        return false;
    }
    status.phase = Phase::Checking;
    true
}

fn set_status(app: &AppHandle, phase: Phase, version: Option<String>) {
    set(app, phase, version, None);
}

fn set_failed(app: &AppHandle, version: Option<String>, failure: Failure) {
    set(app, Phase::Failed, version, Some(failure));
}

fn set(app: &AppHandle, phase: Phase, version: Option<String>, failure: Option<Failure>) {
    let last_checked = app.state::<AppState>().get().last_update_check;
    {
        let state = app.state::<UpdateState>();
        let mut status = state.status.lock().unwrap();
        status.phase = phase;
        status.version = version;
        status.failure = failure;
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_one_check_runs_at_a_time() {
        let waiting = AtomicBool::new(false);
        let mut status = Status::default();
        assert!(claim(&mut status, Trigger::Automatic, &waiting));
        assert_eq!(status.phase, Phase::Checking);
        // BAD-304: the second of two checks started together is turned away.
        assert!(!claim(&mut status, Trigger::Settings, &waiting));
        status.phase = Phase::Installing;
        assert!(!claim(&mut status, Trigger::Automatic, &waiting));
        for phase in [Phase::Idle, Phase::UpToDate, Phase::Available, Phase::Failed] {
            let mut status = Status { phase, ..Status::default() };
            assert!(claim(&mut status, Trigger::Automatic, &waiting));
        }
        // Neither of those was asked for from the menu: nothing waits for a dialog.
        assert!(!waiting.load(Ordering::SeqCst));
    }

    #[test]
    fn a_menu_check_during_another_is_answered_by_that_one() {
        // GEMINI-007: "Check for Updates…" while the automatic check runs was dropped without a word.
        let waiting = AtomicBool::new(false);
        let mut status = Status { phase: Phase::Checking, ..Status::default() };
        assert!(!claim(&mut status, Trigger::Menu, &waiting));
        assert!(waiting.load(Ordering::SeqCst));
        let mut status = Status { phase: Phase::Installing, ..Status::default() };
        let waiting = AtomicBool::new(false);
        assert!(!claim(&mut status, Trigger::Menu, &waiting));
        assert!(waiting.load(Ordering::SeqCst));
    }

    #[test]
    fn a_failed_install_is_failed_with_its_version_not_available() {
        // GEMINI-007: the row went back to "available" and its "failed" text was never shown.
        let status = Status { phase: Phase::Failed, version: Some("2.1.0".into()), failure: Some(Failure::Install), last_checked: None };
        let json = serde_json::to_value(&status).unwrap();
        assert_eq!(json["phase"], "failed");
        assert_eq!(json["failure"], "install");
        assert_eq!(json["version"], "2.1.0");
        let check = serde_json::to_value(Status { phase: Phase::Failed, failure: Some(Failure::Check), ..Status::default() }).unwrap();
        assert_eq!((check["failure"].as_str(), check["version"].is_null()), (Some("check"), true));
    }
}
