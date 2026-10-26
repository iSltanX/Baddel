#!/usr/bin/env bash
# Acceptance B: conversion, undo, copy and paste, and focus switches, with real key presses.
#
#   audit/remediation-2026-10/acceptance/device.sh <work-dir> [case…]
#
# Cases: regression switch undo-switch window (all four when none is named).
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
export BADDEL_ACCEPT_SHOTS="$WORK"
shift
CASES=("$@")
[ ${#CASES[@]} -gt 0 ] || CASES=(regression switch undo-switch window)
wants() { for c in "${CASES[@]}"; do [ "$c" = "$1" ] && return 0; done; return 1; }

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
# Undo needs a shortcut of its own for the test: ⌥⇧Z. Welcome done, no update check, and the input
# source left alone by conversions (the run holds it on ABC and puts the user's back after).
printf '{"settings":{"welcomed":true,"autoUpdate":false,"switchInputSource":false,"shortcutUndo":"Alt+Shift+Z","defaultsVersion":2}}\n' > "$SUPPORT/settings.json"
INPUT_SOURCE="$(swift scripts/device-test/select-input.swift)"
touch "${TMPDIR:-/tmp}/baddel-device-test.start"  # cleanup.sh closes only what is newer

show "quitting the installed Baddel for the run"
INSTALLED_RAN=0
if pgrep -f "^/Applications/Baddel.app/Contents/MacOS/baddel" >/dev/null; then
  INSTALLED_RAN=1
  osascript -e 'tell application id "com.isltanx.baddel" to quit' >/dev/null 2>&1 || pkill -f "^/Applications/Baddel.app/Contents/MacOS/baddel" || true
  sleep 2
fi
reopen_installed() { [ "$INSTALLED_RAN" = 1 ] && open -g /Applications/Baddel.app || true; }
finish() {
  for pid in $(debug_pids); do kill -TERM "$pid"; done
  ./scripts/device-test/cleanup.sh >/dev/null 2>&1 || true
  # The apps the run opened are closed again; ones already open stay as they were.
  [ "${SAFARI_RAN:-1}" = 1 ] || /usr/bin/perl -e 'alarm 15; exec @ARGV' osascript -e 'if application "Safari" is running then tell application "Safari" to quit saving no' >/dev/null 2>&1 || true
  [ "${TEXTEDIT_RAN:-1}" = 1 ] || /usr/bin/perl -e 'alarm 15; exec @ARGV' osascript -e 'if application "TextEdit" is running then tell application "TextEdit" to quit saving no' >/dev/null 2>&1 || true
  swift scripts/device-test/select-input.swift "$INPUT_SOURCE" || true
  reopen_installed
}
trap finish EXIT
swift scripts/device-test/select-input.swift com.apple.keylayout.ABC
show "input source for the run: $(swift scripts/device-test/select-input.swift) (was $INPUT_SOURCE)"

# A pending permission prompt would hold osascript forever: give up after 15 s instead.
osa_timed() { /usr/bin/perl -e 'alarm 15; exec @ARGV' osascript -e "$1" >/dev/null || { echo "osascript did not answer: $1" >&2; exit 1; }; }
SAFARI_RAN=0; pgrep -x Safari >/dev/null && SAFARI_RAN=1
TEXTEDIT_RAN=0; pgrep -x TextEdit >/dev/null && TEXTEDIT_RAN=1
osa_timed 'tell application "TextEdit" to make new document'
# The second app: Safari on the repository's own test page, its textarea focused.
osa_timed "tell application \"Safari\"
  if (count of windows) = 0 then make new document
  tell front window to set current tab to (make new tab with properties {URL:\"file://$PWD/scripts/device-test/page.html#ta\"})
end tell"
sleep 2
start_debug
for case in regression switch undo-switch; do
  wants "$case" || continue
  show "$case"
  python3 "$HERE/device.py" "$case"
done

if wants window; then
  show "window (Baddel's report window, an accessory app's window)"
  start_debug --env BADDEL_WINDOW=report
  python3 "$HERE/device.py" window
fi
show "debug outcomes"; grep '^\[baddel\] [A-Z]' "$BADDEL_LOG" | tail -20
