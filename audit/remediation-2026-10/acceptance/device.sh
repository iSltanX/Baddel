#!/usr/bin/env bash
# Acceptance B: conversion, undo, copy and paste, and focus switches, with real key presses.
#
#   audit/remediation-2026-10/acceptance/device.sh <work-dir>
#
# ⚠️ Sends real key presses and changes the front app. Only with the owner's approval in the
# session, the screen unlocked, and the owner's hands off the Mac for the run (about 10 minutes).
# It also quits the installed Baddel for the run — it holds ⌥⇧Space — and opens it again at the
# end. Baddel's own settings are not touched: the debug build runs with a HOME of its own inside
# <work-dir> (the Accessibility grant goes with the signature, not the home folder). Test text
# ends up in a new TextEdit document and the test page in Safari (closed by cleanup.sh) and in the history of a clipboard
# manager, if one runs; the clipboard itself is put back.
set -euo pipefail
cd "$(dirname "$0")/../../.."
WORK="$1"
HERE="audit/remediation-2026-10/acceptance"
APP="target/debug/bundle/macos/Baddel.app"
EXE="$PWD/$APP/Contents/MacOS/baddel"
H="$WORK/home"
SUPPORT="$H/Library/Application Support/com.isltanx.baddel"
export BADDEL_LOG="$WORK/baddel.log"

show() { echo "── $*"; }
locked() { [ "$(lsappinfo info -only bundleid "$(lsappinfo front)" | sed -n 's/.*bundleID="\([^"]*\)".*/\1/p')" = "com.apple.loginwindow" ]; }
debug_pids() { pgrep -f "^$EXE" || true; }
start_debug() { # [extra env]
  for pid in $(debug_pids); do kill -TERM "$pid"; done
  sleep 1
  : > "$BADDEL_LOG"
  open -n -g --env HOME="$H" "$@" --stderr "$BADDEL_LOG" "$APP"
  sleep 4
}

if locked; then echo "the screen is locked: stopping before any key press" >&2; exit 1; fi
[ -x "$EXE" ] || ./scripts/dev-build.sh
mkdir -p "$SUPPORT"
# Undo needs a shortcut of its own for the test: ⌥⇧Z. Welcome done, no update check.
printf '{"settings":{"welcomed":true,"autoUpdate":false,"shortcutUndo":"Alt+Shift+Z","defaultsVersion":2}}\n' > "$SUPPORT/settings.json"
touch "${TMPDIR:-/tmp}/baddel-device-test.start"  # cleanup.sh closes only what is newer

show "quitting the installed Baddel for the run"
INSTALLED_RAN=0
if pgrep -f "^/Applications/Baddel.app/Contents/MacOS/baddel" >/dev/null; then
  INSTALLED_RAN=1
  osascript -e 'tell application id "com.isltanx.baddel" to quit' >/dev/null 2>&1 || pkill -f "^/Applications/Baddel.app/Contents/MacOS/baddel" || true
  sleep 2
fi
reopen_installed() { [ "$INSTALLED_RAN" = 1 ] && open -g /Applications/Baddel.app || true; }
trap 'for pid in $(debug_pids); do kill -TERM "$pid"; done; ./scripts/device-test/cleanup.sh >/dev/null 2>&1 || true; reopen_installed' EXIT

osascript -e 'tell application "TextEdit" to make new document' >/dev/null
# The second app: Safari on the repository's own test page, its textarea focused.
osascript -e "tell application \"Safari\"
  if (count of windows) = 0 then make new document
  tell front window to set current tab to (make new tab with properties {URL:\"file://$PWD/scripts/device-test/page.html#ta\"})
end tell" >/dev/null
sleep 2
start_debug
for case in regression switch undo-switch; do
  show "$case"
  python3 "$HERE/device.py" "$case"
done

show "window (Baddel's report window, an accessory app's window)"
start_debug --env BADDEL_WINDOW=report
python3 "$HERE/device.py" window
show "debug outcomes"; grep '^\[baddel\] [A-Z]' "$BADDEL_LOG" | tail -20
