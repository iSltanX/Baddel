//! Binding the global shortcuts, and showing them the way macOS writes them.

use std::str::FromStr;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use baddel_core::Direction;

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
        Binding::ConvertLine => app.state::<Commands>().send(Command::ConvertLine),
        Binding::ToArabic => app.state::<Commands>().send(Command::ConvertTo(Direction::LatinToArabic)),
        Binding::ToLatin => app.state::<Commands>().send(Command::ConvertTo(Direction::ArabicToLatin)),
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
        "enter" | "return" | "numpadenter" => "↩".into(),
        "escape" | "esc" => "⎋".into(),
        "backspace" => "⌫".into(),
        "delete" => "⌦".into(),
        "tab" => "⇥".into(),
        "arrowleft" | "left" => "←".into(),
        "arrowright" | "right" => "→".into(),
        "arrowup" | "up" => "↑".into(),
        "arrowdown" | "down" => "↓".into(),
        // As `NAMED` in src/lib/accelerator.ts, so the menu and Settings read the same.
        "comma" => ",".into(),
        "period" => ".".into(),
        "slash" => "/".into(),
        "backslash" => "\\".into(),
        "semicolon" => ";".into(),
        "quote" => "'".into(),
        "backquote" => "`".into(),
        "minus" => "-".into(),
        "equal" => "=".into(),
        "bracketleft" => "[".into(),
        "bracketright" => "]".into(),
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
    fn the_optional_commands_conflict_like_the_others() {
        let settings = Settings { shortcut_to_arabic: "Alt+Shift+A".into(), ..Settings::default() };
        assert_eq!(owner(&settings, Binding::ToLatin, "Shift+Alt+A"), Some(Binding::ToArabic));
        assert_eq!(owner(&settings, Binding::ConvertLine, "Alt+Shift+Space"), Some(Binding::Convert));
        assert_eq!(owner(&settings, Binding::Undo, "Alt+Shift+A"), Some(Binding::ToArabic));
    }

    /// The `NAMED` table of `src/lib/accelerator.ts`, as `(code, legend)` pairs.
    fn named_in_settings() -> Vec<(String, String)> {
        let source = include_str!("../../src/lib/accelerator.ts");
        let start = source.find("const NAMED").expect("accelerator.ts has its NAMED table");
        let body = &source[start..];
        let body = &body[body.find('{').unwrap() + 1..body.find("\n}").unwrap()];
        let mut pairs = Vec::new();
        let mut rest = body;
        while let Some(colon) = rest.find(':') {
            let name = rest[..colon].trim().trim_start_matches(',').trim().to_string();
            let value = rest[colon + 1..].trim_start();
            let quote = value.chars().next().unwrap();
            let mut legend = String::new();
            let mut chars = value[1..].char_indices();
            let end = loop {
                match chars.next().unwrap() {
                    (_, '\\') => legend.push(chars.next().unwrap().1),
                    (i, c) if c == quote => break i + 2,
                    (_, c) => legend.push(c),
                }
            };
            pairs.push((name, legend));
            rest = &value[end..];
        }
        pairs
    }

    #[test]
    fn the_menu_names_keys_as_the_settings_window_does() {
        // BAD-307: «⌥ ⇧ ,» in Settings was «⌥ ⇧ COMMA» in the menu.
        let named = named_in_settings();
        assert!(named.len() >= 20, "{named:?}");
        for (code, legend) in named {
            assert_eq!(pretty_key(&code), legend, "{code}");
        }
        assert_eq!(pretty_key("KeyA"), "A");
        assert_eq!(pretty_key("Digit1"), "1");
    }

    #[test]
    fn a_command_does_not_conflict_with_itself_or_with_an_empty_binding() {
        let settings = Settings::default();
        assert_eq!(owner(&settings, Binding::Convert, "Alt+Shift+Space"), None);
        assert_eq!(owner(&settings, Binding::Undo, "Alt+Shift+U"), None);
    }
}
