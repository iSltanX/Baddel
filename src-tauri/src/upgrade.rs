//! Replacing Baddel with a newer version, and what replacing it leaves behind.
//!
//! The updater plugin downloads the release, checks its signature and swaps the bundle. Around that:
//!
//! - **Before the swap,** a copy of the running bundle is kept in Baddel's own folder
//!   (`<app data>/update/previous/<name>.app`, an APFS clone where the disk allows), with a note of
//!   the update beside it (`update/state.json`). No copy, no install: the update fails, untouched.
//! - **If the swap fails** and what is at the app's path is no longer a whole Baddel (the plugin
//!   moves the old bundle out before moving the new one in, and does not move it back), the copy is
//!   put back. Settings and data live outside the bundle and are never touched. The update stays on
//!   offer, to try again.
//! - **After the swap** the new bundle is checked — identifier and version — before Baddel restarts
//!   into it; a bundle that is not the version asked for is treated as a failed swap.
//! - **At the next launch** the running version tells how it went: the new one, and the copy goes;
//!   the old one (cut short before the swap), and the copy goes too, since the old bundle is the one
//!   running. Anything else — the copy and the note are kept and reported, never guessed at.
//! - **The plugin's own staging** in the temporary folder (`tauri_current_app…`, `tauri_updated_app…`)
//!   is removed by the plugin when an install ends; one cut short (power, a forced quit) leaves a
//!   whole old Baddel there. Such a folder is removed at launch only when it holds exactly a bundle
//!   with Baddel's identifier and has not been touched for an hour. Other apps use the same names.
//!
//! What is never removed: anything outside these paths; any app bundle the user put anywhere; and
//! Application Support, preferences, caches, WebKit data — all of which the running version uses.

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard};
use std::time::{Duration, SystemTime};

use serde::{Deserialize, Serialize};

use crate::sys::system::bundle_value;

const STATE: &str = "state.json";
const PREVIOUS: &str = "previous";
/// The updater plugin's staging folders (tauri-plugin-updater 2, macOS), and what each holds.
const STAGING: [(&str, &str); 2] = [("tauri_current_app", "current_app"), ("tauri_updated_app", "Contents")];
/// Staging untouched for this long belongs to no install still running.
pub const STAGING_MIN_AGE: Duration = Duration::from_secs(60 * 60);

/// Held while the note and the copy are being read or changed: by an install (from the copy to the
/// check of the new bundle) and by [`after_launch`], which could otherwise run at the same moment
/// when an install is started within a launch's first seconds.
static BUSY: Mutex<()> = Mutex::new(());

/// The hold an install keeps on the note and the copy from [`prepare`] until [`finish`] returns.
pub fn exclusive() -> MutexGuard<'static, ()> {
    BUSY.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
}

/// Where an update keeps its note and the copy of the previous version.
pub struct Paths {
    dir: PathBuf,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Note {
    pub from: String,
    pub to: String,
    /// The bundle being replaced.
    pub app: PathBuf,
    /// The copy of it.
    pub copy: PathBuf,
    pub phase: Stage,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Stage {
    /// The copy is made; the swap has not finished.
    Installing,
    /// The new version is in place and checked; Baddel is restarting into it.
    Installed,
}

/// How the swap went, as [`finish`] found it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Finished {
    /// The new version is in place: restart into it.
    Installed,
    /// The previous version is in place — untouched, or put back from the copy — and runs as before.
    Failed,
    /// Neither version could be made whole at the app's path. The copy is kept, at `copy`.
    Unrecoverable { copy: PathBuf },
}

/// What [`after_launch`] found of an earlier update.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AfterLaunch {
    Nothing,
    /// The update to this version went through; its copy of `from` is gone.
    Completed { from: String },
    /// An update to `to` never got as far as the swap; this, the version it would have replaced, runs.
    NotCompleted { to: String },
    /// Neither version of the note is running: everything is left as it is.
    Kept { note: Note },
    /// The copy or the note could not be removed; tried again at the next launch.
    CleanupFailed,
}

impl Paths {
    pub fn new(app_data: &Path) -> Self {
        Paths { dir: app_data.join("update") }
    }

    fn state(&self) -> PathBuf {
        self.dir.join(STATE)
    }

    fn copy_of(&self, bundle: &Path) -> PathBuf {
        self.dir.join(PREVIOUS).join(bundle.file_name().unwrap_or("Baddel.app".as_ref()))
    }

    fn note(&self) -> Option<Note> {
        serde_json::from_slice(&std::fs::read(self.state()).ok()?).ok()
    }

    fn write(&self, note: &Note) -> std::io::Result<()> {
        let json = serde_json::to_vec_pretty(note).map_err(std::io::Error::other)?;
        // Written whole then moved into place: an interrupted write leaves the old note, not half of one.
        let partial = self.dir.join("state.json.partial");
        std::fs::write(&partial, json)?;
        std::fs::rename(partial, self.state())
    }

    /// Removes the copy of the previous version and the note: only inside this folder, and never
    /// through a link.
    fn clear(&self) -> std::io::Result<()> {
        let previous = self.dir.join(PREVIOUS);
        match std::fs::symlink_metadata(&previous) {
            Ok(metadata) if metadata.file_type().is_dir() => std::fs::remove_dir_all(&previous)?,
            Ok(_) => std::fs::remove_file(&previous)?,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error),
        }
        match std::fs::remove_file(self.state()) {
            Err(error) if error.kind() != std::io::ErrorKind::NotFound => Err(error),
            _ => Ok(()),
        }
    }
}

/// Whether `bundle` is a whole Baddel of `version`: the identifier and version its Info.plist gives
/// (a release's "v" prefix, if any, aside).
fn is_baddel(bundle: &Path, identifier: &str, version: &str) -> bool {
    std::fs::symlink_metadata(bundle).is_ok_and(|m| m.file_type().is_dir())
        && bundle_value(bundle, "CFBundleIdentifier").as_deref() == Some(identifier)
        && bundle_value(bundle, "CFBundleShortVersionString").as_deref().map(|v| v.trim_start_matches('v'))
            == Some(version.trim_start_matches('v'))
}

/// Copies a bundle, keeping its signature intact: an APFS clone where the disk can, else `ditto`.
fn copy_bundle(from: &Path, to: &Path) -> std::io::Result<()> {
    let cloned = Command::new("/bin/cp").arg("-cR").arg(from).arg(to).status()?.success();
    if cloned {
        return Ok(());
    }
    let _ = std::fs::remove_dir_all(to);
    if Command::new("/usr/bin/ditto").arg(from).arg(to).status()?.success() {
        Ok(())
    } else {
        Err(std::io::Error::other("could not copy the app"))
    }
}

/// Before the swap: keeps a checked copy of `app` (version `from`) and notes the update to `to`.
pub fn prepare(paths: &Paths, app: &Path, identifier: &str, from: &str, to: &str) -> std::io::Result<()> {
    if !is_baddel(app, identifier, from) {
        return Err(std::io::Error::other("the running app is not where it says it is"));
    }
    // A copy left by an attempt that never finished is replaced.
    paths.clear()?;
    std::fs::create_dir_all(paths.dir.join(PREVIOUS))?;
    let copy = paths.copy_of(app);
    copy_bundle(app, &copy)?;
    if !is_baddel(&copy, identifier, from) {
        let _ = paths.clear();
        return Err(std::io::Error::other("the copy of the app is not whole"));
    }
    paths.write(&Note { from: from.into(), to: to.into(), app: app.into(), copy, phase: Stage::Installing })
}

/// After the plugin's install returned, whatever it returned: what is at the app's path decides.
/// The new version there, checked, is installed; anything less puts the copy back.
pub fn finish(paths: &Paths, identifier: &str) -> Finished {
    let Some(mut note) = paths.note() else {
        // No note means no copy was made, and the install is never let go ahead without one.
        return Finished::Failed;
    };
    if is_baddel(&note.app, identifier, &note.to) {
        // A note that cannot be updated still says "installing" with the new version running,
        // which the next launch settles the same way.
        note.phase = Stage::Installed;
        let _ = paths.write(&note);
        return Finished::Installed;
    }
    if is_baddel(&note.app, identifier, &note.from) {
        // The swap never happened: the previous version is untouched.
        let _ = paths.clear();
        return Finished::Failed;
    }
    if restore(&note, identifier) && is_baddel(&note.app, identifier, &note.from) {
        let _ = paths.clear();
        return Finished::Failed;
    }
    Finished::Unrecoverable { copy: note.copy }
}

/// Puts the copy back at the app's path, if nothing whole is there. Something that is not Baddel's
/// is never removed to make room.
fn restore(note: &Note, identifier: &str) -> bool {
    match std::fs::symlink_metadata(&note.app) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
        // A partial bundle of ours: its Info.plist names Baddel, or there is none left to say. One
        // that is there but cannot be read could be anything, the new version included: left as
        // it is, with the copy kept.
        Ok(metadata) if metadata.file_type().is_dir() => {
            let has_plist = note.app.join("Contents/Info.plist").exists();
            match bundle_value(&note.app, "CFBundleIdentifier") {
                Some(id) if id == identifier => {}
                None if !has_plist => {}
                _ => return false,
            }
            if std::fs::remove_dir_all(&note.app).is_err() {
                return false;
            }
        }
        _ => return false,
    }
    // A rename keeps the copy's every byte and is instant on one disk; across disks, copy it back.
    std::fs::rename(&note.copy, &note.app).is_ok() || copy_bundle(&note.copy, &note.app).is_ok()
}

/// At launch: settles the note of an earlier update against the version now running.
pub fn after_launch(paths: &Paths, running: &str) -> AfterLaunch {
    let _busy = exclusive();
    let Some(note) = paths.note() else {
        // A copy without a note is a `prepare` cut short: nothing was swapped.
        if paths.dir.join(PREVIOUS).exists() && paths.clear().is_err() {
            return AfterLaunch::CleanupFailed;
        }
        return AfterLaunch::Nothing;
    };
    let outcome = if note.to == running {
        AfterLaunch::Completed { from: note.from.clone() }
    } else if note.from == running {
        AfterLaunch::NotCompleted { to: note.to.clone() }
    } else {
        return AfterLaunch::Kept { note };
    };
    match paths.clear() {
        Ok(()) => outcome,
        Err(_) => AfterLaunch::CleanupFailed,
    }
}

/// The updater plugin's staging folders under `tmp` that a cut-short install of Baddel left: named
/// as the plugin names them, a real folder (no link), holding nothing but the one bundle (whose
/// Info.plist gives `identifier`), and untouched for at least `min_age`.
pub fn stale_staging(tmp: &Path, identifier: &str, now: SystemTime, min_age: Duration) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(tmp) else { return Vec::new() };
    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let (_, inside) = STAGING.iter().find(|(prefix, _)| name.starts_with(prefix))?;
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(&path).ok()?;
            let old = now.duration_since(metadata.modified().ok()?).is_ok_and(|age| age >= min_age);
            if !metadata.file_type().is_dir() || !old {
                return None;
            }
            let children: Vec<String> =
                std::fs::read_dir(&path).ok()?.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
            if children != [*inside] {
                return None;
            }
            // `current_app` is the whole bundle; the extracted update is the bundle's insides.
            let bundle = if *inside == "Contents" { path.clone() } else { path.join(inside) };
            (bundle_value(&bundle, "CFBundleIdentifier").as_deref() == Some(identifier)).then_some(path)
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    const ID: &str = "com.isltanx.baddel";

    struct Scratch(PathBuf);

    impl Scratch {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("baddel-upgrade-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(&dir).unwrap();
            Scratch(dir)
        }
    }

    impl Drop for Scratch {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    /// A bundle that is enough of an app for these checks: an Info.plist and an executable.
    fn bundle(at: &Path, identifier: &str, version: &str) {
        std::fs::create_dir_all(at.join("Contents/MacOS")).unwrap();
        std::fs::write(at.join("Contents/MacOS/baddel"), format!("binary {version}")).unwrap();
        let plist = format!(
            "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\"><dict><key>CFBundleIdentifier</key><string>{identifier}</string><key>CFBundleShortVersionString</key><string>{version}</string></dict></plist>"
        );
        std::fs::write(at.join("Contents/Info.plist"), plist).unwrap();
    }

    fn version_of(at: &Path) -> Option<String> {
        bundle_value(at, "CFBundleShortVersionString")
    }

    #[test]
    fn a_successful_update_restarts_into_the_new_version_and_the_next_launch_clears_up() {
        let s = Scratch::new("success");
        let app = s.0.join("Applications/Baddel.app");
        bundle(&app, ID, "2.0.2");
        let paths = Paths::new(&s.0.join("support"));
        prepare(&paths, &app, ID, "2.0.2", "2.1.0").unwrap();
        assert_eq!(version_of(&paths.copy_of(&app)).as_deref(), Some("2.0.2"));
        // The plugin swaps the bundle.
        std::fs::remove_dir_all(&app).unwrap();
        bundle(&app, ID, "2.1.0");
        assert_eq!(finish(&paths, ID), Finished::Installed);
        // The copy stays until the new version is up.
        assert!(paths.copy_of(&app).exists());
        assert_eq!(after_launch(&paths, "2.1.0"), AfterLaunch::Completed { from: "2.0.2".into() });
        assert!(!paths.dir.join(PREVIOUS).exists() && !paths.state().exists());
        assert_eq!(after_launch(&paths, "2.1.0"), AfterLaunch::Nothing);
    }

    #[test]
    fn a_failed_download_or_check_leaves_the_previous_version_as_it_was() {
        let s = Scratch::new("untouched");
        let app = s.0.join("Baddel.app");
        bundle(&app, ID, "2.0.2");
        let paths = Paths::new(&s.0.join("support"));
        prepare(&paths, &app, ID, "2.0.2", "2.1.0").unwrap();
        assert_eq!(finish(&paths, ID), Finished::Failed);
        assert_eq!(version_of(&app).as_deref(), Some("2.0.2"));
        assert!(!paths.dir.join(PREVIOUS).exists());
    }

    #[test]
    fn a_swap_that_stops_halfway_puts_the_previous_version_back() {
        let s = Scratch::new("halfway");
        let app = s.0.join("Baddel.app");
        bundle(&app, ID, "2.0.2");
        let paths = Paths::new(&s.0.join("support"));
        prepare(&paths, &app, ID, "2.0.2", "2.1.0").unwrap();
        // The plugin moved the old bundle out, then failed to move the new one in.
        std::fs::remove_dir_all(&app).unwrap();
        assert_eq!(finish(&paths, ID), Finished::Failed);
        assert_eq!(version_of(&app).as_deref(), Some("2.0.2"));
        assert_eq!(std::fs::read_to_string(app.join("Contents/MacOS/baddel")).unwrap(), "binary 2.0.2");
        // A partial new bundle in its place is replaced as well.
        prepare(&paths, &app, ID, "2.0.2", "2.1.0").unwrap();
        std::fs::remove_file(app.join("Contents/Info.plist")).unwrap();
        assert_eq!(finish(&paths, ID), Finished::Failed);
        assert_eq!(version_of(&app).as_deref(), Some("2.0.2"));
    }

    #[test]
    fn a_bundle_that_is_not_the_version_asked_for_is_not_taken_as_installed() {
        let s = Scratch::new("wrong-version");
        let app = s.0.join("Baddel.app");
        bundle(&app, ID, "2.0.2");
        let paths = Paths::new(&s.0.join("support"));
        prepare(&paths, &app, ID, "2.0.2", "2.1.0").unwrap();
        std::fs::remove_dir_all(&app).unwrap();
        bundle(&app, ID, "2.0.9");
        assert_eq!(finish(&paths, ID), Finished::Failed);
        assert_eq!(version_of(&app).as_deref(), Some("2.0.2"));
    }

    #[test]
    fn a_bundle_that_cannot_be_read_is_not_removed_to_make_room() {
        // Reviewer finding: an Info.plist that is there but unreadable was taken for a partial
        // bundle and removed — possibly the new version, whole.
        let s = Scratch::new("unreadable");
        let app = s.0.join("Baddel.app");
        bundle(&app, ID, "2.0.2");
        let paths = Paths::new(&s.0.join("support"));
        prepare(&paths, &app, ID, "2.0.2", "2.1.0").unwrap();
        std::fs::write(app.join("Contents/Info.plist"), b"not a plist").unwrap();
        assert!(matches!(finish(&paths, ID), Finished::Unrecoverable { .. }));
        assert_eq!(std::fs::read(app.join("Contents/Info.plist")).unwrap(), b"not a plist");
        assert!(paths.copy_of(&app).exists());
    }

    #[test]
    fn something_else_in_the_apps_place_is_never_removed() {
        let s = Scratch::new("someone-else");
        let app = s.0.join("Baddel.app");
        bundle(&app, ID, "2.0.2");
        let paths = Paths::new(&s.0.join("support"));
        prepare(&paths, &app, ID, "2.0.2", "2.1.0").unwrap();
        std::fs::remove_dir_all(&app).unwrap();
        bundle(&app, "com.example.other", "1.0");
        let copy = paths.copy_of(&app);
        assert_eq!(finish(&paths, ID), Finished::Unrecoverable { copy: copy.clone() });
        assert_eq!(bundle_value(&app, "CFBundleIdentifier").as_deref(), Some("com.example.other"));
        // The copy is kept, and so is the note, for the next launch to report.
        assert_eq!(version_of(&copy).as_deref(), Some("2.0.2"));
        assert!(matches!(after_launch(&paths, "2.0.9"), AfterLaunch::Kept { .. }));
        assert!(copy.exists());
    }

    #[test]
    fn an_update_cut_short_before_the_swap_is_settled_by_the_version_that_runs() {
        let s = Scratch::new("cut-short");
        let app = s.0.join("Baddel.app");
        bundle(&app, ID, "2.0.2");
        let paths = Paths::new(&s.0.join("support"));
        prepare(&paths, &app, ID, "2.0.2", "2.1.0").unwrap();
        // Baddel quit (or the Mac lost power) before the swap; 2.0.2 starts again.
        assert_eq!(after_launch(&paths, "2.0.2"), AfterLaunch::NotCompleted { to: "2.1.0".into() });
        assert!(!paths.dir.join(PREVIOUS).exists());
        assert_eq!(version_of(&app).as_deref(), Some("2.0.2"));
        // A copy without its note: `prepare` itself was cut short.
        std::fs::create_dir_all(paths.dir.join(PREVIOUS).join("Baddel.app")).unwrap();
        assert_eq!(after_launch(&paths, "2.0.2"), AfterLaunch::Nothing);
        assert!(!paths.dir.join(PREVIOUS).exists());
    }

    #[test]
    fn an_app_not_where_it_says_it_is_is_not_updated() {
        let s = Scratch::new("missing");
        let paths = Paths::new(&s.0.join("support"));
        assert!(prepare(&paths, &s.0.join("Baddel.app"), ID, "2.0.2", "2.1.0").is_err());
        let other = s.0.join("Other.app");
        bundle(&other, "com.example.other", "2.0.2");
        assert!(prepare(&paths, &other, ID, "2.0.2", "2.1.0").is_err());
        assert!(!paths.state().exists());
    }

    #[test]
    fn only_baddels_own_stale_staging_is_found() {
        let s = Scratch::new("staging");
        let tmp = s.0.join("T");
        // A cut-short install of Baddel: the backup of the old app, and the extracted new one.
        bundle(&tmp.join("tauri_current_appAb12Cd/current_app"), ID, "2.0.1");
        bundle(&tmp.join("tauri_updated_appEf34Gh"), ID, "2.0.2");
        std::fs::remove_dir_all(tmp.join("tauri_updated_appEf34Gh/Contents/MacOS")).unwrap();
        // Another Tauri app's, by the very same names.
        bundle(&tmp.join("tauri_current_appZz99Yy/current_app"), "com.example.other", "1.0");
        // Ours by its bundle, but with something else beside it.
        bundle(&tmp.join("tauri_current_appMixed/current_app"), ID, "2.0.1");
        std::fs::write(tmp.join("tauri_current_appMixed/notes.txt"), b"keep").unwrap();
        // A link by that name, and an unrelated folder of Baddel's.
        std::os::unix::fs::symlink(tmp.join("tauri_current_appAb12Cd"), tmp.join("tauri_current_appLink")).unwrap();
        bundle(&tmp.join("Baddel copy.app"), ID, "2.0.1");

        let later = SystemTime::now() + Duration::from_secs(2 * 60 * 60);
        let mut found = stale_staging(&tmp, ID, later, STAGING_MIN_AGE);
        found.sort();
        assert_eq!(found, vec![tmp.join("tauri_current_appAb12Cd"), tmp.join("tauri_updated_appEf34Gh")]);
        // An install running now (touched a moment ago) is left alone.
        assert!(stale_staging(&tmp, ID, SystemTime::now(), STAGING_MIN_AGE).is_empty());
    }
}
