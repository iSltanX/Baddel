// Read-only: the accessibility view of one app's windows — which is focused, which is main,
// and how many text fields each holds. For telling whether an accessory app's window has the
// keyboard. No input is sent.
//
//   swift ax-windows.swift <pid>
import AppKit
import ApplicationServices

guard CommandLine.arguments.count > 1, let pid = pid_t(CommandLine.arguments[1]) else { exit(2) }
let app = AXUIElementCreateApplication(pid)
func attr(_ e: AXUIElement, _ name: String) -> AnyObject? {
    var value: AnyObject?
    return AXUIElementCopyAttributeValue(e, name as CFString, &value) == .success ? value : nil
}
func fields(_ e: AXUIElement, _ depth: Int = 0) -> Int {
    guard depth < 60 else { return 0 }
    let role = attr(e, "AXRole") as? String ?? ""
    let own = (role == "AXTextArea" || role == "AXTextField") ? 1 : 0
    return own + (attr(e, "AXChildren") as? [AXUIElement] ?? []).reduce(0) { $0 + fields($1, depth + 1) }
}
let running = NSRunningApplication(processIdentifier: pid)
print("app active=\(running?.isActive ?? false) policy=\(running?.activationPolicy.rawValue ?? -1) frontmost=\(attr(app, "AXFrontmost") as? Bool ?? false)")
let focused = attr(app, "AXFocusedWindow").map { $0 as! AXUIElement }
let main = attr(app, "AXMainWindow").map { $0 as! AXUIElement }
for window in attr(app, "AXWindows") as? [AXUIElement] ?? [] {
    let title = attr(window, "AXTitle") as? String ?? "-"
    print("window \"\(title)\" focused=\(focused.map { CFEqual($0, window) } ?? false) main=\(main.map { CFEqual($0, window) } ?? false) textFields=\(fields(window))")
}
if let element = attr(app, "AXFocusedUIElement") { print("focused element role=\(attr(element as! AXUIElement, "AXRole") as? String ?? "-")") }
