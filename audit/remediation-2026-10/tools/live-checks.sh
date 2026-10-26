#!/usr/bin/env bash
# Live checks of the debug build for the 2026-10 remediation, each in a throwaway home.
#
#   audit/remediation-2026-10/tools/live-checks.sh <work-dir> <observer> <scenario>
#
# Every scenario runs target/debug/bundle/macos/Baddel.app with HOME and TMPDIR inside <work-dir>,
# so the user's own settings, LaunchAgents and temporary folder are never read or written;
# BADDEL_LOGIN_ITEM_FAKE stands in for SMAppService, so nothing is registered with macOS; and
# automatic update checks are off, so nothing goes on the network. No input is sent to any app.
# Only processes running from the debug bundle's own path are ever stopped — never an installed
# Baddel.
set -euo pipefail
cd "$(dirname "$0")/../../.."

WORK="$1"
OBSERVE="$2"
SCENARIO="$3"
APP="$PWD/target/debug/bundle/macos/Baddel.app"
EXE="$APP/Contents/MacOS/baddel"
ID="com.isltanx.baddel"

[ -x "$EXE" ] || { echo "build it first: ./scripts/dev-build.sh" >&2; exit 1; }

# The debug instances running from this bundle, and nothing else.
debug_pids() { pgrep -f "^$EXE" || true; }
stop_debug() {
  for pid in $(debug_pids); do kill -TERM "$pid" 2>/dev/null || true; done
  for _ in $(seq 1 50); do [ -z "$(debug_pids)" ] && return 0; sleep 0.1; done
  echo "debug Baddel did not stop" >&2
  return 1
}

# A home of its own: settings, an optional LaunchAgent, a fake login item.
fresh_home() { # <name>
  H="$WORK/$1/home"
  T="$WORK/$1/tmp"
  rm -rf "$WORK/$1"
  mkdir -p "$H/Library/Application Support/$ID" "$H/Library/LaunchAgents" "$T"
  FAKE="$WORK/$1/login-item.json"
  echo '{"status":"notRegistered"}' > "$FAKE"
  LOG="$WORK/$1/stderr.log"
}
settings() { # <json for the "settings" key>
  printf '{"settings":%s}\n' "$1" > "$H/Library/Application Support/$ID/settings.json"
}
legacy_plist() { # the file Baddel 1.0–2.0.2 wrote, for a bundle inside this home
  local program="$H/Applications/Baddel.app/Contents/MacOS/baddel"
  printf '<?xml version="1.0" encoding="UTF-8"?>\n<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">\n<plist version="1.0">\n  <dict>\n  <key>Label</key>\n  <string>Baddel</string>\n  <key>ProgramArguments</key>\n  <array><string>%s</string></array>\n  <key>RunAtLoad</key>\n  <true/>\n  </dict>\n</plist>' "$program" \
    > "$H/Library/LaunchAgents/Baddel.plist"
}
launch() { # [open options…] — through Launch Services, as Finder or a login item would
  open -n "$@" --env HOME="$H" --env TMPDIR="$T/" --env BADDEL_LOGIN_ITEM_FAKE="$FAKE" --stderr "$LOG" "$APP"
}
show() { echo "── $*"; }
record() { show "maintenance.json"; cat "$H/Library/Application Support/$ID/maintenance.json" 2>/dev/null || echo "(none)"; echo; }

stop_debug

case "$SCENARIO" in
  cold)
    # Launched in the background, as at login: no window, no Dock icon at any moment.
    fresh_home cold
    settings '{"welcomed":true,"autoUpdate":false}'
    "$OBSERVE" --bundle "$APP" --seconds 6 & obs=$!
    launch -g
    wait $obs
    show "lsappinfo"; lsappinfo info -only applicationtype "$(debug_pids)" || true
    ;;
  onboarding)
    # First run: the welcome window opens, in front, with keyboard focus — still no Dock icon.
    fresh_home onboarding
    settings '{"welcomed":false,"autoUpdate":false}'
    "$OBSERVE" --bundle "$APP" --seconds 8 & obs=$!
    launch
    wait $obs
    ;;
  reopen)
    # A second launch by hand while one runs: handed over, and Settings opens in the running one,
    # in front and focused, without a Dock icon; closed (BADDEL_CLOSE_AFTER), the keys go back.
    fresh_home reopen
    settings '{"welcomed":true,"autoUpdate":false}'
    "$OBSERVE" --bundle "$APP" --seconds 16 & obs=$!
    open -n -g --env HOME="$H" --env TMPDIR="$T/" --env BADDEL_LOGIN_ITEM_FAKE="$FAKE" --env BADDEL_CLOSE_AFTER=11 --stderr "$LOG" "$APP"
    sleep 3
    owner="$(debug_pids)"
    show "second launch (the executable itself, by hand)"
    HOME="$H" TMPDIR="$T/" BADDEL_LOGIN_ITEM_FAKE="$FAKE" "$EXE" 2>&1 | grep '\[baddel\] another' || true
    show "running after the second launch: $(debug_pids | wc -l | tr -d ' ') (owner $owner, now $(debug_pids | tr '\n' ' '))"
    wait $obs
    ;;
  race)
    # Four independent launches at the same instant, as launchd at login: one owner, no Settings.
    fresh_home race
    settings '{"welcomed":true,"autoUpdate":false}'
    for i in 1 2 3 4; do
      HOME="$H" TMPDIR="$T/" BADDEL_LOGIN_ITEM_FAKE="$FAKE" XPC_SERVICE_NAME=Baddel "$EXE" >"$WORK/race/launch-$i.log" 2>&1 &
    done
    sleep 5
    show "running: $(debug_pids | wc -l | tr -d ' ')"
    for i in 1 2 3 4; do grep -h '\[baddel\] another' "$WORK/race/launch-$i.log" || true; done
    show "windows on screen of the owner"
    "$OBSERVE" --bundle "$APP" --seconds 1 | tail -1
    ;;
  migrate)
    # 2.0.2's LaunchAgent, launch at login on: replaced by the login item, then removed.
    fresh_home migrate
    settings '{"welcomed":true,"autoUpdate":false,"launchAtLogin":true}'
    legacy_plist
    launch -g
    sleep 4
    show "legacy plist: $([ -e "$H/Library/LaunchAgents/Baddel.plist" ] && echo present || echo removed)"
    show "login item (fake): $(cat "$FAKE")"
    record
    ;;
  retry)
    # macOS refuses the login item (kSMErrorLaunchDeniedByUser): the old agent stays; next launch, it goes.
    fresh_home retry
    settings '{"welcomed":true,"autoUpdate":false,"launchAtLogin":true}'
    legacy_plist
    echo '{"status":"notRegistered","refuse":11}' > "$FAKE"
    launch -g
    sleep 4
    show "first launch — legacy plist: $([ -e "$H/Library/LaunchAgents/Baddel.plist" ] && echo present || echo removed); fake: $(cat "$FAKE")"
    record
    stop_debug
    echo '{"status":"notRegistered"}' > "$FAKE"
    launch -g
    sleep 4
    show "second launch — legacy plist: $([ -e "$H/Library/LaunchAgents/Baddel.plist" ] && echo present || echo removed); fake: $(cat "$FAKE")"
    record
    ;;
  corrupt)
    # Unreadable settings: kept aside, and no reason to drop the old agent's launch at login.
    fresh_home corrupt
    printf '{corrupted json' > "$H/Library/Application Support/$ID/settings.json"
    legacy_plist
    launch -g
    sleep 4
    show "legacy plist: $([ -e "$H/Library/LaunchAgents/Baddel.plist" ] && echo present || echo removed); fake: $(cat "$FAKE")"
    show "copies kept of the unreadable file:"; ls "$H/Library/Application Support/$ID/" | grep unreadable || echo "(none)"
    cat "$H/Library/Application Support/$ID/"settings.unreadable-*.json; echo
    record
    ;;
  upgrade)
    # The state 2.0.2 leaves, then the new build's first launch: the user's settings and files are
    # kept; only Baddel's own obsolete pieces go.
    fresh_home upgrade
    support="$H/Library/Application Support/$ID"
    settings '{"launchAtLogin":true,"showTrayIcon":true,"language":"en","switchInputSource":false,"showHud":true,"sound":true,"autoUpdate":false,"shortcutConvert":"Control+Alt+Space","shortcutUndo":"Alt+Shift+Z","shortcutPause":"","shortcutConvertLine":"Alt+Shift+L","shortcutToArabic":"","shortcutToLatin":"","arabicLayout":"com.apple.keylayout.ArabicPC","latinLayout":"com.apple.keylayout.ABC","excludedApps":["com.apple.Terminal","com.example.mine"],"paused":false,"pausedUntil":null,"welcomed":true,"lastUpdateCheck":1793042029164,"defaultsVersion":2}'
    cp "$support/settings.json" "$WORK/upgrade/settings.before.json"
    echo "the user's own file" > "$support/notes.txt"
    legacy_plist
    # A cut-short 2.0.x install: the updater's backup of the old app — a real, signed Baddel (the
    # 2.0.2 release, unpacked) — with nothing in it touched for over an hour.
    mkdir -p "$T/tauri_current_appQ1w2E3"
    tar -xzf "release/2.0.2/Baddel_2.0.2_universal.app.tar.gz" -C "$T/tauri_current_appQ1w2E3"
    mv "$T/tauri_current_appQ1w2E3/Baddel.app" "$T/tauri_current_appQ1w2E3/current_app"
    find "$T/tauri_current_appQ1w2E3" -exec touch -h -t 202610250000 {} +
    # One that only says it is Baddel: never removed.
    mkdir -p "$T/tauri_current_appFake22/current_app/Contents"
    printf '<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>%s</string><key>CFBundleShortVersionString</key><string>2.0.1</string></dict></plist>' "$ID" \
      > "$T/tauri_current_appFake22/current_app/Contents/Info.plist"
    find "$T/tauri_current_appFake22" -exec touch -h -t 202610250000 {} +
    # Not Baddel's, or not stale, or not staging at all: all must stay.
    mkdir -p "$T/tauri_current_appOther9/current_app/Contents"
    printf '<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>com.example.other</string></dict></plist>' \
      > "$T/tauri_current_appOther9/current_app/Contents/Info.plist"
    touch -t 202610250000 "$T/tauri_current_appOther9"
    mkdir -p "$T/tauri_updated_appFresh1/Contents"
    cp "$T/tauri_current_appFake22/current_app/Contents/Info.plist" "$T/tauri_updated_appFresh1/Contents/Info.plist"
    mkdir -p "$T/Baddel copy.app/Contents" "$H/Downloads/Baddel.app/Contents"
    cp "$T/tauri_current_appFake22/current_app/Contents/Info.plist" "$T/Baddel copy.app/Contents/Info.plist"
    cp "$T/tauri_current_appFake22/current_app/Contents/Info.plist" "$H/Downloads/Baddel.app/Contents/Info.plist"
    show "another Baddel running now (the installed one): $(pgrep -lf '/Applications/Baddel.app/Contents/MacOS/baddel' | wc -l | tr -d ' ')"
    show "before"; (cd "$WORK/upgrade" && find home tmp -mindepth 1 -maxdepth 4 -not -path '*/Contents/*' | sort)
    launch -g
    sleep 5
    show "after"; (cd "$WORK/upgrade" && find home tmp -mindepth 1 -maxdepth 4 -not -path '*/Contents/*' | sort)
    show "settings: before → after (only launchAtLogin may follow macOS)"
    python3 - "$WORK/upgrade/settings.before.json" "$support/settings.json" <<'PY'
import json, sys
before, after = (json.load(open(p))["settings"] for p in sys.argv[1:3])
changed = {k: (before.get(k), after.get(k)) for k in set(before) | set(after) if before.get(k) != after.get(k)}
print("changed:", changed or "nothing")
PY
    record
    ;;
  *) echo "unknown scenario: $SCENARIO" >&2; exit 2 ;;
esac

stop_debug
show "debug stderr"; cat "$LOG" 2>/dev/null | grep '\[baddel\]' | grep -v '^\[baddel\]   ' | head -20 || true
