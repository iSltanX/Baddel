#!/usr/bin/env bash
# scripts/install.sh against a local copy of a published release, into a folder of its own.
#
#   audit/remediation-2026-10/tools/install-check.sh <work-dir> [version]
#
# Serves release/<version>/ through file:// (BADDEL_RELEASES_URL), installs with --dest inside
# <work-dir> and --no-open, so /Applications and any running Baddel are never touched. Cases: a
# replacement that works, and one whose copy fails (a `ditto` that refuses), after which the
# previous app must be back exactly as it was.
set -euo pipefail
cd "$(dirname "$0")/../../.."
WORK="$1"
VERSION="${2:-2.0.2}"
REL="$PWD/release/$VERSION"
ARCHIVE="Baddel_${VERSION}_universal.app.tar.gz"
[ -f "$REL/$ARCHIVE" ] || { echo "no $REL/$ARCHIVE" >&2; exit 1; }

rm -rf "$WORK"
mkdir -p "$WORK/releases/latest/download" "$WORK/releases/download/v$VERSION" "$WORK/shim"
cp "$REL/latest.json" "$WORK/releases/latest/download/latest.json"
cp "$REL/$ARCHIVE" "$REL/SHA256SUMS" "$WORK/releases/download/v$VERSION/"
export BADDEL_RELEASES_URL="file://$WORK/releases"

# The "previous version": the same release unpacked, marked so it can be told apart afterwards.
seed() { # <dest>
  rm -rf "$1"
  mkdir -p "$1"
  tar -xzf "$REL/$ARCHIVE" -C "$1"
  echo "previous" > "$1/Baddel.app/Contents/Resources/marker-previous"
  (cd "$1" && find Baddel.app -type f -exec shasum -a 256 {} + | sort -k2) > "$1.sums"
}
leftovers() { find "$1" -maxdepth 1 -name '.Baddel.app.previous.*' | wc -l | tr -d ' '; }

echo "── 1. replacing an existing Baddel"
DEST="$WORK/ok"
seed "$DEST"
bash scripts/install.sh --dest "$DEST" --no-open
echo "installed version: $(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$DEST/Baddel.app/Contents/Info.plist")"
echo "old one gone (marker absent): $([ -e "$DEST/Baddel.app/Contents/Resources/marker-previous" ] && echo no || echo yes)"
echo "copies left aside: $(leftovers "$DEST")"
codesign --verify --deep --strict "$DEST/Baddel.app" && echo "signature: valid"

echo "── 2. the copy fails"
DEST="$WORK/fail"
seed "$DEST"
printf '#!/bin/sh\necho "ditto: refused for the test" >&2\nexit 1\n' > "$WORK/shim/ditto"
chmod +x "$WORK/shim/ditto"
set +e
PATH="$WORK/shim:$PATH" bash scripts/install.sh --dest "$DEST" --no-open
status=$?
set -e
echo "install.sh exit status: $status"
(cd "$DEST" && find Baddel.app -type f -exec shasum -a 256 {} + | sort -k2) > "$DEST.after"
echo "previous app back, byte for byte: $(cmp -s "$DEST.sums" "$DEST.after" && echo yes || echo NO)"
echo "copies left aside: $(leftovers "$DEST")"

echo "── 3. stopped halfway through the copy (SIGTERM while ditto runs)"
DEST="$WORK/interrupted"
seed "$DEST"
printf '#!/bin/sh\n# Copies part of the app, then the installer is stopped.\nmkdir -p "$2/Contents"; echo partial > "$2/Contents/partial"\nkill -TERM "$PPID"\nsleep 5\n' > "$WORK/shim/ditto"
chmod +x "$WORK/shim/ditto"
set +e
PATH="$WORK/shim:$PATH" bash scripts/install.sh --dest "$DEST" --no-open
status=$?
set -e
echo "install.sh exit status: $status"
(cd "$DEST" && find Baddel.app -type f -exec shasum -a 256 {} + | sort -k2) > "$DEST.after"
echo "previous app back, byte for byte: $(cmp -s "$DEST.sums" "$DEST.after" && echo yes || echo NO)"
echo "copies left aside: $(leftovers "$DEST")"

echo "── 4. something else named Baddel.app"
DEST="$WORK/other"
mkdir -p "$DEST/Baddel.app/Contents"
printf '<?xml version="1.0" encoding="UTF-8"?><plist version="1.0"><dict><key>CFBundleIdentifier</key><string>com.example.other</string></dict></plist>' > "$DEST/Baddel.app/Contents/Info.plist"
set +e
bash scripts/install.sh --dest "$DEST" --no-open
status=$?
set -e
echo "install.sh exit status: $status; left alone: $(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$DEST/Baddel.app/Contents/Info.plist")"
