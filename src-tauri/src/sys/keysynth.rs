//! Synthetic key presses.

use std::thread::sleep;
use std::time::{Duration, Instant};

use core_graphics::event::{CGEvent, CGEventFlags, CGEventTapLocation};
use core_graphics::event_source::{CGEventSource, CGEventSourceStateID};

pub const KEY_C: u16 = 8;
pub const KEY_V: u16 = 9;
pub const KEY_LEFT: u16 = 123;
pub const KEY_RIGHT: u16 = 124;

/// Gap between a key's down and up events, and after the up event.
const KEY_GAP: Duration = Duration::from_millis(8);

#[link(name = "CoreGraphics", kind = "framework")]
extern "C" {
    fn CGEventSourceFlagsState(state_id: i32) -> u64;
}

/// Presses and releases `keycode` with exactly `flags` held.
pub fn tap(keycode: u16, flags: CGEventFlags) -> bool {
    // A private source keeps the user's physically held modifiers out of our events.
    let Ok(source) = CGEventSource::new(CGEventSourceStateID::Private) else { return false };
    for down in [true, false] {
        let Ok(event) = CGEvent::new_keyboard_event(source.clone(), keycode, down) else { return false };
        event.set_flags(flags);
        event.post(CGEventTapLocation::HID);
        sleep(KEY_GAP);
    }
    true
}

pub fn copy() -> bool {
    tap(KEY_C, CGEventFlags::CGEventFlagCommand)
}

pub fn paste() -> bool {
    tap(KEY_V, CGEventFlags::CGEventFlagCommand)
}

/// Command-Shift-arrow: extend the selection to that end of the line.
pub fn select_to_line_edge(arrow: u16) -> bool {
    tap(arrow, CGEventFlags::CGEventFlagCommand | CGEventFlags::CGEventFlagShift)
}

/// Shift-arrow: extend the selection by one character.
pub fn select_char(arrow: u16) -> bool {
    tap(arrow, CGEventFlags::CGEventFlagShift)
}

/// Plain arrow: collapse the selection towards that side.
pub fn arrow(arrow: u16) -> bool {
    tap(arrow, CGEventFlags::CGEventFlagNull)
}

/// Waits until the user has let go of ⌘⌥⌃⇧ (they were just holding the hotkey), so the
/// target app does not combine them with our synthetic keys. `false` on timeout.
pub fn wait_for_modifiers_released(timeout: Duration) -> bool {
    let held = CGEventFlags::CGEventFlagCommand
        | CGEventFlags::CGEventFlagAlternate
        | CGEventFlags::CGEventFlagControl
        | CGEventFlags::CGEventFlagShift;
    let deadline = Instant::now() + timeout;
    loop {
        // SAFETY: plain query; 0 = kCGEventSourceStateCombinedSessionState.
        let flags = unsafe { CGEventSourceFlagsState(0) };
        if flags & held.bits() == 0 {
            return true;
        }
        if Instant::now() >= deadline {
            return false;
        }
        sleep(Duration::from_millis(10));
    }
}
