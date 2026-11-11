#!/bin/bash
# Essai de patch sous hardened runtime (README.md) : compile l'hôte, le bundle et la dylib injectée
# pour une architecture, signe l'hôte de cinq façons, lance chaque variante avec
# DYLD_INSERT_LIBRARIES et rassemble les lignes RESULTAT en tableau.
#
# Usage, sur un Mac avec les outils en ligne de commande de Xcode :
#   analyses/macos/essai_patch/essai.sh [arm64|x86_64]
# Sans argument : l'architecture de la machine. x86_64 sur Apple Silicon passe par Rosetta.
#
# Code de sortie : 1 si l'essai lui-même n'a pas tourné (compilation, signature, injection absente
# de la variante `jeu`), 0 sinon. Les verdicts ne font pas échouer le script : c'est une mesure.

set -u
ici="$(cd "$(dirname "$0")" && pwd)"
arch="${1:-$(uname -m)}"
case "$arch" in
  arm64) min=11.0 ;;   # comme la tranche arm64 du jeu
  x86_64) min=10.14 ;; # comme sa tranche x86_64
  *) echo "architecture inconnue : $arch" >&2; exit 1 ;;
esac

out="$(mktemp -d "${TMPDIR:-/tmp}/bge_essai_${arch}.XXXXXX")"
echo "Dossier de travail : $out"
sw_vers 2>/dev/null
echo "Machine : $(uname -m), essai en $arch"
# SIP coupé, le système n'applique pas toutes les règles de signature : les verdicts ne valent
# alors pas pour le Mac d'un joueur. La variante `strict` le confirme (injection acceptée).
sip="$(csrutil status 2>&1 | head -1)"
echo "SIP : $sip"
echo "boot-args : $(nvram boot-args 2>&1)"

# x86_64 sur Apple Silicon : Rosetta, à installer s'il manque.
if [ "$arch" = x86_64 ] && [ "$(uname -m)" = arm64 ] && ! /usr/bin/pgrep -q oahd; then
  echo "Installation de Rosetta"
  softwareupdate --install-rosetta --agree-to-license || echo "Rosetta non installé"
fi

cc=(xcrun clang -arch "$arch" "-mmacosx-version-min=$min" -O1 -Wall "-I$ici")

# Le jeu est lié à l'ancienne : imports paresseux (__la_symbol_ptr), pas de « chained fixups ».
"${cc[@]}" -Wl,-no_fixup_chains -o "$out/hote" "$ici/hote.c" || "${cc[@]}" -o "$out/hote" "$ici/hote.c" || exit 1
"${cc[@]}" -bundle -o "$out/GameCore_XP2.dll" "$ici/gamecore.c" || exit 1
"${cc[@]}" -dynamiclib -o "$out/bge.dylib" "$ici/injecte.c" || exit 1

# Bundle et dylib : signature ad hoc sans hardened runtime, comme le GameCore du jeu (drapeaux 0).
codesign -f -s - "$out/GameCore_XP2.dll" "$out/bge.dylib" || exit 1

droits() { # droits <fichier> <clé>...
  local f="$1"; shift
  {
    echo '<?xml version="1.0" encoding="UTF-8"?>'
    echo '<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">'
    echo '<plist version="1.0"><dict>'
    for cle in "$@"; do echo "  <key>com.apple.security.cs.$cle</key><true/>"; done
    echo '</dict></plist>'
  } > "$f"
}
# Les trois droits de Civ6_Exe_Child, relevés dans sa signature.
droits "$out/jeu.plist" allow-dyld-environment-variables allow-jit disable-library-validation
droits "$out/sans_protection.plist" allow-dyld-environment-variables allow-jit disable-library-validation disable-executable-page-protection
droits "$out/memoire_libre.plist" allow-dyld-environment-variables allow-jit disable-library-validation allow-unsigned-executable-memory

# Variantes de l'hôte :
#   jeu             hardened runtime + les droits du jeu : le cas à trancher ;
#   nu              signature ad hoc de l'éditeur de liens, sans hardened runtime : témoin ;
#   strict          hardened runtime sans aucun droit : l'injection doit être refusée, ce qui
#                   prouve que l'essai applique bien les règles ;
#   sans_protection droits du jeu + disable-executable-page-protection ;
#   memoire_libre   droits du jeu + allow-unsigned-executable-memory.
for v in jeu nu strict sans_protection memoire_libre; do cp "$out/hote" "$out/hote_$v"; done
codesign -f -s - -o runtime --entitlements "$out/jeu.plist" "$out/hote_jeu" || exit 1
codesign -f -s - -o runtime "$out/hote_strict" || exit 1
codesign -f -s - -o runtime --entitlements "$out/sans_protection.plist" "$out/hote_sans_protection" || exit 1
codesign -f -s - -o runtime --entitlements "$out/memoire_libre.plist" "$out/hote_memoire_libre" || exit 1

etiquette="$arch"
[ "$arch" = x86_64 ] && [ "$(uname -m)" = arm64 ] && etiquette="x86_64 (Rosetta)"

for v in jeu nu strict sans_protection memoire_libre; do
  echo
  echo "=== variante $v"
  codesign -d --entitlements - -v "$out/hote_$v" 2>&1 | grep -E 'flags|security' || true
  # Bash fixe la variable pour ce seul processus : aucun binaire protégé par SIP ne s'intercale,
  # qui la retirerait.
  BGE_VARIANTE="$v" DYLD_INSERT_LIBRARIES="$out/bge.dylib" "$out/hote_$v" "$out/GameCore_XP2.dll" > "$out/$v.log" 2>&1
  code=$?
  cat "$out/$v.log"
  echo "code de sortie : $code"
  if ! grep -q '^INJECTION' "$out/$v.log"; then
    echo "RESULTAT|$v|$etiquette|injection par DYLD_INSERT_LIBRARIES|refusée (dylib non chargée)" >> "$out/$v.log"
  fi
  if [ $code -ne 0 ]; then
    echo "RESULTAT|$v|$etiquette|fin de l'hôte|code de sortie $code" >> "$out/$v.log"
  fi
done

# Tableau : dans le résumé du job sous GitHub Actions, sur la sortie sinon.
{
  echo
  echo "## Essai de patch, $etiquette, macOS $(sw_vers -productVersion 2>/dev/null)"
  echo
  echo "$sip"
  echo
  echo "| Variante | Essai | Verdict |"
  echo "|---|---|---|"
  for v in jeu nu strict sans_protection memoire_libre; do
    grep '^RESULTAT|' "$out/$v.log" | while IFS='|' read -r _ variante _ essai verdict; do
      echo "| $variante | $essai | $verdict |"
    done
  done
} | tee -a "${GITHUB_STEP_SUMMARY:-/dev/null}"

# L'essai n'a de valeur que si la variante `jeu` a bien été injectée.
if ! grep -q '^INJECTION' "$out/jeu.log"; then
  echo "::error::la dylib n'a pas été injectée dans la variante jeu" >&2
  exit 1
fi
exit 0
