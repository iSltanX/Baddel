# Sourced by dev-build.sh and release.sh. Loads the keys made by create-keys.sh and puts
# the signing identity where codesign — ours or Tauri's — can find it, for as long as the
# calling script runs. Secrets stay in that script's environment; nothing is written out.
#
# Tauri's own certificate import (APPLE_CERTIFICATE) only accepts Apple-issued identities,
# so a self-signed one goes in through a throwaway keychain instead. It is added to the
# keychain search list while the build runs (as Tauri's import does), then removed.

BADDEL_KEYS="${BADDEL_KEYS:-$HOME/.baddel}"

# Returns 1 (and loads nothing) when the keys have not been created on this Mac.
baddel_load_keys() {
  [ -f "$BADDEL_KEYS/secrets.env" ] && [ -f "$BADDEL_KEYS/signing.p12" ] || return 1
  # shellcheck source=/dev/null
  source "$BADDEL_KEYS/secrets.env"
  # Signs the updater artifacts (.app.tar.gz → .sig).
  export TAURI_SIGNING_PRIVATE_KEY="$(cat "$BADDEL_KEYS/updater.key")"
  export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="$BADDEL_UPDATER_PASSWORD"
}

# Imports the identity into a fresh keychain, makes it visible to codesign, and exports
# APPLE_SIGNING_IDENTITY (the certificate's SHA-1, unambiguous). Undo with baddel_close_keychain.
baddel_open_keychain() {
  BADDEL_KEYCHAIN="$(mktemp -d)/baddel-sign.keychain-db"
  BADDEL_SEARCH_LIST="$(security list-keychains -d user | tr -d '"' | xargs)"
  local password
  password="$(openssl rand -hex 16)"
  security create-keychain -p "$password" "$BADDEL_KEYCHAIN"
  security set-keychain-settings -lut 3600 "$BADDEL_KEYCHAIN"
  security unlock-keychain -p "$password" "$BADDEL_KEYCHAIN"
  security import "$BADDEL_KEYS/signing.p12" -k "$BADDEL_KEYCHAIN" -P "$BADDEL_P12_PASSWORD" -T /usr/bin/codesign >/dev/null
  security set-key-partition-list -S apple-tool:,apple:,codesign: -s -k "$password" "$BADDEL_KEYCHAIN" >/dev/null
  # shellcheck disable=SC2086
  security list-keychains -d user -s $BADDEL_SEARCH_LIST "$BADDEL_KEYCHAIN"
  APPLE_SIGNING_IDENTITY="$(security find-certificate -a -Z -c "$BADDEL_SIGN_NAME" "$BADDEL_KEYCHAIN" | awk '/SHA-1/ { print $3; exit }')"
  export APPLE_SIGNING_IDENTITY
  [ -n "$APPLE_SIGNING_IDENTITY" ]
}

baddel_close_keychain() {
  [ -n "${BADDEL_KEYCHAIN:-}" ] || return 0
  # shellcheck disable=SC2086
  security list-keychains -d user -s $BADDEL_SEARCH_LIST
  security delete-keychain "$BADDEL_KEYCHAIN" 2>/dev/null || true
  rm -rf "$(dirname "$BADDEL_KEYCHAIN")"
  unset BADDEL_KEYCHAIN APPLE_SIGNING_IDENTITY
}

# Signs an already-built bundle (the debug build, which Tauri leaves unsigned).
sign_bundle() {
  local app="$1" status=0
  baddel_open_keychain
  codesign --force --deep --options runtime --sign "$APPLE_SIGNING_IDENTITY" "$app" || status=$?
  baddel_close_keychain
  return $status
}
