// Read-only observer for the 2026-10 remediation checks. Sends no input and changes nothing.
//
//   swiftc -O observe.swift -o observe
//   ./observe --bundle <path/to/Baddel.app> --seconds 8 [--interval-ms 5]
//
// Waits for a process running from that bundle to appear, then samples it until the time is up:
// its activation policy (0 regular = Dock icon, 1 accessory, 2 prohibited), whether it is active,
// which app is frontmost, and its on-screen windows (ordinary = layer 0). Prints one JSON line
// whenever any of these changes, then a summary line. NSRunningApplication only refreshes while
// the run loop turns, so the loop is spun between samples.
import AppKit

var bundlePath = ""
var seconds = 8.0
var intervalMs = 5.0
var args = CommandLine.arguments.dropFirst().makeIterator()
while let arg = args.next() {
    switch arg {
    case "--bundle": bundlePath = args.next() ?? ""
    case "--seconds": seconds = Double(args.next() ?? "") ?? seconds
    case "--interval-ms": intervalMs = Double(args.next() ?? "") ?? intervalMs
    default: break
    }
}
let wanted = URL(fileURLWithPath: bundlePath).standardizedFileURL.resolvingSymlinksInPath()
let start = Date()
func ms() -> Int { Int(Date().timeIntervalSince(start) * 1000) }
func spin() { RunLoop.current.run(until: Date(timeIntervalSinceNow: intervalMs / 1000)) }
func emit(_ object: [String: Any]) {
    let data = try! JSONSerialization.data(withJSONObject: object, options: [.sortedKeys])
    print(String(data: data, encoding: .utf8)!)
    fflush(stdout)
}

func windows(of pid: pid_t) -> (ordinary: Int, any: Int) {
    let list = CGWindowListCopyWindowInfo([.optionOnScreenOnly, .excludeDesktopElements], kCGNullWindowID) as? [[String: Any]] ?? []
    let own = list.filter { ($0[kCGWindowOwnerPID as String] as? Int32) == pid }
    return (own.filter { ($0[kCGWindowLayer as String] as? Int) == 0 }.count, own.count)
}

var app: NSRunningApplication?
while app == nil && Date().timeIntervalSince(start) < seconds {
    app = NSWorkspace.shared.runningApplications.first {
        $0.bundleURL?.standardizedFileURL.resolvingSymlinksInPath() == wanted
    }
    if app == nil { spin() }
}
guard let app else {
    emit(["event": "not-found", "bundle": bundlePath, "t_ms": ms()])
    exit(1)
}
emit(["event": "found", "pid": app.processIdentifier, "t_ms": ms()])

var last: [String: Any] = [:]
var samples = 0, regular = 0, active = 0, maxOrdinary = 0, gaps: [Int] = []
var previous = ms()
while Date().timeIntervalSince(start) < seconds {
    if app.isTerminated {
        emit(["event": "terminated", "t_ms": ms()])
        break
    }
    let w = windows(of: app.processIdentifier)
    let state: [String: Any] = [
        "policy": app.activationPolicy.rawValue,
        "active": app.isActive,
        "front": NSWorkspace.shared.frontmostApplication?.processIdentifier ?? 0,
        "ordinary": w.ordinary,
        "windows": w.any,
    ]
    samples += 1
    if app.activationPolicy == .regular { regular += 1 }
    if app.isActive { active += 1 }
    maxOrdinary = max(maxOrdinary, w.ordinary)
    let now = ms()
    gaps.append(now - previous)
    previous = now
    if !NSDictionary(dictionary: state).isEqual(to: last) {
        var line = state
        line["t_ms"] = now
        emit(line)
        last = state
    }
    spin()
}
gaps.sort()
emit([
    "event": "summary", "pid": app.processIdentifier, "samples": samples, "regular_samples": regular,
    "active_samples": active, "max_ordinary_windows": maxOrdinary,
    "median_gap_ms": gaps.isEmpty ? 0 : gaps[gaps.count / 2], "max_gap_ms": gaps.last ?? 0,
])
