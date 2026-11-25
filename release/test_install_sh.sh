#!/bin/sh
# Tests de la vérification de signature d'install.sh (`sh install.sh verify <fichier>`), lancés depuis
# la racine du dépôt : sh release/test_install_sh.sh. Il faut openssl 3 et python avec cryptography
# ($PYTHON, sinon python3).
#
# 1. La clé publique embarquée par install.sh est celle de release/public_key.txt.
# 2. Avec une clé de test substituée dans une copie d'install.sh : un fichier signé passe ; un fichier
#    modifié, une signature d'une autre clé et une signature absente sont refusés.
set -eu

echec=0
ok() { printf 'ok   %s\n' "$1"; }
ko() { printf 'FAIL %s\n' "$1"; echec=1; }

# 1. Clé embarquée.
attendue=$(${PYTHON:-python3} release/manifest.py public-pem | tr -d '\r')
embarquee=$(sed -n '/^PUBLIC_KEY_PEM="/,/-----END PUBLIC KEY-----"/p' install.sh | sed 's/^PUBLIC_KEY_PEM="//; s/"$//')
if [ "$attendue" = "$embarquee" ]; then ok "clé d'install.sh = release/public_key.txt"; else ko "clé d'install.sh différente de release/public_key.txt"; fi

# 2. Vérification, avec une clé de test.
tmp=$(mktemp -d)
trap 'rm -rf "$tmp"' EXIT
openssl genpkey -algorithm ed25519 -out "$tmp/test.key" 2>/dev/null
openssl pkey -in "$tmp/test.key" -pubout -out "$tmp/test.pub"
openssl genpkey -algorithm ed25519 -out "$tmp/autre.key" 2>/dev/null

# Copie d'install.sh dont le bloc PEM est remplacé par la clé de test.
awk -v pem="$(cat "$tmp/test.pub")" '
    /^PUBLIC_KEY_PEM="/ { print "PUBLIC_KEY_PEM=\"" pem "\""; saute = 1; next }
    saute && /-----END PUBLIC KEY-----"/ { saute = 0; next }
    !saute { print }
' install.sh >"$tmp/install.sh"

signer() { # signer <clé> <fichier> : <fichier>.sig en base64, comme release/manifest.py sign-file
    openssl pkeyutl -sign -inkey "$1" -rawin -in "$2" -out "$2.bin"
    openssl base64 -A -in "$2.bin" -out "$2.sig"
    rm "$2.bin"
}

printf 'binaire de test\n' >"$tmp/bin"
signer "$tmp/test.key" "$tmp/bin"
if sh "$tmp/install.sh" verify "$tmp/bin" >/dev/null 2>&1; then ok "fichier signé accepté"; else ko "fichier signé refusé"; fi

cp "$tmp/bin.sig" "$tmp/modifie.sig"
printf 'binaire de test modifié\n' >"$tmp/modifie"
if sh "$tmp/install.sh" verify "$tmp/modifie" >/dev/null 2>&1; then ko "fichier modifié accepté"; else ok "fichier modifié refusé"; fi

printf 'binaire de test\n' >"$tmp/autre"
signer "$tmp/autre.key" "$tmp/autre"
if sh "$tmp/install.sh" verify "$tmp/autre" >/dev/null 2>&1; then ko "signature d'une autre clé acceptée"; else ok "signature d'une autre clé refusée"; fi

printf 'binaire de test\n' >"$tmp/sans"
if sh "$tmp/install.sh" verify "$tmp/sans" >/dev/null 2>&1; then ko "fichier sans signature accepté"; else ok "fichier sans signature refusé"; fi

printf 'pas du base64 !\n' >"$tmp/bin.sig"
if sh "$tmp/install.sh" verify "$tmp/bin" >/dev/null 2>&1; then ko "signature illisible acceptée"; else ok "signature illisible refusée"; fi

exit $echec
