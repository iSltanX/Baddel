//! The conversion itself: hotkey → read the text → convert → write it back.
//!
//! Runs on its own thread; commands arrive over a channel. Nothing the user typed
//! is ever logged or written to disk — the last operation lives in memory only,
//! for undo and for extending the conversion one word further back.

use std::collections::HashSet;
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::thread::{self, sleep};
use std::time::{Duration, Instant};

use baddel_core::{detect_direction, Direction, LayoutMap};
use tauri::{AppHandle, Emitter, Manager};

use crate::menu_text;
use crate::settings::{AppState, Language};
use crate::sys::input_source::Snapshot as Layouts;
use crate::sys::keysynth::{self, KEY_LEFT, KEY_RIGHT};
use crate::sys::text_access::{Focused, Selection};
use crate::sys::{frontmost, input_source, on_main, pasteboard, permissions, sound, text_access};
use crate::{hud, tray};

// ── Timings ──────────────────────────────────────────────────────────────────
/// How long we wait for the user to release the hotkey's modifiers.
const MODIFIER_RELEASE_TIMEOUT: Duration = Duration::from_millis(700);
/// How long an app may take to put the selection on the pasteboard after ⌘C.
const COPY_TIMEOUT: Duration = Duration::from_millis(250);
/// Minimum time for an app to act on a selection key before we look at the selection.
const SELECTION_SETTLE: Duration = Duration::from_millis(40);
/// How long we keep looking for the selection a key press should have produced. Apps handle
/// the key asynchronously; reading too early sees "nothing selected".
const SELECTION_TIMEOUT: Duration = Duration::from_millis(300);
/// Time for an app to consume ⌘V before we put the user's pasteboard back.
const PASTE_SETTLE: Duration = Duration::from_millis(200);
/// How long a Chromium/Electron app gets to build its accessibility tree after we ask for it.
const AX_WAKE_TIMEOUT: Duration = Duration::from_millis(600);
/// Pressing the hotkey again within this window extends the last conversion.
const EXTEND_WINDOW: Duration = Duration::from_secs(2);
/// Undo is offered for this long after a conversion.
const UNDO_WINDOW: Duration = Duration::from_secs(30);

const MAX_CHARS: usize = 10_000;
/// How far back from the caret we read when looking for the previous word (UTF-16 units).
const LOOKBEHIND_UNITS: usize = 400;
/// Undo reselects the result character by character; beyond this it is not worth it.
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
    Undo,
}

/// The channel into the conversion thread. Held in Tauri state so the menu and the
/// global shortcuts can reach it from wherever they run.
pub struct Commands(Mutex<Sender<Command>>);

impl Commands {
    pub fn send(&self, command: Command) {
        let _ = self.0.lock().unwrap().send(command);
    }
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
    Failed,
}

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
    at: Instant,
}

pub fn spawn(app: AppHandle) -> Commands {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("baddel-controller".into())
        .spawn(move || {
            let mut controller = Controller { app, last: None, shown: None, woken: HashSet::new() };
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
                let outcome = match command {
                    Command::Convert => controller.convert(),
                    Command::Undo => controller.undo(),
                };
                #[cfg(debug_assertions)]
                eprintln!("[baddel] {outcome:?} in {:?}", started.elapsed());
                let _ = started;
                controller.sync_menu();
                controller.announce(outcome);
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
    /// Which conversion the menu currently shows (by its time), so it is redrawn only on change.
    shown: Option<Instant>,
    /// Apps we already asked to switch their accessibility tree on.
    woken: HashSet<i32>,
}

/// How the current text was obtained, and therefore how to write it back.
struct Target {
    focused: Option<Focused>,
    method: Method,
    /// Pasteboard contents to put back, taken before our first ⌘C.
    saved: Option<pasteboard::Snapshot>,
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
    /// The direction implied by the active layout: the one the wrong text was typed with.
    expected: Direction,
    switch_input_source: bool,
}

impl Controller {
    fn convert(&mut self) -> Outcome {
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
        if frontmost::bundle_id().is_some_and(|id| settings.excluded_apps.contains(&id)) {
            return Outcome::Excluded;
        }
        let (arabic, latin) = (settings.arabic_layout.clone(), settings.latin_layout.clone());
        let Some(layouts) = on_main(&self.app, move || input_source::snapshot(&arabic, &latin)) else {
            return Outcome::Failed;
        };
        let map = build_map(&layouts);
        if map.is_empty() {
            return Outcome::NoLayouts;
        }
        keysynth::wait_for_modifiers_released(MODIFIER_RELEASE_TIMEOUT);

        let expected =
            if layouts.current_is_arabic { Direction::ArabicToLatin } else { Direction::LatinToArabic };
        let cx = Context { map: &map, layouts: &layouts, expected, switch_input_source: settings.switch_input_source };
        let mut target = Target { focused: self.focused_element(), method: Method::Accessibility, saved: None };
        // Debug aid: exercise the key-based path in apps that would never need it.
        #[cfg(debug_assertions)]
        if std::env::var_os("BADDEL_FORCE_KEYS").is_some() {
            target.focused = None;
            target.method = Method::Pasteboard;
        }
        let outcome = self.convert_in(&mut target, &cx);
        if let Some(saved) = &target.saved {
            pasteboard::restore(saved);
        }
        outcome
    }

    /// The focused element, waking the app's accessibility tree the first time it has none.
    fn focused_element(&mut self) -> Option<Focused> {
        if let Some(focused) = Focused::get() {
            return Some(focused);
        }
        let pid = frontmost::pid()?;
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

    fn convert_in(&mut self, target: &mut Target, cx: &Context) -> Outcome {
        // 1. An explicit selection wins.
        if let Some(text) = target.read_selection() {
            if text.chars().count() > MAX_CHARS {
                return Outcome::TooLong;
            }
            let direction = detect_direction(&text).unwrap_or(cx.expected);
            let result = cx.map.convert_as(&text, direction);
            if result == text {
                return Outcome::Unchanged;
            }
            if !target.write(&text, &result) {
                return Outcome::Failed;
            }
            self.finish(LastOp::new(text, result, direction, None, 0), cx);
            return Outcome::Converted;
        }

        // 2. No selection, right after a conversion: take one more word.
        if let Some(last) = self.last.take().filter(|l| l.at.elapsed() < EXTEND_WINDOW && l.words > 0) {
            match self.extend(target, cx, last) {
                Extension::Done(outcome) => return outcome,
                // The caret is somewhere else: this is a new conversion, not a continuation.
                Extension::Elsewhere => {}
                // Could not tell. Falling through would find the word we just fixed and
                // convert it back, so stop here.
                Extension::Unknown => return Outcome::NoText,
            }
        }

        // 3. No selection: the word before the caret.
        self.convert_last_word(target, cx)
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
                let direction = detect_direction(&word.text).unwrap_or(cx.expected);
                let result = cx.map.convert_as(&word.text, direction);
                if result == word.text {
                    return Outcome::Unchanged;
                }
                if !target.replace_range(&word, &result, caret) {
                    return Outcome::Failed;
                }
                self.finish(LastOp::new(word.text, result, direction, Some(word.start), 1), cx);
                return Outcome::Converted;
            }
        }

        trace!("key path ({:?})", target.method);
        // Fallback for apps without a text range. Arabic script runs right to left, so in apps
        // that move the caret visually "back" is the right arrow; try the likelier side first.
        let arrows = match cx.expected {
            Direction::ArabicToLatin => [KEY_RIGHT, KEY_LEFT],
            Direction::LatinToArabic => [KEY_LEFT, KEY_RIGHT],
        };
        for arrow in arrows {
            let edit = target.edit_line_with_keys(arrow, |line| {
                let tail = last_word_and_trailing_space(line)?;
                let direction = detect_direction(tail).unwrap_or(cx.expected);
                let converted = cx.map.convert_as(tail, direction);
                (converted != tail).then(|| LineEdit { keep: line.len() - tail.len(), converted, direction })
            });
            match edit {
                KeyEdit::Done { original, result, direction } => {
                    let mut op = LastOp::new(original, result, direction, None, 1);
                    op.back_arrow = arrow;
                    self.finish(op, cx);
                    return Outcome::Converted;
                }
                KeyEdit::Failed => return Outcome::Failed,
                KeyEdit::NothingHere => continue,
            }
        }
        Outcome::NoText
    }

    /// Converts the word before the text we produced last time.
    fn extend(&mut self, target: &mut Target, cx: &Context, last: LastOp) -> Extension {
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
            let converted = cx.map.convert_as(&word.text, last.direction);
            if converted == word.text {
                return Extension::Done(Outcome::Unchanged);
            }
            if !target.replace_range(&word, &converted, caret) {
                return Extension::Done(Outcome::Failed);
            }
            let original = format!("{}{gap}{}", word.text, last.original);
            let result = format!("{converted}{gap}{}", last.result);
            self.finish(LastOp::new(original, result, last.direction, Some(word.start), last.words + 1), cx);
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
            let converted = format!("{}{}", cx.map.convert_as(word, last.direction), &line[start..]);
            (converted != line[keep..]).then_some(LineEdit { keep, converted, direction: last.direction })
        });
        match edit {
            KeyEdit::Done { original, result, direction } => {
                let mut op = LastOp::new(original, result, direction, None, last.words + 1);
                op.back_arrow = last.back_arrow;
                self.finish(op, cx);
                Extension::Done(Outcome::Extended)
            }
            KeyEdit::Failed => Extension::Done(Outcome::Failed),
            KeyEdit::NothingHere => Extension::Unknown,
        }
    }

    fn finish(&mut self, op: LastOp, cx: &Context) {
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

    fn undo(&mut self) -> Outcome {
        let Some(last) = self.last.take().filter(|l| l.at.elapsed() < UNDO_WINDOW) else {
            return Outcome::NoText;
        };
        if !permissions::is_trusted(false) || permissions::secure_input_enabled() {
            return Outcome::Failed;
        }
        keysynth::wait_for_modifiers_released(MODIFIER_RELEASE_TIMEOUT);
        let mut target = Target { focused: Focused::get(), method: Method::Accessibility, saved: None };
        let outcome = self.undo_in(&mut target, &last);
        if let Some(saved) = &target.saved {
            pasteboard::restore(saved);
        }
        outcome
    }

    fn undo_in(&mut self, target: &mut Target, last: &LastOp) -> Outcome {
        if let (Some(start), Some(focused)) = (last.start, target.focused.as_ref()) {
            let len = utf16_len(&last.result);
            if focused.string_for_range(start, len).as_deref() != Some(last.result.as_str()) {
                return Outcome::Failed;
            }
            let word = Word { start, len, text: last.result.clone() };
            let caret = focused.selected_range().map_or(start + len, |(c, _)| c);
            return if target.replace_range(&word, &last.original, caret) { Outcome::Undone } else { Outcome::Failed };
        }

        let stops = caret_stops(&last.result);
        if stops > MAX_UNDO_CHARS {
            return Outcome::Failed;
        }
        for _ in 0..stops {
            keysynth::select_char(last.back_arrow);
        }
        match target.await_selection_where(|s| s == last.result) {
            Some(selected) if selected == last.result && target.write(&selected, &last.original) => Outcome::Undone,
            _ => {
                keysynth::arrow(opposite(last.back_arrow));
                Outcome::Failed
            }
        }
    }
}

impl Controller {
    /// Makes the menu's "last conversion" item match what undo can still act on: shown while
    /// it is remembered, gone once it is undone, expired, or lost to a failed undo.
    fn sync_menu(&mut self) {
        let current = self.last.as_ref().map(|l| l.at);
        if current != self.shown {
            self.shown = current;
            let last = self.last.as_ref().map(|l| (l.original.clone(), l.result.clone()));
            tray::set_last_conversion(&self.app, last);
        }
    }

    /// Shows the outcome: the notice panel and the click sound. The panel is native and
    /// reads the text straight out of memory — it is never written anywhere, nor handed
    /// to a webview.
    fn announce(&self, outcome: Outcome) {
        let settings = self.app.state::<AppState>().settings.lock().unwrap().clone();
        let text = menu_text::strings(settings.language);
        let rtl = settings.language == Language::Ar;

        if matches!(outcome, Outcome::Converted | Outcome::Extended) && settings.sound {
            on_main(&self.app, sound::click);
        }

        if !settings.show_hud {
            return;
        }
        let notice = match outcome {
            Outcome::Converted | Outcome::Extended => self.last.as_ref().map(|last| hud::Notice {
                kind: hud::Kind::Success,
                text: hud::conversion_line(&last.original, &last.result, rtl),
                badge: settings.switch_input_source.then(|| badge(last.direction).to_string()),
            }),
            Outcome::Undone => Some(notice(hud::Kind::Undone, text.hud_undone)),
            Outcome::Blocked => Some(notice(hud::Kind::Blocked, text.hud_blocked)),
            Outcome::TooLong => Some(notice(hud::Kind::TooLong, text.hud_too_long)),
            Outcome::NoText | Outcome::Unchanged => Some(notice(hud::Kind::NoText, text.hud_no_text)),
            // Paused, excluded, missing permission: the menu already says so, and a
            // notice on every keypress would be noise.
            _ => None,
        };
        if let Some(notice) = notice {
            hud::show(&self.app, notice);
        }
    }
}

/// The input source the conversion switched to.
fn badge(direction: Direction) -> &'static str {
    match direction {
        Direction::ArabicToLatin => "EN",
        Direction::LatinToArabic => "ع",
    }
}

fn notice(kind: hud::Kind, text: &str) -> hud::Notice {
    hud::Notice { kind, text: text.to_string(), badge: None }
}

impl LastOp {
    fn new(original: String, result: String, direction: Direction, start: Option<usize>, words: usize) -> Self {
        // Walking back over the result: Latin text runs left to right, Arabic right to left.
        let back_arrow = match direction {
            Direction::ArabicToLatin => KEY_LEFT,
            Direction::LatinToArabic => KEY_RIGHT,
        };
        LastOp { original, result, direction, start, back_arrow, words, at: Instant::now() }
    }
}

impl Target {
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
        if self.saved.is_none() {
            self.saved = Some(pasteboard::snapshot());
        }
        let before = pasteboard::change_count();
        keysynth::copy();
        if !pasteboard::wait_for_change(before, COPY_TIMEOUT) {
            return None;
        }
        let text = pasteboard::read_string().filter(|t| !t.is_empty())?;
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
        keysynth::select_to_line_edge(arrow);
        let line = self.await_selection();
        trace!("key path: line read = {:?} chars", line.as_ref().map(|l| l.chars().count()));
        let Some(line) = line else { return KeyEdit::NothingHere };
        let Some(edit) = plan(&line) else {
            keysynth::arrow(opposite(arrow)); // collapse back onto the caret
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
        if !self.write(&word.text, replacement) {
            return false;
        }
        let new_caret = (caret + utf16_len(replacement)).saturating_sub(word.len).max(word.start);
        // Best effort: a misplaced caret is not a failed conversion.
        if let Some(focused) = self.focused.as_ref() {
            focused.select_range(new_caret, 0);
        }
        true
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
        if self.saved.is_none() {
            self.saved = Some(pasteboard::snapshot());
        }
        // Where the app reports its selection, the paste can be seen landing: the selected
        // original gives way to the caret. Only then is it safe to put the user's pasteboard back.
        let watched = self
            .focused
            .as_ref()
            .filter(|f| matches!(f.selection(), Selection::Text(t) if t == original));
        pasteboard::write_transient(result);
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
