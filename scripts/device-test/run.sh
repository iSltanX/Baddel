#!/usr/bin/env bash
# Phase 7: builds the signed debug app, relaunches it with its stderr captured, and runs the
# automated part of the device matrix. ⚠️ Sends real key presses — phase 7 only, with the
# user away from the machine. Afterwards: follow the clean-up list in EXECUTION.md.
set -euo pipefail
cd "$(dirname "$0")/../.."

export BADDEL_LOG="${BADDEL_LOG:-${TMPDIR:-/tmp}/baddel-device-test.log}"
touch "${TMPDIR:-/tmp}/baddel-device-test.start"  # cleanup.sh trashes only what is newer
HERE="scripts/device-test"
PAGE="file://$PWD/$HERE/page.html"

./scripts/dev-build.sh
pkill -x baddel || true
sleep 1
: > "$BADDEL_LOG"
open -g --stderr "$BADDEL_LOG" target/debug/bundle/macos/Baddel.app
sleep 4

# `open -b <id> <file-url>#field` drops the fragment, so the page would always focus the
# textarea: open tabs through each browser's own dictionary instead. AppleScript's `activate`
# no longer reliably brings an app forward, so `open -b` does that.
open_page() { # <app name> <bundle id> <url>
  if [ "$1" = Safari ]; then
    osascript -e "tell application \"Safari\"
      if (count of windows) = 0 then make new document
      tell front window to set current tab to (make new tab with properties {URL:\"$3\"})
    end tell" >/dev/null
  else
    osascript -e "tell application \"$1\"
      if (count of windows) = 0 then make new window
      tell front window to make new tab with properties {URL:\"$3\"}
    end tell" >/dev/null
  fi
  open -b "$2"
}

osascript -e 'tell application "TextEdit" to make new document' >/dev/null
python3 "$HERE/matrix.py" TextEdit com.apple.TextEdit
for app in "Safari:Safari:com.apple.Safari" "Brave:Brave Browser:com.brave.Browser" "Chrome:Google Chrome:com.google.Chrome"; do
  IFS=: read -r name app_name id <<< "$app"
  [ -n "$(mdfind "kMDItemCFBundleIdentifier == '$id'" | head -1)" ] || { echo "$name: not installed"; continue; }
  for field in ta ce; do
    open_page "$app_name" "$id" "$PAGE#$field"
    sleep 2.5
    python3 "$HERE/matrix.py" "$name#$field" "$id"
  done
  open_page "$app_name" "$id" "$PAGE#pw"
  sleep 2.5
  python3 "$HERE/matrix.py" "$name#password" "$id" --expect Blocked
done
echo "log: $BADDEL_LOG"
