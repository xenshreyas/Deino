#!/usr/bin/env bash
# Creates "Deino Local Signing", a self-signed code-signing certificate in the
# login keychain, once. scripts/build-app.sh signs with it so the app keeps the
# same identity across rebuilds and macOS keeps its permissions. Nothing is
# trusted system-wide and no password is needed beyond the keychain prompt.
set -euo pipefail
readonly NAME="Deino Local Signing"
if security find-certificate -c "$NAME" >/dev/null 2>&1; then
    echo "\"$NAME\" already exists."
    exit 0
fi
dir="$(mktemp -d)"
trap 'rm -rf "$dir"' EXIT
openssl req -x509 -newkey rsa:2048 -nodes -days 3650 -subj "/CN=$NAME" \
    -keyout "$dir/key.pem" -out "$dir/cert.pem" \
    -addext "keyUsage=critical,digitalSignature" \
    -addext "extendedKeyUsage=critical,codeSigning" 2>/dev/null
openssl pkcs12 -export -legacy -inkey "$dir/key.pem" -in "$dir/cert.pem" \
    -out "$dir/id.p12" -passout pass:deino 2>/dev/null \
    || openssl pkcs12 -export -inkey "$dir/key.pem" -in "$dir/cert.pem" \
        -out "$dir/id.p12" -passout pass:deino
security import "$dir/id.p12" -k "$HOME/Library/Keychains/login.keychain-db" \
    -P deino -T /usr/bin/codesign
echo "Created \"$NAME\". Rebuild with scripts/build-app.sh --install, then grant"
echo "Accessibility to Deino once more; it will stick from then on."
