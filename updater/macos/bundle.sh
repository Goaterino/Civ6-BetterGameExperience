#!/usr/bin/env bash
# Construit BetterGameExperience.app, le lanceur macOS (voir updater/README.md, « macOS ») :
# binaire universel (arm64 et x86_64, comme le jeu), Info.plist, signature ad hoc (obligatoire en
# arm64), puis une archive zip qui garde la structure du bundle.
#
#   bash updater/macos/bundle.sh                 # version 0.0.0, dans updater/target/macos
#   BGD_RELEASE=0.12.0 bash updater/macos/bundle.sh
#   BGE_TEST=1 bash updater/macos/bundle.sh      # installeur de test, dans updater/target/macos-test
#
# L'installeur de test (feature cle-de-test) ne fait confiance qu'à la clé de test du dépôt
# (updater/macos/test) : à ne jamais distribuer. Voir updater/macos/test/README.md.
#
# Demande les cibles Rust aarch64-apple-darwin et x86_64-apple-darwin (rustup target add …) et les
# outils en ligne de commande de Xcode (lipo, codesign, ditto).
set -euo pipefail

ici="$(cd "$(dirname "$0")" && pwd)"
updater="$(dirname "$ici")"
version="${BGD_RELEASE:-0.0.0}"
if [ -n "${BGE_TEST:-}" ]; then
    sortie="$updater/target/macos-test"
    features=(--features cle-de-test)
    suffixe=_TEST
else
    sortie="$updater/target/macos"
    features=()
    suffixe=
fi
app="$sortie/BetterGameExperience.app"

cd "$updater"
for cible in aarch64-apple-darwin x86_64-apple-darwin; do
    cargo build --release --locked --target "$cible" ${features[@]+"${features[@]}"}
done

rm -rf "$app"
mkdir -p "$app/Contents/MacOS"
lipo -create -output "$app/Contents/MacOS/BetterGameExperience" \
    target/aarch64-apple-darwin/release/BetterGameExperience \
    target/x86_64-apple-darwin/release/BetterGameExperience
sed "s/__VERSION__/$version/g" "$ici/Info.plist" > "$app/Contents/Info.plist"
printf 'APPL????' > "$app/Contents/PkgInfo"

# Ad hoc : pas de certificat Developer ID. Gatekeeper demande alors, au premier lancement d'une
# archive téléchargée, de passer par clic droit > Ouvrir (voir README).
codesign --force --sign - --timestamp=none "$app"
codesign --verify --strict --verbose=2 "$app"
lipo -info "$app/Contents/MacOS/BetterGameExperience"

nom="BetterGameExperience_V${version}${suffixe}_macOS.zip"
rm -f "$sortie/$nom"
(cd "$sortie" && ditto -c -k --keepParent BetterGameExperience.app "$nom")
echo "$sortie/$nom"
