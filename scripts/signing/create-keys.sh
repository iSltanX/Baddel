#!/usr/bin/env bash
# Creates Baddel's two long-lived keys, once, outside the repository:
#
#   1. A self-signed code-signing identity. macOS ties the Accessibility permission to the
#      app's designated requirement, which for this identity is the certificate's own hash.
#      Every release signed with it keeps the permission; a new identity loses it for every
#      user. It is not trusted by Gatekeeper, so the first launch shows a warning.
#   2. The updater key. The app only installs updates signed with it.
#
# Losing either means users must reinstall by hand (and, for 1, re-grant the permission).
# Back up the whole folder somewhere safe — it is the only copy.
#
# Nothing is added to the login keychain: builds import the identity into a throwaway
# keychain (see signing-env.sh).
set -euo pipefail
cd "$(dirname "$0")/../.."
source scripts/env.sh

DIR="${BADDEL_KEYS:-$HOME/.baddel}"
P12="$DIR/signing.p12"
UPDATER_KEY="$DIR/updater.key"
SECRETS="$DIR/secrets.env"
NAME="Baddel Code Signing"

if [ -e "$P12" ] || [ -e "$UPDATER_KEY" ] || [ -e "$SECRETS" ]; then
  echo "keys already exist in $DIR — refusing to replace them (a new identity costs every user the permission)" >&2
  exit 1
fi

umask 077
mkdir -p "$DIR"
WORK="$(mktemp -d)"
trap 'rm -rf "$WORK"' EXIT

P12_PASSWORD="$(openssl rand -hex 24)"
UPDATER_PASSWORD="$(openssl rand -hex 24)"

cat > "$WORK/cert.cnf" <<EOF
[req]
distinguished_name = dn
x509_extensions = ext
prompt = no
[dn]
CN = $NAME
[ext]
basicConstraints = critical, CA:false
keyUsage = critical, digitalSignature
extendedKeyUsage = critical, codeSigning
subjectKeyIdentifier = hash
EOF
openssl req -x509 -newkey rsa:3072 -nodes -days 7300 -config "$WORK/cert.cnf" \
  -keyout "$WORK/key.pem" -out "$WORK/cert.pem" 2>/dev/null
# `-legacy`: the Security framework on older macOS cannot read OpenSSL 3's default PKCS#12 ciphers.
openssl pkcs12 -export -legacy -name "$NAME" -inkey "$WORK/key.pem" -in "$WORK/cert.pem" \
  -out "$P12" -passout "pass:$P12_PASSWORD" 2>/dev/null \
  || openssl pkcs12 -export -name "$NAME" -inkey "$WORK/key.pem" -in "$WORK/cert.pem" \
       -out "$P12" -passout "pass:$P12_PASSWORD"

npx tauri signer generate --ci -p "$UPDATER_PASSWORD" -w "$UPDATER_KEY" >/dev/null

cat > "$SECRETS" <<EOF
# Baddel signing secrets. Never commit, never share. Sourced by scripts/signing/signing-env.sh.
BADDEL_SIGN_NAME='$NAME'
BADDEL_P12_PASSWORD='$P12_PASSWORD'
BADDEL_UPDATER_PASSWORD='$UPDATER_PASSWORD'
EOF
chmod 600 "$P12" "$UPDATER_KEY" "$UPDATER_KEY.pub" "$SECRETS"

echo "created in $DIR:"
ls -1 "$DIR"
echo
echo "certificate SHA-1 (pinned by the app's designated requirement):"
openssl x509 -in "$WORK/cert.pem" -noout -fingerprint -sha1 | sed 's/.*=//; s/://g'
echo
echo "updater public key (goes in src-tauri/tauri.conf.json → plugins.updater.pubkey):"
cat "$UPDATER_KEY.pub"; echo
echo
echo "⚠️  Back up $DIR now. It is the only copy."
