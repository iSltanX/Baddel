#!/usr/bin/env bash
# Builds a debug Baddel under a test identity, for the acceptance checks in this folder.
#
#   audit/remediation-2026-10/acceptance/build.sh <work-dir> <version> <port>
#
# The test identity keeps it apart from the real Baddel everywhere macOS keys things by app:
# bundle identifier com.isltanx.baddel.acceptance, name BaddelAcceptance, its own Application
# Support folder, lock, login item and Launch Services record. Its updater trusts only a
# throwaway key made here (never Baddel's own), and asks only http://127.0.0.1:<port>. The bundle
# is signed ad hoc: Baddel's signing identity and secrets are not used.
#
# Leaves in <work-dir>/<version>/: BaddelAcceptance.app, its updater archive and signature.
set -euo pipefail
cd "$(dirname "$0")/../../.."
source scripts/env.sh

WORK="$1"
VERSION="$2"
PORT="$3"
OUT="$WORK/$VERSION"
KEY="$WORK/updater-test.key"
mkdir -p "$WORK"

if [ ! -f "$KEY" ]; then
  npx tauri signer generate --ci -w "$KEY" -p "" >/dev/null
fi
PUBKEY="$(cat "$KEY.pub")"

CONFIG=$(python3 - "$VERSION" "$PORT" "$PUBKEY" <<'PY'
import json, sys
version, port, pubkey = sys.argv[1:4]
print(json.dumps({
    "identifier": "com.isltanx.baddel.acceptance",
    "productName": "BaddelAcceptance",
    "version": version,
    "bundle": {"createUpdaterArtifacts": True},
    "plugins": {"updater": {
        "pubkey": pubkey,
        "endpoints": [f"http://127.0.0.1:{port}/latest.json"],
        "dangerousInsecureTransportProtocol": True,
    }},
}))
PY
)

# The throwaway key, in this script's environment only.
TAURI_SIGNING_PRIVATE_KEY="$(cat "$KEY")" TAURI_SIGNING_PRIVATE_KEY_PASSWORD="" \
  npm run tauri build -- --debug --bundles app --config "$CONFIG"

BUNDLE="target/debug/bundle/macos"
rm -rf "$OUT"
mkdir -p "$OUT"
ditto "$BUNDLE/BaddelAcceptance.app" "$OUT/BaddelAcceptance.app"
cp "$BUNDLE/BaddelAcceptance.app.tar.gz" "$BUNDLE/BaddelAcceptance.app.tar.gz.sig" "$OUT/"
# The build outputs of the test identity go: the real debug bundle beside them is left as it was.
rm -rf "$BUNDLE/BaddelAcceptance.app" "$BUNDLE/BaddelAcceptance.app.tar.gz" "$BUNDLE/BaddelAcceptance.app.tar.gz.sig"
echo "$OUT: $(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$OUT/BaddelAcceptance.app/Contents/Info.plist") $(/usr/libexec/PlistBuddy -c 'Print :CFBundleShortVersionString' "$OUT/BaddelAcceptance.app/Contents/Info.plist")"
