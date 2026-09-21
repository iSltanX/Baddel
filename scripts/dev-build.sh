#!/usr/bin/env bash
# Builds the debug app bundle and signs it with a stable local identity.
#
# Why sign a debug build: macOS ties the Accessibility permission to the code signature.
# An unsigned (ad-hoc) build gets a new identity on every rebuild and silently loses the
# permission; signed with the same certificate, the grant survives rebuilds.
#
# Identity: Baddel's own (scripts/signing/create-keys.sh) — the same one releases use, so a
# debug build and an installed release share one Accessibility grant. Without it:
# $BADDEL_SIGN_ID, else the first "Apple Development" / "Developer ID Application" certificate
# in the keychain; with none, the bundle stays ad-hoc (works, but re-grant after each build).
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/env.sh
source scripts/signing/signing-env.sh

# Debug builds are never published: no updater archive, so no updater key needed.
npm run tauri build -- --debug --bundles app --config '{"bundle":{"createUpdaterArtifacts":false}}'

APP="target/debug/bundle/macos/Baddel.app"
if baddel_load_keys; then
  sign_bundle "$APP"
  echo "signed with: $BADDEL_SIGN_NAME"
else
  ID="${BADDEL_SIGN_ID:-$(security find-identity -v -p codesigning | sed -nE 's/.*"((Apple Development|Developer ID Application)[^"]*)".*/\1/p' | head -1)}"
  if [ -n "$ID" ]; then
    codesign --force --deep --sign "$ID" "$APP"
    echo "signed with: $ID"
  else
    echo "no signing identity found — bundle left ad-hoc"
  fi
fi
codesign -dr - "$APP" 2>&1 | sed -n 's/^designated => /designated requirement: /p'
echo "$APP"
