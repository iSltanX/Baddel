//! The conversion itself: hotkey → read the text → convert → write it back.
//!
//! Runs on its own thread; commands arrive over a channel. Nothing the user typed
//! is ever logged or written to disk — the last operation lives in memory only,
//! for undo and for extending the conversion one word further back.

use std::collections::HashSet;
use std::sync::mpsc::{self, Sender};
use std::thread::{self, sleep};
use std::time::{Duration, Instant};

use baddel_core::{detect_direction, Direction, LayoutMap};
use tauri::{AppHandle, Emitter, Manager};

use crate::state::AppState;
use crate::sys::input_source::Snapshot as Layouts;
use crate::sys::keysynth::{self, KEY_LEFT, KEY_RIGHT};
use crate::sys::text_access::{Focused, Selection};
use crate::sys::{frontmost, input_source, on_main, pasteboard, permissions, text_access};

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

pub fn spawn(app: AppHandle) -> Sender<Command> {
    let (tx, rx) = mpsc::channel();
    thread::Builder::new()
        .name("baddel-controller".into())
        .spawn(move || {
            let mut controller = Controller { app, last: None, woken: HashSet::new() };
            for command in rx {
                let started = Instant::now();
                let outcome = match command {
                    Command::Convert => controller.convert(),
                    Command::Undo => controller.undo(),
                };
                #[cfg(debug_assertions)]
                eprintln!("[baddel] {outcome:?} in {:?}", started.elapsed());
                let _ = started;
                let _ = controller.app.emit("conversion", outcome);
            }
        })
        .expect("failed to start the controller thread");
    tx
}

struct Controller {
    app: AppHandle,
    last: Option<LastOp>,
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
        let Some(layouts) = on_main(&self.app, input_source::snapshot) else { return Outcome::Failed };
        let map = build_map(&layouts);
        if map.is_empty() {
            return Outcome::NoLayouts;
        }
        keysynth::wait_for_modifiers_released(MODIFIER_RELEASE_TIMEOUT);

        let expected =
            if layouts.current_is_arabic { Direction::ArabicToLatin } else { Direction::LatinToArabic };
        let cx = Context { map: &map, layouts: &layouts, expected, switch_input_source: settings.switch_input_source };
        let mut target = Target { focused: self.focused_element(), method: Method::Accessibility, saved: None };
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
        if !self.woken.insert(pid) || !text_access::enable_accessibility(pid) {
            return None;
        }
        trace!("asked pid {pid} to enable accessibility");
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
            let Some(text) = target.select_tail_with_keys(arrow, last_word_and_trailing_space) else { continue };
            let direction = detect_direction(&text).unwrap_or(cx.expected);
            let result = cx.map.convert_as(&text, direction);
            if result == text {
                keysynth::arrow(opposite(arrow));
                return Outcome::Unchanged;
            }
            if !target.write(&text, &result) {
                return Outcome::Failed;
            }
            let mut op = LastOp::new(text, result, direction, None, 1);
            op.back_arrow = arrow;
            self.finish(op, cx);
            return Outcome::Converted;
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

        // Key-based: reselect what we produced plus the word before it, and convert that word.
        trace!("extend: key path");
        let produced = last.result.clone();
        let tail = target.select_tail_with_keys(last.back_arrow, move |line| {
            let end = line.trim_end().len();
            let start = end.checked_sub(produced.len()).filter(|&i| line.is_char_boundary(i) && line[i..end] == produced)?;
            let word = last_word_and_trailing_space(&line[..start])?;
            Some(&line[start - word.len()..])
        });
        let Some(tail) = tail else { return Extension::Unknown };
        let head_len = tail.trim_end().len() - last.result.len();
        let (head, rest) = tail.split_at(head_len);
        let result = format!("{}{rest}", cx.map.convert_as(head, last.direction));
        if result == tail {
            keysynth::arrow(opposite(last.back_arrow));
            return Extension::Done(Outcome::Unchanged);
        }
        if !target.write(&tail, &result) {
            return Extension::Done(Outcome::Failed);
        }
        let original = format!("{head}{}", last.original);
        let mut op = LastOp::new(original, result, last.direction, None, last.words + 1);
        op.back_arrow = last.back_arrow;
        self.finish(op, cx);
        Extension::Done(Outcome::Extended)
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

        let chars = last.result.chars().count();
        if chars > MAX_UNDO_CHARS {
            return Outcome::Failed;
        }
        for _ in 0..chars {
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
        loop {
            let selection = self.read_selection();
            let settled = selection.as_deref().is_some_and(&done);
            if settled || self.method == Method::Pasteboard || Instant::now() >= deadline {
                return selection;
            }
            sleep(Duration::from_millis(15));
        }
    }

    /// Key-based selection of the end of the current line. Selects back to the line's edge to
    /// read it, lets `pick` choose the suffix to work on, then selects exactly that suffix
    /// character by character. Word-selection keys are useless here: wrongly typed Arabic is
    /// full of `;` `,` `[` `'`, which those keys treat as word breaks.
    fn select_tail_with_keys(&mut self, arrow: u16, pick: impl Fn(&str) -> Option<&str>) -> Option<String> {
        keysynth::select_to_line_edge(arrow);
        let line = self.await_selection();
        trace!("key path: line read = {:?} chars", line.as_ref().map(|l| l.chars().count()));
        let line = line?;
        keysynth::arrow(opposite(arrow)); // collapse back onto the caret
        let tail = pick(&line)?.to_string();
        trace!("key path: selecting a tail of {} caret stops", caret_stops(&tail));
        for _ in 0..caret_stops(&tail) {
            keysynth::select_char(arrow);
        }
        match self.await_selection_where(|s| s == tail) {
            Some(selected) if selected == tail => Some(tail),
            other => {
                trace!("key path: tail mismatch, got {:?} chars", other.as_ref().map(|s| s.chars().count()));
                keysynth::arrow(opposite(arrow));
                None
            }
        }
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
        pasteboard::write_transient(result);
        let pasted = keysynth::paste();
        sleep(PASTE_SETTLE);
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

fn build_map(layouts: &Layouts) -> LayoutMap {
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
