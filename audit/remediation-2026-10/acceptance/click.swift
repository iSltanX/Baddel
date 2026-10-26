// Acceptance helper: one real left click at a point on screen (points, origin top left), as the
// mouse makes it — for the menu bar icon and its menu, which only a click opens as a person does.
//
//   swift click.swift <x> <y>
import CoreGraphics
import Foundation

let args = CommandLine.arguments
guard args.count == 3, let x = Double(args[1]), let y = Double(args[2]) else { exit(2) }
let point = CGPoint(x: x, y: y)
guard let source = CGEventSource(stateID: .hidSystemState) else { exit(1) }
for type in [CGEventType.mouseMoved, .leftMouseDown, .leftMouseUp] {
    guard let event = CGEvent(mouseEventSource: source, mouseType: type, mouseCursorPosition: point, mouseButton: .left) else { exit(1) }
    event.post(tap: .cghidEventTap)
    usleep(type == .mouseMoved ? 120_000 : 60_000)
}
