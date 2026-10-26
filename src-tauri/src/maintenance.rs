//! What runs once at launch, off the main thread, after the app is up: the login item, an
//! earlier update's copy of the previous version, and the updater's leftovers from an install that
//! was cut short ([`crate::startup`], [`crate::upgrade`]).
//!
//! Each step looks at things as they are now rather than at a record of what was done before, so
//! running it again — after a failure, an interruption, or with nothing left to do — is safe and
//! repeats nothing that already happened. What each step found is written to
//! `<app data>/maintenance.json` (outcomes only, never text the user typed), replacing the last
//! launch's; a step that failed is simply tried again at the next launch.

use std::time::SystemTime;

use serde_json::json;
use tauri::{AppHandle, Emitter, Manager};

use crate::settings::Source;
use crate::startup::Startup;
use crate::{sync, upgrade, Launch};

const RECORD: &str = "maintenance.json";

pub fn at_launch(app: &AppHandle, saved_launch_at_login: bool, source: Source) {
    // The login item: the old LaunchAgent moved over, and the preference as macOS has it.
    let done = app.state::<Startup>().reconcile(saved_launch_at_login, source);
    sync::remember_launch_at_login(app, done.preference);
    let _ = app.emit("startup", &done.status);

    let version = app.package_info().version.to_string();
    let identifier = app.config().identifier.clone();
    let update = app
        .path()
        .app_data_dir()
        .map(|dir| format!("{:?}", upgrade::after_launch(&upgrade::Paths::new(&dir), &version)))
        .unwrap_or_else(|_| "no app data folder".into());

    let (mut removed, mut failed) = (0, 0);
    for staging in upgrade::stale_staging(&std::env::temp_dir(), &identifier, SystemTime::now(), upgrade::STAGING_MIN_AGE) {
        match std::fs::remove_dir_all(&staging) {
            Ok(()) => removed += 1,
            Err(_) => failed += 1,
        }
    }

    #[cfg(debug_assertions)]
    eprintln!("[baddel] at launch: login item {:?} ({:?}); update {update}; staging removed {removed}, failed {failed}", done.status, done.legacy);
    let instance = match &app.state::<Launch>().unguarded {
        None => "guarded".to_string(),
        Some(why) => format!("unguarded: {why}"),
    };
    let record = json!({
        "at": crate::sys::clock::now_ms(),
        "version": version,
        "instance": instance,
        "settings": format!("{source:?}"),
        "loginItem": { "status": done.status, "legacy": format!("{:?}", done.legacy) },
        "update": update,
        "staging": { "removed": removed, "failed": failed },
    });
    if let Ok(dir) = app.path().app_data_dir() {
        let _ = std::fs::write(dir.join(RECORD), serde_json::to_vec_pretty(&record).unwrap_or_default());
    }
}
