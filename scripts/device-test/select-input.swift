// Selects a keyboard input source by id, or prints the current one with no argument.
//   swift select-input.swift com.apple.keylayout.ABC
import Carbon

if CommandLine.arguments.count < 2 {
    let src = TISCopyCurrentKeyboardInputSource().takeRetainedValue()
    print(Unmanaged<CFString>.fromOpaque(TISGetInputSourceProperty(src, kTISPropertyInputSourceID)).takeUnretainedValue())
    exit(0)
}
let wanted = CommandLine.arguments[1]
let filter = [kTISPropertyInputSourceID as String: wanted] as CFDictionary
guard let list = TISCreateInputSourceList(filter, false)?.takeRetainedValue() as? [TISInputSource], let source = list.first else {
    fputs("no input source \(wanted)\n", stderr); exit(1)
}
exit(TISSelectInputSource(source) == noErr ? 0 : 1)
