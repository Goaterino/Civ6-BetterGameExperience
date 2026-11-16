#!/usr/bin/env bash
# Installation de test de BetterGameExperience sous macOS, de zéro (voir updater/macos/test/README.md).
#
#   bash essai.sh
#
# À lancer dans le dossier de l'artefact BetterGameExperience-macOS-test (ou updater/target/macos-test),
# qui contient l'installeur de test (…_TEST_macOS.zip), release/ et faux_jeu/. Tout se passe dans
# essai/, à côté : rien n'est écrit dans ~/Library ni dans la configuration de Steam.
#   1. extrait l'installeur, comme un téléchargement ;
#   2. l'ouvre sur la fausse release, avec un faux Steam : des copies de tes localconfig.vdf, et le faux
#      jeu dans sa bibliothèque. Clique Installer ; si ton Steam est ouvert, Redémarrer Steam le ferme
#      puis le rouvre (les écritures vont dans les copies) ;
#   3. une fois la fenêtre fermée, vérifie les dylibs posées et l'option de lancement écrite ;
#   4. lance le faux jeu par le lanceur installé, comme Steam (%command%), et montre l'injection.
set -euo pipefail

ici="$(cd "$(dirname "$0")" && pwd)"
zip="$(ls "$ici"/BetterGameExperience_*_TEST_macOS.zip 2>/dev/null | head -1)"
[ -n "$zip" ] || { echo "Installeur de test absent : $ici/BetterGameExperience_*_TEST_macOS.zip"; exit 1; }
[ -f "$ici/release/manifest-macos.json.sig" ] || { echo "Fausse release absente : $ici/release"; exit 1; }

# Téléchargé depuis GitHub, l'artefact est en quarantaine : ses binaires de test (faux jeu, dylibs)
# seraient bloqués. Ce sont les nôtres, construits par la CI.
xattr -dr com.apple.quarantine "$ici" 2>/dev/null || true
# Les artefacts de GitHub perdent les droits d'exécution.
chmod +x "$ici/faux_jeu/Sid Meier's Civilization VI/Civ6.app/Contents/MacOS/Civ6_Exe_Child"

essai="$ici/essai"
rm -rf "$essai"
mkdir -p "$essai/Telechargements"
ditto -x -k "$zip" "$essai/Telechargements"

maison="$essai/maison"
steam="$maison/Library/Application Support/Steam"
mkdir -p "$steam/steamapps/common"
ln -s "$ici/faux_jeu/Sid Meier's Civilization VI" "$steam/steamapps/common/Sid Meier's Civilization VI"
comptes=0
for vdf in "$HOME/Library/Application Support/Steam/userdata"/*/config/localconfig.vdf; do
    [ -f "$vdf" ] || continue
    compte="$(basename "$(dirname "$(dirname "$vdf")")")"
    mkdir -p "$steam/userdata/$compte/config"
    cp "$vdf" "$steam/userdata/$compte/config/localconfig.vdf"
    comptes=$((comptes + 1))
done
if [ "$comptes" = 0 ]; then
    # Pas de Steam sur ce Mac : un compte au fichier minimal.
    mkdir -p "$steam/userdata/1/config"
    printf '"UserLocalConfigStore"\n{\n\t"Software"\n\t{\n\t\t"Valve"\n\t\t{\n\t\t\t"Steam"\n\t\t\t{\n\t\t\t\t"apps"\n\t\t\t\t{\n\t\t\t\t}\n\t\t\t}\n\t\t}\n\t}\n}\n' \
        > "$steam/userdata/1/config/localconfig.vdf"
    comptes=1
fi
echo "Faux Steam : $comptes compte(s) copié(s) dans $steam/userdata"

export HOME="$maison" BGE_DATA_DIR="$essai/donnees" BGD_DIST="$ici/release"
echo
echo "1. L'installeur de test s'ouvre : clique Installer, puis ferme la dernière fenêtre."
"$essai/Telechargements/BetterGameExperience.app/Contents/MacOS/BetterGameExperience" || true

echo
echo "2. Fichiers posés dans $BGE_DATA_DIR :"
ok=1
for f in BGD_GameCore_XP2.dylib libBGE_Chargeur.dylib; do
    attendu="$(grep -A3 "\"$f\"" "$ici/release/manifest-macos.json" | sed -n 's/.*"sha256": "\(.*\)".*/\1/p')"
    if [ -f "$BGE_DATA_DIR/dylib/$f" ] && [ "$(shasum -a 256 "$BGE_DATA_DIR/dylib/$f" | cut -d' ' -f1)" = "$attendu" ]; then
        echo "   ok  dylib/$f (SHA-256 du manifeste)"
    else
        echo "   KO  dylib/$f absente ou différente"; ok=0
    fi
done
[ -d "$BGE_DATA_DIR/BetterGameExperience.app" ] && echo "   ok  BetterGameExperience.app (copie du lanceur)" || { echo "   KO  copie du lanceur absente"; ok=0; }

echo
echo "3. Option de lancement de Civilization VI dans les copies de configuration Steam :"
for vdf in "$steam"/userdata/*/config/localconfig.vdf; do
    ligne="$(grep '"LaunchOptions".*BetterGameExperience' "$vdf" | head -1 | sed 's/^[[:space:]]*//')"
    compte="$(basename "$(dirname "$(dirname "$vdf")")")"
    if [ -n "$ligne" ]; then echo "   ok  compte $compte : $ligne"; else echo "   --  compte $compte : pas d'option (Steam non configuré)"; ok=0; fi
done

echo
echo "4. Lancement du faux jeu par le lanceur installé, comme Steam :"
rm -f "$BGE_DATA_DIR/injection.log"
"$BGE_DATA_DIR/BetterGameExperience.app/Contents/MacOS/BetterGameExperience" \
    "$ici/faux_jeu/Sid Meier's Civilization VI/Civ6.app/Contents/MacOS/Civ6_Exe_Child" -argument_de_steam || true
if [ -s "$BGE_DATA_DIR/injection.log" ]; then
    sed 's/^/   ok  /' "$BGE_DATA_DIR/injection.log"
else
    echo "   KO  rien n'a été injecté"; ok=0
fi

echo
if [ "$ok" = 1 ]; then echo "Installation de test réussie."; else echo "Installation de test incomplète : voir les KO ci-dessus."; exit 1; fi
