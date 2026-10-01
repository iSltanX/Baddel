//! The conversion itself: hotkey → read the text → convert → write it back.
//!
//! Runs on its own thread; commands arrive over a channel. Nothing the user typed
//! is ever logged or written to disk — the last operation lives in memory only,
//! for undo and for extending the conversion one word further back.
//!
//! The last operation remembers where it happened (the app, and the field when the
//! accessibility API names it), so undo and extend never act anywhere else, and an undo
//! that cannot go through right now does not forget it: the original stays on offer in
//! the menu until the undo window closes.

use std::collections::{HashMap, HashSet};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::thread::{self, sleep};
use std::time::{Duration, Instant};

use baddel_core::{Direction, LayoutMap};
use tauri::{AppHandle, Emitter, Manager};

use crate::diagnostics::{History, Path};
use crate::menu_text;
use crate::settings::{AppState, Language};
use crate::sys::frontmost::{self, App};
use crate::sys::input_source::Snapshot as Layouts;
use crate::sys::keysynth::{self, KEY_LEFT, KEY_RIGHT};
use crate::sys::text_access::{Focused, Selection};
use crate::sys::{input_source, on_main, pasteboard, permissions, sound, text_access};
use crate::tray::LastConversion;
use crate::{hud, settings, sync, tray};

// ── Timings ──────────────────────────────────────────────────────────────────
/// How long we wait for the user to release the hotkey's modifiers.
const MODIFIER_RELEASE_TIMEOUT: Duration = Duration::from_millis(700);
/// How long an app may take to put the selection on the pasteboard after ⌘C, until its own
/// answers have been timed (see [`Route`]).
const COPY_TIMEOUT: Duration = Duration::from_millis(250);
/// The shortest ⌘C wait a remembered route may bring the timeout down to.
const COPY_TIMEOUT_MIN: Duration = Duration::from_millis(80);
/// How many answered ⌘C presses an app needs before its own timing is trusted.
const COPY_SAMPLES: u32 = 3;
/// Minimum time for an app to act on a selection key before we look at the selection.
const SELECTION_SETTLE: Duration = Duration::from_millis(40);
/// How long we keep looking for the selection a key press should have produced. Apps handle
/// the key asynchronously; reading too early sees "nothing selected".
const SELECTION_TIMEOUT: Duration = Duration::from_millis(300);
/// How long an accessibility write may take to show up in the text before we call it dropped.
const WRITE_SETTLE: Duration = Duration::from_millis(100);
/// Time for an app to consume ⌘V before we put the user's pasteboard back.
const PASTE_SETTLE: Duration = Duration::from_millis(200);
/// How long a Chromium/Electron app gets to build its accessibility tree after we ask for it.
const AX_WAKE_TIMEOUT: Duration = Duration::from_millis(600);
/// Pressing the hotkey again within this window extends the last conversion.
const EXTEND_WINDOW: Duration = Duration::from_secs(2);
/// Undo is offered for this long after a conversion.
const UNDO_WINDOW: Duration = Duration::from_secs(30);
/// The missing-permission notice is shown at most this often: the menu bar icon says it too.
const PERMISSION_NOTICE_EVERY: Duration = Duration::from_secs(30);

const MAX_CHARS: usize = 10_000;
/// Longer than this, a conversion is not shown word for word in the notice or the menu: the
/// notice gives its length and the menu a plain "Undo Last Conversion". The text itself still
/// converts in full; this only keeps paragraphs off the screen.
const DISPLAY_MAX: usize = 24;

/// Whether a conversion is short enough to show as it is.
pub fn fits_display(original: &str, result: &str) -> bool {
    original.chars().count() <= DISPLAY_MAX && result.chars().count() <= DISPLAY_MAX
}
/// How far back from the caret we read when looking for the previous word (UTF-16 units).
const LOOKBEHIND_UNITS: usize = 400;
/// On the key-based path, undo reselects the result character by character up to this length;
/// a longer result within one paragraph is reselected to the paragraph's start instead.
const MAX_UNDO_CHARS: usize = 300;

/// Debug-build tracing of which path a conversion took. Never prints user text.
macro_rules! trace {
    ($($arg:tt)*) => {
        #[cfg(debug_assertions)]
        eprintln!("[baddel]   {}", format_args!($($arg)*));
    };
}

pub enum Command {
    Convert,
    /// "Convert Line": from the start of the line to the caret, unless something is selected.
    ConvertLine,
    /// "Convert to Arabic / to English": the selection or the last word, in that direction, with
    /// nothing held back.
    ConvertTo(Direction),
    Undo,
    /// "Exclude Current App" in the menu: the app in front stops being converted in.
    ExcludeCurrent,
    /// "Copy Original Text" in the menu, after an undo did not go through.
    CopyOriginal,
}

/// The channel into the conversion thread. Held in Tauri state so the menu and the
/// global shortcuts can reach it from wherever they run.
pub struct Commands(Mutex<Sender<Command>>);

impl Commands {
    pub fn send(&self, command: Command) {
        let _ = self.0.lock().unwrap().send(command);
    }
}

/// Which conversion command is running.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Mode {
    Auto,
    Line,
    To(Direction),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Method {
    Accessibility,
    Pasteboard,
}

/// What happened, for the UI. Carries no user text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Outcome {
    Converted,
    Extended,
    Undone,
    Unchanged,
    NoText,
    TooLong,
    Blocked,
    Excluded,
    Paused,
    NoPermission,
    NoLayouts,
    /// See the [`Reason`] recorded with it.
    Failed,
}

/// Why a command failed: it picks the notice, and goes to diagnostics with the outcome.
/// Carries no text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Reason {
    /// No ⌘C of ours was answered, and the app exposes no text: nothing to read.
    CopyTimedOut,
    /// The app refused the new text, or it never landed.
    WriteRejected,
    /// Undo, with another app in front than the one that converted.
    OtherApp,
    /// Undo: our text is not where we left it — another field, or the caret moved on.
    NotHere,
    /// Undo: in the very field we wrote to, our text has since been edited. Undo is over.
    TextChanged,
    /// Undo: the text could not be put back.
    UndoRejected,
    /// Undo on the key-based path of a result too long to reselect.
    TooLong,
}

#[derive(Clone)]
struct LastOp {
    original: String,
    result: String,
    direction: Direction,
    /// Where `result` starts in the element's text (UTF-16), when it was placed through the
    /// accessibility text range. `None` on the key-based path.
    start: Option<usize>,
    /// The arrow key that walks back over the text we produced (key-based path).
    back_arrow: u16,
    /// Words converted so far without an explicit selection; 0 for a selection.
    words: usize,
    /// Converted by an explicit "to Arabic / to English" command: literally, nothing held back.
    /// Extending it continues the same way, and only that command extends it.
    literal: bool,
    at: Instant,
    /// The app it happened in; undo and extend act there and nowhere else.
    app: App,
    /// The field it happened in, when the accessibility API named one.
    element: Option<Focused>,
    /// False once undo can no longer work (our text was edited since). The original is then
    /// kept only to be copied from the menu.
    undoable: bool,
    /// Set by a failed undo: the menu then offers the original for the user to paste back.
    copyable: bool,
}

/// What worked last time in one app, for this session only (never written anywhere): the side
/// of the caret the key-based path found its line on, and how fast the app answers ⌘C. The next
/// conversion there starts with what worked and waits no longer than it needs. Forgotten at the
/// first failure, or the first ⌘C answered later than the wait it set.
#[derive(Clone, Debug, Default, PartialEq)]
struct Route {
    arrow: Option<u16>,
    /// The slowest ⌘C answer timed in this app, over `copies` answers.
    slowest_copy: Duration,
    copies: u32,
}

impl Route {
    /// Twice the slowest answer seen, within [`COPY_TIMEOUT_MIN`]–[`COPY_TIMEOUT`], once there are
    /// enough answers to go on.
    fn copy_timeout(&self) -> Duration {
        if self.copies < COPY_SAMPLES {
            return COPY_TIMEOUT;
        }
        (self.slowest_copy * 2).clamp(COPY_TIMEOUT_MIN, COPY_TIMEOUT)
    }

    /// Folds in what one conversion learned.
    fn learn(&mut self, target: &Target) {
        if target.arrow.is_some() {
            self.arrow = target.arrow;
        }
        self.copies += target.copy_times.len() as u32;
        if let Some(slowest) = target.copy_times.iter().max() {
            self.slowest_copy = self.slowest_copy.max(*slowest);
        }
    }
}

/// Notices for states the user expects (permission missing, paused, excluded app) are rationed:
/// the first press explains, the next ones do not repeat it.
#[derive(Default)]
struct Rationed {
    /// When the missing-permission notice last showed.
    permission: Option<Instant>,
    /// The pause during which the paused notice showed ([`sync::pause_epoch`]).
    pause: Option<u64>,
    /// Apps whose exclusion has been announced since launch.
    excluded: HashSet<String>,
}

impl Rationed {
    /// At most once every [`PERMISSION_NOTICE_EVERY`].
    fn permission_due(&mut self) -> bool {
        let due = self.permission.is_none_or(|at| at.elapsed() >= PERMISSION_NOTICE_EVERY);
        if due {
            self.permission = Some(Instant::now());
        }
        due
    }

    /// Once per pause: `epoch` changes each time Baddel is paused or resumed.
    fn pause_due(&mut self, epoch: u64) -> bool {
        self.pause.replace(epoch) != Some(epoch)
    }

    /// Once per app since launch.
    fn excluded_due(&mut self, bundle_id: Option<&str>) -> bool {
        bundle_id.is_some_and(|id| self.excluded.insert(id.to_string()))
    }
}

pub fn spawn(app: AppHandle) -> Commands {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("baddel-controller".into())
        .spawn(move || {
            let mut controller = Controller {
                app,
                last: None,
                shown: None,
                woken: HashSet::new(),
                routes: HashMap::new(),
                path: None,
                reason: None,
                rationed: Rationed::default(),
            };
            loop {
                // While a conversion is remembered, wake up when its undo window closes so the
                // text leaves memory and the menu on time, not whenever the next command comes.
                let next = match controller.last.as_ref().map(|l| UNDO_WINDOW.saturating_sub(l.at.elapsed())) {
                    Some(left) => rx.recv_timeout(left),
                    None => rx.recv().map_err(|_| RecvTimeoutError::Disconnected),
                };
                let command = match next {
                    Ok(command) => command,
                    Err(RecvTimeoutError::Timeout) => {
                        controller.last = None;
                        controller.sync_menu();
                        continue;
                    }
                    Err(RecvTimeoutError::Disconnected) => break,
                };
                let started = Instant::now();
                let front = frontmost::current();
                let outcome = match command {
                    Command::Convert => controller.convert(&front, Mode::Auto),
                    Command::ConvertLine => controller.convert(&front, Mode::Line),
                    Command::ConvertTo(direction) => controller.convert(&front, Mode::To(direction)),
                    Command::Undo => controller.undo(&front),
                    Command::CopyOriginal => {
                        controller.copy_original();
                        continue;
                    }
                    Command::ExcludeCurrent => {
                        controller.exclude(&front);
                        continue;
                    }
                };
                // For diagnostics: the outcome, why, its path and time, and the app — never the text.
                let path = controller.path.take();
                let reason = controller.reason.take();
                #[cfg(debug_assertions)]
                eprintln!("[baddel] {outcome:?} in {:?} reason={reason:?}", started.elapsed());
                let history = controller.app.state::<History>();
                history.record(outcome, reason, path, started.elapsed(), front.bundle_id.clone());
                controller.sync_menu();
                controller.announce(outcome, reason, &front);
                // The event carries the outcome and nothing else: no converted text
                // ever crosses into a webview.
                let _ = controller.app.emit("conversion", outcome);
            }
        })
        .expect("failed to start the controller thread");
    Commands(Mutex::new(tx))
}

struct Controller {
    app: AppHandle,
    last: Option<LastOp>,
    /// What the menu currently shows of the last conversion (its time, and whether undo and
    /// copy are offered), so it is redrawn only on change.
    shown: Option<(Instant, bool, bool)>,
    /// Apps we already asked to switch their accessibility tree on.
    woken: HashSet<i32>,
    /// What worked in each app this session, by bundle identifier.
    routes: HashMap<String, Route>,
    /// How the last command reached the text, for diagnostics; `None` when a guard stopped it first.
    path: Option<Path>,
    /// Why the last command failed, when it did.
    reason: Option<Reason>,
    rationed: Rationed,
}

/// How the current text was obtained, and therefore how to write it back.
struct Target {
    focused: Option<Focused>,
    method: Method,
    /// Pasteboard contents to put back, taken before our first ⌘C.
    saved: Option<pasteboard::Snapshot>,
    /// The pasteboard's change count as our own last use of it left it. Different at the end
    /// means someone else copied meanwhile, and their copy is kept rather than the snapshot.
    ours: isize,
    /// A ⌘C of ours went unanswered and we have not written since: a change now is far likelier
    /// its late answer than the user's own copy.
    pending_copy: bool,
    /// ⌘C presses sent, and how many the app answered.
    copies: (u32, u32),
    /// Whether synthetic arrow keys selected the text (diagnostics only).
    keys: bool,
    /// How long to wait for the app to answer a ⌘C ([`Route::copy_timeout`]).
    copy_timeout: Duration,
    /// How long each answered ⌘C took.
    copy_times: Vec<Duration>,
    /// A ⌘C was answered only after we had stopped waiting for it.
    late_copy: bool,
    /// The arrow the key-based path found the line with, when it did.
    arrow: Option<u16>,
}

/// What to do with a line read through the keyboard: keep its first `keep` bytes, replace the rest.
struct LineEdit {
    keep: usize,
    converted: String,
    direction: Direction,
}

enum KeyEdit {
    Done { original: String, result: String, direction: Direction },
    /// Nothing usable on this side of the caret.
    NothingHere,
    Failed,
}

/// Result of trying to continue the previous conversion one word further back.
enum Extension {
    Done(Outcome),
    Elsewhere,
    Unknown,
}

/// A word located through the accessibility text range. Offsets are UTF-16 units.
struct Word {
    start: usize,
    len: usize,
    text: String,
}

struct Context<'a> {
    map: &'a LayoutMap,
    layouts: &'a Layouts,
    /// The app in front, where the conversion happens.
    front: &'a App,
    /// The direction implied by the active layout: the one the wrong text was typed with.
    expected: Direction,
    /// Set by "Convert to Arabic / to English": this direction, converted literally.
    forced: Option<Direction>,
    /// The arrow that found the line last time in this app ([`Route`]).
    first_arrow: Option<u16>,
    switch_input_source: bool,
}

impl Context<'_> {
    /// Converts `text` the way the running command does: in the forced direction and literally,
    /// or in the direction the text votes for, keeping links and technical words as they are.
    fn convert(&self, text: &str) -> (String, Direction) {
        match self.forced {
            Some(direction) => (self.map.convert_literal(text, direction), direction),
            None => {
                let direction = self.map.direction(text).unwrap_or(self.expected);
                (self.map.convert_as(text, direction), direction)
            }
        }
    }

    /// Converts more text the way an earlier conversion went.
    fn continue_as(&self, text: &str, last: &LastOp) -> String {
        if last.literal {
            self.map.convert_literal(text, last.direction)
        } else {
            self.map.convert_as(text, last.direction)
        }
    }

    /// The order to try the two sides of the caret in on the key-based path. Arabic script runs
    /// right to left, so in apps that move the caret visually "back" is the right arrow: the
    /// likelier side first, unless this app has shown which side it is.
    fn arrows(&self) -> [u16; 2] {
        let likely = match self.expected {
            Direction::ArabicToLatin => KEY_RIGHT,
            Direction::LatinToArabic => KEY_LEFT,
        };
        let first = self.first_arrow.unwrap_or(likely);
        [first, opposite(first)]
    }
}

impl Controller {
    fn convert(&mut self, front: &App, mode: Mode) -> Outcome {
        // A timed pause that has just run out is over, even if the clock check has not come round.
        sync::resume_if_expired(&self.app);
        let settings = self.app.state::<AppState>().settings.lock().unwrap().clone();
        if settings.paused {
            return Outcome::Paused;
        }
        if !permissions::is_trusted(false) {
            return Outcome::NoPermission;
        }
        if permissions::secure_input_enabled() {
            return Outcome::Blocked;
        }
        if front.bundle_id.as_ref().is_some_and(|id| settings.excluded_apps.contains(id)) {
            return Outcome::Excluded;
        }
        let (arabic, latin) = (settings.arabic_layout.clone(), settings.latin_layout.clone());
        let Some(layouts) = on_main(&self.app, move || input_source::snapshot(&arabic, &latin)) else {
            return Outcome::NoLayouts;
        };
        let map = build_map(&layouts);
        if map.is_empty() {
            return Outcome::NoLayouts;
        }
        keysynth::wait_for_modifiers_released(MODIFIER_RELEASE_TIMEOUT);

        let expected =
            if layouts.current_is_arabic { Direction::ArabicToLatin } else { Direction::LatinToArabic };
        let route = front.bundle_id.as_ref().and_then(|id| self.routes.get(id)).cloned().unwrap_or_default();
        let cx = Context {
            map: &map,
            layouts: &layouts,
            front,
            expected,
            forced: match mode {
                Mode::To(direction) => Some(direction),
                Mode::Auto | Mode::Line => None,
            },
            first_arrow: route.arrow,
            switch_input_source: settings.switch_input_source,
        };
        let mut target = Target::new(self.focused_element(front.pid));
        target.copy_timeout = route.copy_timeout();
        trace!("route: arrow {:?}, copy wait {:?} over {} answers", route.arrow, target.copy_timeout, route.copies);
        if target.focused.as_ref().is_some_and(Focused::is_secure) {
            return Outcome::Blocked;
        }
        // A stand-in says "caret at 0, nothing selected" whatever is really there (Figma's
        // canvas text). Believing it means never converting; the keyboard knows.
        if target.focused.as_ref().is_some_and(Focused::is_stand_in) {
            trace!("focused element is a stand-in, using the keyboard");
            target.method = Method::Pasteboard;
        }
        // Debug aid: exercise the key-based path in apps that would never need it.
        #[cfg(debug_assertions)]
        if std::env::var_os("BADDEL_FORCE_KEYS").is_some() {
            target.focused = None;
            target.method = Method::Pasteboard;
        }
        let outcome = self.convert_in(&mut target, &cx, mode);
        self.path = Some(target.path());
        target.restore_pasteboard();
        self.remember_route(front, &target, outcome);
        outcome
    }

    /// Keeps what this conversion learned about the app, or forgets the app on a failure or a ⌘C
    /// answered after we stopped waiting: what worked before may no longer.
    fn remember_route(&mut self, front: &App, target: &Target, outcome: Outcome) {
        let Some(id) = front.bundle_id.clone() else { return };
        if outcome == Outcome::Failed || target.late_copy {
            trace!("route forgotten (failed: {}, late copy: {})", outcome == Outcome::Failed, target.late_copy);
            self.routes.remove(&id);
            return;
        }
        self.routes.entry(id).or_default().learn(target);
    }

    /// The focused element, waking the app's accessibility tree the first time it has none.
    fn focused_element(&mut self, pid: Option<i32>) -> Option<Focused> {
        if let Some(focused) = Focused::get() {
            return Some(focused);
        }
        let pid = pid?;
        if !self.woken.insert(pid) {
            return None;
        }
        // Chromium reports an error for this request and honours it anyway, so the answer
        // tells us nothing: just watch for the tree to appear.
        let accepted = text_access::enable_accessibility(pid);
        trace!("asked pid {pid} to enable accessibility (accepted: {accepted})");
        let deadline = Instant::now() + AX_WAKE_TIMEOUT;
        while Instant::now() < deadline {
            sleep(Duration::from_millis(50));
            if let Some(focused) = Focused::get() {
                return Some(focused);
            }
        }
        None
    }

    fn convert_in(&mut self, target: &mut Target, cx: &Context, mode: Mode) -> Outcome {
        // 1. An explicit selection wins.
        if let Some(text) = target.read_selection() {
            if text.chars().count() > MAX_CHARS {
                return Outcome::TooLong;
            }
            let (result, direction) = cx.convert(&text);
            if result == text {
                return Outcome::Unchanged;
            }
            // Where the selection came through the text APIs, note where it starts, so undo can
            // find the result by its place in the text — exactly, at any length — and not by
            // reselecting it with the keyboard.
            let start = target.selection_start(&text);
            if !target.write(&text, &result) {
                return self.fail(Reason::WriteRejected);
            }
            self.finish(LastOp::new(text, result, direction, start, 0), target, cx);
            return Outcome::Converted;
        }

        // 2. "Convert Line": the line up to the caret.
        if mode == Mode::Line {
            return self.convert_line(target, cx);
        }

        // 3. No selection, right after a conversion in this same app by the same kind of command:
        // take one more word.
        let previous = self.last.clone().filter(|l| {
            l.at.elapsed() < EXTEND_WINDOW
                && l.words > 0
                && l.undoable
                && l.app.pid == cx.front.pid
                && l.literal == cx.forced.is_some()
                && cx.forced.is_none_or(|forced| forced == l.direction)
        });
        if let Some(last) = previous {
            match self.extend(target, cx, &last) {
                Extension::Done(outcome) => return outcome,
                // The caret is somewhere else: this is a new conversion, not a continuation.
                Extension::Elsewhere => {}
                // Could not tell. Falling through would find the word we just fixed and
                // convert it back, so stop here.
                Extension::Unknown => return Outcome::NoText,
            }
        }

        // 4. No selection: the word before the caret.
        self.convert_last_word(target, cx)
    }

    /// "Convert Line": everything from the start of the line to the caret, in one direction (the
    /// majority's, as for a selection). A line break the text holds stays where it is.
    fn convert_line(&mut self, target: &mut Target, cx: &Context) -> Outcome {
        if target.method == Method::Accessibility {
            if let Some((focused, (caret, 0))) =
                target.focused.as_ref().and_then(|f| Some((f, f.selected_range()?)))
            {
                trace!("line: text range path, caret at {caret}");
                let line = match line_before(focused, caret) {
                    Ok(Some(line)) => line,
                    Ok(None) => return Outcome::NoText,
                    Err(outcome) => return outcome,
                };
                let (result, direction) = cx.convert(&line.text);
                if result == line.text {
                    return Outcome::Unchanged;
                }
                if !target.replace_range(&line, &result, caret) {
                    return self.fail(Reason::WriteRejected);
                }
                self.finish(LastOp::new(line.text, result, direction, Some(line.start), 1), target, cx);
                return Outcome::Converted;
            }
        }

        // Key-based: select back to the paragraph's start (the logical line, not the visual edge
        // of a wrapped one), convert what follows its leading whitespace, paste it back.
        trace!("line: key path ({:?})", target.method);
        let back = cx.arrows()[0];
        let read = std::cell::Cell::new(None::<Outcome>);
        let edit = target.edit_with_keys(keysynth::select_to_paragraph_start, opposite(back), |text| {
            // At a paragraph's very start the keys select the paragraph before: nothing of this one.
            let line_start = text.rfind('\n').map_or(0, |i| i + 1);
            let keep = line_start + (text[line_start..].len() - text[line_start..].trim_start().len());
            let line = &text[keep..];
            if line.trim().is_empty() {
                return None;
            }
            if line.chars().count() > MAX_CHARS {
                read.set(Some(Outcome::TooLong));
                return None;
            }
            let (converted, direction) = cx.convert(line);
            if converted == line {
                read.set(Some(Outcome::Unchanged));
                return None;
            }
            Some(LineEdit { keep, converted, direction })
        });
        match edit {
            KeyEdit::Done { original, result, direction } => {
                self.finish(LastOp::new(original, result, direction, None, 1), target, cx);
                Outcome::Converted
            }
            KeyEdit::Failed => self.fail(Reason::WriteRejected),
            KeyEdit::NothingHere => match read.get() {
                Some(outcome) => outcome,
                None if target.focused.is_none() && target.copies.0 > 0 && target.copies.1 == 0 => {
                    self.fail(Reason::CopyTimedOut)
                }
                None => Outcome::NoText,
            },
        }
    }

    fn convert_last_word(&mut self, target: &mut Target, cx: &Context) -> Outcome {
        // Preferred: locate the word through the text range. Offsets are logical, so this is
        // immune to the visual caret movement of right-to-left and mixed-direction text.
        if target.method == Method::Accessibility {
            if let Some((focused, (caret, 0))) =
                target.focused.as_ref().and_then(|f| Some((f, f.selected_range()?)))
            {
                trace!("text range path, caret at {caret}");
                let Some(word) = previous_word(focused, caret) else { return Outcome::NoText };
                let (result, direction) = cx.convert(&word.text);
                if result == word.text {
                    return Outcome::Unchanged;
                }
                if !target.replace_range(&word, &result, caret) {
                    return self.fail(Reason::WriteRejected);
                }
                self.finish(LastOp::new(word.text, result, direction, Some(word.start), 1), target, cx);
                return Outcome::Converted;
            }
        }

        trace!("key path ({:?})", target.method);
        // Fallback for apps without a text range: select to the line's edge on one side of the
        // caret, then the other ([`Context::arrows`]).
        for arrow in cx.arrows() {
            let edit = target.edit_line_with_keys(arrow, |line| {
                let tail = last_word_and_trailing_space(line)?;
                let (converted, direction) = cx.convert(tail);
                (converted != tail).then(|| LineEdit { keep: line.len() - tail.len(), converted, direction })
            });
            match edit {
                KeyEdit::Done { original, result, direction } => {
                    target.arrow = Some(arrow);
                    let mut op = LastOp::new(original, result, direction, None, 1);
                    op.back_arrow = arrow;
                    self.finish(op, target, cx);
                    return Outcome::Converted;
                }
                KeyEdit::Failed => return self.fail(Reason::WriteRejected),
                KeyEdit::NothingHere => continue,
            }
        }
        // No text element, and not one ⌘C answered: this app gives us no way to its text. Say
        // so, with what the user can do; an empty line elsewhere is simply "no text".
        let (sent, answered) = target.copies;
        if target.focused.is_none() && sent > 0 && answered == 0 {
            return self.fail(Reason::CopyTimedOut);
        }
        Outcome::NoText
    }

    /// Converts the word before the text we produced last time.
    fn extend(&mut self, target: &mut Target, cx: &Context, last: &LastOp) -> Extension {
        if let (Some(start), Some(focused)) = (last.start, target.focused.as_ref()) {
            // Only a continuation if our text is still there and the caret still follows it.
            let end = start + utf16_len(&last.result);
            let Some((caret, 0)) = focused.selected_range() else { return Extension::Elsewhere };
            let still_there = focused.string_for_range(start, end - start).as_deref() == Some(last.result.as_str());
            let follows = caret >= end
                && focused.string_for_range(end, caret - end).is_some_and(|gap| gap.trim().is_empty());
            if !still_there || !follows {
                trace!("extend: caret moved away, treating as a new conversion");
                return Extension::Elsewhere;
            }
            trace!("extend: text range path, caret at {caret}");
            let Some(word) = previous_word(focused, start) else { return Extension::Done(Outcome::NoText) };
            let Some(gap) = focused.string_for_range(word.start + word.len, start - (word.start + word.len)) else {
                return Extension::Unknown;
            };
            let converted = cx.continue_as(&word.text, last);
            if converted == word.text {
                return Extension::Done(Outcome::Unchanged);
            }
            if !target.replace_range(&word, &converted, caret) {
                return Extension::Done(self.fail(Reason::WriteRejected));
            }
            let original = format!("{}{gap}{}", word.text, last.original);
            let result = format!("{converted}{gap}{}", last.result);
            self.finish(LastOp::new(original, result, last.direction, Some(word.start), last.words + 1), target, cx);
            return Extension::Done(Outcome::Extended);
        }

        // Key-based: re-read the line; our text must still end it, then convert the word before.
        trace!("extend: key path");
        let edit = target.edit_line_with_keys(last.back_arrow, |line| {
            let end = line.trim_end().len();
            let start = end.checked_sub(last.result.trim_end().len())?;
            if !line.is_char_boundary(start) || line[start..end] != *last.result.trim_end() {
                return None;
            }
            let word = last_word_and_trailing_space(&line[..start])?;
            let keep = start - word.len();
            let converted = format!("{}{}", cx.continue_as(word, last), &line[start..]);
            (converted != line[keep..]).then_some(LineEdit { keep, converted, direction: last.direction })
        });
        match edit {
            KeyEdit::Done { original, result, direction } => {
                let mut op = LastOp::new(original, result, direction, None, last.words + 1);
                op.back_arrow = last.back_arrow;
                self.finish(op, target, cx);
                Extension::Done(Outcome::Extended)
            }
            KeyEdit::Failed => Extension::Done(self.fail(Reason::WriteRejected)),
            KeyEdit::NothingHere => Extension::Unknown,
        }
    }

    fn fail(&mut self, reason: Reason) -> Outcome {
        self.reason = Some(reason);
        Outcome::Failed
    }

    /// Remembers a conversion that went through, with where it happened.
    fn finish(&mut self, mut op: LastOp, target: &Target, cx: &Context) {
        op.literal = cx.forced.is_some();
        op.app = cx.front.clone();
        op.element = target.focused.clone();
        if cx.switch_input_source {
            let wanted = match op.direction {
                Direction::ArabicToLatin => &cx.layouts.latin,
                Direction::LatinToArabic => &cx.layouts.arabic,
            };
            if let Some(id) = wanted.as_ref().map(|l| l.id.clone()) {
                on_main(&self.app, move || input_source::select(&id));
            }
        }
        self.last = Some(op);
    }

    fn undo(&mut self, front: &App) -> Outcome {
        let Some(last) = self.last.clone().filter(|l| l.undoable && l.at.elapsed() < UNDO_WINDOW) else {
            return Outcome::NoText;
        };
        // Everything up to the write is a passing condition: the conversion stays remembered,
        // and pressing undo again once it has cleared still works.
        if !permissions::is_trusted(false) {
            return Outcome::NoPermission;
        }
        if permissions::secure_input_enabled() {
            return Outcome::Blocked;
        }
        if last.app.pid.is_some() && front.pid != last.app.pid {
            return self.undo_failed(Reason::OtherApp);
        }
        keysynth::wait_for_modifiers_released(MODIFIER_RELEASE_TIMEOUT);
        let mut target = Target::new(Focused::get());
        if target.focused.as_ref().is_some_and(Focused::is_secure) {
            return Outcome::Blocked;
        }
        // As in a conversion: a stand-in element knows nothing of the text; the keyboard does.
        if target.focused.as_ref().is_some_and(Focused::is_stand_in) {
            target.method = Method::Pasteboard;
        }
        let result = undo_in(&mut target, &last);
        self.path = Some(target.path());
        target.restore_pasteboard();
        match result {
            Ok(()) => {
                self.last = None;
                Outcome::Undone
            }
            Err(reason) => self.undo_failed(reason),
        }
    }

    /// An undo that did not go through. From now on the menu also offers the original, to paste
    /// back by hand; undo itself stays offered unless it can no longer work.
    fn undo_failed(&mut self, reason: Reason) -> Outcome {
        if let Some(last) = self.last.as_mut() {
            last.undo_failed(reason);
        }
        self.fail(reason)
    }

    /// "Copy Original Text": puts the original on the pasteboard and forgets the conversion.
    fn copy_original(&mut self) {
        let Some(last) = self.last.take_if(|l| l.copyable && l.at.elapsed() < UNDO_WINDOW) else { return };
        // The user asked for this copy, so it is an ordinary one that clipboard managers may keep
        // (PRIVACY.md), not the flagged, passing kind a conversion uses.
        pasteboard::write_text(&last.original);
        self.sync_menu();
        let settings = self.app.state::<AppState>().get();
        if settings.show_hud {
            let text = menu_text::strings(settings.language);
            hud::show(&self.app, notice(hud::Kind::Copied, text.hud_copied, settings.language == Language::Ar));
        }
    }
}

impl Controller {
    /// "Exclude Current App": adds the app in front to the exceptions and confirms it. The menu
    /// bar's menu never activates Baddel, so the app in front is the one the user was in.
    fn exclude(&mut self, front: &App) {
        let Some(id) = front.bundle_id.clone() else { return };
        // With a settings window open Baddel itself may be in front; it has nothing to exclude.
        if id == self.app.config().identifier {
            return;
        }
        let previous = self.app.state::<AppState>().get();
        if !previous.excluded_apps.contains(&id) {
            let next = settings::update(&self.app, |s| s.excluded_apps.push(id.clone()));
            sync::apply(&self.app, &previous, &next);
        }
        // Just confirmed: the "stays out of" notice need not follow at the next press.
        self.rationed.excluded.insert(id);
        let settings = self.app.state::<AppState>().get();
        if let (true, Some(name)) = (settings.show_hud, app_name(front)) {
            let text = menu_text::strings(settings.language);
            let sentence = text.hud_excluded_now.replace("{app}", &name);
            hud::show(&self.app, notice(hud::Kind::ExcludedNow, &sentence, settings.language == Language::Ar));
        }
    }
}

/// Puts `last.original` back where `last.result` is.
fn undo_in(target: &mut Target, last: &LastOp) -> Result<(), Reason> {
    if let Some(start) = last.start {
        let Some(focused) = target.focused.as_ref() else { return Err(Reason::NotHere) };
        let len = utf16_len(&last.result);
        if focused.string_for_range(start, len).as_deref() != Some(last.result.as_str()) {
            // In the very field we wrote to, a different text there means it was edited: undo
            // would overwrite that. Anywhere else it may still be intact where we left it.
            let same_field = last.element.as_ref().is_some_and(|e| focused.same_as(e));
            return Err(if same_field { Reason::TextChanged } else { Reason::NotHere });
        }
        let word = Word { start, len, text: last.result.clone() };
        let caret = focused.selected_range().map_or(start + len, |(c, _)| c);
        return if target.replace_range(&word, &last.original, caret) { Ok(()) } else { Err(Reason::UndoRejected) };
    }

    // Key-based, and longer than is worth reselecting character by character: select back to
    // the paragraph's start (a long line wraps, so its visual edge is not enough), check that
    // our text still ends it, and paste it back with the original in its place. Nothing is
    // touched unless the paragraph ends exactly with what we wrote. (A short result keeps the
    // exact reselection below, which leaves the rest of the line — and its formatting — alone.)
    let stops = caret_stops(&last.result);
    if stops > MAX_UNDO_CHARS {
        if last.result.contains('\n') {
            return Err(Reason::TooLong);
        }
        let edit = target.edit_with_keys(keysynth::select_to_paragraph_start, opposite(last.back_arrow), |line| {
            undo_line(line, &last.original, &last.result).map(|(keep, converted)| LineEdit {
                keep,
                converted,
                direction: last.direction.reversed(),
            })
        });
        return match edit {
            KeyEdit::Done { .. } => Ok(()),
            KeyEdit::NothingHere => Err(Reason::NotHere),
            KeyEdit::Failed => Err(Reason::UndoRejected),
        };
    }

    target.keys = true;
    for _ in 0..stops {
        keysynth::select_char(last.back_arrow);
    }
    match target.await_selection_where(|s| s == last.result) {
        Some(selected) if selected == last.result => {
            if target.write(&selected, &last.original) {
                Ok(())
            } else {
                Err(Reason::UndoRejected)
            }
        }
        _ => {
            keysynth::arrow(opposite(last.back_arrow));
            Err(Reason::NotHere)
        }
    }
}

impl Controller {
    /// Makes the menu's "last conversion" items match what can still be done with it: undo while
    /// it can work, the original to copy after an undo failed, nothing once it is undone,
    /// copied or expired.
    fn sync_menu(&mut self) {
        let current = self.last.as_ref().map(|l| (l.at, l.undoable, l.copyable));
        if current != self.shown {
            self.shown = current;
            let last = self.last.as_ref().map(|l| LastConversion {
                original: l.original.clone(),
                result: l.result.clone(),
                undoable: l.undoable,
                copyable: l.copyable,
            });
            tray::set_last_conversion(&self.app, last);
        }
    }

    /// Shows the outcome: the notice panel and the click sound. The panel is native and
    /// reads the text straight out of memory — it is never written anywhere, nor handed
    /// to a webview. Every state reads as in `A2 — States`: one icon, one tone, one sentence.
    fn announce(&mut self, outcome: Outcome, reason: Option<Reason>, front: &App) {
        let settings = self.app.state::<AppState>().settings.lock().unwrap().clone();
        let text = menu_text::strings(settings.language);
        let rtl = settings.language == Language::Ar;

        if matches!(outcome, Outcome::Converted | Outcome::Extended) && settings.sound {
            on_main(&self.app, sound::click);
        }

        if !settings.show_hud {
            return;
        }
        let message = |kind, sentence: &str| Some(notice(kind, sentence, rtl));
        let notice = match outcome {
            Outcome::Converted | Outcome::Extended => self.last.as_ref().map(|last| {
                if !fits_display(&last.original, &last.result) {
                    let count = menu_text::characters(settings.language, last.result.chars().count());
                    return notice(hud::Kind::Success, &text.hud_converted_long.replace("{count}", &count), rtl);
                }
                hud::Notice {
                    kind: hud::Kind::Success,
                    body: hud::Body::Conversion { from: last.original.clone(), to: last.result.clone() },
                    rtl,
                    badge: settings.switch_input_source.then(|| badge(last.direction).to_string()),
                }
            }),
            Outcome::Undone => message(hud::Kind::Undone, text.hud_undone),
            Outcome::Blocked => message(hud::Kind::Blocked, text.hud_blocked),
            Outcome::TooLong => message(hud::Kind::TooLong, text.hud_too_long),
            Outcome::NoText => message(hud::Kind::NoText, text.hud_no_text),
            Outcome::Unchanged => message(hud::Kind::Unchanged, text.hud_unchanged),
            Outcome::NoLayouts => message(hud::Kind::NoLayouts, text.hud_no_layouts),
            Outcome::Failed => match reason {
                Some(Reason::OtherApp) => match self.last.as_ref().and_then(|l| app_name(&l.app)) {
                    Some(name) => message(hud::Kind::Elsewhere, &text.hud_elsewhere.replace("{app}", &name)),
                    None => message(hud::Kind::NotHere, text.hud_not_here),
                },
                Some(Reason::NotHere | Reason::TextChanged) => message(hud::Kind::NotHere, text.hud_not_here),
                Some(Reason::UndoRejected) => message(hud::Kind::UndoFailed, text.hud_undo_failed),
                Some(Reason::TooLong) => message(hud::Kind::TooLong, text.hud_too_long),
                Some(Reason::CopyTimedOut | Reason::WriteRejected) | None => {
                    message(hud::Kind::Failed, text.hud_failed)
                }
            },
            // Expected states. The menu bar icon and the menu say them too, so the notice is
            // rationed: whoever pressed and saw nothing happen gets told once, not every time.
            Outcome::NoPermission => {
                self.rationed.permission_due().then(|| notice(hud::Kind::NoPermission, text.hud_no_permission, rtl))
            }
            Outcome::Paused => {
                let due = self.rationed.pause_due(sync::pause_epoch());
                due.then(|| notice(hud::Kind::Paused, text.hud_paused, rtl))
            }
            Outcome::Excluded => {
                let due = self.rationed.excluded_due(front.bundle_id.as_deref());
                due.then(|| app_name(front)).flatten().map(|name| {
                    notice(hud::Kind::Excluded, &text.hud_excluded.replace("{app}", &name), rtl)
                })
            }
        };
        if let Some(notice) = notice {
            hud::show(&self.app, notice);
        }
    }
}

/// The name macOS shows for an app, falling back to its bundle identifier.
fn app_name(app: &App) -> Option<String> {
    app.name.clone().or_else(|| app.bundle_id.clone())
}

/// The input source the conversion switched to.
fn badge(direction: Direction) -> &'static str {
    match direction {
        Direction::ArabicToLatin => "EN",
        Direction::LatinToArabic => "ع",
    }
}

fn notice(kind: hud::Kind, text: &str, rtl: bool) -> hud::Notice {
    hud::Notice { kind, body: hud::Body::Message(text.to_string()), badge: None, rtl }
}

impl LastOp {
    /// `app` and `element` are filled in by [`Controller::finish`].
    fn new(original: String, result: String, direction: Direction, start: Option<usize>, words: usize) -> Self {
        // Walking back over the result: Latin text runs left to right, Arabic right to left.
        let back_arrow = match direction {
            Direction::ArabicToLatin => KEY_LEFT,
            Direction::LatinToArabic => KEY_RIGHT,
        };
        LastOp {
            original,
            result,
            direction,
            start,
            back_arrow,
            words,
            literal: false,
            at: Instant::now(),
            app: App::default(),
            element: None,
            undoable: true,
            copyable: false,
        }
    }

    /// After an undo that did not go through, the original is offered for copying; undo itself
    /// stays unless it can no longer work.
    fn undo_failed(&mut self, reason: Reason) {
        self.copyable = true;
        if matches!(reason, Reason::TextChanged | Reason::TooLong) {
            self.undoable = false;
        }
    }
}

impl Target {
    fn new(focused: Option<Focused>) -> Self {
        Target {
            focused,
            method: Method::Accessibility,
            saved: None,
            ours: 0,
            pending_copy: false,
            copies: (0, 0),
            keys: false,
            copy_timeout: COPY_TIMEOUT,
            copy_times: Vec::new(),
            late_copy: false,
            arrow: None,
        }
    }

    /// Takes the snapshot to put back, before our first use of the pasteboard.
    fn save_pasteboard(&mut self) {
        if self.saved.is_none() {
            self.saved = Some(pasteboard::snapshot());
            self.ours = pasteboard::change_count();
        }
    }

    /// While a ⌘C of ours may still be answered late, a change is counted as that answer.
    fn adopt_late_copy(&mut self) {
        if self.pending_copy {
            let now = pasteboard::change_count();
            self.late_copy |= now != self.ours;
            self.ours = now;
        }
    }

    /// Puts the user's pasteboard back — unless they (or another app) copied something while we
    /// worked. The newest copy wins: the snapshot is dropped rather than written over it.
    fn restore_pasteboard(&mut self) {
        self.adopt_late_copy();
        if let Some(saved) = &self.saved {
            if !pasteboard::restore_if_unchanged(saved, self.ours) {
                trace!("the pasteboard changed while converting; keeping the newer copy");
            }
        }
    }

    fn path(&self) -> Path {
        match (self.keys, self.method) {
            (true, _) => Path::Keys,
            (false, Method::Accessibility) => Path::Accessibility,
            (false, Method::Pasteboard) => Path::Pasteboard,
        }
    }

    /// Where `text`, just read as the selection, starts in the element (UTF-16) — when it was
    /// read through the text APIs and the element's own range agrees with it.
    fn selection_start(&self, text: &str) -> Option<usize> {
        if self.method != Method::Accessibility {
            return None;
        }
        let focused = self.focused.as_ref()?;
        let (start, len) = focused.selected_range()?;
        let agrees = len == utf16_len(text) && focused.string_for_range(start, len).as_deref() == Some(text);
        agrees.then_some(start)
    }

    /// The selected text, or `None` when nothing is selected.
    fn read_selection(&mut self) -> Option<String> {
        if self.method == Method::Accessibility {
            match self.focused.as_ref().map(Focused::selection) {
                Some(Selection::Text(text)) => return Some(text),
                Some(Selection::Empty) => return None,
                // This app does not expose its selection: use the pasteboard from here on.
                Some(Selection::Unavailable) | None => {
                    trace!("selection unavailable (focused element: {})", self.focused.is_some());
                    self.method = Method::Pasteboard
                }
            }
        }
        self.save_pasteboard();
        self.adopt_late_copy();
        let before = pasteboard::change_count();
        keysynth::copy();
        let sent = Instant::now();
        self.copies.0 += 1;
        if !pasteboard::wait_for_change(before, self.copy_timeout) {
            self.pending_copy = true;
            return None;
        }
        self.copy_times.push(sent.elapsed());
        self.pending_copy = false;
        self.copies.1 += 1;
        self.ours = pasteboard::change_count();
        let text = pasteboard::read_string().filter(|t| !t.is_empty())?;
        // The app put the selection on the pasteboard as an ordinary copy, which a clipboard
        // manager may record. Replace it at once with the same text flagged as passing, so the
        // unflagged copy lasts milliseconds rather than the whole conversion.
        self.ours = pasteboard::write_transient(&text);
        // Editors like VS Code copy the whole line when nothing is selected.
        let line_copy = text.ends_with('\n') && text.trim_end_matches('\n').lines().count() <= 1;
        (!line_copy).then_some(text)
    }

    /// The selection a key press we just sent should produce.
    fn await_selection(&mut self) -> Option<String> {
        self.await_selection_where(|_| true)
    }

    /// Polls until the selection satisfies `done`; on timeout returns whatever is selected.
    /// Only the accessibility path needs polling: on the pasteboard path our ⌘C queues up
    /// behind the selection key in the app's own event queue.
    fn await_selection_where(&mut self, done: impl Fn(&str) -> bool) -> Option<String> {
        sleep(SELECTION_SETTLE);
        let deadline = Instant::now() + SELECTION_TIMEOUT;
        let mut retried = false;
        loop {
            let selection = self.read_selection();
            let settled = selection.as_deref().is_some_and(&done);
            // On the pasteboard path every read is a ⌘C: allow one retry (a busy app can miss
            // the copy timeout), not a polling loop.
            let out_of_tries = self.method == Method::Pasteboard && (selection.is_some() || retried);
            if settled || out_of_tries || Instant::now() >= deadline {
                return selection;
            }
            retried = self.method == Method::Pasteboard;
            sleep(Duration::from_millis(15));
        }
    }

    /// Key-based edit of the text between the line's edge and the caret: select it, read it,
    /// let `plan` decide what its tail becomes, and paste the line back with that tail replaced.
    ///
    /// Deliberately coarse. Word-selection keys stop at `;` `,` `[` `'` — which wrongly typed
    /// Arabic is full of — and walking back character by character goes astray in
    /// mixed-direction text. The line's edge is the one selection that is reliable everywhere.
    fn edit_line_with_keys(&mut self, arrow: u16, plan: impl Fn(&str) -> Option<LineEdit>) -> KeyEdit {
        self.edit_with_keys(|| keysynth::select_to_line_edge(arrow), opposite(arrow), plan)
    }

    /// The same, with `select` choosing how far back to select, and `collapse` the arrow that
    /// puts the caret back where it was when the plan declines.
    fn edit_with_keys(
        &mut self,
        select: impl FnOnce() -> bool,
        collapse: u16,
        plan: impl Fn(&str) -> Option<LineEdit>,
    ) -> KeyEdit {
        self.keys = true;
        select();
        let line = self.await_selection();
        trace!("key path: line read = {:?} chars", line.as_ref().map(|l| l.chars().count()));
        let Some(line) = line else { return KeyEdit::NothingHere };
        let Some(edit) = plan(&line) else {
            keysynth::arrow(collapse); // collapse back onto the caret
            return KeyEdit::NothingHere;
        };
        let replacement = format!("{}{}", &line[..edit.keep], edit.converted);
        if !self.write(&line, &replacement) {
            return KeyEdit::Failed;
        }
        KeyEdit::Done { original: line[edit.keep..].to_string(), result: edit.converted, direction: edit.direction }
    }

    /// Selects `word` through the text range, replaces it, and puts the caret back where it
    /// was relative to the text after it (the user may have typed a space after the word).
    fn replace_range(&mut self, word: &Word, replacement: &str, caret: usize) -> bool {
        if !self.select_word(word) || !self.write(&word.text, replacement) {
            return false;
        }
        if self.method == Method::Accessibility && self.write_dropped(word, replacement) {
            trace!("accessibility write dropped, pasting instead");
            self.method = Method::Pasteboard;
            if !self.select_word(word) || !self.write(&word.text, replacement) {
                return false;
            }
        }
        let new_caret = (caret + utf16_len(replacement)).saturating_sub(word.len).max(word.start);
        // Best effort: a misplaced caret is not a failed conversion.
        if let Some(focused) = self.focused.as_ref() {
            focused.select_range(new_caret, 0);
        }
        true
    }

    fn select_word(&self, word: &Word) -> bool {
        let Some(focused) = self.focused.as_ref() else { return false };
        if !focused.select_range(word.start, word.len) {
            return false;
        }
        // The app applies the selection asynchronously; writing before it lands would insert
        // at the old caret instead of replacing the word.
        let deadline = Instant::now() + SELECTION_TIMEOUT;
        while focused.selected_range() != Some((word.start, word.len)) {
            if Instant::now() >= deadline {
                return false;
            }
            sleep(Duration::from_millis(10));
        }
        true
    }

    /// Whether an accessibility write was accepted and then thrown away (ProseMirror in
    /// Electron does this for some replacements). Only when the original is provably still in
    /// place is it safe to write again; anything else counts as landed, so nothing is doubled.
    fn write_dropped(&self, word: &Word, replacement: &str) -> bool {
        let Some(focused) = self.focused.as_ref() else { return false };
        let deadline = Instant::now() + WRITE_SETTLE;
        loop {
            if focused.string_for_range(word.start, utf16_len(replacement)).as_deref() == Some(replacement) {
                return false;
            }
            if Instant::now() >= deadline {
                return focused.string_for_range(word.start, word.len).as_deref() == Some(word.text.as_str());
            }
            sleep(Duration::from_millis(10));
        }
    }

    /// Replaces the selection (`original`) with `result`.
    fn write(&mut self, original: &str, result: &str) -> bool {
        if self.method == Method::Accessibility {
            if let Some(focused) = &self.focused {
                let accepted = focused.replace_selection(result);
                // Some apps accept the request and ignore it: the old text is then still selected.
                let ignored = matches!(focused.selection(), Selection::Text(t) if t == original && t != result);
                if accepted && !ignored {
                    return true;
                }
            }
            self.method = Method::Pasteboard;
        }
        self.save_pasteboard();
        // Where the app reports its selection, the paste can be seen landing: the selected
        // original gives way to the caret. Only then is it safe to put the user's pasteboard back.
        let watched = self
            .focused
            .as_ref()
            .filter(|f| matches!(f.selection(), Selection::Text(t) if t == original));
        self.ours = pasteboard::write_transient(result);
        self.pending_copy = false;
        let pasted = keysynth::paste();
        match watched {
            Some(focused) => {
                let deadline = Instant::now() + PASTE_SETTLE;
                while Instant::now() < deadline {
                    sleep(Duration::from_millis(10));
                    match focused.selection() {
                        Selection::Empty => break,
                        Selection::Text(t) if t != original => break,
                        _ => {}
                    }
                }
            }
            None => sleep(PASTE_SETTLE),
        }
        pasted
    }
}

/// Undo on a line read up to the caret: when the line ends with `result` (give or take trailing
/// whitespace), how much of it to keep and what replaces the rest. `None` when it does not.
fn undo_line(line: &str, original: &str, result: &str) -> Option<(usize, String)> {
    let result = result.trim_end();
    let end = line.trim_end().len();
    let start = end.checked_sub(result.len())?;
    let ours = !result.is_empty() && line.is_char_boundary(start) && line[start..end] == *result;
    ours.then(|| (start, format!("{}{}", original.trim_end(), &line[end..])))
}

/// The last whitespace-delimited word ending at or before `end` (UTF-16 offset).
fn previous_word(focused: &Focused, end: usize) -> Option<Word> {
    let from = end.saturating_sub(LOOKBEHIND_UNITS);
    let text = focused.string_for_range(from, end - from)?;
    if utf16_len(&text) != end - from {
        return None; // the app's idea of offsets is not UTF-16; do not guess
    }
    let trimmed = text.trim_end();
    let word_start = trimmed.rfind(char::is_whitespace).map_or(0, |i| i + trimmed[i..].chars().next().unwrap().len_utf8());
    let word = &trimmed[word_start..];
    (!word.is_empty()).then(|| Word {
        start: from + utf16_len(&text[..word_start]),
        len: utf16_len(word),
        text: word.to_string(),
    })
}

/// The line up to the caret (UTF-16 offset), without its leading whitespace: `Ok(None)` when it
/// holds nothing but whitespace, `Err(TooLong)` when no line start is found within [`MAX_CHARS`].
fn line_before(focused: &Focused, caret: usize) -> Result<Option<Word>, Outcome> {
    // MAX_CHARS characters take at most twice as many UTF-16 units; one more tells a line that
    // starts exactly there from one that goes on.
    let from = caret.saturating_sub(2 * MAX_CHARS + 1);
    let Some(text) = focused.string_for_range(from, caret - from) else { return Err(Outcome::NoText) };
    if utf16_len(&text) != caret - from {
        return Err(Outcome::NoText); // the app's idea of offsets is not UTF-16; do not guess
    }
    let Some(line) = line_tail(&text, from == 0) else { return Err(Outcome::TooLong) };
    let start = from + utf16_len(&text[..text.len() - line.len()]);
    if line.trim().is_empty() {
        return Ok(None);
    }
    if line.chars().count() > MAX_CHARS {
        return Err(Outcome::TooLong);
    }
    Ok(Some(Word { start, len: utf16_len(line), text: line.to_string() }))
}

/// The end of `text` after its last line break, leading whitespace dropped. `None` when there is
/// no line break and `text` is not the start of the field (`at_start`): the line starts further back.
fn line_tail(text: &str, at_start: bool) -> Option<&str> {
    let line = match text.rfind(['\n', '\r', '\u{2028}', '\u{2029}']) {
        Some(i) => &text[i + text[i..].chars().next().unwrap().len_utf8()..],
        None if at_start => text,
        None => return None,
    };
    Some(line.trim_start())
}

/// The last whitespace-delimited word of `line`, with any whitespace that follows it.
fn last_word_and_trailing_space(line: &str) -> Option<&str> {
    let trimmed = line.trim_end();
    let start = trimmed.rfind(char::is_whitespace).map_or(0, |i| i + trimmed[i..].chars().next().unwrap().len_utf8());
    (start < trimmed.len()).then(|| &line[start..])
}

/// How many Shift-arrow presses cover `s`: Arabic marks ride on their base letter.
fn caret_stops(s: &str) -> usize {
    s.chars().filter(|&c| !matches!(c as u32, 0x064B..=0x065F | 0x0670)).count()
}

fn utf16_len(s: &str) -> usize {
    s.encode_utf16().count()
}

pub fn build_map(layouts: &Layouts) -> LayoutMap {
    match (&layouts.arabic, &layouts.latin) {
        (Some(arabic), Some(latin)) => LayoutMap::build(arabic, latin),
        // No Arabic layout enabled: the bundled macOS "Arabic" is the best guess.
        _ => LayoutMap::arabic_mac(),
    }
}

fn opposite(arrow: u16) -> u16 {
    if arrow == KEY_LEFT {
        KEY_RIGHT
    } else {
        KEY_LEFT
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_short_conversions_are_shown_word_for_word() {
        assert!(fits_display("اثممخ", "hello"));
        let long = "اثممخ صخقمي اثممخ صخقمي اثممخ";
        assert!(!fits_display(long, "hello world hello world hello"));
        assert!(fits_display(&"ب".repeat(DISPLAY_MAX), &"b".repeat(DISPLAY_MAX)));
        assert!(!fits_display(&"ب".repeat(DISPLAY_MAX + 1), "b"));
    }

    #[test]
    fn the_line_is_what_follows_the_last_line_break() {
        assert_eq!(line_tail("Dear team,\nsghl ugd;l", false), Some("sghl ugd;l"));
        assert_eq!(line_tail("first\r\n  indented", false), Some("indented"));
        assert_eq!(line_tail("a\u{2028}b", false), Some("b"));
        assert_eq!(line_tail("only line", true), Some("only line"));
        // No line break, and the field goes on further back: the line's start is out of reach.
        assert_eq!(line_tail("no break here", false), None);
        // Right after a line break: an empty line.
        assert_eq!(line_tail("previous\n", false), Some(""));
    }

    #[test]
    fn a_route_waits_for_copies_no_longer_than_the_app_needs() {
        let mut route = Route::default();
        assert_eq!(route.copy_timeout(), COPY_TIMEOUT);
        let mut target = Target::new(None);
        target.copy_times = vec![Duration::from_millis(12), Duration::from_millis(30)];
        target.arrow = Some(KEY_RIGHT);
        route.learn(&target);
        // Two answers are not enough to go on.
        assert_eq!(route.copy_timeout(), COPY_TIMEOUT);
        assert_eq!(route.arrow, Some(KEY_RIGHT));
        target.copy_times = vec![Duration::from_millis(20)];
        target.arrow = None;
        route.learn(&target);
        assert_eq!(route.copy_timeout(), COPY_TIMEOUT_MIN.max(Duration::from_millis(60)));
        // The arrow that worked stays until another one works.
        assert_eq!(route.arrow, Some(KEY_RIGHT));
        target.copy_times = vec![Duration::from_millis(100)];
        route.learn(&target);
        assert_eq!(route.copy_timeout(), Duration::from_millis(200));
        target.copy_times = vec![Duration::from_millis(400)];
        route.learn(&target);
        assert_eq!(route.copy_timeout(), COPY_TIMEOUT);
    }

    #[test]
    fn undo_puts_the_original_back_at_the_end_of_the_line() {
        assert_eq!(undo_line("Dear team, hello", "اثممخ", "hello"), Some((11, "اثممخ".into())));
        // The space typed after the word stays where it was.
        assert_eq!(undo_line("Dear team, hello ", "اثممخ ", "hello "), Some((11, "اثممخ ".into())));
        assert_eq!(undo_line("السلام عليكم sghl", "sghl", "سلام").map(|(keep, _)| keep), None);
        assert_eq!(undo_line("قال سلام", "sghl", "سلام"), Some(("قال ".len(), "sghl".into())));
    }

    #[test]
    fn undo_leaves_a_line_alone_unless_it_ends_with_our_text() {
        // The caret moved on, the word was edited, or the line is too short to hold it.
        assert_eq!(undo_line("hello world", "اثممخ", "hello"), None);
        assert_eq!(undo_line("Dear team, hellp", "اثممخ", "hello"), None);
        assert_eq!(undo_line("llo", "اثممخ", "hello"), None);
        assert_eq!(undo_line("anything", "", "   "), None);
    }

    #[test]
    fn a_failed_undo_offers_the_original_and_keeps_undo_while_it_can_still_work() {
        let fresh = || LastOp::new("اثممخ".into(), "hello".into(), Direction::ArabicToLatin, Some(0), 1);
        for reason in [Reason::OtherApp, Reason::NotHere, Reason::UndoRejected] {
            let mut op = fresh();
            op.undo_failed(reason);
            assert!(op.undoable && op.copyable, "{reason:?}");
        }
        for reason in [Reason::TextChanged, Reason::TooLong] {
            let mut op = fresh();
            op.undo_failed(reason);
            assert!(!op.undoable && op.copyable, "{reason:?}");
        }
    }

    #[test]
    fn expected_states_are_announced_once_not_on_every_press() {
        let mut rationed = Rationed::default();
        assert!(rationed.permission_due());
        assert!(!rationed.permission_due());
        rationed.permission = Some(Instant::now() - PERMISSION_NOTICE_EVERY);
        assert!(rationed.permission_due());

        assert!(rationed.pause_due(4));
        assert!(!rationed.pause_due(4));
        // Resumed and paused again: a new pause, told once more.
        assert!(rationed.pause_due(6));

        assert!(rationed.excluded_due(Some("com.apple.Terminal")));
        assert!(!rationed.excluded_due(Some("com.apple.Terminal")));
        assert!(rationed.excluded_due(Some("com.googlecode.iterm2")));
        assert!(!rationed.excluded_due(None));
    }
}
