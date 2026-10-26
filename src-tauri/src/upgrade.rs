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
//!   whole old Baddel there. Other apps use the same names, so at launch such a folder is removed
//!   only when all of this holds ([`stale_staging`], [`remove_stale_staging`]): it and everything in
//!   it belong to this user; it holds nothing but one bundle, which `codesign` verifies against this
//!   app's own designated requirement (identifier and certificate), not merely a plist naming
//!   Baddel; nothing anywhere in it changed for an hour; no other Baddel runs; nothing runs from it
//!   or has a file in it open; no install of this process is under way (the install lock is held);
//!   and it is exactly as found when looked at again just before the removal.
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

/// One of the updater plugin's staging folders that looks like a cut-short install of Baddel.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Staging {
    /// The folder, removed whole.
    pub dir: PathBuf,
    /// The bundle in it: the folder itself for an extracted update, `current_app` for a backup.
    pub bundle: PathBuf,
    inside: &'static str,
}

/// Why a staging folder was left where it is.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Kept {
    /// Another Baddel — another copy, or another version — runs, and may be installing.
    AnotherBaddelRuns,
    /// The bundle in it is not signed as this Baddel is.
    NotSignedAsBaddel,
    /// A running app comes from inside it, or a process has a file under it open.
    InUse,
    /// Between the first look and the removal it changed: touched, replaced, or no longer ours.
    ChangedSinceChecked,
    RemoveFailed,
}

/// What [`remove_stale_staging`] did with a folder.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StagingOutcome {
    pub dir: PathBuf,
    pub kept: Option<Kept>,
}

/// What the cleanup asks of the system beyond the file system.
pub trait StagingChecks {
    fn another_baddel_runs(&self) -> bool;
    /// Whether `bundle` holds to this app's own designated code requirement.
    fn signed_as_us(&self, bundle: &Path) -> bool;
    fn in_use(&self, dir: &Path) -> bool;
}

/// The plugin's staging folders under `tmp` that may be a cut-short install of Baddel, by what the
/// file system alone can say: named as the plugin names them; real folders (no link followed) owned
/// by `uid`, as is everything in them; holding nothing but the one bundle, whose Info.plist gives
/// `identifier`; and nothing in them changed for `min_age` — the newest time anywhere in the tree,
/// not the folder's own. Not proof on its own: see [`remove_stale_staging`].
pub fn stale_staging(tmp: &Path, identifier: &str, uid: u32, now: SystemTime, min_age: Duration) -> Vec<Staging> {
    let Ok(entries) = std::fs::read_dir(tmp) else { return Vec::new() };
    entries
        .flatten()
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().into_owned();
            let (_, inside) = STAGING.iter().find(|(prefix, _)| name.starts_with(prefix))?;
            check(&entry.path(), inside, identifier, uid, now, min_age)
        })
        .collect()
}

fn check(dir: &Path, inside: &'static str, identifier: &str, uid: u32, now: SystemTime, min_age: Duration) -> Option<Staging> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::symlink_metadata(dir).ok()?;
    if !metadata.file_type().is_dir() || metadata.uid() != uid {
        return None;
    }
    let children: Vec<String> =
        std::fs::read_dir(dir).ok()?.flatten().map(|e| e.file_name().to_string_lossy().into_owned()).collect();
    if children != [inside] {
        return None;
    }
    let newest = newest_change(dir, uid)?;
    if now.duration_since(newest).map_or(true, |age| age < min_age) {
        return None;
    }
    // `current_app` is the whole bundle; the extracted update is the bundle's insides.
    let bundle = if inside == "Contents" { dir.to_path_buf() } else { dir.join(inside) };
    (bundle_value(&bundle, "CFBundleIdentifier").as_deref() == Some(identifier))
        .then(|| Staging { dir: dir.to_path_buf(), bundle, inside })
}

/// The latest change anywhere under `dir`, links included but never followed; `None` when anything
/// there cannot be read or belongs to another user.
fn newest_change(dir: &Path, uid: u32) -> Option<SystemTime> {
    use std::os::unix::fs::MetadataExt;
    let metadata = std::fs::symlink_metadata(dir).ok()?;
    if metadata.uid() != uid {
        return None;
    }
    let mut newest = metadata.modified().ok()?;
    if metadata.file_type().is_dir() {
        for entry in std::fs::read_dir(dir).ok()? {
            newest = newest.max(newest_change(&entry.ok()?.path(), uid)?);
        }
    }
    Some(newest)
}

/// Removes the staging folders that are, beyond doubt, Baddel's and idle. Holds the install lock
/// throughout, so no install by this process can start meanwhile; touches nothing while another
/// Baddel runs (it may be installing); and for each folder requires the bundle to be signed as this
/// app is, nothing to be running from or holding a file in it, and — looked at again just before
/// the removal — the folder to be exactly as it was found.
pub fn remove_stale_staging(
    candidates: Vec<Staging>,
    checks: &dyn StagingChecks,
    identifier: &str,
    uid: u32,
    now: SystemTime,
    min_age: Duration,
) -> Vec<StagingOutcome> {
    let _busy = exclusive();
    let another = checks.another_baddel_runs();
    candidates
        .into_iter()
        .map(|staging| {
            let kept = if another {
                Some(Kept::AnotherBaddelRuns)
            } else if !checks.signed_as_us(&staging.bundle) {
                Some(Kept::NotSignedAsBaddel)
            } else if checks.in_use(&staging.dir) {
                Some(Kept::InUse)
            } else if check(&staging.dir, staging.inside, identifier, uid, now, min_age).as_ref() != Some(&staging) {
                Some(Kept::ChangedSinceChecked)
            } else if std::fs::remove_dir_all(&staging.dir).is_err() {
                Some(Kept::RemoveFailed)
            } else {
                None
            };
            StagingOutcome { dir: staging.dir, kept }
        })
        .collect()
}

/// The Mac itself, for [`remove_stale_staging`]. `requirement` is this app's own designated code
/// requirement; without one (an unsigned or ad-hoc build), no bundle passes, and nothing is removed.
pub struct SystemChecks {
    pub requirement: Option<String>,
    pub identifier: String,
}

impl SystemChecks {
    /// The designated requirement of the bundle at `ours`, as `codesign` states it.
    pub fn requirement_of(ours: &Path) -> Option<String> {
        let output = Command::new("/usr/bin/codesign").args(["-d", "-r-"]).arg(ours).output().ok()?;
        let text = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        let requirement = text.lines().find_map(|line| line.strip_prefix("designated => "))?.trim().to_string();
        // An ad-hoc signature's requirement is its own hash: nothing else could ever hold to it.
        (output.status.success() && !requirement.starts_with("cdhash")).then_some(requirement)
    }
}

impl StagingChecks for SystemChecks {
    fn another_baddel_runs(&self) -> bool {
        crate::sys::frontmost::running_apps().iter().any(|app| {
            app.pid != std::process::id() as i32 && app.bundle_id.as_deref() == Some(self.identifier.as_str())
        })
    }

    fn signed_as_us(&self, bundle: &Path) -> bool {
        let Some(requirement) = &self.requirement else { return false };
        Command::new("/usr/bin/codesign")
            .args(["--verify", "--deep", "--strict"])
            .arg(format!("-R={requirement}"))
            .arg(bundle)
            .status()
            .is_ok_and(|status| status.success())
    }

    fn in_use(&self, dir: &Path) -> bool {
        let running_from = crate::sys::frontmost::running_apps()
            .iter()
            .any(|app| app.bundle.as_deref().is_some_and(|bundle| bundle.starts_with(dir)));
        // `lsof +D` lists the processes with a file under `dir` open (exit status 0), or none (1).
        // Anything else — lsof missing, or failing — is no proof of idleness.
        let open = Command::new("/usr/sbin/lsof")
            .args(["-t", "+D"])
            .arg(dir)
            .output()
            .map_or(true, |output| output.status.code() != Some(1) || !output.stdout.is_empty());
        running_from || open
    }
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

    fn uid() -> u32 {
        crate::sys::system::uid()
    }

    /// Sets the modification time of everything under `path` (links not followed).
    fn age(path: &Path, when: SystemTime) {
        let metadata = std::fs::symlink_metadata(path).unwrap();
        if metadata.file_type().is_dir() {
            for entry in std::fs::read_dir(path).unwrap() {
                age(&entry.unwrap().path(), when);
            }
        }
        if !metadata.file_type().is_symlink() {
            std::fs::File::open(path).unwrap().set_modified(when).unwrap();
        }
    }

    /// The fixtures of the staging tests: Baddel's two kinds, and what must never be taken for them.
    fn staging_fixtures(tmp: &Path) {
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
        age(tmp, SystemTime::now() - Duration::from_secs(2 * 60 * 60));
    }

    #[test]
    fn only_folders_that_look_like_baddels_own_staging_are_candidates() {
        let s = Scratch::new("staging");
        let tmp = s.0.join("T");
        staging_fixtures(&tmp);
        let mut found: Vec<PathBuf> =
            stale_staging(&tmp, ID, uid(), SystemTime::now(), STAGING_MIN_AGE).into_iter().map(|s| s.dir).collect();
        found.sort();
        assert_eq!(found, vec![tmp.join("tauri_current_appAb12Cd"), tmp.join("tauri_updated_appEf34Gh")]);
        // Another user's folders are never candidates.
        assert!(stale_staging(&tmp, ID, uid() + 1, SystemTime::now(), STAGING_MIN_AGE).is_empty());
    }

    #[test]
    fn a_change_anywhere_inside_an_old_folder_keeps_it() {
        // The folder's own time changes only when its entries do: a file written deep inside it a
        // moment ago (an install running now) leaves the folder looking old. The tree decides.
        let s = Scratch::new("deep-change");
        let tmp = s.0.join("T");
        staging_fixtures(&tmp);
        std::fs::write(tmp.join("tauri_current_appAb12Cd/current_app/Contents/MacOS/baddel"), b"being written").unwrap();
        let found: Vec<PathBuf> =
            stale_staging(&tmp, ID, uid(), SystemTime::now(), STAGING_MIN_AGE).into_iter().map(|s| s.dir).collect();
        assert_eq!(found, vec![tmp.join("tauri_updated_appEf34Gh")]);
    }

    struct Checks {
        another: bool,
        signed: bool,
        in_use: bool,
    }

    impl StagingChecks for Checks {
        fn another_baddel_runs(&self) -> bool {
            self.another
        }
        fn signed_as_us(&self, _: &Path) -> bool {
            self.signed
        }
        fn in_use(&self, _: &Path) -> bool {
            self.in_use
        }
    }

    #[test]
    fn staging_is_removed_only_when_ours_beyond_doubt_and_idle() {
        let ok = Checks { another: false, signed: true, in_use: false };
        for (checks, kept) in [
            (Checks { another: true, ..ok }, Some(Kept::AnotherBaddelRuns)),
            (Checks { signed: false, ..ok }, Some(Kept::NotSignedAsBaddel)),
            (Checks { in_use: true, ..ok }, Some(Kept::InUse)),
            (ok, None),
        ] {
            let s = Scratch::new("removal");
            let tmp = s.0.join("T");
            staging_fixtures(&tmp);
            let candidates = stale_staging(&tmp, ID, uid(), SystemTime::now(), STAGING_MIN_AGE);
            let dirs: Vec<PathBuf> = candidates.iter().map(|c| c.dir.clone()).collect();
            let outcomes = remove_stale_staging(candidates, &checks, ID, uid(), SystemTime::now(), STAGING_MIN_AGE);
            assert!(outcomes.iter().all(|o| o.kept == kept), "{outcomes:?}");
            for dir in dirs {
                assert_eq!(dir.exists(), kept.is_some(), "{dir:?}");
            }
            // Never anything else.
            for other in ["tauri_current_appZz99Yy", "tauri_current_appMixed", "tauri_current_appLink", "Baddel copy.app"] {
                assert!(std::fs::symlink_metadata(tmp.join(other)).is_ok(), "{other}");
            }
        }
    }

    #[test]
    fn a_folder_changed_between_the_look_and_the_removal_is_kept() {
        let s = Scratch::new("changed-staging");
        let tmp = s.0.join("T");
        staging_fixtures(&tmp);
        let candidates = stale_staging(&tmp, ID, uid(), SystemTime::now(), STAGING_MIN_AGE);
        // An install starts writing into one of them after the first look.
        std::fs::write(tmp.join("tauri_updated_appEf34Gh/Contents/new"), b"x").unwrap();
        let ok = Checks { another: false, signed: true, in_use: false };
        let outcomes = remove_stale_staging(candidates, &ok, ID, uid(), SystemTime::now(), STAGING_MIN_AGE);
        let kept: Vec<_> = outcomes.iter().filter(|o| o.kept.is_some()).map(|o| (o.dir.clone(), o.kept)).collect();
        assert_eq!(kept, vec![(tmp.join("tauri_updated_appEf34Gh"), Some(Kept::ChangedSinceChecked))]);
        assert!(tmp.join("tauri_updated_appEf34Gh/Contents/new").exists());
    }

    /// Against the real tools: `codesign` with Baddel's own requirement, and `lsof`. Uses the 2.0.2
    /// release archive when it is on disk (release/ is a local build output), and is skipped without.
    #[test]
    fn the_real_checks_tell_a_signed_baddel_and_an_open_file() {
        let archive = Path::new(env!("CARGO_MANIFEST_DIR")).join("../release/2.0.2/Baddel_2.0.2_universal.app.tar.gz");
        if !archive.exists() {
            eprintln!("skipped: no {archive:?}");
            return;
        }
        let s = Scratch::new("real-checks");
        let status = Command::new("/usr/bin/tar").arg("-xzf").arg(&archive).arg("-C").arg(&s.0).status().unwrap();
        assert!(status.success());
        let real = s.0.join("Baddel.app");
        // The requirement a signed Baddel states for itself, as `requirement_of` reads it.
        let requirement = SystemChecks::requirement_of(&real).expect("a release states its requirement");
        assert!(requirement.contains("com.isltanx.baddel") && requirement.contains("certificate leaf"), "{requirement}");
        let checks = SystemChecks { requirement: Some(requirement), identifier: ID.into() };
        assert!(checks.signed_as_us(&real));
        // Its insides at the root of a folder, as the plugin extracts an update.
        let extracted = s.0.join("tauri_updated_appReal01");
        std::fs::create_dir(&extracted).unwrap();
        std::fs::rename(real.join("Contents"), extracted.join("Contents")).unwrap();
        assert!(checks.signed_as_us(&extracted));
        // A folder that only says it is Baddel.
        let fake = s.0.join("Fake.app");
        bundle(&fake, ID, "2.0.1");
        assert!(!checks.signed_as_us(&fake));
        // In use: a file under it held open by another process.
        assert!(!checks.in_use(&fake));
        let mut holder = Command::new("/bin/sh")
            .arg("-c")
            .arg(format!("exec 3<'{}'; sleep 5", fake.join("Contents/Info.plist").display()))
            .spawn()
            .unwrap();
        std::thread::sleep(Duration::from_millis(300));
        let in_use = checks.in_use(&fake);
        holder.kill().unwrap();
        let _ = holder.wait();
        assert!(in_use);
    }

    #[test]
    fn an_unsigned_or_ad_hoc_build_removes_nothing() {
        // No requirement to hold staging to: nothing passes as signed by us.
        let checks = SystemChecks { requirement: None, identifier: ID.into() };
        assert!(!checks.signed_as_us(Path::new("/Applications/Baddel.app")));
    }
}
