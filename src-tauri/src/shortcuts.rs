//! Binding the global shortcuts, and showing them the way macOS writes them.

use std::str::FromStr;

use tauri::{AppHandle, Manager};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, Shortcut, ShortcutState};

use crate::controller::{Command, Commands};
use crate::settings::{Binding, Settings};
use crate::sync;

/// Re-registers every bound shortcut, returning the ones macOS would not give us —
/// almost always because another app holds them.
pub fn apply(app: &AppHandle, settings: &Settings) -> Vec<Binding> {
    let manager = app.global_shortcut();
    let _ = manager.unregister_all();
    let mut refused = Vec::new();
    for binding in Binding::ALL {
        let accelerator = settings.binding(binding);
        if accelerator.is_empty() {
            continue;
        }
        let Ok(shortcut) = Shortcut::from_str(accelerator) else {
            refused.push(binding);
            continue;
        };
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
