// Prints the CGWindowID of a Baddel window, for `screencapture -l`.
//   swift scripts/window-id.swift [--panel]
// Without --panel it picks the frontmost ordinary window (settings or welcome);
// with it, the floating notice panel.
import CoreGraphics
import Foundation

let wantsPanel = CommandLine.arguments.contains("--panel")
let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] ?? []

for window in windows {
    guard window[kCGWindowOwnerName as String] as? String == "Baddel" else { continue }
    let layer = window[kCGWindowLayer as String] as? Int ?? 0
    guard wantsPanel ? layer > 0 : layer == 0 else { continue }
    if CommandLine.arguments.contains("--bounds") {
        guard let bounds = window[kCGWindowBounds as String] as? [String: CGFloat],
              let x = bounds["X"], let y = bounds["Y"], let w = bounds["Width"], let h = bounds["Height"]
        else { continue }
        print("\(x) \(y) \(w) \(h)")
        exit(0)
    }
    if let number = window[kCGWindowNumber as String] as? Int {
        print(number)
        exit(0)
    }
}
exit(1)
