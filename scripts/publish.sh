#!/usr/bin/env bash
# Publishes a release built by scripts/release.sh: the tag, then a GitHub Release carrying
# the four files that new users and installed apps need.
#
#   ./scripts/publish.sh <version>          check everything and print what would happen
#   ./scripts/publish.sh <version> --yes    publish
#
# Outward-facing and hard to undo: run with --yes only after the user has approved this
# release in the session (EXECUTION.md, phase 6 step 5). The steps before and after it are
# in docs/launch/release-checklist.md.
set -euo pipefail
cd "$(dirname "$0")/.."

VERSION="${1:-}"
CONFIRM="${2:-}"
[[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || { echo "usage: $0 <major.minor.patch> [--yes]" >&2; exit 2; }

REPO="iSltanX/Baddel"
TAG="v$VERSION"
OUT="release/$VERSION"
DMG="Baddel_${VERSION}_universal.dmg"
ARCHIVE="Baddel_${VERSION}_universal.app.tar.gz"
FILES=("$OUT/$DMG" "$OUT/$ARCHIVE" "$OUT/$ARCHIVE.sig" "$OUT/latest.json")

fail() { echo "✗ $*" >&2; exit 1; }
ok() { echo "✓ $*"; }

# ── Checks ───────────────────────────────────────────────────────────────────
for file in "${FILES[@]}"; do
  [ -s "$file" ] || fail "missing $file — run ./scripts/release.sh $VERSION first"
done
ok "artifacts in $OUT"

python3 - "$VERSION" "$REPO" "$TAG" "$ARCHIVE" "$OUT" <<'EOF' || fail "latest.json does not match this release"
import json, sys
version, repo, tag, archive, out = sys.argv[1:]
manifest = json.load(open(f"{out}/latest.json"))
signature = open(f"{out}/{archive}.sig").read().strip()
url = f"https://github.com/{repo}/releases/download/{tag}/{archive}"
assert manifest["version"] == version, f"version {manifest['version']}"
for name in ("darwin-aarch64", "darwin-x86_64"):
    platform = manifest["platforms"][name]
    assert platform["url"] == url, f"{name} url {platform['url']}"
    assert platform["signature"] == signature, f"{name} signature differs from {archive}.sig"
EOF
ok "latest.json points at $TAG and carries the archive's signature"

grep -q "\"version\": \"$VERSION\"" src-tauri/tauri.conf.json || fail "tauri.conf.json is not at $VERSION"
[ "$(git branch --show-current)" = "main" ] || fail "not on main"
[ -z "$(git status --porcelain)" ] || fail "working tree not clean — commit the version bump first"
ok "main is clean and at $VERSION"

if git rev-parse -q --verify "refs/tags/$TAG" >/dev/null; then fail "tag $TAG already exists"; fi
if gh release view "$TAG" -R "$REPO" >/dev/null 2>&1; then fail "release $TAG already exists"; fi
ok "no $TAG yet"

# Release notes: the CHANGELOG entry for this version, which must carry its date by now.
NOTES=$(mktemp -t baddel-notes)
trap 'rm -f "$NOTES"' EXIT
awk -v head="## $TAG" '
  index($0, head) == 1 { found = 1; next }
  found && /^## / { exit }
  found && !/^<!--/ { print }
' CHANGELOG.md > "$NOTES"
[ -s "$NOTES" ] || fail "no \"## $TAG\" entry in CHANGELOG.md"
if grep -q "^## $TAG — لم يُنشر بعد" CHANGELOG.md; then fail "CHANGELOG.md: replace «لم يُنشر بعد» with the publish date"; fi
ok "release notes from CHANGELOG.md ($(wc -l < "$NOTES" | tr -d ' ') lines)"

VISIBILITY=$(gh repo view "$REPO" --json visibility -q .visibility)
echo
echo "repository:  $REPO ($VISIBILITY)"
echo "tag:         $TAG on $(git rev-parse --short HEAD)"
echo "files:"
for file in "${FILES[@]}"; do printf '  %-48s %s\n' "$(basename "$file")" "$(du -h "$file" | cut -f1)"; done
[ "$VISIBILITY" = "PUBLIC" ] || echo "note: the repository is private — installed apps cannot reach latest.json until it is public."

if [ "$CONFIRM" != "--yes" ]; then
  echo
  echo "dry run — nothing was published. Add --yes to publish."
  exit 0
fi

# ── Publish ──────────────────────────────────────────────────────────────────
git tag -a "$TAG" -m "Baddel $VERSION"
git push origin main "$TAG"
gh release create "$TAG" "${FILES[@]}" -R "$REPO" --verify-tag \
  --title "بدّل — Baddel $VERSION" --notes-file "$NOTES"
echo "https://github.com/$REPO/releases/tag/$TAG"

# What installed apps will read. A private repository answers 404 here.
if manifest=$(curl -fsSL "https://github.com/$REPO/releases/latest/download/latest.json"); then
  echo "latest.json → $(python3 -c 'import json, sys; print(json.load(sys.stdin)["version"])' <<<"$manifest")"
else
  echo "latest.json is not reachable yet (a private repository answers 404)."
fi
