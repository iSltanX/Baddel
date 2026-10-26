//! Launch at login: one mechanism, the app itself as a login item (`SMAppService.mainApp`, macOS
//! 13 and later), and the truth about it.
//!
//! - **What Settings shows is macOS's answer,** read each time, not the saved preference. The
//!   preference records the user's last choice, wherever it was made: switched off in System
//!   Settings › General › Login Items, it reads off here as well, and Baddel never turns it back
//!   on by itself. Switched on there (by the user), it reads on.
//! - **A change macOS refuses is reported as refused,** with its reason. Nothing is saved as if it
//!   had worked, and the switch goes back to what macOS says.
//! - **Unreadable settings decide nothing:** the defaults that stand in for them say "off", which is
//!   no reason to take away a registration that works.
//! - **The LaunchAgent of Baddel 1.0–2.0.2** (`~/Library/LaunchAgents/Baddel.plist`, written by
//!   tauri-plugin-autostart) is moved over at launch — only a file that is byte for byte what those
//!   versions wrote. Wanted, it is replaced by the login item; not wanted (off in Baddel, or switched
//!   off by the user in System Settings), it is removed and nothing takes its place. It is removed
//!   only once the login item is confirmed, so a failed or interrupted move leaves the old one
//!   working, and the next launch tries again. Anything else at that path is left alone and named.
//!   A Login Item the user added by hand is the very entry `SMAppService.mainApp` reports: it is
//!   shown as on, and never removed except by the switch.

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use serde::Serialize;

use crate::settings::Source;
use crate::sys::login_item::{self, Status};

/// The label tauri-plugin-autostart gave Baddel's LaunchAgent: the product name.
const LEGACY_LABEL: &str = "Baddel";
/// Larger than any plist those versions wrote, by far.
const LEGACY_MAX_BYTES: u64 = 4096;

/// What the switch shows.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum State {
    Enabled,
    Disabled,
    /// Registered, but waiting for the user in System Settings › Login Items (or switched off there).
    RequiresApproval,
    /// macOS cannot find Baddel as a login item at all.
    Unavailable,
}

impl From<Status> for State {
    fn from(status: Status) -> Self {
        match status {
            Status::Enabled => State::Enabled,
            Status::NotRegistered => State::Disabled,
            Status::RequiresApproval => State::RequiresApproval,
            Status::NotFound => State::Unavailable,
        }
    }
}

/// Why the last change did not go through, as the row says it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Problem {
    /// macOS refused: the user has not allowed it (`kSMErrorLaunchDeniedByUser`).
    DeniedByUser,
    /// macOS did not accept the app's signature (`kSMErrorInvalidSignature`).
    InvalidSignature,
    /// Running off a disk image or a translocated download: a login item there would not last.
    TemporaryLocation,
    /// Switched off, macOS still lists Baddel as a login item.
    StillEnabled,
    /// The old LaunchAgent could not be moved out of the way; it is kept, and tried again.
    LegacyKept,
    Other,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Failure {
    pub problem: Problem,
    /// macOS's error code, when it gave one.
    pub code: Option<i64>,
}

impl Failure {
    fn new(problem: Problem) -> Self {
        Failure { problem, code: None }
    }

    fn from_error(error: &login_item::Error) -> Self {
        let problem = match error.code {
            login_item::LAUNCH_DENIED_BY_USER => Problem::DeniedByUser,
            login_item::INVALID_SIGNATURE => Problem::InvalidSignature,
            _ => Problem::Other,
        };
        Failure { problem, code: Some(error.code) }
    }
}

/// An older Baddel's LaunchAgent, when one was found at launch.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum Legacy {
    /// Baddel's own, still in place: kept until the login item that replaces it is confirmed.
    Kept,
    /// Something at that path that is not provably Baddel's: left as it is.
    Unverified,
}

/// What Settings shows for launch at login.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartupStatus {
    pub state: State,
    pub failure: Option<Failure>,
    pub legacy: Option<Legacy>,
}

/// A launchd job, as `launchctl print` reports it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Job {
    NotLoaded,
    /// `idle`: launchd said in so many words that nothing runs under it (`state = not running`).
    Loaded { pid: Option<i32>, idle: bool },
    /// `launchctl` gave no answer that could be read.
    Unknown,
}

/// Everything this module asks of macOS, so the decisions can be tested without touching it.
pub trait System: Send + Sync {
    fn status(&self) -> Status;
    fn register(&self) -> Result<(), login_item::Error>;
    fn unregister(&self) -> Result<(), login_item::Error>;
    fn legacy_status(&self, plist: &Path) -> Status;
    /// The old LaunchAgent's job in `gui/<uid>`.
    fn job(&self) -> Job;
    /// Unloads that job. Only ever asked of a job no process runs under.
    fn bootout(&self) -> bool;
    fn bundle_id(&self, bundle: &Path) -> Option<String>;
    /// The bundle this copy of Baddel runs from.
    fn bundle(&self) -> Option<PathBuf>;
    /// Whether this copy runs from a disk image or a translocated download.
    fn temporary_location(&self) -> bool;
}

/// Held in Tauri state.
pub struct Startup {
    system: Box<dyn System>,
    /// `~`, where the old LaunchAgent lives. Without one, there is no old agent to look for.
    home: Option<PathBuf>,
    /// This app's bundle identifier.
    identifier: String,
    /// What the last attempt left to say: a failure, and the old LaunchAgent if one is still there.
    last: Mutex<(Option<Failure>, Option<Legacy>)>,
}

/// What [`Startup::reconcile`] found, for the caller to save and announce.
#[derive(Debug, PartialEq, Eq)]
pub struct Reconciled {
    pub status: StartupStatus,
    /// The preference as macOS now has it; `None` when macOS gave no answer to go by.
    pub preference: Option<bool>,
    /// What was done to the old LaunchAgent, for diagnostics.
    pub legacy: LegacyOutcome,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LegacyOutcome {
    None,
    /// Wanted: replaced by the login item, and removed.
    Migrated,
    /// Not wanted: removed, and nothing registered.
    Removed,
    /// Kept until the login item is confirmed (approval, or a refusal to retry).
    Kept,
    Unverified,
}

impl Startup {
    pub fn new(system: Box<dyn System>, home: Option<PathBuf>, identifier: String) -> Self {
        Startup { system, home, identifier, last: Mutex::new((None, None)) }
    }

    fn legacy_plist(&self) -> Option<PathBuf> {
        Some(self.home.as_ref()?.join("Library/LaunchAgents").join(format!("{LEGACY_LABEL}.plist")))
    }

    fn inspect_legacy(&self) -> (Option<PathBuf>, Found) {
        match self.legacy_plist() {
            Some(plist) => {
                let found = inspect(&plist, self.system.as_ref(), &self.identifier);
                (Some(plist), found)
            }
            None => (None, Found::Absent),
        }
    }

    /// The status as macOS has it now, with what the last attempt left to say.
    pub fn status(&self) -> StartupStatus {
        let (failure, legacy) = self.last.lock().unwrap().clone();
        StartupStatus { state: self.system.status().into(), failure, legacy }
    }

    /// At launch: moves the old LaunchAgent over (see the module notes), then reads the outcome.
    /// Registers nothing unless the old agent was on and is being replaced.
    pub fn reconcile(&self, saved: bool, source: Source) -> Reconciled {
        let mut failure = None;
        let (legacy, outcome) = match self.inspect_legacy() {
            (_, Found::Absent) | (None, _) => (None, LegacyOutcome::None),
            (_, Found::Unverified) => (Some(Legacy::Unverified), LegacyOutcome::Unverified),
            (Some(plist), Found::Ours { contents, bundle }) => {
                // 2.0.x removed its agent at every launch with the option off, so with the settings
                // unreadable (or gone) the file itself says it was on.
                let wanted = if source == Source::File { saved } else { true };
                // Switched off by the user in System Settings: their decision, kept.
                let switched_off = self.system.legacy_status(&plist) == Status::RequiresApproval;
                // The login item would be this copy of Baddel. One that will not last (a disk image,
                // a translocated download), or another copy than the one the old agent starts, is
                // no replacement for it: the old agent stays, untouched, until Baddel runs from there.
                let elsewhere = self.system.temporary_location()
                    || bundle.is_some_and(|theirs| self.system.bundle().is_none_or(|ours| ours != theirs));
                if wanted && !switched_off && elsewhere {
                    (Some(Legacy::Kept), LegacyOutcome::Kept)
                } else if wanted && !switched_off {
                    let mut status = self.system.status();
                    if matches!(status, Status::NotRegistered | Status::NotFound) {
                        if let Err(error) = self.system.register() {
                            if error.code != login_item::ALREADY_REGISTERED {
                                failure = Some(Failure::from_error(&error));
                            }
                        }
                        status = self.system.status();
                    }
                    if status == Status::Enabled {
                        match self.retire(&plist, &contents) {
                            Ok(()) => (None, LegacyOutcome::Migrated),
                            Err(kept) => {
                                failure = Some(kept);
                                (Some(Legacy::Kept), LegacyOutcome::Kept)
                            }
                        }
                    } else {
                        (Some(Legacy::Kept), LegacyOutcome::Kept)
                    }
                } else {
                    match self.retire(&plist, &contents) {
                        Ok(()) => (None, LegacyOutcome::Removed),
                        Err(kept) => {
                            failure = Some(kept);
                            (Some(Legacy::Kept), LegacyOutcome::Kept)
                        }
                    }
                }
            }
        };
        *self.last.lock().unwrap() = (failure, legacy);
        let status = self.status();
        // While the old agent is kept, it is what starts Baddel at login, and the user's choice is
        // the one it stands for: what macOS says of the login item meanwhile does not replace it.
        // Nor while an agent of unknown origin by that name may start Baddel too.
        let preference = match (status.state, outcome) {
            (_, LegacyOutcome::Kept | LegacyOutcome::Unverified) => None,
            (State::Enabled | State::RequiresApproval, _) => Some(true),
            (State::Disabled, _) => Some(false),
            (State::Unavailable, _) => None,
        };
        Reconciled { status, preference, legacy: outcome }
    }

    /// The switch in Settings. Returns the status macOS reports afterwards, which is what the
    /// switch then shows, whether or not the change went through.
    pub fn set_enabled(&self, enabled: bool) -> StartupStatus {
        let failure = if enabled && self.system.temporary_location() {
            Some(Failure::new(Problem::TemporaryLocation))
        } else {
            self.change(enabled)
        };
        self.last.lock().unwrap().0 = failure;
        self.status()
    }

    fn change(&self, enabled: bool) -> Option<Failure> {
        let result = if enabled { self.system.register() } else { self.system.unregister() };
        let harmless = if enabled { login_item::ALREADY_REGISTERED } else { login_item::JOB_NOT_FOUND };
        let mut failure = result.err().filter(|e| e.code != harmless).map(|e| Failure::from_error(&e));
        let status = self.system.status();
        if failure.is_none() {
            if enabled && !matches!(status, Status::Enabled | Status::RequiresApproval) {
                failure = Some(Failure::new(Problem::Other));
            }
            if !enabled && status == Status::Enabled {
                failure = Some(Failure::new(Problem::StillEnabled));
            }
        }
        // Whichever way it went, the old agent may now go: replaced once the login item is on,
        // and unwanted once the switch is off. Not while the new one is unconfirmed.
        let replaced = enabled && status == Status::Enabled;
        if (replaced || !enabled) && failure.is_none() {
            if let (Some(plist), Found::Ours { contents, .. }) = self.inspect_legacy() {
                if let Err(kept) = self.retire(&plist, &contents) {
                    return Some(kept);
                }
                self.last.lock().unwrap().1 = None;
            }
        }
        failure
    }

    /// Removes the old LaunchAgent, then unloads its job if launchd says nothing runs under it. A
    /// job a process runs under — this very Baddel, started by it at login — or one launchd does not
    /// say so plainly of, is left loaded: unloading it would end that process, and with its file
    /// gone it does not outlive the login session.
    fn retire(&self, plist: &Path, contents: &[u8]) -> Result<(), Failure> {
        remove_legacy(plist, contents).map_err(|_| Failure::new(Problem::LegacyKept))?;
        if let Job::Loaded { pid: None, idle: true } = self.system.job() {
            self.system.bootout();
        }
        Ok(())
    }
}

/// What is at the old LaunchAgent's path.
#[derive(Debug, PartialEq, Eq)]
enum Found {
    Absent,
    /// Exactly what Baddel 1.0–2.0.2 wrote; its bytes, to check again just before removing it, and
    /// the bundle it starts, while that is still there.
    Ours { contents: Vec<u8>, bundle: Option<PathBuf> },
    Unverified,
}

/// The plist tauri-plugin-autostart 2 (auto-launch 0.5) wrote for `program`, byte for byte.
fn legacy_contents(program: &str) -> String {
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
         <!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
         <plist version=\"1.0\">\n  <dict>\n  <key>Label</key>\n  <string>{LEGACY_LABEL}</string>\n  \
         <key>ProgramArguments</key>\n  <array><string>{program}</string></array>\n  \
         <key>RunAtLoad</key>\n  <true/>\n  </dict>\n</plist>"
    )
}

/// Whether the file at `plist` is Baddel's old LaunchAgent: a regular file (no link followed),
/// byte for byte the plist those versions wrote, for a program at `…/<name>.app/Contents/MacOS/baddel`
/// — and, if that bundle is still there, a bundle with this app's identifier. Gone (moved, deleted),
/// the file is still provably ours; another app's bundle there makes it not ours to touch.
fn inspect(plist: &Path, system: &dyn System, identifier: &str) -> Found {
    let metadata = match std::fs::symlink_metadata(plist) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Found::Absent,
        Err(_) => return Found::Unverified,
    };
    if !metadata.file_type().is_file() || metadata.len() > LEGACY_MAX_BYTES {
        return Found::Unverified;
    }
    let Ok(contents) = std::fs::read(plist) else { return Found::Unverified };
    let Ok(text) = std::str::from_utf8(&contents) else { return Found::Unverified };
    let Some(program) = text.split_once("<array><string>").and_then(|(_, rest)| rest.split_once("</string></array>"))
    else {
        return Found::Unverified;
    };
    let program = program.0;
    if text != legacy_contents(program) {
        return Found::Unverified;
    }
    let path = Path::new(program);
    let bundle = path.ancestors().nth(3);
    let shaped = path.is_absolute()
        && path.ends_with("Contents/MacOS/baddel")
        && bundle.is_some_and(|b| b.extension().is_some_and(|e| e == "app"));
    if !shaped {
        return Found::Unverified;
    }
    let bundle = bundle.filter(|b| b.exists()).map(Path::to_path_buf);
    if let Some(bundle) = &bundle {
        if system.bundle_id(bundle).as_deref() != Some(identifier) {
            return Found::Unverified;
        }
    }
    Found::Ours { contents, bundle }
}

/// Removes the old LaunchAgent if it is still the very file inspected: not a link, not changed.
fn remove_legacy(plist: &Path, contents: &[u8]) -> std::io::Result<()> {
    let metadata = std::fs::symlink_metadata(plist)?;
    if !metadata.file_type().is_file() || std::fs::read(plist)? != contents {
        return Err(std::io::Error::other("the file changed since it was checked"));
    }
    std::fs::remove_file(plist)
}

/// The Mac itself.
pub struct MacOs;

impl System for MacOs {
    fn status(&self) -> Status {
        login_item::status()
    }

    fn register(&self) -> Result<(), login_item::Error> {
        login_item::register()
    }

    fn unregister(&self) -> Result<(), login_item::Error> {
        login_item::unregister()
    }

    fn legacy_status(&self, plist: &Path) -> Status {
        login_item::legacy_status(plist)
    }

    fn job(&self) -> Job {
        let service = format!("gui/{}/{LEGACY_LABEL}", crate::sys::system::uid());
        let Ok(output) = std::process::Command::new("/bin/launchctl").args(["print", &service]).output() else {
            return Job::Unknown;
        };
        parse_job(output.status.code(), &String::from_utf8_lossy(&output.stdout))
    }

    fn bootout(&self) -> bool {
        let service = format!("gui/{}/{LEGACY_LABEL}", crate::sys::system::uid());
        std::process::Command::new("/bin/launchctl")
            .args(["bootout", &service])
            .status()
            .is_ok_and(|status| status.success())
    }

    fn bundle_id(&self, bundle: &Path) -> Option<String> {
        crate::sys::system::bundle_value(bundle, "CFBundleIdentifier")
    }

    fn bundle(&self) -> Option<PathBuf> {
        crate::sys::system::bundle_path()
    }

    fn temporary_location(&self) -> bool {
        use crate::sys::system;
        system::bundle_path().is_some_and(|bundle| system::is_temporary_location(&bundle, &system::mounts()))
    }
}

/// `launchctl print gui/<uid>/<label>`: exit status 113 is "no such service"; a loaded one lists
/// `pid = <n>` while a process runs under it, and `state = not running` when none does.
fn parse_job(code: Option<i32>, stdout: &str) -> Job {
    match code {
        Some(113) => Job::NotLoaded,
        Some(0) => {
            let lines = || stdout.lines().map(str::trim);
            let pid = lines().filter_map(|line| line.strip_prefix("pid = ")).find_map(|pid| pid.trim().parse().ok());
            let idle = pid.is_none() && lines().any(|line| line == "state = not running");
            Job::Loaded { pid, idle }
        }
        _ => Job::Unknown,
    }
}

/// Debug builds only: `BADDEL_LOGIN_ITEM_FAKE=<file.json>` stands in for macOS, so every state the
/// row can show is reachable without registering anything for real. The file holds
/// `{"status": "enabled" | "notRegistered" | "requiresApproval" | "notFound", "refuse": <code>}`;
/// `refuse` makes the next register or unregister fail with that `kSMError…` code.
#[cfg(debug_assertions)]
pub mod fake {
    use super::*;

    pub struct FileBacked(pub PathBuf);

    impl FileBacked {
        fn read(&self) -> serde_json::Value {
            std::fs::read(&self.0).ok().and_then(|b| serde_json::from_slice(&b).ok()).unwrap_or_default()
        }

        fn set(&self, status: &str) -> Result<(), login_item::Error> {
            let mut value = self.read();
            if let Some(code) = value.get("refuse").and_then(|c| c.as_i64()) {
                return Err(login_item::Error { code, domain: "fake".into() });
            }
            value["status"] = status.into();
            let _ = std::fs::write(&self.0, value.to_string());
            Ok(())
        }
    }

    impl System for FileBacked {
        fn status(&self) -> Status {
            match self.read().get("status").and_then(|s| s.as_str()) {
                Some("enabled") => Status::Enabled,
                Some("requiresApproval") => Status::RequiresApproval,
                Some("notFound") => Status::NotFound,
                _ => Status::NotRegistered,
            }
        }
        fn register(&self) -> Result<(), login_item::Error> {
            self.set("enabled")
        }
        fn unregister(&self) -> Result<(), login_item::Error> {
            self.set("notRegistered")
        }
        fn legacy_status(&self, _: &Path) -> Status {
            Status::Enabled
        }
        fn job(&self) -> Job {
            Job::NotLoaded
        }
        fn bootout(&self) -> bool {
            false
        }
        fn bundle_id(&self, bundle: &Path) -> Option<String> {
            MacOs.bundle_id(bundle)
        }
        fn bundle(&self) -> Option<PathBuf> {
            MacOs.bundle()
        }
        fn temporary_location(&self) -> bool {
            MacOs.temporary_location()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
    use std::sync::Arc;

    const ID: &str = "com.isltanx.baddel";

    /// macOS as a test wants it, counting what was asked of it.
    struct Fake {
        status: Mutex<Status>,
        legacy: Status,
        /// The error code the next register / unregister fails with.
        refuse: Mutex<Option<i64>>,
        /// What a register leaves: enabled, or waiting for approval.
        registers_as: Status,
        /// An unregister that reports success while macOS goes on listing the app.
        sticky: bool,
        job: Job,
        /// The bundle this Baddel runs from.
        running_from: Option<PathBuf>,
        temporary: AtomicBool,
        registers: AtomicU32,
        unregisters: AtomicU32,
        bootouts: AtomicU32,
        bundles: Vec<(PathBuf, String)>,
    }

    impl Fake {
        fn new(status: Status) -> Self {
            Fake {
                status: Mutex::new(status),
                legacy: Status::Enabled,
                refuse: Mutex::new(None),
                registers_as: Status::Enabled,
                sticky: false,
                job: Job::NotLoaded,
                running_from: None,
                temporary: AtomicBool::new(false),
                registers: AtomicU32::new(0),
                unregisters: AtomicU32::new(0),
                bootouts: AtomicU32::new(0),
                bundles: Vec::new(),
            }
        }

        fn registers(&self) -> u32 {
            self.registers.load(Ordering::SeqCst)
        }
    }

    impl System for Arc<Fake> {
        fn status(&self) -> Status {
            *self.status.lock().unwrap()
        }
        fn register(&self) -> Result<(), login_item::Error> {
            self.registers.fetch_add(1, Ordering::SeqCst);
            if let Some(code) = self.refuse.lock().unwrap().take() {
                return Err(login_item::Error { code, domain: "test".into() });
            }
            *self.status.lock().unwrap() = self.registers_as;
            Ok(())
        }
        fn unregister(&self) -> Result<(), login_item::Error> {
            self.unregisters.fetch_add(1, Ordering::SeqCst);
            if let Some(code) = self.refuse.lock().unwrap().take() {
                return Err(login_item::Error { code, domain: "test".into() });
            }
            if !self.sticky {
                *self.status.lock().unwrap() = Status::NotRegistered;
            }
            Ok(())
        }
        fn legacy_status(&self, _: &Path) -> Status {
            self.legacy
        }
        fn job(&self) -> Job {
            self.job
        }
        fn bootout(&self) -> bool {
            self.bootouts.fetch_add(1, Ordering::SeqCst);
            true
        }
        fn bundle_id(&self, bundle: &Path) -> Option<String> {
            self.bundles.iter().find(|(path, _)| path == bundle).map(|(_, id)| id.clone())
        }
        fn bundle(&self) -> Option<PathBuf> {
            self.running_from.clone()
        }
        fn temporary_location(&self) -> bool {
            self.temporary.load(Ordering::SeqCst)
        }
    }

    /// A home folder of its own for each test, removed when it ends.
    struct Home(PathBuf);

    impl Home {
        fn new(name: &str) -> Self {
            let dir = std::env::temp_dir().join(format!("baddel-startup-{}-{name}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(dir.join("Library/LaunchAgents")).unwrap();
            Home(dir)
        }
        fn plist(&self) -> PathBuf {
            self.0.join("Library/LaunchAgents/Baddel.plist")
        }
        fn write_legacy(&self, program: &str) {
            std::fs::write(self.plist(), legacy_contents(program)).unwrap();
        }
        /// Where the old agent's Baddel was, inside this home: a bundle since moved or deleted,
        /// so the test never depends on what is installed on the Mac running it.
        fn program(&self) -> String {
            self.0.join("Applications/Baddel.app/Contents/MacOS/baddel").to_string_lossy().into_owned()
        }
        fn write_ours(&self) {
            self.write_legacy(&self.program());
        }
    }

    impl Drop for Home {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }

    fn startup(home: &Home, fake: Fake) -> (Startup, Arc<Fake>) {
        let fake = Arc::new(fake);
        (Startup::new(Box::new(fake.clone()), Some(home.0.clone()), ID.into()), fake)
    }

    #[test]
    fn the_file_2_0_2_wrote_is_recognised_byte_for_byte() {
        // The exact output of auto-launch 0.5.0's `enable` for Baddel (src/macos.rs there).
        let written = "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n<plist version=\"1.0\">\n  <dict>\n  <key>Label</key>\n  <string>Baddel</string>\n  <key>ProgramArguments</key>\n  <array><string>/Applications/Baddel.app/Contents/MacOS/baddel</string></array>\n  <key>RunAtLoad</key>\n  <true/>\n  </dict>\n</plist>";
        assert_eq!(legacy_contents("/Applications/Baddel.app/Contents/MacOS/baddel"), written);
    }

    #[test]
    fn an_old_agent_that_was_on_becomes_the_login_item_and_goes() {
        let home = Home::new("migrate");
        home.write_ours();
        let (startup, fake) = startup(&home, Fake::new(Status::NotRegistered));
        let done = startup.reconcile(true, Source::File);
        assert_eq!(done.legacy, LegacyOutcome::Migrated);
        assert_eq!(done.status, StartupStatus { state: State::Enabled, failure: None, legacy: None });
        assert_eq!(done.preference, Some(true));
        assert!(!home.plist().exists());
        assert_eq!(fake.registers(), 1);
        // Done once: the next launch has nothing to move, and registers nothing.
        assert_eq!(startup.reconcile(true, Source::File).legacy, LegacyOutcome::None);
        assert_eq!(fake.registers(), 1);
    }

    #[test]
    fn a_refused_registration_keeps_the_old_agent_and_the_next_launch_tries_again() {
        let home = Home::new("retry");
        home.write_ours();
        let (startup, fake) = startup(&home, Fake::new(Status::NotRegistered));
        *fake.refuse.lock().unwrap() = Some(login_item::LAUNCH_DENIED_BY_USER);
        let first = startup.reconcile(true, Source::File);
        assert_eq!(first.legacy, LegacyOutcome::Kept);
        assert_eq!(first.status.state, State::Disabled);
        assert_eq!(first.status.failure, Some(Failure { problem: Problem::DeniedByUser, code: Some(11) }));
        assert_eq!(first.status.legacy, Some(Legacy::Kept));
        // The old agent still starts Baddel at login meanwhile, and the choice it stands for is
        // not overwritten by the refusal (found by the live check: the next launch then dropped it).
        assert!(home.plist().exists());
        assert_eq!(first.preference, None);
        // macOS no longer refuses: the next launch completes the move.
        let second = startup.reconcile(true, Source::File);
        assert_eq!(second.legacy, LegacyOutcome::Migrated);
        assert_eq!(second.status, StartupStatus { state: State::Enabled, failure: None, legacy: None });
        assert!(!home.plist().exists());
    }

    #[test]
    fn a_login_item_waiting_for_approval_does_not_replace_the_old_agent_yet() {
        let home = Home::new("approval");
        home.write_ours();
        let mut fake = Fake::new(Status::NotRegistered);
        fake.registers_as = Status::RequiresApproval;
        let (startup, fake) = startup(&home, fake);
        let done = startup.reconcile(true, Source::File);
        assert_eq!(done.legacy, LegacyOutcome::Kept);
        assert_eq!(done.status.state, State::RequiresApproval);
        assert_eq!(done.preference, None);
        assert!(home.plist().exists());
        // Approved in System Settings: the next launch removes the old one, registering nothing more.
        *fake.status.lock().unwrap() = Status::Enabled;
        assert_eq!(startup.reconcile(true, Source::File).legacy, LegacyOutcome::Migrated);
        assert_eq!(fake.registers(), 1);
        assert!(!home.plist().exists());
    }

    #[test]
    fn an_old_agent_the_user_switched_off_is_removed_and_not_replaced() {
        let home = Home::new("switched-off");
        home.write_ours();
        let mut fake = Fake::new(Status::NotRegistered);
        fake.legacy = Status::RequiresApproval;
        let (startup, fake) = startup(&home, fake);
        let done = startup.reconcile(true, Source::File);
        assert_eq!(done.legacy, LegacyOutcome::Removed);
        assert_eq!(done.status.state, State::Disabled);
        assert_eq!(done.preference, Some(false));
        assert_eq!(fake.registers(), 0);
        assert!(!home.plist().exists());
    }

    #[test]
    fn an_old_agent_left_over_with_the_option_off_is_removed() {
        let home = Home::new("leftover");
        home.write_ours();
        let (startup, fake) = startup(&home, Fake::new(Status::NotRegistered));
        let done = startup.reconcile(false, Source::File);
        assert_eq!(done.legacy, LegacyOutcome::Removed);
        assert_eq!(fake.registers(), 0);
        assert!(!home.plist().exists());
    }

    #[test]
    fn unreadable_settings_are_no_reason_to_remove_a_working_registration() {
        // The defaults stand in for unreadable settings, and they say off.
        let home = Home::new("unreadable");
        home.write_ours();
        let (startup, _) = startup(&home, Fake::new(Status::NotRegistered));
        let done = startup.reconcile(false, Source::Unreadable);
        // The old agent was on: it is carried over, not dropped.
        assert_eq!(done.legacy, LegacyOutcome::Migrated);
        assert_eq!(done.status.state, State::Enabled);
        // And a login item already on is never touched at launch, whatever the settings say.
        let home = Home::new("unreadable-enabled");
        let (startup, fake) = super::tests::startup(&home, Fake::new(Status::Enabled));
        let done = startup.reconcile(false, Source::Unreadable);
        assert_eq!(done.status.state, State::Enabled);
        assert_eq!(fake.unregisters.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn what_macos_says_wins_over_the_saved_preference() {
        let home = Home::new("external");
        // Switched off in System Settings while the preference said on: off it is, and stays off.
        let (startup, fake) = startup(&home, Fake::new(Status::NotRegistered));
        let done = startup.reconcile(true, Source::File);
        assert_eq!(done.status.state, State::Disabled);
        assert_eq!(done.preference, Some(false));
        assert_eq!(fake.registers(), 0);
        // Added by hand in System Settings while the preference said off (as the 2026-10-26 audit
        // found it): shown as on, and not removed.
        let (startup, fake) = super::tests::startup(&home, Fake::new(Status::Enabled));
        let done = startup.reconcile(false, Source::File);
        assert_eq!(done.status.state, State::Enabled);
        assert_eq!(done.preference, Some(true));
        assert_eq!(fake.unregisters.load(Ordering::SeqCst), 0);
        // macOS has no answer: the preference is left as it was.
        let (startup, _) = super::tests::startup(&home, Fake::new(Status::NotFound));
        let done = startup.reconcile(true, Source::File);
        assert_eq!((done.status.state, done.preference), (State::Unavailable, None));
    }

    #[test]
    fn anything_else_at_that_path_is_left_alone() {
        type Make = Box<dyn Fn(&Home)>;
        let cases: [(&str, Make); 6] = [
            ("extra-key", Box::new(|h: &Home| {
                let edited = legacy_contents(&h.program())
                    .replace("<key>RunAtLoad</key>", "<key>KeepAlive</key>\n  <true/>\n  <key>RunAtLoad</key>");
                std::fs::write(h.plist(), edited).unwrap();
            })),
            ("other-program", Box::new(|h: &Home| h.write_legacy("/usr/local/bin/baddel"))),
            ("relative-program", Box::new(|h: &Home| h.write_legacy("Baddel.app/Contents/MacOS/baddel"))),
            ("other-label", Box::new(|h: &Home| {
                let edited = legacy_contents(&h.program()).replace("<string>Baddel</string>", "<string>Other</string>");
                std::fs::write(h.plist(), edited).unwrap();
            })),
            ("symlink", Box::new(|h: &Home| {
                let real = h.0.join("real.plist");
                std::fs::write(&real, legacy_contents(&h.program())).unwrap();
                std::os::unix::fs::symlink(&real, h.plist()).unwrap();
            })),
            ("directory", Box::new(|h: &Home| std::fs::create_dir(h.plist()).unwrap())),
        ];
        for (name, make) in cases {
            let home = Home::new(name);
            make(&home);
            let (startup, fake) = startup(&home, Fake::new(Status::NotRegistered));
            let done = startup.reconcile(true, Source::File);
            assert_eq!(done.legacy, LegacyOutcome::Unverified, "{name}");
            assert_eq!(done.status.legacy, Some(Legacy::Unverified), "{name}");
            assert!(std::fs::symlink_metadata(home.plist()).is_ok(), "{name}: left in place");
            assert_eq!(fake.registers(), 0, "{name}");
        }
    }

    #[test]
    fn an_agent_for_another_apps_bundle_is_not_ours() {
        let home = Home::new("other-bundle");
        let bundle = home.0.join("Baddel.app");
        std::fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
        let program = bundle.join("Contents/MacOS/baddel");
        home.write_legacy(program.to_str().unwrap());
        let mut fake = Fake::new(Status::NotRegistered);
        fake.bundles.push((bundle.clone(), "com.example.other".into()));
        let (startup, _) = startup(&home, fake);
        assert_eq!(startup.reconcile(true, Source::File).legacy, LegacyOutcome::Unverified);
        assert!(home.plist().exists());
        // The same file for Baddel's own bundle is ours — and moved over by that copy of Baddel.
        let mut fake = Fake::new(Status::NotRegistered);
        fake.bundles.push((bundle.clone(), ID.into()));
        fake.running_from = Some(bundle);
        let (startup, _) = super::tests::startup(&home, fake);
        assert_eq!(startup.reconcile(true, Source::File).legacy, LegacyOutcome::Migrated);
    }

    #[test]
    fn a_file_changed_between_the_check_and_the_removal_is_kept() {
        let home = Home::new("changed");
        home.write_ours();
        let checked = std::fs::read(home.plist()).unwrap();
        std::fs::write(home.plist(), b"edited since").unwrap();
        assert!(remove_legacy(&home.plist(), &checked).is_err());
        assert_eq!(std::fs::read(home.plist()).unwrap(), b"edited since");
    }

    #[test]
    fn a_loaded_job_is_unloaded_only_when_nothing_runs_under_it() {
        for (job, bootouts) in [
            (Job::Loaded { pid: None, idle: true }, 1),
            // A process runs under it — Baddel itself, started by it at login: unloading it would
            // end that process. It goes at logout.
            (Job::Loaded { pid: Some(4242), idle: false }, 0),
            // launchd did not say plainly that nothing runs: left loaded.
            (Job::Loaded { pid: None, idle: false }, 0),
            (Job::NotLoaded, 0),
            (Job::Unknown, 0),
        ] {
            let home = Home::new("job");
            home.write_ours();
            let mut fake = Fake::new(Status::Enabled);
            fake.job = job;
            let (startup, fake) = startup(&home, fake);
            assert_eq!(startup.reconcile(true, Source::File).legacy, LegacyOutcome::Migrated, "{job:?}");
            assert_eq!(fake.bootouts.load(Ordering::SeqCst), bootouts, "{job:?}");
        }
    }

    #[test]
    fn the_switch_reports_what_macos_did_not_what_was_asked() {
        let home = Home::new("switch");
        let (startup, fake) = startup(&home, Fake::new(Status::NotRegistered));
        assert_eq!(startup.set_enabled(true), StartupStatus { state: State::Enabled, failure: None, legacy: None });
        // Refused: the error is said, and the switch shows what macOS has.
        *fake.refuse.lock().unwrap() = Some(login_item::INVALID_SIGNATURE);
        let refused = startup.set_enabled(false);
        assert_eq!(refused.state, State::Enabled);
        assert_eq!(refused.failure, Some(Failure { problem: Problem::InvalidSignature, code: Some(3) }));
        // The next change that works clears it.
        assert_eq!(startup.set_enabled(false), StartupStatus { state: State::Disabled, failure: None, legacy: None });
        // Already in the state asked for is no failure.
        *fake.refuse.lock().unwrap() = Some(login_item::JOB_NOT_FOUND);
        assert_eq!(startup.set_enabled(false).failure, None);
        *fake.status.lock().unwrap() = Status::Enabled;
        *fake.refuse.lock().unwrap() = Some(login_item::ALREADY_REGISTERED);
        assert_eq!(startup.set_enabled(true).failure, None);
    }

    #[test]
    fn switching_off_a_login_item_that_stays_listed_says_so() {
        let home = Home::new("still-on");
        let mut fake = Fake::new(Status::Enabled);
        fake.sticky = true;
        let (startup, _) = startup(&home, fake);
        let status = startup.set_enabled(false);
        assert_eq!(status.state, State::Enabled);
        assert_eq!(status.failure, Some(Failure::new(Problem::StillEnabled)));
    }

    #[test]
    fn switching_off_also_retires_an_old_agent_kept_while_waiting_for_approval() {
        let home = Home::new("off-retires");
        home.write_ours();
        let mut fake = Fake::new(Status::NotRegistered);
        fake.registers_as = Status::RequiresApproval;
        let (startup, _) = startup(&home, fake);
        assert_eq!(startup.reconcile(true, Source::File).legacy, LegacyOutcome::Kept);
        let off = startup.set_enabled(false);
        assert_eq!(off, StartupStatus { state: State::Disabled, failure: None, legacy: None });
        assert!(!home.plist().exists());
    }

    #[test]
    fn a_copy_that_will_not_last_or_another_copy_does_not_replace_the_old_agent() {
        // Reviewer finding: the login item would be the copy running now — off a disk image, or
        // another copy than the one the old agent starts — and the old agent would be gone.
        let home = Home::new("elsewhere");
        let bundle = home.0.join("Applications/Baddel.app");
        std::fs::create_dir_all(bundle.join("Contents/MacOS")).unwrap();
        home.write_ours();
        let mut fake = Fake::new(Status::NotRegistered);
        fake.bundles.push((bundle, ID.into()));
        fake.running_from = Some(home.0.join("Downloads/Baddel.app"));
        let (startup, fake) = startup(&home, fake);
        let done = startup.reconcile(true, Source::File);
        assert_eq!((done.legacy, done.preference), (LegacyOutcome::Kept, None));
        assert_eq!(fake.registers(), 0);
        assert!(home.plist().exists());
        // Off a disk image, with the agent's own bundle gone: kept too.
        let home = Home::new("dmg-launch");
        home.write_ours();
        let (startup, fake) = super::tests::startup(&home, Fake::new(Status::NotRegistered));
        fake.temporary.store(true, Ordering::SeqCst);
        assert_eq!(startup.reconcile(true, Source::File).legacy, LegacyOutcome::Kept);
        assert!(home.plist().exists());
    }

    #[test]
    fn an_agent_of_unknown_origin_leaves_the_preference_alone() {
        let home = Home::new("unknown-pref");
        std::fs::write(home.plist(), b"<plist>someone else's</plist>").unwrap();
        let (startup, _) = startup(&home, Fake::new(Status::NotRegistered));
        assert_eq!(startup.reconcile(true, Source::File).preference, None);
    }

    #[test]
    fn without_a_home_folder_there_is_no_old_agent_to_look_for() {
        let fake = Arc::new(Fake::new(Status::Enabled));
        let startup = Startup::new(Box::new(fake.clone()), None, ID.into());
        let done = startup.reconcile(false, Source::File);
        assert_eq!((done.legacy, done.status.state), (LegacyOutcome::None, State::Enabled));
    }

    #[test]
    fn a_disk_image_copy_is_not_registered() {
        let home = Home::new("dmg");
        let (startup, fake) = startup(&home, Fake::new(Status::NotRegistered));
        fake.temporary.store(true, Ordering::SeqCst);
        let status = startup.set_enabled(true);
        assert_eq!(status.state, State::Disabled);
        assert_eq!(status.failure, Some(Failure::new(Problem::TemporaryLocation)));
        assert_eq!(fake.registers(), 0);
    }

    #[test]
    fn launchctl_answers_are_read_as_launchd_gives_them() {
        assert_eq!(parse_job(Some(113), ""), Job::NotLoaded);
        let running = "gui/501/Baddel = {\n\tstate = running\n\tpid = 15783\n}";
        assert_eq!(parse_job(Some(0), running), Job::Loaded { pid: Some(15783), idle: false });
        let idle = "gui/501/Baddel = {\n\tstate = not running\n}";
        assert_eq!(parse_job(Some(0), idle), Job::Loaded { pid: None, idle: true });
        // An answer in a shape not known: never taken for idle.
        assert_eq!(parse_job(Some(0), "gui/501/Baddel = {\n\tstatus = ?\n}"), Job::Loaded { pid: None, idle: false });
        assert_eq!(parse_job(Some(5), ""), Job::Unknown);
        assert_eq!(parse_job(None, ""), Job::Unknown);
    }
}
