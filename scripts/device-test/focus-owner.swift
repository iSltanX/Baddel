// Prints the bundle id of the app that owns keyboard focus — or nothing when it cannot be
// trusted. Stricter than "frontmost app": a system prompt (a permission request, say) must
// never receive the test's key presses. Compiled once by matrix.py.
//
// The accessibility system is asked first. On some Macs its system-wide query fails for
// every app (kAXErrorCannotComplete) for long stretches; then the frontmost app is accepted
// only while no system prompt is on screen.
import AppKit
import ApplicationServices

let system = AXUIElementCreateSystemWide()
AXUIElementSetMessagingTimeout(system, 0.5)
var value: AnyObject?
if AXUIElementCopyAttributeValue(system, kAXFocusedApplicationAttribute as CFString, &value) == .success,
   let app = value
{
    var pid: pid_t = 0
    AXUIElementGetPid(app as! AXUIElement, &pid)
    print(NSRunningApplication(processIdentifier: pid)?.bundleIdentifier ?? "pid:\(pid)")
    exit(0)
}

let promptOwners: Set<String> = [
    "UserNotificationCenter", "SecurityAgent", "CoreServicesUIAgent", "universalAccessAuthWarn",
]
let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] ?? []
let promptOnScreen = windows.contains { window in
    promptOwners.contains(window[kCGWindowOwnerName as String] as? String ?? "")
        && (window[kCGWindowLayer as String] as? Int ?? 0) > 0
}
guard !promptOnScreen, let front = NSWorkspace.shared.frontmostApplication?.bundleIdentifier else { exit(1) }
print(front)
