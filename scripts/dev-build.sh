#!/usr/bin/env bash
# Builds the debug app bundle and signs it with a stable local identity.
#
# Why sign a debug build: macOS ties the Accessibility permission to the code signature.
# An unsigned (ad-hoc) build gets a new identity on every rebuild and silently loses the
# permission; signed with the same certificate, the grant survives rebuilds.
#
# Identity: $BADDEL_SIGN_ID, else the first "Apple Development" / "Developer ID Application"
# certificate in the keychain. With none, the bundle stays ad-hoc (works, but re-grant after
# each build: remove Baddel from System Settings › Privacy › Accessibility and add it again).
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/env.sh

npm run tauri build -- --debug --bundles app

APP="target/debug/bundle/macos/Baddel.app"
ID="${BADDEL_SIGN_ID:-$(security find-identity -v -p codesigning | sed -nE 's/.*"((Apple Development|Developer ID Application)[^"]*)".*/\1/p' | head -1)}"
if [ -n "$ID" ]; then
  codesign --force --deep --sign "$ID" "$APP"
  echo "signed with: $ID"
else
  echo "no signing identity found — bundle left ad-hoc"
fi
codesign -dr - "$APP" 2>&1 | sed -n 's/^designated => /designated requirement: /p'
echo "$APP"
