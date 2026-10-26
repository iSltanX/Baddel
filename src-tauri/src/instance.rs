//! One Baddel per user.
//!
//! The first process to start takes an exclusive lock (`flock`) on a file in Baddel's own
//! Application Support folder, and holds it for as long as it runs; the system lets go of it when
//! the process ends, however it ends. Taking it is atomic, so of two launches at the same instant —
//! a login item and an older LaunchAgent, two copies of the app, `open -n` — exactly one gets it.
//! This happens first thing in [`crate::run`], before the menu bar icon, the shortcuts, the
//! login-item reconciliation or any write to the settings.
//!
//! Any other launch hands its request to the one running, over a socket beside the lock, and
//! ends. A launch by hand asks for Settings, as opening a running app does ([`Request::Reopen`]);
//! one by launchd at login asks for nothing ([`Request::Background`]). A launch that finds the
//! lock held but nobody to take its request — the owner is on its way out, as when an update
//! restarts Baddel ([`Held::step_aside`]) — keeps trying for the lock, and owns once it is free.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Read, Write};
use std::os::unix::fs::FileTypeExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{Duration, Instant};

const LOCK: &str = "instance.lock";
const SOCKET: &str = "instance.sock";
/// How long a second launch waits for the running Baddel to take its request.
const HANDOFF_TIMEOUT: Duration = Duration::from_secs(3);
/// A socket path longer than macOS's `sun_path` (104 bytes with its NUL) cannot be bound.
const SOCKET_PATH_MAX: usize = 103;
/// The label Baddel 1.0–2.0.2 gave its LaunchAgent, which launchd puts in `XPC_SERVICE_NAME`.
const LEGACY_AGENT: &str = "Baddel";

/// What a second launch asks of the Baddel already running.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Request {
    /// Opened by hand: show Settings, as reopening a running app does.
    Reopen,
    /// Started by launchd at login, beside the one already running: nothing to show.
    Background,
}

impl Request {
    fn word(self) -> &'static str {
        match self {
            Request::Reopen => "reopen",
            Request::Background => "background",
        }
    }

    fn parse(word: &str) -> Option<Self> {
        match word.trim() {
            "reopen" => Some(Request::Reopen),
            "background" => Some(Request::Background),
            _ => None,
        }
    }

    /// How this process was started, as far as can be told before AppKit is up: launchd names the
    /// job it runs in `XPC_SERVICE_NAME`, and the old LaunchAgent's job is `Baddel`. A login item
    /// opened through Launch Services cannot be told from a launch by hand here; the running
    /// Baddel tells it by time instead (see `lib.rs`).
    pub fn of_this_launch() -> Self {
        Self::of_service(std::env::var("XPC_SERVICE_NAME").ok().as_deref())
    }

    fn of_service(service: Option<&str>) -> Self {
        if service == Some(LEGACY_AGENT) {
            Request::Background
        } else {
            Request::Reopen
        }
    }
}

/// The result of [`claim`].
pub enum Claim {
    /// This process is Baddel from now on. Hand it to [`Owner::serve`] once the app can act.
    Owner(Owner),
    /// Another Baddel runs. `delivered` says whether it took the request; either way this
    /// process must end now, before it has done anything.
    Running { delivered: bool },
}

/// The lock, and the socket other launches reach this one through.
pub struct Owner {
    lock: File,
    listener: Option<UnixListener>,
    socket: Option<PathBuf>,
}

/// Where Baddel keeps its lock and socket: its own Application Support folder, the one Tauri
/// gives it (`~/Library/Application Support/<identifier>`).
pub fn folder(home: &Path, identifier: &str) -> PathBuf {
    home.join("Library/Application Support").join(identifier)
}

/// Becomes the one Baddel, or hands `request` to the one that already is.
pub fn claim(dir: &Path, request: Request) -> std::io::Result<Claim> {
    std::fs::create_dir_all(dir)?;
    let lock = open_lock(&dir.join(LOCK))?;
    let deadline = Instant::now() + HANDOFF_TIMEOUT;
    loop {
        match lock.try_lock() {
            Ok(()) => break,
            Err(std::fs::TryLockError::WouldBlock) => {}
            Err(std::fs::TryLockError::Error(error)) => return Err(error),
        }
        if socket_path(dir).is_some_and(|socket| hand_over(&socket, request, deadline)) {
            return Ok(Claim::Running { delivered: true });
        }
        // Nobody took it: the owner may be quitting. The lock is tried again until it is free,
        // or the time is up — and then this launch ends all the same, never a second owner.
        if Instant::now() >= deadline {
            return Ok(Claim::Running { delivered: false });
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    // Holding the lock, any socket left at that path is a dead owner's.
    let socket = socket_path(dir);
    let listener = socket.as_deref().and_then(|path| {
        remove_stale_socket(path);
        UnixListener::bind(path).ok()
    });
    let socket = socket.filter(|_| listener.is_some());
    Ok(Claim::Owner(Owner { lock, listener, socket }))
}

/// Opens (or creates) the lock file without following a link put in its place.
fn open_lock(path: &Path) -> std::io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    /// `O_NOFOLLOW` on macOS.
    const NO_FOLLOW: i32 = 0x0100;
    OpenOptions::new().read(true).write(true).create(true).truncate(false).custom_flags(NO_FOLLOW).open(path)
}

/// Beside the lock, or — where that path is too long for a socket — in this user's private
/// temporary folder, under a name drawn from the lock's folder, so every launch with that folder
/// finds the same one and no other Baddel's.
fn socket_path(dir: &Path) -> Option<PathBuf> {
    let beside = dir.join(SOCKET);
    if beside.as_os_str().len() <= SOCKET_PATH_MAX {
        return Some(beside);
    }
    let elsewhere = crate::sys::system::user_temp_dir()?.join(format!("baddel-{:016x}.sock", fnv1a(dir)));
    (elsewhere.as_os_str().len() <= SOCKET_PATH_MAX).then_some(elsewhere)
}

/// FNV-1a over the folder's path: the same in every process and every build, unlike std's hasher.
fn fnv1a(dir: &Path) -> u64 {
    dir.as_os_str().as_encoded_bytes().iter().fold(0xcbf2_9ce4_8422_2325, |hash, &byte| {
        (hash ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// Removes what is at `path` only if it is a socket: never a file or a link someone put there.
fn remove_stale_socket(path: &Path) {
    if std::fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_socket()) {
        let _ = std::fs::remove_file(path);
    }
}

/// One attempt to give `request` to the Baddel listening on `socket`: `true` once it answered.
/// The owner may have taken the lock an instant ago and not be listening yet; the caller tries
/// again until `deadline`.
fn hand_over(socket: &Path, request: Request, deadline: Instant) -> bool {
    let Ok(mut stream) = UnixStream::connect(socket) else { return false };
    let left = deadline.saturating_duration_since(Instant::now()).clamp(Duration::from_millis(100), Duration::from_secs(1));
    let _ = stream.set_read_timeout(Some(left));
    let _ = stream.set_write_timeout(Some(left));
    if stream.write_all(format!("{}\n", request.word()).as_bytes()).is_err() {
        return false;
    }
    let mut answer = String::new();
    BufReader::new(stream).read_line(&mut answer).is_ok() && answer.trim() == "ok"
}

/// The lock and socket of the running Baddel, kept in Tauri state for as long as it runs.
pub struct Held {
    lock: File,
    socket: Mutex<Option<PathBuf>>,
}

impl Owner {
    /// Keeps the lock and answers other launches, passing each request to `handler` (on a thread
    /// of its own).
    pub fn serve(self, handler: impl Fn(Request) + Send + 'static) -> Held {
        let Owner { lock, listener, socket } = self;
        if let Some(listener) = listener {
            let _ = std::thread::Builder::new().name("baddel-instance".into()).spawn(move || {
                for stream in listener.incoming().flatten() {
                    if let Some(request) = read_request(stream) {
                        handler(request);
                    }
                }
            });
        }
        Held { lock, socket: Mutex::new(socket) }
    }
}

impl Held {
    /// Just before Baddel restarts into an update: the socket goes, so the new process is not
    /// handed back to this one, and the lock is let go, so the new process owns from its first
    /// moment. This one takes no further part.
    pub fn step_aside(&self) {
        self.release();
        let _ = self.lock.unlock();
    }

    /// Removes the socket as Baddel quits. The lock goes with the process.
    pub fn release(&self) {
        if let Some(path) = self.socket.lock().unwrap().take() {
            remove_stale_socket(&path);
        }
    }
}

/// One line, at most a few bytes, then "ok".
fn read_request(stream: UnixStream) -> Option<Request> {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(1)));
    let mut line = String::new();
    BufReader::new((&stream).take(32)).read_line(&mut line).ok()?;
    let request = Request::parse(&line)?;
    let _ = (&stream).write_all(b"ok\n");
    Some(request)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::mpsc;

    fn dir(name: &str) -> PathBuf {
        let dir = std::env::temp_dir().join(format!("bi-{}-{name}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    /// One of these tests at a time. A process spawned while another test holds a lock shares the
    /// lock's open file until it execs, which holds that lock a moment past its `drop`.
    static SERIAL: Mutex<()> = Mutex::new(());

    fn serial() -> std::sync::MutexGuard<'static, ()> {
        SERIAL.lock().unwrap_or_else(|poisoned| poisoned.into_inner())
    }

    #[test]
    fn the_second_launch_hands_its_request_to_the_first_and_does_not_own() {
        let _serial = serial();
        let dir = dir("pair");
        let Ok(Claim::Owner(owner)) = claim(&dir, Request::Reopen) else { panic!("the first launch owns") };
        let (tx, rx) = mpsc::channel();
        // `serve` keeps the lock for the life of the test process; another test's folder is its own.
        let lock = owner.lock.try_clone().unwrap();
        let listener = owner.listener.unwrap();
        std::thread::spawn(move || {
            let _lock = lock;
            for stream in listener.incoming().flatten() {
                if let Some(request) = read_request(stream) {
                    let _ = tx.send(request);
                }
            }
        });
        let Ok(Claim::Running { delivered }) = claim(&dir, Request::Reopen) else { panic!("the second does not own") };
        assert!(delivered);
        assert_eq!(rx.recv_timeout(Duration::from_secs(2)).unwrap(), Request::Reopen);
        let Ok(Claim::Running { delivered: true }) = claim(&dir, Request::Background) else { panic!() };
        assert_eq!(rx.recv_timeout(Duration::from_secs(2)).unwrap(), Request::Background);
    }

    #[test]
    fn a_lock_left_by_a_process_that_ended_is_free_again() {
        let _serial = serial();
        let dir = dir("released");
        {
            let Ok(Claim::Owner(owner)) = claim(&dir, Request::Reopen) else { panic!() };
            drop(owner);
        }
        // The socket of the owner that went is stale; the new owner binds its own.
        assert!(matches!(claim(&dir, Request::Reopen), Ok(Claim::Owner(_))));
    }

    #[test]
    fn an_owner_restarting_into_an_update_hands_over_ownership_not_the_request() {
        let _serial = serial();
        let dir = dir("restart");
        let Ok(Claim::Owner(owner)) = claim(&dir, Request::Reopen) else { panic!() };
        let (tx, rx) = mpsc::channel();
        let held = owner.serve(move |request| {
            let _ = tx.send(request);
        });
        // As `app.restart()` does after an update: the old process steps aside, then the new one
        // starts while the old one is still on its way out.
        held.step_aside();
        assert!(matches!(claim(&dir, Request::Reopen), Ok(Claim::Owner(_))));
        // Nothing was handed to the old one, which would have taken it with it.
        assert!(rx.recv_timeout(Duration::from_millis(200)).is_err());
        // Its quitting afterwards does not take the new owner's socket away.
        held.release();
    }

    #[test]
    fn a_launch_while_the_owner_quits_owns_once_the_lock_is_free() {
        let _serial = serial();
        let dir = dir("quitting");
        let Ok(Claim::Owner(owner)) = claim(&dir, Request::Reopen) else { panic!() };
        // Bound but not answering, as in the moments before a process ends.
        let quitting = std::thread::spawn(move || {
            std::thread::sleep(Duration::from_millis(300));
            drop(owner);
        });
        assert!(matches!(claim(&dir, Request::Reopen), Ok(Claim::Owner(_))));
        quitting.join().unwrap();
    }

    #[test]
    fn nothing_but_a_socket_is_removed_from_the_socket_path() {
        let _serial = serial();
        let dir = dir("not-a-socket");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join(SOCKET), b"someone's file").unwrap();
        let Ok(Claim::Owner(owner)) = claim(&dir, Request::Reopen) else { panic!() };
        // Still the one Baddel, without a socket: the file is left exactly as it was.
        assert!(owner.listener.is_none());
        assert_eq!(std::fs::read(dir.join(SOCKET)).unwrap(), b"someone's file");
    }

    #[test]
    fn a_link_in_place_of_the_lock_is_not_followed() {
        let _serial = serial();
        let dir = dir("link");
        std::fs::create_dir_all(&dir).unwrap();
        let elsewhere = dir.join("elsewhere");
        std::os::unix::fs::symlink(&elsewhere, dir.join(LOCK)).unwrap();
        assert!(claim(&dir, Request::Reopen).is_err());
        assert!(!elsewhere.exists());
    }

    #[test]
    fn a_folder_too_deep_for_a_socket_still_has_one_of_its_own() {
        let deep = std::env::temp_dir().join("x".repeat(120));
        let socket = socket_path(&deep).unwrap();
        assert!(socket.as_os_str().len() <= SOCKET_PATH_MAX);
        // The same for every launch with that folder, and another for another folder.
        assert_eq!(socket_path(&deep), Some(socket.clone()));
        assert_ne!(socket_path(&std::env::temp_dir().join("y".repeat(120))), Some(socket));
        // A short folder keeps its socket beside the lock.
        assert_eq!(socket_path(Path::new("/Users/me/Library/Application Support/x")).unwrap().file_name().unwrap(), SOCKET);
    }

    #[test]
    fn only_the_old_launch_agent_counts_as_a_background_launch() {
        assert_eq!(Request::of_service(Some("Baddel")), Request::Background);
        // Launch Services' own job for an app opened in Finder, or a terminal's.
        assert_eq!(Request::of_service(Some("application.com.isltanx.baddel.1356.1357")), Request::Reopen);
        assert_eq!(Request::of_service(Some("0")), Request::Reopen);
        assert_eq!(Request::of_service(None), Request::Reopen);
        assert_eq!(Request::parse("reopen\n"), Some(Request::Reopen));
        assert_eq!(Request::parse("open everything"), None);
    }

    /// Run by [`simultaneous_launches_leave_exactly_one_owner`] in separate processes; does nothing
    /// on its own.
    #[test]
    #[ignore]
    fn contender() {
        let (Ok(dir), Ok(start)) = (std::env::var("BADDEL_TEST_INSTANCE_DIR"), std::env::var("BADDEL_TEST_START_MS")) else {
            return;
        };
        let start: u128 = start.parse().unwrap();
        // All of them go at the same instant.
        while std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() < start {
            std::hint::spin_loop();
        }
        let dir = PathBuf::from(dir);
        match claim(&dir, Request::Reopen).unwrap() {
            Claim::Owner(owner) => {
                println!("RESULT owner {}", std::process::id());
                let log = dir.join("received");
                let held = owner.serve(move |request| {
                    let mut file = OpenOptions::new().create(true).append(true).open(&log).unwrap();
                    writeln!(file, "{}", request.word()).unwrap();
                });
                std::thread::sleep(Duration::from_secs(4));
                held.release();
            }
            Claim::Running { delivered } => println!("RESULT running delivered={delivered}"),
        }
    }

    /// Simultaneous independent launches, each its own process: one owner before anything else
    /// happens, and every other launch's request reaches it.
    #[test]
    fn simultaneous_launches_leave_exactly_one_owner() {
        let _serial = serial();
        const LAUNCHES: usize = 8;
        let dir = dir("race");
        let start = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap().as_millis() + 700;
        let exe = std::env::current_exe().unwrap();
        let children: Vec<_> = (0..LAUNCHES)
            .map(|_| {
                std::process::Command::new(&exe)
                    .args(["instance::tests::contender", "--exact", "--ignored", "--nocapture", "--test-threads=1"])
                    .env("BADDEL_TEST_INSTANCE_DIR", &dir)
                    .env("BADDEL_TEST_START_MS", start.to_string())
                    .stdout(std::process::Stdio::piped())
                    .spawn()
                    .unwrap()
            })
            .collect();
        let results: Vec<String> = children
            .into_iter()
            .map(|child| {
                let output = child.wait_with_output().unwrap();
                String::from_utf8_lossy(&output.stdout)
                    .lines()
                    // The harness prints the test's name on the same line, before what it prints.
                    .find_map(|line| line.split_once("RESULT ").map(|(_, result)| result.trim().to_string()))
                    .unwrap_or_default()
            })
            .collect();
        let owners = results.iter().filter(|r| r.starts_with("owner")).count();
        let delivered = results.iter().filter(|r| *r == "running delivered=true").count();
        assert_eq!(owners, 1, "{results:?}");
        assert_eq!(delivered, LAUNCHES - 1, "{results:?}");
        let received = std::fs::read_to_string(dir.join("received")).unwrap_or_default();
        assert_eq!(received.lines().count(), LAUNCHES - 1, "{received}");
        let _ = std::fs::remove_dir_all(&dir);
    }
}
