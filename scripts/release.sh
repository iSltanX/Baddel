#!/usr/bin/env bash
# Builds a signed universal release and everything a GitHub Release needs.
#
#   ./scripts/release.sh <version> [notes]
#
# 1. Sets <version> in package.json (+ lock), Cargo.toml and tauri.conf.json.
# 2. Builds for Apple Silicon and Intel in one bundle, signed with Baddel's identity
#    (scripts/signing/create-keys.sh), with the updater archive signed by the updater key.
# 3. Collects into release/<version>/: the DMG, the updater archive and its signature,
#    and latest.json — the file the installed apps poll.
#
# Publishing (tag + GitHub Release) is a separate, deliberate step: see EXECUTION.md.
set -euo pipefail
cd "$(dirname "$0")/.."
source scripts/env.sh
source scripts/signing/signing-env.sh

VERSION="${1:-}"
NOTES="${2:-}"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "usage: $0 <major.minor.patch> [notes]" >&2; exit 2; }
baddel_load_keys || { echo "no signing keys in $BADDEL_KEYS — run scripts/signing/create-keys.sh first" >&2; exit 1; }
trap baddel_close_keychain EXIT

REPO="iSltanX/Baddel"
TARGET="universal-apple-darwin"
BUNDLE="target/$TARGET/release/bundle"
OUT="release/$VERSION"

# ── 1. Version ───────────────────────────────────────────────────────────────
npm version "$VERSION" --no-git-tag-version --allow-same-version >/dev/null
python3 - "$VERSION" <<'EOF'
import json, re, sys
version = sys.argv[1]
path = "src-tauri/tauri.conf.json"
config = json.load(open(path))
config["version"] = version
open(path, "w").write(json.dumps(config, ensure_ascii=False, indent=2) + "\n")
path = "Cargo.toml"
text = open(path).read()
text, n = re.subn(r'(\[workspace\.package\][^\[]*?\nversion = ")[^"]+(")', rf'\g<1>{version}\2', text, count=1)
assert n == 1, "workspace.package version not found in Cargo.toml"
open(path, "w").write(text)
EOF

# ── 2. Build ─────────────────────────────────────────────────────────────────
rustup target add aarch64-apple-darwin x86_64-apple-darwin >/dev/null
rm -rf "$BUNDLE"
baddel_open_keychain
npm run tauri build -- --target "$TARGET"
baddel_close_keychain

APP="$BUNDLE/macos/Baddel.app"
codesign --verify --deep --strict "$APP"
REQUIREMENT="$(codesign -dr - "$APP" 2>&1 | sed -n 's/^designated => //p')"
echo "designated requirement: $REQUIREMENT"
[[ "$REQUIREMENT" == *'certificate leaf = H"'* ]] || { echo "not signed with Baddel's identity" >&2; exit 1; }

# ── 3. Artifacts ─────────────────────────────────────────────────────────────
rm -rf "$OUT"
mkdir -p "$OUT"
DMG="Baddel_${VERSION}_universal.dmg"
ARCHIVE="Baddel_${VERSION}_universal.app.tar.gz"
cp "$BUNDLE"/dmg/*.dmg "$OUT/$DMG"
cp "$BUNDLE/macos/Baddel.app.tar.gz" "$OUT/$ARCHIVE"
cp "$BUNDLE/macos/Baddel.app.tar.gz.sig" "$OUT/$ARCHIVE.sig"

python3 - "$VERSION" "$NOTES" "$REPO" "$ARCHIVE" "$OUT" <<'EOF'
import datetime, json, sys
version, notes, repo, archive, out = sys.argv[1:]
signature = open(f"{out}/{archive}.sig").read().strip()
url = f"https://github.com/{repo}/releases/download/v{version}/{archive}"
platform = {"signature": signature, "url": url}
manifest = {
    "version": version,
    "notes": notes,
    "pub_date": datetime.datetime.now(datetime.timezone.utc).strftime("%Y-%m-%dT%H:%M:%SZ"),
    # One universal archive serves both architectures.
    "platforms": {"darwin-aarch64": platform, "darwin-x86_64": platform},
}
open(f"{out}/latest.json", "w").write(json.dumps(manifest, ensure_ascii=False, indent=2) + "\n")
EOF

echo
echo "release $VERSION ready in $OUT:"
ls -lh "$OUT" | tail -n +2
