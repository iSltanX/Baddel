import Carbon
import Foundation

func layoutData(_ id: String) -> Data? {
    let filter = [kTISPropertyInputSourceID as String: id] as CFDictionary
    guard let list = TISCreateInputSourceList(filter, true)?.takeRetainedValue() as? [TISInputSource],
          let src = list.first,
          let ptr = TISGetInputSourceProperty(src, kTISPropertyUnicodeKeyLayoutData) else { return nil }
    return Unmanaged<CFData>.fromOpaque(ptr).takeUnretainedValue() as Data
}

func translate(_ data: Data, _ key: UInt16, _ mods: UInt32) -> String {
    var dead: UInt32 = 0
    var len = 0
    var chars = [UniChar](repeating: 0, count: 8)
    let st = data.withUnsafeBytes { raw -> OSStatus in
        let kl = raw.baseAddress!.assumingMemoryBound(to: UCKeyboardLayout.self)
        return UCKeyTranslate(kl, key, UInt16(kUCKeyActionDown), (mods >> 8) & 0xFF,
                              UInt32(LMGetKbdType()), OptionBits(kUCKeyTranslateNoDeadKeysMask),
                              &dead, 8, &len, &chars)
    }
    guard st == noErr, len > 0 else { return "" }
    return String(utf16CodeUnits: chars, count: len)
}

let layers: [(String, UInt32)] = [("base", 0), ("shift", UInt32(shiftKey)), ("opt", UInt32(optionKey)), ("optshift", UInt32(optionKey | shiftKey))]
var out: [String: [String: [String]]] = [:]
for id in CommandLine.arguments.dropFirst() {
    guard let d = layoutData(id) else { FileHandle.standardError.write("missing \(id)\n".data(using: .utf8)!); continue }
    var l: [String: [String]] = [:]
    for (name, m) in layers { l[name] = (0...50).map { translate(d, UInt16($0), m) } }
    out[id] = l
}
let json = try JSONSerialization.data(withJSONObject: out, options: [.sortedKeys])
FileHandle.standardOutput.write(json)
