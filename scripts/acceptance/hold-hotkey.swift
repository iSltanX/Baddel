// Acceptance B helper: presses a Baddel hotkey with ⌥⇧ and keeps ⌥⇧ held, or lets them go.
// System Events' `key code … using {…}` lets the modifiers go with the key, so Baddel never has
// to wait for them; this keeps them down, as a person still holding the shortcut does.
//
//   swift hold-hotkey.swift press <keycode>   ⌥⇧ down, then the key down and up (⌥⇧ stay down)
//   swift hold-hotkey.swift release           ⌥⇧ up
import CoreGraphics
import Foundation

let option: CGKeyCode = 58, shift: CGKeyCode = 56
let held: CGEventFlags = [.maskAlternate, .maskShift]
guard let source = CGEventSource(stateID: .hidSystemState) else { exit(1) }

func post(_ key: CGKeyCode, down: Bool, flags: CGEventFlags) {
    guard let event = CGEvent(keyboardEventSource: source, virtualKey: key, keyDown: down) else { exit(1) }
    event.flags = flags
    event.post(tap: .cghidEventTap)
    usleep(15_000)
}

switch CommandLine.arguments.dropFirst().first {
case "press":
    guard let key = CommandLine.arguments.dropFirst(2).first.flatMap({ CGKeyCode($0) }) else { exit(2) }
    post(option, down: true, flags: [.maskAlternate])
    post(shift, down: true, flags: held)
    post(key, down: true, flags: held)
    post(key, down: false, flags: held)
case "release":
    post(shift, down: false, flags: [.maskAlternate])
    post(option, down: false, flags: [])
default:
    exit(2)
}
