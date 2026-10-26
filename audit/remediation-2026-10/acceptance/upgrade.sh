#!/usr/bin/env bash
# Acceptance C: a real update through the app's own updater, isolated from the real Baddel.
#
#   audit/remediation-2026-10/acceptance/upgrade.sh <work-dir>
#
# Builds two versions under the test identity (build.sh), installs the older one in <work-dir>,
# serves updates from 127.0.0.1 only, and lets the app check and install them itself
# (BADDEL_ACCEPT=update, a debug-only stand-in for the two clicks in Settings). HOME and TMPDIR
# point inside <work-dir>: no real settings, login item, temporary folder or installed Baddel is
# read or touched, and only processes running from <work-dir> are ever stopped.
#
#   1. a download whose signature does not hold → refused; nothing changed, nothing kept aside
#   2. an archive signed for the version offered but holding another (a misbuilt release) →
#      swapped in by the updater, found wrong by Baddel, the previous version put back from its
#      copy; the update stays on offer
#   3. the right archive → installed, the copy kept until the new version runs, Baddel restarted
#      into it (one process), and the copy and note cleared by the new version
set -euo pipefail
cd "$(dirname "$0")/../../.."
WORK="$1"
HERE="audit/remediation-2026-10/acceptance"
OLD="2.0.90"
NEW="2.0.91"
# The builds have the port baked in: reused, with their port, when they are already there.
if [ -f "$WORK/port" ] && [ -d "$WORK/builds/$OLD/BaddelAcceptance.app" ] && [ -d "$WORK/builds/$NEW/BaddelAcceptance.app" ]; then
  PORT="$(cat "$WORK/port")"
  BUILT=1
else
  PORT=$((47000 + RANDOM % 1000))
  BUILT=0
fi
while lsof -i ":$PORT" >/dev/null 2>&1; do echo "port $PORT is busy" >&2; exit 1; done

APPS="$WORK/apps"
APP="$APPS/BaddelAcceptance.app"
EXE="$APP/Contents/MacOS/baddel"
H="$WORK/home"
T="$WORK/tmp"
SUPPORT="$H/Library/Application Support/com.isltanx.baddel.acceptance"
SERVE="$WORK/serve"
LOG="$WORK/stderr.log"

show() { echo "── $*"; }
version_at() { /usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$APP/Contents/Info.plist" 2>/dev/null || echo "(none)"; }
ours() { pgrep -f "^$EXE" || true; }
stop_ours() {
  for pid in $(ours); do kill -TERM "$pid" 2>/dev/null || true; done
  for _ in $(seq 1 50); do [ -z "$(ours)" ] && return 0; sleep 0.1; done
}
offer() { # <version offered> <archive> <signature file>
  local archive="${2##*/}"
  python3 - "$1" "$PORT" "$archive" "$3" > "$SERVE/latest.json" <<'PY'
import json, sys
version, port, archive, sig = sys.argv[1:5]
entry = {"signature": open(sig).read().strip(), "url": f"http://127.0.0.1:{port}/{archive}"}
print(json.dumps({"version": version, "notes": "acceptance", "pub_date": "2026-10-27T00:00:00Z",
                  "platforms": {k: entry for k in ("darwin-aarch64", "darwin-aarch64-app", "darwin-x86_64", "darwin-x86_64-app")}}))
PY
}
launch() {
  : > "$LOG"
  open -n -g --env HOME="$H" --env TMPDIR="$T/" --env BADDEL_ACCEPT=update --stderr "$LOG" "$APP"
}
wait_for() { # <pattern> <seconds>
  for _ in $(seq 1 $(($2 * 4))); do grep -q "$1" "$LOG" 2>/dev/null && return 0; sleep 0.25; done
  return 1
}
state() {
  show "version at the app's path: $(version_at); processes: $(ours | wc -l | tr -d ' ') ($(ours | tr '\n' ' '))"
  show "copy kept aside: $(ls "$SUPPORT/update/previous" 2>/dev/null | tr '\n' ' ' || true)$([ -d "$SUPPORT/update/previous" ] || echo none); note: $([ -f "$SUPPORT/update/state.json" ] && echo present || echo none)"
  show "updater staging in TMPDIR: $(ls "$T" | grep -c '^tauri_' || true)"
}

stop_ours
rm -rf "$APPS" "$H" "$T" "$SERVE"
mkdir -p "$APPS" "$SUPPORT" "$T" "$SERVE"

if [ "$BUILT" = 0 ]; then
  show "building $OLD and $NEW under the test identity (port $PORT)"
  "$HERE/build.sh" "$WORK/builds" "$OLD" "$PORT" | tail -1
  "$HERE/build.sh" "$WORK/builds" "$NEW" "$PORT" | tail -1
  echo "$PORT" > "$WORK/port"
else
  show "reusing the builds of $OLD and $NEW (port $PORT)"
fi
ditto "$WORK/builds/$OLD/BaddelAcceptance.app" "$APP"
printf '{"settings":{"welcomed":true,"autoUpdate":false,"language":"en","shortcutConvert":"Control+Alt+Space","excludedApps":["com.example.mine"],"defaultsVersion":2}}\n' > "$SUPPORT/settings.json"
cp "$SUPPORT/settings.json" "$WORK/settings.before.json"
echo "the user's own file" > "$SUPPORT/notes.txt"

cp "$WORK/builds/$NEW/BaddelAcceptance.app.tar.gz" "$SERVE/BaddelAcceptance_$NEW.app.tar.gz"
# The same archive with one byte more: its signature no longer holds.
cp "$SERVE/BaddelAcceptance_$NEW.app.tar.gz" "$SERVE/tampered.app.tar.gz"
printf 'x' >> "$SERVE/tampered.app.tar.gz"
# The $NEW archive signed — with the same throwaway key — as if it were 2.0.92.
cp "$SERVE/BaddelAcceptance_$NEW.app.tar.gz" "$SERVE/mislabelled.app.tar.gz"
(source scripts/env.sh && npx tauri signer sign --app-version 2.0.92 -f "$WORK/builds/updater-test.key" -p "" "$SERVE/mislabelled.app.tar.gz" >/dev/null)
python3 -m http.server "$PORT" --bind 127.0.0.1 --directory "$SERVE" >"$WORK/server.log" 2>&1 &
SERVER=$!
trap 'stop_ours; kill $SERVER 2>/dev/null || true' EXIT
sleep 1

show "1. a download whose signature does not hold"
offer "2.0.92" "$SERVE/tampered.app.tar.gz" "$WORK/builds/$NEW/BaddelAcceptance.app.tar.gz.sig"
launch
wait_for "acceptance update install" 60 || echo "(no install outcome within 60 s)"
grep '\[baddel\] acceptance\|update download failed' "$LOG" | sed 's/last_checked: Some([0-9]*)/last_checked: …/'
state
stop_ours

show "2. an archive signed for 2.0.92 but holding $NEW: the updater swaps it in, Baddel finds it wrong"
offer "2.0.92" "$SERVE/mislabelled.app.tar.gz" "$SERVE/mislabelled.app.tar.gz.sig"
launch
wait_for "acceptance update install" 60 || echo "(no install outcome within 60 s)"
grep '\[baddel\] acceptance\|update install failed\|update download failed' "$LOG" | sed 's/last_checked: Some([0-9]*)/last_checked: …/'
state
show "the running process's bundle is the restored $OLD: $(cmp -s "$APP/Contents/MacOS/baddel" "$WORK/builds/$OLD/BaddelAcceptance.app/Contents/MacOS/baddel" && echo yes || echo NO)"
stop_ours

show "3. $NEW offered as $NEW"
offer "$NEW" "$SERVE/BaddelAcceptance_$NEW.app.tar.gz" "$WORK/builds/$NEW/BaddelAcceptance.app.tar.gz.sig"
launch
first=""
for _ in $(seq 1 120); do first="$(ours | head -1)"; [ -n "$first" ] && break; sleep 0.25; done
show "launched $OLD as pid $first"
# The new version, started by the old one's restart, says at launch how the update went.
wait_for "update Completed" 90 || echo "(the new version did not report within 90 s)"
sleep 2
grep '\[baddel\] acceptance\|\[baddel\] at launch\|another Baddel' "$LOG" | sed 's/last_checked: Some([0-9]*)/last_checked: …/'
state
show "settings: before → after"
python3 - "$WORK/settings.before.json" "$SUPPORT/settings.json" <<'PY'
import json, sys
before, after = (json.load(open(p))["settings"] for p in sys.argv[1:3])
print("changed:", {k: (before.get(k), after.get(k)) for k in before if before.get(k) != after.get(k)} or "nothing the user set")
PY
show "the user's own file: $(cat "$SUPPORT/notes.txt")"
show "server requests"; grep -o '"GET [^"]*"' "$WORK/server.log" | sort | uniq -c
