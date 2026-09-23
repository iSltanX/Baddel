#!/usr/bin/env bash
# Phase 7 clean-up: closes the test page's tabs in every browser, and discards TextEdit's
# untitled documents (the test's scratch documents; saved documents are left alone).
# `close … saving no` leaves TextEdit's untitled documents open on recent macOS, so each
# one goes through its own close sheet and that sheet's Delete button.
# Anything else the run opened (a note, a draft, an app) is closed by hand: see EXECUTION.md.
set -uo pipefail

osascript -e 'tell application "Safari" to repeat with w in (every window)
  close (every tab of w whose URL contains "device-test/page.html")
end repeat' >/dev/null 2>&1
for browser in "Google Chrome" "Brave Browser"; do
  osascript -e "tell application \"$browser\" to repeat with w in (every window)
  close (every tab of w whose URL contains \"device-test/page.html\")
end repeat" >/dev/null 2>&1
done

pgrep -x TextEdit >/dev/null || exit 0
open -b com.apple.TextEdit
sleep 1
# The Delete button's name follows the system language.
for _ in $(seq 50); do
  result=$(osascript -e 'tell application "System Events" to tell process "TextEdit"
  if (count of windows) = 0 then return "none"
  set w to window 1
  if value of attribute "AXDocument" of w is not missing value then return "saved"
  if (count of sheets of w) = 0 then
    click (first button of w whose subrole is "AXCloseButton")
    delay 0.8
  end if
  try
    if (count of sheets of w) = 0 then return "closed"
  on error
    return "closed" -- it closed without asking
  end try
  set g to splitter group 1 of sheet 1 of w
  repeat with label in {"حذف", "Delete"}
    if exists button label of g then
      click button label of g
      delay 0.8
      return "discarded"
    end if
  end repeat
  return "unknown sheet"
end tell' 2>&1)
  case "$result" in
    discarded|closed) ;;
    none) break ;;
    *) echo "TextEdit: $result"; break ;;
  esac
done
