#!/usr/bin/env bash
# Phase 7: builds the signed debug app, relaunches it with its stderr captured, and runs the
# automated part of the device matrix. ⚠️ Sends real key presses — phase 7 only, with the
# user away from the machine. Afterwards: follow the clean-up list in EXECUTION.md.
set -euo pipefail
cd "$(dirname "$0")/../.."

export BADDEL_LOG="${BADDEL_LOG:-${TMPDIR:-/tmp}/baddel-device-test.log}"
HERE="scripts/device-test"
PAGE="file://$PWD/$HERE/page.html"

./scripts/dev-build.sh
pkill -x baddel || true
sleep 1
: > "$BADDEL_LOG"
open -g --stderr "$BADDEL_LOG" target/debug/bundle/macos/Baddel.app
sleep 4

osascript -e 'tell application "TextEdit" to make new document' >/dev/null
python3 "$HERE/matrix.py" TextEdit com.apple.TextEdit
for app in "Safari:com.apple.Safari" "Brave:com.brave.Browser" "Chrome:com.google.Chrome"; do
  name=${app%%:*} id=${app#*:}
  [ -n "$(mdfind "kMDItemCFBundleIdentifier == '$id'" | head -1)" ] || { echo "$name: not installed"; continue; }
  for field in ta ce; do
    open -b "$id" "$PAGE#$field"
    sleep 2.5
    python3 "$HERE/matrix.py" "$name#$field" "$id"
  done
  open -b "$id" "$PAGE#pw"
  sleep 2.5
  python3 "$HERE/matrix.py" "$name#password" "$id" --expect Blocked
done
echo "log: $BADDEL_LOG"
