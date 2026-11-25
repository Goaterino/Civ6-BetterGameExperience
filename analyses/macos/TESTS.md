# Portage macOS : tests à faire en partie

État au 2026-11-25 (branche `feat/macos-launcher`). Les tests unitaires et ceux qui lisent les
binaires installés passent sur les deux architectures (arm64, et x86_64 sous Rosetta) ; la CI
couvre Windows, Linux et macOS. Rien de ce qui suit n'est couvert par eux : il faut le jeu.

Lancement avec les dylibs d'un dossier `$V/dylib` (journal dans `$V/RolloverProxy.log`) :

```bash
G="$HOME/Library/Application Support/Steam/steamapps/common/Sid Meier's Civilization VI/Civ6.app/Contents/MacOS"
cd "$V" && SteamAppId=289070 SteamGameId=289070 DYLD_INSERT_LIBRARIES="$V/dylib/libBGE_Chargeur.dylib" BGE_DYLIBS="$V/dylib" "$G/Civ6_Exe"
```

Sous Rosetta (x86_64), la même commande précédée de `arch -x86_64`.

Légende : **S** = faisable seul (multijoueur seul, pour les préréglages) ; **2H** = demande un
second joueur humain ; **PC** = demande un PC Windows.

## 1. Pose (chaque architecture) — S

Au chargement d'une partie, le journal doit avoir une ligne par correctif, sans erreur :
citoyens, fin de tour, ports, production, empilement, spectateurs, congrès, départs, absents,
échange d'unités, vision après une guerre, unités promises, partie chargée, départ simultané ;
puis `pol : correctifs POL actifs`. Côté chargeur : `absent :`, `xml :`, les scripts du
congrès, de la partie et du départ `compilation réussie`, `debit steam : tampon de réception de
131072 octets…`, `reload :` sans erreur. Aucun rapport de crash
(`~/Library/Logs/DiagnosticReports/Civ6_Exe_Child-*.ips`).

- arm64 et x86_64 : tout doit passer. En x86_64, vérifier en plus les lignes « premier appel » du
  départ simultané : preuve que les sauts écrits sous Rosetta sont exécutés.
- La version annoncée au salon finit par `-macos-partiel`, sans `-t0`.

## 2. Correctifs, un par un (arm64 d'abord, puis x86_64)

| Correctif | Comment le provoquer | Ce qu'on doit voir | Qui |
|---|---|---|---|
| Fin de tour | finir son tour pendant qu'une diplomatie arrive | tour suivant normal (déjà vérifié arm64) | S |
| Ports | téléporter une unité depuis un quartier | téléportation permise (déjà vérifié arm64) | S |
| Production | poser puis retirer Ilkum | modificateur rendu exactement (déjà vérifié arm64) | S |
| Empilement | combat avec des unités empilées à côté | `empilement : unité … en excédent, ignorée` ; bonus de soutien et contournement sans l'excédent | S |
| Spectateurs | Better Spectator Mod, un spectateur | tourisme divisé sans lui ; ses votes de résolution **et de proposition** refusés (le second est corrigé depuis le dernier test) | S |
| Congrès | voter, finir son tour, revenir au vote | bouton « Revoter », `congrès : annulation …`, `revote … faveur rendue`, faveur débitée une seule fois | 2H |
| Citoyens | focus nourriture + production, ville qui grandit | `citoyens : choix corrigé (n)` quand le choix diffère du jeu | S |
| Échange d'unités | échanger deux unités voisines (surtout avec un chemin) | aucune unité laissée à mi-chemin ou empilée hors de sa case | S |
| Vision après une guerre | guerre puis paix avec une cité-État | la vision partagée reprend après la paix | S |
| Départs (drop_slot) | un joueur quitte la partie | sa place reste humaine, l'IA ne joue pas pour lui | 2H |
| Absents | après son départ, l'hôte finit son tour | le tour passe et ne se rouvre pas (BGE-4EB5B0) ; `Civ6Proxy_EndTurnFor` | 2H |
| Unités promises | partie avec IA jusqu'aux premiers quartiers, sauvegarder, recharger | pas de désynchro au rechargement ; journal du correctif | S puis 2H |
| Partie chargée | sauvegarder, recharger, continuer quelques tours | pas de désynchro après rechargement (améliorations, écouteurs, préférences d'IA) | S puis 2H |
| Gros morceaux réseau | deux Mac de cette version, partie en réseau | `morceaux de 16384 octets (1 joueur(s) équipé(s))`, retour à 1166 au départ d'un joueur | 2H (deux Mac) |
| Rechargement de tous | provoquer une désynchro (deux Mac) | l'hôte recharge tout le monde, lignes `reload:` et `rejoin:`, le tour reprend après le chargement de tous | 2H (deux Mac) |
| Départ simultané | option « No First/Last moves (BGE) » au salon | `depart : armé`, `interface retenue avant le début du tour`, ordres `retenu, tour pas encore lancé` avant le départ, minuteur gelé, `depart : parti (…)` ; signaux `.bgd_` cachés dans le chat | S (seul : départ immédiat), 2H (vrai test) |

## 3. Compatibilité PC / Mac — PC

Tant que le Mac annonce `<version>-macos-partiel` :
- dans le salon, le PC voit la version partielle et coupe les correctifs POL pour toute la partie ;
- la partie se joue sans désynchro, Mac hôte puis PC hôte ;
- ni gros morceaux réseau (16 Ko) ni rechargement de tous (reload_all) entre le PC et le Mac.

## 4. Avant de retirer `-macos-partiel`

Tout est porté (2026-11-25). Retirer le suffixe seulement après, avec deux Mac puis avec un PC
de la même version : partie complète avec POL actif, gros morceaux réseau, une désynchro
provoquée (rechargement de tous, Mac client puis Mac hôte), un joueur qui part, le départ
simultané à deux. Points les plus fragiles : la réécriture en ligne de la réception de
`reload` (`0x100c11038`), l'écriture des vtables de messages (`__DATA_CONST` en arm64), et les
détours Steam si le client Steam signait ses pointeurs (arm64e). Un hôte Mac recharge tout le
monde sans le message de chat explicatif (`sync_notes` non porté).

Limite connue : `maths_communes` aligne `exp`/`pow` de l'UCRT entre PC ; le Mac calcule avec sa
propre bibliothèque. Une partie PC + Mac avec POL actif doit donc être surveillée pour des
désynchros sans autre cause.

## 5. Installation et Windows

- Lanceur macOS (`updater/`, bundle de la CI) : installation des dylibs et option de lancement
  Steam, puis jeu lancé par Steam. — S
- Windows : `drop_slot`, `absent_reactivation` et `unites_promises` ont leur logique déplacée dans
  des `*_regle.rs` communs (tests Windows de la CI verts) ; une partie Windows de cette branche
  confirmerait qu'ils se comportent comme sur main. — PC
