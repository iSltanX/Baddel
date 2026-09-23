// Prints the bundle id of the app that owns keyboard focus, as the accessibility system sees
// it — or nothing when it cannot tell. Stricter than "frontmost app": a system prompt (a
// permission request, say) can hold the keys while another app is still in front.
// Compiled once by matrix.py.
import AppKit
import ApplicationServices

let system = AXUIElementCreateSystemWide()
AXUIElementSetMessagingTimeout(system, 0.5)
var value: AnyObject?
guard AXUIElementCopyAttributeValue(system, kAXFocusedApplicationAttribute as CFString, &value) == .success,
      let app = value
else { exit(1) }
var pid: pid_t = 0
AXUIElementGetPid(app as! AXUIElement, &pid)
print(NSRunningApplication(processIdentifier: pid)?.bundleIdentifier ?? "pid:\(pid)")
