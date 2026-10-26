#!/usr/bin/env bash
# Acceptance A: the real login item, under the test identity — never the real Baddel's.
#
#   audit/remediation-2026-10/acceptance/login-item.sh <work-dir> install|on|status|off|remove
#
# ⚠️ Changes the Mac: puts BaddelAcceptance.app in ~/Applications and registers it as a real
# login item (System Settings › General › Login Items lists it). Only with the owner's approval.
# The real Baddel, its settings and its login item are never touched: the test identity
# (com.isltanx.baddel.acceptance) has its own of each. A logout and login, if asked for, is the
# owner's to do; `status` afterwards reads what the login did.
#
#   install  build it signed with Baddel's own identity (SMAppService answers "not found" for an
#            ad-hoc bundle in a temporary folder), and put it in ~/Applications
#   on       launch it and switch launch at login on, as the switch in Settings does
#   status   launch it (or read the running one) and show macOS's answer and what the launch saw
#   off      switch it off again
#   remove   quit it, switch it off if it is on, remove the app and its data folder
set -euo pipefail
cd "$(dirname "$0")/../../.."
WORK="$1"
STEP="$2"
ID="com.isltanx.baddel.acceptance"
APP="$HOME/Applications/BaddelAcceptance.app"
EXE="$APP/Contents/MacOS/baddel"
SUPPORT="$HOME/Library/Application Support/$ID"
LOG="$WORK/login-item.log"
mkdir -p "$WORK"

show() { echo "── $*"; }
ours() { pgrep -f "^$EXE" || true; }
stop_ours() {
  for pid in $(ours); do kill -TERM "$pid" 2>/dev/null || true; done
  for _ in $(seq 1 50); do [ -z "$(ours)" ] && return 0; sleep 0.1; done
}
btm() { # what Background Task Management lists for the test identity (read only)
  sfltool dumpbtm 2>/dev/null | awk -v id="$ID" 'BEGIN{RS="\n\n"} index($0, id) {print; print ""}' | grep -E "Name|Type|Disposition|Identifier|URL" || echo "(nothing listed for $ID)"
}
launch() { # [step]
  : > "$LOG"
  if [ -n "${1:-}" ]; then
    open -n -g --env BADDEL_ACCEPT="$1" --stderr "$LOG" "$APP"
  else
    open -n -g --stderr "$LOG" "$APP"
  fi
  sleep 5
  grep '\[baddel\] acceptance\|\[baddel\] at launch' "$LOG" || true
}
record() { show "maintenance.json"; python3 -c 'import json,sys; r=json.load(open(sys.argv[1])); print(json.dumps({k: r[k] for k in ("started","instance","loginItem","settings")}, indent=1))' "$SUPPORT/maintenance.json" 2>/dev/null || echo "(none)"; }

# Never next to an old LaunchAgent of the real Baddel: the test identity leaves it alone (another
# app's), but its presence would make this no clean test of the login item alone.
if [ -e "$HOME/Library/LaunchAgents/Baddel.plist" ]; then
  echo "~/Library/LaunchAgents/Baddel.plist exists: stopping, see acceptance/README.md" >&2
  exit 1
fi

case "$STEP" in
  install)
    source scripts/env.sh
    source scripts/signing/signing-env.sh
    npm run tauri build -- --debug --bundles app --config "{\"identifier\":\"$ID\",\"productName\":\"BaddelAcceptance\",\"version\":\"2.0.95\",\"bundle\":{\"createUpdaterArtifacts\":false}}"
    BUILT="target/debug/bundle/macos/BaddelAcceptance.app"
    baddel_load_keys && sign_bundle "$BUILT"
    stop_ours
    mkdir -p "$HOME/Applications"
    rm -rf "$APP"
    ditto "$BUILT" "$APP"
    rm -rf "$BUILT"
    codesign -dr - "$APP" 2>&1 | sed -n 's/^designated => /designated requirement: /p'
    ;;
  on)
    stop_ours; launch login-on; show "Background Task Management"; btm ;;
  status)
    [ -n "$(ours)" ] || launch
    show "running: $(ours | wc -l | tr -d ' ') ($(ours | tr '\n' ' '))"
    record; show "Background Task Management"; btm ;;
  off)
    stop_ours; launch login-off; show "Background Task Management"; btm ;;
  remove)
    stop_ours
    if [ -d "$APP" ]; then launch login-off; stop_ours; fi
    rm -rf "$APP" "$SUPPORT"
    show "Background Task Management"; btm ;;
  *) echo "unknown step: $STEP" >&2; exit 2 ;;
esac
stop_ours_after="${KEEP_RUNNING:-0}"
[ "$stop_ours_after" = 1 ] || stop_ours
