#!/usr/bin/env bash
# Phase 7 clean-up: closes the test page's tabs in every browser, and discards TextEdit's
# untitled documents (the test's scratch documents; named documents are left alone).
# On recent macOS TextEdit files a new document in iCloud Drive at once, so "untitled" is
# told by the window's name, not by the absence of a file; `close … saving no` does not
# close them, so each goes through its own close sheet and that sheet's Delete button.
# Untitled files the run left in TextEdit's iCloud folder go to the Trash (never deleted),
# and only those newer than the marker run.sh leaves when it starts.
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
  if name of w does not contain "بلا عنوان" and name of w does not contain "Untitled" then return "named"
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

marker="${TMPDIR:-/tmp}/baddel-device-test.start"
icloud="$HOME/Library/Mobile Documents/com~apple~TextEdit/Documents"
if [ -f "$marker" ] && [ -d "$icloud" ]; then
  find "$icloud" -maxdepth 1 -type f \( -name 'بلا عنوان*' -o -name 'Untitled*' \) -newer "$marker" -print0 |
    while IFS= read -r -d '' file; do
      osascript -e "tell application \"Finder\" to delete (POSIX file \"$file\" as alias)" >/dev/null &&
        echo "moved to Trash: $(basename "$file")"
    done
fi

