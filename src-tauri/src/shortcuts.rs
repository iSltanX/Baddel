//! Binding the global shortcuts, and showing them the way macOS writes them.

use std::str::FromStr;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::controller::{Command, Commands};
use crate::settings::{Binding, Settings};
use crate::sync;

/// Re-registers every bound shortcut, returning the ones macOS would not give us —
/// almost always because another app holds them. Two commands never share one shortcut:
/// should settings ever hold a duplicate, the later command goes without.
pub fn apply(app: &AppHandle, settings: &Settings) -> Vec<Binding> {
    let manager = app.global_shortcut();
    let _ = manager.unregister_all();
    let mut refused = Vec::new();
    let mut taken: Vec<Shortcut> = Vec::new();
    for binding in Binding::ALL {
        let accelerator = settings.binding(binding);
        if accelerator.is_empty() {
            continue;
        }
        let Ok(shortcut) = Shortcut::from_str(accelerator) else {
            refused.push(binding);
            continue;
        };
        if taken.contains(&shortcut) {
            refused.push(binding);
            continue;
        }
        taken.push(shortcut);
        let handle = app.clone();
        // Act on release: by then the hotkey's own keys are on their way up and
        // cannot combine with the keys the conversion synthesizes.
        let registered = manager.on_shortcut(shortcut, move |_app, _shortcut, event| {
            if event.state() == ShortcutState::Released {
                fire(&handle, binding);
            }
        });
        if registered.is_err() {
            refused.push(binding);
        }
    }
    refused
}

/// While the shortcut recorder listens, Baddel's own shortcuts are let go: registered, they
/// would catch the very keys being recorded (a conversion instead of a recording), so a
/// combination Baddel already uses could never reach the recorder — nor its conflict message.
/// `false` binds them again from the settings.
pub fn suspend(app: &AppHandle, suspended: bool) {
    if suspended {
        let _ = app.global_shortcut().unregister_all();
    } else {
        apply(app, &app.state::<crate::settings::AppState>().get());
    }
}

fn fire(app: &AppHandle, binding: Binding) {
    match binding {
        Binding::Convert => app.state::<Commands>().send(Command::Convert),
        Binding::Undo => app.state::<Commands>().send(Command::Undo),
        Binding::Pause => sync::toggle_pause(app),
    }
}

/// Whether an accelerator can be registered at all, without taking it.
pub fn is_valid(accelerator: &str) -> bool {
    Shortcut::from_str(accelerator).is_ok()
}

/// The other Baddel command already bound to `accelerator`, if any. Compared as key
/// combinations, so "Shift+Alt+Space" and "Alt+Shift+Space" are the same shortcut.
pub fn owner(settings: &Settings, binding: Binding, accelerator: &str) -> Option<Binding> {
    let wanted = Shortcut::from_str(accelerator).ok()?;
    Binding::ALL.into_iter().filter(|&other| other != binding).find(|&other| {
        Shortcut::from_str(settings.binding(other)).is_ok_and(|bound| bound == wanted)
    })
}

/// Renders an accelerator the way macOS does: "Alt+Shift+Space" → "⌥⇧Space".
/// Modifier glyphs are never mirrored in a right-to-left layout.
pub fn glyphs(accelerator: &str) -> Option<String> {
    if accelerator.is_empty() {
        return None;
    }
    let mut out = String::new();
    let mut key = "";
    for part in accelerator.split('+') {
        match part.to_ascii_lowercase().as_str() {
            "control" | "ctrl" => out.push('⌃'),
            "alt" | "option" => out.push('⌥'),
            "shift" => out.push('⇧'),
            "command" | "cmd" | "super" | "meta" => out.push('⌘'),
            _ => key = part,
        }
    }
    out.push_str(&pretty_key(key));
    Some(out)
}

/// The legend printed on the physical key.
fn pretty_key(key: &str) -> String {
    match key.to_ascii_lowercase().as_str() {
        "space" => "Space".into(),
        "enter" | "return" => "↩".into(),
        "escape" | "esc" => "⎋".into(),
        "backspace" => "⌫".into(),
        "delete" => "⌦".into(),
        "tab" => "⇥".into(),
        "arrowleft" | "left" => "←".into(),
        "arrowright" | "right" => "→".into(),
        "arrowup" | "up" => "↑".into(),
        "arrowdown" | "down" => "↓".into(),
        other => {
            // "KeyA" → "A", "Digit1" → "1", "F5" → "F5".
            let trimmed = other.trim_start_matches("key").trim_start_matches("digit");
            trimmed.to_uppercase()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_shortcut_bound_to_another_command_names_that_command() {
        let settings = Settings { shortcut_undo: "Alt+Shift+Z".into(), ..Settings::default() };
        assert_eq!(owner(&settings, Binding::Convert, "Alt+Shift+Z"), Some(Binding::Undo));
        // The same keys written in another order are the same shortcut.
        assert_eq!(owner(&settings, Binding::Pause, "Shift+Alt+Space"), Some(Binding::Convert));
        assert_eq!(owner(&settings, Binding::Pause, "Alt+Shift+P"), None);
    }

    #[test]
    fn a_command_does_not_conflict_with_itself_or_with_an_empty_binding() {
        let settings = Settings::default();
        assert_eq!(owner(&settings, Binding::Convert, "Alt+Shift+Space"), None);
        assert_eq!(owner(&settings, Binding::Undo, "Alt+Shift+U"), None);
    }
}
