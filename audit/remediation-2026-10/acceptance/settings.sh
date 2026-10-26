#!/usr/bin/env bash
# Acceptance B, the Settings path: Settings opened from the menu bar icon with real clicks, and
# typing, ⌘A ⌘C and ⌘V in its one text field (Layouts › «جرّبها هنا»).
#
#   settings.sh <work> open                 quit the installed Baddel, start the debug build (HOME
#                                           of its own), click its menu bar icon, then «الإعدادات…»
#   settings.sh <work> click <x> <y>        click at a point of the Settings window (points from
#                                           its top-left corner)
#   settings.sh <work> type <text>          type into the focused field; read it back with ⌘A ⌘C;
#                                           then paste " + pasted" at its end and read it back
#   settings.sh <work> capture <name>       the Settings window alone, into the evidence folder
#   settings.sh <work> finish               ⌘W, then put everything back
#
# ⚠️ Real clicks and key presses: only with the owner's approval in the session. Every click and
# key first checks that the debug Baddel is in front and holds the keyboard, and stops otherwise.
# Captures take Baddel's window only (`screencapture -l`), never the rest of the screen.
set -euo pipefail
cd "$(dirname "$0")/../../.."
WORK="$1"
STEP="$2"
shift 2
HERE="audit/remediation-2026-10/acceptance"
EVIDENCE="audit/remediation-2026-10/evidence"
APP="target/debug/bundle/macos/Baddel.app"
EXE="$PWD/$APP/Contents/MacOS/baddel"
H="$WORK/home"
SUPPORT="$H/Library/Application Support/com.isltanx.baddel"

show() { echo "── $*"; }
front() { lsappinfo info -only bundleid "$(lsappinfo front)" | sed -n 's/.*bundleID="\([^"]*\)".*/\1/p'; }
locked() { [ "$(front)" = "com.apple.loginwindow" ]; }
debug_pid() { pgrep -f "^$EXE" | head -1; }
keys_owner() {
  local tool="${TMPDIR:-/tmp}/baddel-focus-owner"
  [ -x "$tool" ] || swiftc -O -o "$tool" scripts/device-test/focus-owner.swift
  "$tool"
}
guard() { # the debug Baddel — and only it — in front with the keyboard
  locked && { echo "the screen is locked: stopping" >&2; exit 1; }
  [ "$(front)" = "com.isltanx.baddel" ] && [ "$(keys_owner)" = "com.isltanx.baddel" ] \
    || { echo "Baddel is not in front with the keyboard (front: $(front), keys: $(keys_owner)): stopping" >&2; exit 1; }
}
click() { swift "$HERE/click.swift" "$1" "$2"; }
osa() { /usr/bin/perl -e 'alarm 15; exec @ARGV' osascript -e "$1"; }
# The Settings window's id and its top-left corner on screen, in points.
window_info() {
  swift - "$(debug_pid)" <<'SWIFT'
import CoreGraphics
let pid = Int32(CommandLine.arguments[1])!
let windows = CGWindowListCopyWindowInfo([.optionOnScreenOnly], kCGNullWindowID) as? [[String: Any]] ?? []
for w in windows where (w[kCGWindowOwnerPID as String] as? Int32) == pid && (w[kCGWindowLayer as String] as? Int) == 0 {
    let b = w[kCGWindowBounds as String] as! [String: Double]
    print(w[kCGWindowNumber as String] as! Int, Int(b["X"]!), Int(b["Y"]!), Int(b["Width"]!), Int(b["Height"]!))
    break
}
SWIFT
}

case "$STEP" in
  open)
    locked && { echo "the screen is locked: stopping" >&2; exit 1; }
    mkdir -p "$SUPPORT"
    printf '{"settings":{"welcomed":true,"autoUpdate":false,"switchInputSource":false,"language":"ar","defaultsVersion":2}}\n' > "$SUPPORT/settings.json"
    swift scripts/device-test/select-input.swift > "$WORK/input-source"
    swift scripts/device-test/select-input.swift com.apple.keylayout.ABC
    pbpaste > "$WORK/clipboard" || true
    if pgrep -f "^/Applications/Baddel.app/Contents/MacOS/baddel" >/dev/null; then
      echo 1 > "$WORK/installed-ran"
      osa 'tell application id "com.isltanx.baddel" to quit' >/dev/null 2>&1 || pkill -f "^/Applications/Baddel.app/Contents/MacOS/baddel" || true
      sleep 2
    fi
    open -n -g --env HOME="$H" --stderr "$WORK/baddel.log" "$APP"
    sleep 4
    PID="$(debug_pid)"
    show "debug Baddel pid $PID; front: $(front)"
    item="$(osa "tell application \"System Events\" to tell (first process whose unix id is $PID) to get {position, size} of menu bar item 1 of menu bar 2")"
    read -r ix iy iw ih <<< "$(echo "$item" | tr -d ',')"
    show "its menu bar icon (accessibility): x=$ix y=$iy w=$iw h=$ih — clicking its centre"
    click $((ix + iw / 2)) $((iy + ih / 2))
    sleep 0.8
    entry="$(osa "tell application \"System Events\" to tell (first process whose unix id is $PID) to tell menu 1 of menu bar item 1 of menu bar 2 to get {position, size} of (first menu item whose name starts with \"الإعدادات\")")"
    read -r mx my mw mh <<< "$(echo "$entry" | tr -d ',')"
    show "«الإعدادات…» in the open menu: x=$mx y=$my — clicking it"
    click $((mx + mw / 2)) $((my + mh / 2))
    sleep 3
    show "after the click: front=$(front) keys=$(keys_owner)"
    swift audit/remediation-2026-10/tools/ax-windows.swift "$PID"
    show "Settings window (id x y w h): $(window_info)"
    ;;
  click)
    guard
    read -r _ wx wy _ _ <<< "$(window_info)"
    show "click at window point ($1, $2) = screen ($((wx + $1)), $((wy + $2)))"
    click $((wx + $1)) $((wy + $2))
    sleep 1
    ;;
  type)
    guard
    osa "tell application \"System Events\" to keystroke \"$1\"" >/dev/null
    sleep 0.6
    guard
    osa 'tell application "System Events" to keystroke "a" using command down' >/dev/null
    osa 'tell application "System Events" to keystroke "c" using command down' >/dev/null
    sleep 0.4
    show "⌘A ⌘C read back: $(pbpaste)"
    printf ' + pasted' | pbcopy
    guard
    osa 'tell application "System Events" to key code 124' >/dev/null
    osa 'tell application "System Events" to keystroke "v" using command down' >/dev/null
    sleep 0.6
    osa 'tell application "System Events" to keystroke "a" using command down' >/dev/null
    osa 'tell application "System Events" to keystroke "c" using command down' >/dev/null
    sleep 0.4
    show "after ⌘V, read back: $(pbpaste)"
    ;;
  capture)
    read -r wid _ _ _ _ <<< "$(window_info)"
    screencapture -x -o -l "$wid" "$EVIDENCE/$1.png"
    show "captured window $wid → $EVIDENCE/$1.png"
    ;;
  finish)
    if [ "$(front)" = "com.isltanx.baddel" ] && [ "$(keys_owner)" = "com.isltanx.baddel" ]; then
      osa 'tell application "System Events" to keystroke "w" using command down' >/dev/null
      sleep 1.5
      show "after ⌘W: Settings window $(window_info | grep -q . && echo 'still open' || echo closed); front=$(front)"
    fi
    for pid in $(pgrep -f "^$EXE"); do kill -TERM "$pid"; done
    [ -f "$WORK/clipboard" ] && pbcopy < "$WORK/clipboard"
    [ -f "$WORK/input-source" ] && swift scripts/device-test/select-input.swift "$(cat "$WORK/input-source")"
    [ -f "$WORK/installed-ran" ] && open -g /Applications/Baddel.app
    sleep 2
    show "installed Baddel: $(pgrep -f '^/Applications/Baddel.app/Contents/MacOS/baddel' || echo 'not running'); input source: $(swift scripts/device-test/select-input.swift)"
    ;;
  *) echo "unknown step $STEP" >&2; exit 2 ;;
esac
