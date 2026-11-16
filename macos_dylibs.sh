#!/usr/bin/env bash
# Construit les deux dylibs macOS de BetterGameExperience, universelles (arm64 et x86_64, comme le
# jeu) et signées ad hoc (obligatoire en arm64) : voir analyses/macos/README.md.
#   libBGE_Chargeur.dylib    startup_proxy/ (src/macos.rs), injecté dans le jeu
#   BGD_GameCore_XP2.dylib   proxy/ (src/macos.rs), ouvert à la place de GameCore_XP2.dll
#
#   bash macos_dylibs.sh [dossier]        # par défaut target/macos
#   BGD_RELEASE=0.12.0 bash macos_dylibs.sh dist
#
# Sans diagnostic, comme la release (--no-default-features).
set -euo pipefail

racine="$(cd "$(dirname "$0")" && pwd)"
sortie="${1:-$racine/target/macos}"
mkdir -p "$sortie"

construire() { # construire <crate> <bibliothèque produite> <nom posé>
    local crate="$1" lib="$2" nom="$3" tranches=()
    for cible in aarch64-apple-darwin x86_64-apple-darwin; do
        (cd "$racine/$crate" && cargo build --release --locked --no-default-features --target "$cible")
        tranches+=("$racine/$crate/target/$cible/release/$lib")
    done
    lipo -create -output "$sortie/$nom" "${tranches[@]}"
    install_name_tool -id "@rpath/$nom" "$sortie/$nom"
    codesign --force --sign - --timestamp=none "$sortie/$nom"
    lipo -info "$sortie/$nom"
}
construire startup_proxy libwinmm.dylib libBGE_Chargeur.dylib
construire proxy libBGD_GameCore_XP2.dylib BGD_GameCore_XP2.dylib
