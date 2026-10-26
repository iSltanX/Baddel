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

    // The updater's staging left by a cut-short install: only what is provably Baddel's and idle.
    let (uid, now, age) = (crate::sys::system::uid(), SystemTime::now(), upgrade::STAGING_MIN_AGE);
    let candidates = upgrade::stale_staging(&std::env::temp_dir(), &identifier, uid, now, age);
    let staging = if candidates.is_empty() {
        Vec::new()
    } else {
        let checks = upgrade::SystemChecks {
            requirement: crate::sys::system::bundle_path().and_then(|ours| upgrade::SystemChecks::requirement_of(&ours)),
            identifier: identifier.clone(),
        };
        upgrade::remove_stale_staging(candidates, &checks, &identifier, uid, now, age)
    };
    let removed = staging.iter().filter(|s| s.kept.is_none()).count();
    let failed = staging.len() - removed;

    #[cfg(debug_assertions)]
    eprintln!("[baddel] at launch: login item {:?} ({:?}); update {update}; staging removed {removed}, kept {failed}", done.status, done.legacy);
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
        "staging": staging,
    });
    if let Ok(dir) = app.path().app_data_dir() {
        let _ = std::fs::write(dir.join(RECORD), serde_json::to_vec_pretty(&record).unwrap_or_default());
    }
}
