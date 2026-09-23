// Phase 7: finds the text fields of the front app's focused window and, with `focus N`,
// gives the Nth one keyboard focus through accessibility — no mouse, no guessing where it is.
// For apps without an AppleScript way in (Electron, Tauri). Prints each field's role, label
// and current length, so a field that already holds the user's text can be left alone.
//
//   swift scripts/device-test/focus-field.swift            list
//   swift scripts/device-test/focus-field.swift focus 1    focus the first one
import AppKit
import ApplicationServices

guard let app = NSWorkspace.shared.frontmostApplication else { exit(1) }
let root = AXUIElementCreateApplication(app.processIdentifier)
// Electron keeps its tree off until an assistive client asks (see sys/text_access.rs).
AXUIElementSetAttributeValue(root, "AXManualAccessibility" as CFString, kCFBooleanTrue)
Thread.sleep(forTimeInterval: 0.5)

func attr(_ e: AXUIElement, _ name: String) -> AnyObject? {
    var value: AnyObject?
    AXUIElementCopyAttributeValue(e, name as CFString, &value)
    return value
}

var fields: [AXUIElement] = []
func walk(_ e: AXUIElement, depth: Int) {
    guard depth < 60, fields.count < 30 else { return }
    let role = attr(e, "AXRole") as? String ?? ""
    if role == "AXTextArea" || role == "AXTextField" {
        fields.append(e)
        let label = [attr(e, "AXDescription"), attr(e, "AXTitle"), attr(e, "AXPlaceholderValue")]
            .compactMap { $0 as? String }.first { !$0.isEmpty } ?? "-"
        let text = (attr(e, "AXValue") as? String ?? "").trimmingCharacters(in: .whitespacesAndNewlines)
        print(fields.count, role, label, "length=\(text.count)")
    }
    for child in attr(e, "AXChildren") as? [AXUIElement] ?? [] { walk(child, depth: depth + 1) }
}
if let window = attr(root, "AXFocusedWindow") { walk(window as! AXUIElement, depth: 0) }

let args = CommandLine.arguments
if args.count > 2, args[1] == "focus", let n = Int(args[2]), (1...fields.count).contains(n) {
    let status = AXUIElementSetAttributeValue(fields[n - 1], "AXFocused" as CFString, kCFBooleanTrue)
    print("focused \(n): \(status == .success ? "ok" : "error \(status.rawValue)")")
    exit(status == .success ? 0 : 1)
}
