#!/usr/bin/env bash
# Installs Baddel from its GitHub releases, without a Gatekeeper warning and without sudo.
#
#   curl -fsSL https://github.com/iSltanX/Baddel/releases/latest/download/install.sh | bash
#
# Options (after `bash -s --` when piped):
#   --version X.Y.Z   install that release instead of the latest
#   --dest DIR        install into DIR instead of /Applications (or ~/Applications)
#   --no-open         do not open Baddel afterwards
#   --dry-run         say what would happen; download nothing but latest.json
#
# What it does, in order — and it stops at the first thing that is not exactly right:
#   1. reads the release's latest.json for its version and archive (the updater's own archive);
#   2. downloads the archive and SHA256SUMS, and checks the archive's SHA-256;
#   3. unpacks it and checks the app is signed by Baddel's own certificate (pinned below);
#   4. replaces an existing Baddel.app only if it really is Baddel, quitting it first;
#   5. opens it. `curl` sets no quarantine flag, so macOS shows no warning, and nothing in
#      Gatekeeper or your security settings is touched.
#
# Contains no secrets or tokens. Read it before you run it: that is what it is for.
set -euo pipefail

REPO="iSltanX/Baddel"
BUNDLE_ID="com.isltanx.baddel"
# The leaf certificate every Baddel release is signed with (docs/signing.md). An archive signed
# by anyone else is refused, even if its checksum matches.
LEAF='H"abeadcf9647763862bc4924c07da7e5a7fd777b0"'
# Where releases are served from. Overridable for testing against a local copy only.
RELEASES="${BADDEL_RELEASES_URL:-https://github.com/$REPO/releases}"

VERSION=""
DEST=""
OPEN=1
DRY_RUN=0

say() { printf '%s\n' "$*"; }
fail() { printf 'install.sh: %s\n' "$*" >&2; exit 1; }

while [ $# -gt 0 ]; do
  case "$1" in
    --version) [ $# -ge 2 ] || fail "--version needs a version, e.g. --version 2.0.0"; VERSION="${2#v}"; shift 2 ;;
    --dest) [ $# -ge 2 ] || fail "--dest needs a folder"; DEST="$2"; shift 2 ;;
    --no-open) OPEN=0; shift ;;
    --dry-run) DRY_RUN=1; shift ;;
    -h|--help) sed -n '2,22p' "$0" 2>/dev/null | sed 's/^# \{0,1\}//'; exit 0 ;;
    *) fail "unknown option: $1 (see --help)" ;;
  esac
done

# ── The Mac ──────────────────────────────────────────────────────────────────
[ "$(uname -s)" = "Darwin" ] || fail "Baddel runs on macOS only."
os="$(sw_vers -productVersion)"
[ "${os%%.*}" -ge 13 ] || fail "Baddel needs macOS 13 or later (this Mac has $os)."
[ "$(id -u)" -ne 0 ] || fail "do not run this with sudo: it installs for you, not for root."
for tool in curl shasum plutil codesign ditto tar; do
  command -v "$tool" >/dev/null || fail "missing $tool, which comes with macOS — is this a standard install?"
done

# ── The release ──────────────────────────────────────────────────────────────
WORK="$(mktemp -d -t baddel-install)"
trap 'rm -rf "$WORK"' EXIT

fetch() { # <url> <file>
  curl -fsL --retry 2 --connect-timeout 15 -o "$2" "$1" \
    || fail "could not download $1 — check your connection, or the version."
}

if [ -n "$VERSION" ]; then
  [[ "$VERSION" =~ ^[0-9]+\.[0-9]+\.[0-9]+$ ]] || fail "not a version: $VERSION (expected e.g. 2.0.0)"
  BASE="$RELEASES/download/v$VERSION"
else
  BASE="$RELEASES/latest/download"
fi
fetch "$BASE/latest.json" "$WORK/latest.json"
# Read through a plist copy: PlistBuddy names keys with a dash ("darwin-aarch64") that a plutil
# key path may not, and both come with every macOS — no jq, no Python.
plutil -convert xml1 -o "$WORK/latest.plist" "$WORK/latest.json" 2>/dev/null || fail "latest.json is not valid JSON."
version="$(/usr/libexec/PlistBuddy -c 'Print :version' "$WORK/latest.plist" 2>/dev/null)" || fail "latest.json has no version."
url="$(/usr/libexec/PlistBuddy -c 'Print :platforms:darwin-aarch64:url' "$WORK/latest.plist" 2>/dev/null)" \
  || fail "latest.json has no archive for this Mac."
[ -z "$VERSION" ] || [ "$version" = "$VERSION" ] || fail "asked for $VERSION but the release says $version."
archive="${url##*/}"
[[ "$archive" == Baddel_*_universal.app.tar.gz ]] || fail "unexpected archive name: $archive"
# The archive is fetched from the same release as latest.json, whatever host latest.json names.
release_base="$RELEASES/download/v$version"

if [ -z "$DEST" ]; then
  if [ -w /Applications ]; then DEST="/Applications"; else DEST="$HOME/Applications"; fi
fi
APP="$DEST/Baddel.app"

say "Baddel $version → $APP"
if [ "$DRY_RUN" -eq 1 ]; then
  say "would download: $release_base/$archive"
  say "would verify:   SHA-256 from $release_base/SHA256SUMS, then the signature ($LEAF)"
  [ -e "$APP" ] && say "would replace:  the Baddel.app already there"
  say "dry run — nothing was installed."
  exit 0
fi

# ── Download and check ───────────────────────────────────────────────────────
fetch "$release_base/$archive" "$WORK/$archive"
curl -fsL --retry 2 --connect-timeout 15 -o "$WORK/SHA256SUMS" "$release_base/SHA256SUMS" \
  || fail "release $version has no SHA256SUMS, so it cannot be checked — install it from the DMG instead."
line="$(grep -E "^[0-9a-f]{64}  $archive\$" "$WORK/SHA256SUMS" || true)"
[ -n "$line" ] || fail "SHA256SUMS does not list $archive."
(cd "$WORK" && printf '%s\n' "$line" | shasum -a 256 -c - >/dev/null 2>&1) \
  || fail "the download does not match its SHA-256 — nothing was installed. Try again later."
say "✓ SHA-256"

mkdir -p "$WORK/unpacked"
tar -xzf "$WORK/$archive" -C "$WORK/unpacked" || fail "could not unpack $archive."
NEW="$WORK/unpacked/Baddel.app"
[ -d "$NEW" ] || fail "$archive does not contain Baddel.app."
codesign --verify --deep --strict "$NEW" 2>/dev/null || fail "Baddel.app's signature is broken — nothing was installed."
codesign --verify -R="identifier \"$BUNDLE_ID\" and certificate leaf = $LEAF" "$NEW" 2>/dev/null \
  || fail "Baddel.app is not signed by Baddel's certificate — nothing was installed."
say "✓ signed by Baddel"

# ── Install ──────────────────────────────────────────────────────────────────
mkdir -p "$DEST" || fail "cannot create $DEST."
[ -w "$DEST" ] || fail "cannot write to $DEST — choose another folder with --dest."
if [ -e "$APP" ]; then
  existing="$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$APP/Contents/Info.plist" 2>/dev/null || true)"
  [ "$existing" = "$BUNDLE_ID" ] || fail "$APP exists and is not Baddel ($existing) — leaving it alone."
  # Quit only a Baddel running from the copy being replaced.
  if pkill -f "^$APP/Contents/MacOS/" 2>/dev/null; then
    say "quit the running Baddel"
    sleep 1
  fi
  rm -rf "$APP"
fi
ditto "$NEW" "$APP" || fail "could not copy Baddel.app into $DEST."
say "✓ installed"

if [ "$OPEN" -eq 1 ]; then
  open "$APP"
  say "Baddel is open. Grant it Accessibility when asked: System Settings → Privacy & Security."
fi
say "To uninstall: quit Baddel from its menu, then move $APP to the Trash."
