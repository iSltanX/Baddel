//! What Baddel can say about its own state, for support: copied from About, and sent with a
//! problem report once the user has seen it.
//!
//! Nothing here is text the user typed or converted, clipboard contents, a file or window name,
//! or anything that identifies the user or the Mac. The recent outcomes live in memory only and
//! are gone when the app quits.

use std::collections::VecDeque;
use std::sync::Mutex;
use std::time::{Duration, Instant};

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::controller::Outcome;
use crate::settings::{AppState, Language, Settings};
use crate::sys::{input_source, on_main, permissions, system};

/// How many recent outcomes are remembered.
const RECENT: usize = 10;

/// How a conversion reached the text.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum Path {
    /// Read and written through the accessibility text APIs.
    Accessibility,
    /// Copied and pasted, around a selection that was already there.
    Pasteboard,
    /// Selected with synthetic arrow keys, then copied and pasted.
    Keys,
}

struct Entry {
    outcome: Outcome,
    path: Option<Path>,
    took: Duration,
    at: Instant,
}

#[derive(Default)]
struct Inner {
    recent: VecDeque<Entry>,
    /// The frontmost app's bundle identifier at the last attempt.
    last_app: Option<String>,
}

/// The ring of recent outcomes, held in Tauri state.
#[derive(Default)]
pub struct History(Mutex<Inner>);

impl History {
    pub fn record(&self, outcome: Outcome, path: Option<Path>, took: Duration, app: Option<String>) {
        let mut inner = self.0.lock().unwrap();
        if inner.recent.len() == RECENT {
            inner.recent.pop_front();
        }
        inner.recent.push_back(Entry { outcome, path, took, at: Instant::now() });
        inner.last_app = app;
    }
}

/// The fields the shared report contract keeps at the top level.
#[derive(Clone, Debug, Serialize)]
pub struct Envelope {
    pub app_version: String,
    pub os: &'static str,
    pub os_version: String,
    pub arch: &'static str,
    pub locale: &'static str,
}

/// Everything else: the report's `diagnostics` object.
#[derive(Clone, Debug, Serialize)]
pub struct Details {
    pub accessibility: bool,
    pub secure_input: bool,
    pub layouts: Layouts,
    pub layout_choice: LayoutChoice,
    pub recent: Vec<Recent>,
    pub last_app: Option<String>,
    pub settings: SettingsSummary,
    pub build: &'static str,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct Layouts {
    pub arabic: Option<String>,
    pub arabic_name: Option<String>,
    pub latin: Option<String>,
    pub latin_name: Option<String>,
    /// Which of the two is the active input source: "arabic" or "latin".
    pub active: &'static str,
    /// "live" when both layouts come from macOS, "bundled" when the built-in Arabic stands in.
    pub source: &'static str,
}

/// The layouts chosen in Settings: an input source id, or "auto" to follow macOS.
#[derive(Clone, Debug, Serialize)]
pub struct LayoutChoice {
    pub arabic: String,
    pub latin: String,
}

#[derive(Clone, Debug, Serialize)]
pub struct Recent {
    pub outcome: Outcome,
    pub path: Option<Path>,
    pub ms: u64,
    pub ago_s: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct Shortcuts {
    pub convert: String,
    pub undo: String,
    pub pause: String,
}

/// The general preferences. The exceptions are a count, not a list: bundle identifiers would
/// tell which apps are installed.
#[derive(Clone, Debug, Serialize)]
pub struct SettingsSummary {
    pub language: Language,
    pub launch_at_login: bool,
    pub show_tray_icon: bool,
    pub switch_input_source: bool,
    pub show_hud: bool,
    pub sound: bool,
    pub auto_update: bool,
    pub paused: bool,
    pub shortcuts: Shortcuts,
    pub excluded_count: usize,
    /// Whether the exceptions are exactly the defaults.
    pub excluded_default: bool,
}

#[derive(Clone, Debug)]
pub struct Snapshot {
    pub envelope: Envelope,
    pub details: Details,
}

/// The copy on the pasteboard for "Copy Diagnostics": the same fields a report carries.
#[derive(Serialize)]
struct Copy<'a> {
    #[serde(flatten)]
    envelope: &'a Envelope,
    diagnostics: &'a Details,
}

impl Snapshot {
    pub fn to_pretty_json(&self) -> String {
        serde_json::to_string_pretty(&Copy { envelope: &self.envelope, diagnostics: &self.details })
            .unwrap_or_default()
    }
}

/// Gathers the current state. Touches the input-source API, so it hops to the main thread.
pub fn collect(app: &AppHandle) -> Snapshot {
    let settings = app.state::<AppState>().get();
    let (arabic, latin) = (settings.arabic_layout.clone(), settings.latin_layout.clone());
    let live = on_main(app, move || (input_source::snapshot(&arabic, &latin), input_source::list()));
    let layouts = live.map(|(snap, names)| {
        let name_of = |id: &str| names.iter().find(|e| e.id == id).map(|e| e.name.clone());
        let (arabic, latin) = (snap.arabic.as_ref().map(|l| l.id.clone()), snap.latin.as_ref().map(|l| l.id.clone()));
        Layouts {
            arabic_name: arabic.as_deref().and_then(name_of),
            latin_name: latin.as_deref().and_then(name_of),
            source: if arabic.is_some() && latin.is_some() { "live" } else { "bundled" },
            active: if snap.current_is_arabic { "arabic" } else { "latin" },
            arabic,
            latin,
        }
    });
    let history = app.state::<History>();
    let recent = {
        let inner = history.0.lock().unwrap();
        let recent = inner.recent.iter().rev().map(|e| Recent {
            outcome: e.outcome,
            path: e.path,
            ms: e.took.as_millis() as u64,
            ago_s: e.at.elapsed().as_secs(),
        });
        (recent.collect(), inner.last_app.clone())
    };
    Snapshot {
        envelope: Envelope {
            app_version: app.package_info().version.to_string(),
            os: "macos",
            os_version: system::os_version(),
            arch: system::arch(),
            locale: locale(settings.language),
        },
        details: Details {
            accessibility: permissions::is_trusted(false),
            secure_input: permissions::secure_input_enabled(),
            layouts: layouts.unwrap_or_default(),
            layout_choice: LayoutChoice { arabic: choice(&settings.arabic_layout), latin: choice(&settings.latin_layout) },
            recent: recent.0,
            last_app: recent.1,
            settings: summary(&settings),
            build: if cfg!(debug_assertions) { "debug" } else { "release" },
        },
    }
}

pub fn locale(language: Language) -> &'static str {
    match language {
        Language::Ar => "ar",
        Language::En => "en",
    }
}

fn choice(id: &str) -> String {
    if id.is_empty() { "auto".into() } else { id.into() }
}

fn summary(s: &Settings) -> SettingsSummary {
    let mut defaults = Settings::default().excluded_apps;
    let mut current = s.excluded_apps.clone();
    defaults.sort();
    current.sort();
    SettingsSummary {
        language: s.language,
        launch_at_login: s.launch_at_login,
        show_tray_icon: s.show_tray_icon,
        switch_input_source: s.switch_input_source,
        show_hud: s.show_hud,
        sound: s.sound,
        auto_update: s.auto_update,
        paused: s.paused,
        shortcuts: Shortcuts {
            convert: s.shortcut_convert.clone(),
            undo: s.shortcut_undo.clone(),
            pause: s.shortcut_pause.clone(),
        },
        excluded_count: s.excluded_apps.len(),
        excluded_default: current == defaults,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_ring_keeps_the_last_ten_newest_first() {
        let history = History::default();
        for i in 0..12u64 {
            history.record(Outcome::Converted, Some(Path::Accessibility), Duration::from_millis(i), Some(format!("app.{i}")));
        }
        let inner = history.0.lock().unwrap();
        assert_eq!(inner.recent.len(), RECENT);
        assert_eq!(inner.recent.front().unwrap().took, Duration::from_millis(2));
        assert_eq!(inner.last_app.as_deref(), Some("app.11"));
    }

    #[test]
    fn the_summary_counts_exceptions_without_naming_them() {
        let mut settings = Settings::default();
        let s = summary(&settings);
        assert!(s.excluded_default);
        settings.excluded_apps.push("com.example.private".into());
        let s = summary(&settings);
        assert_eq!(s.excluded_count, 15);
        assert!(!s.excluded_default);
        assert!(!serde_json::to_string(&s).unwrap().contains("com.example"));
    }
}
