#!/usr/bin/env bash
# Prépare de quoi tester l'installeur macOS sans la clé des releases (voir README.md) :
#   - release/ : deux fausses dylibs universelles et leur manifest-macos.json, version 9.9.9, signé
#     avec la clé de test du dépôt ;
#   - faux_jeu/ : un faux Civ6.app, que l'injection doit atteindre ;
#   - essai.sh, à lancer ensuite.
# L'installeur de test (BGE_TEST=1 bash updater/macos/bundle.sh) est à placer à côté : le workflow ci
# assemble le tout dans l'artefact BetterGameExperience-macOS-test.
#
#   bash updater/macos/test/fausse_release.sh [dossier]     # par défaut updater/target/macos-test
#
# Demande clang (outils en ligne de commande de Xcode) et Python avec cryptography.
set -euo pipefail

ici="$(cd "$(dirname "$0")" && pwd)"
racine="$(cd "$ici/../../.." && pwd)"
sortie="${1:-$racine/updater/target/macos-test}"
python="${PYTHON:-python3}"
mkdir -p "$sortie"
sortie="$(cd "$sortie" && pwd)"
release="$sortie/release"
rm -rf "$release" "$sortie/faux_jeu"
mkdir -p "$release"

dylib() {
    clang -arch arm64 -arch x86_64 -dynamiclib -O2 -DMARQUE="\"$2\"" -DNOM="\"$1\"" "$ici/dylib_test.c" -o "$release/$1"
}
# Les marques de proxy/src et startup_proxy/src, que l'installeur cherche (updater/src/install.rs).
dylib BGD_GameCore_XP2.dylib RolloverProxyStatus
dylib libBGE_Chargeur.dylib StartupProxyStatus

jeu="$sortie/faux_jeu/Sid Meier's Civilization VI/Civ6.app/Contents/MacOS"
mkdir -p "$jeu"
clang -arch arm64 -arch x86_64 -O2 "$ici/faux_jeu.c" -o "$jeu/Civ6_Exe_Child"
echo "faux GameCore" > "$jeu/GameCore_XP2.dll"

"$python" "$racine/release/manifest.py" build --platform macos --version 9.9.9 --out "$release/manifest-macos.json" "$release" > /dev/null
BGD_SIGNING_KEY="$(cat "$ici/cle_privee_de_test.txt")" \
    "$python" "$racine/release/manifest.py" sign --public-key "$ici/public_key.txt" "$release/manifest-macos.json"
"$python" "$racine/release/manifest.py" verify --public-key "$ici/public_key.txt" "$release/manifest-macos.json"

cp "$ici/essai.sh" "$sortie/essai.sh"
echo "Prêt : $sortie"
