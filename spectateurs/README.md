# Spectateurs de BSM : congrès mondial et tourisme

Analyse statique (2026-11-03) de `GameCore_XP2_FinalRelease.dll` (projet Ghidra `civ6xp2`) et de
Better Spectator Mod (Workshop 1916397407, v1.27), en vue de deux **correctifs POL** :
1. exclure les spectateurs du vote du congrès, sans les empêcher de consulter les résolutions ;
2. exclure les spectateurs du calcul du tourisme.

Le correctif est dans le proxy de GameCore (section [Le correctif](#le-correctif)).
Civ6CoreInjector n'aide pas ici : c'est un chargeur de GameCore qui reprend `coalesce` et `nuke`, sans
rien sur le congrès, le tourisme ni les spectateurs.

## Ce qu'est un spectateur

BSM (`Data/Spectator.lua`) ne crée pas de type de joueur : un spectateur est un **majeur humain
vivant** dont la configuration porte `LEADER_SPECTATOR` (civilisation `CIVILIZATION_SPECTATOR`). Au
lancement, BSM détruit ses unités, lui fait rencontrer tous les majeurs (`SetHasMet`) et lui donne
Code des lois. Il reste dans toutes les listes de majeurs de GameCore.

Reconnaître un spectateur dans GameCore, à partir de l'ID `p` (même schéma que `IsMajor`,
`1802f9860`) :

| adresse | rôle |
|---|---|
| `180164d60(DAT_180b8aa78, p)` | configuration du joueur `p` (`DAT_180b8aa78` : configuration de partie) |
| `18016a000(config)` | hachage de `LEADER_TYPE` (`180169cd0` : idem pour le niveau de civilisation) |

`LEADER_SPECTATOR` = `f4e7df50` (`~CRC32`, contrôlé sur `CIVILIZATION_LEVEL_FULL_CIV` = `253718b0`,
la constante de `1802f9860`). La configuration est la même sur toutes les machines : le test est
déterministe, donc utilisable dans la simulation. On peut aussi calculer un masque de 64 bits une fois
par partie, à condition de le recalculer au chargement et aux reconnexions.

## Listes de joueurs (`PlayerManager`, `180306b80()`)

Établies par les bindings Lua `PlayerManager.*` (`1806f4…`) :

| décalage | contenu |
|---|---|
| `+0x4a0` | ID des vivants |
| `+0xa20` | joueurs ayant vécu |
| `+0xb50` | majeurs vivants (pointeurs) |
| `+0xfa0` | ID des majeurs vivants |
| `+0x10d0` | majeurs ayant vécu (pointeurs) |
| `+0x1650`, `+0x1aa0` | mineurs vivants, leurs ID |
| `+0x25a0` | barbares vivants |
| `+0x2b48[p]` | `-1` si l'emplacement est vide |
| `+0x20[p]` | objet joueur (`180044f00(p)` le rend aussi) |

Les spectateurs figurent dans `+0xb50`, `+0xfa0` et `+0x10d0`.

## Congrès : où un spectateur pèse

### 1. Ses votes sont acceptés

`1803c0ba0(congrès, joueur, hachage de résolution, votes)`, `WORLD_CONGRESS_RESOLUTION_VOTE` (appelé
par `18030d330` pour les humains, par `180419a10` pour l'IA) :
- retrouve la résolution dans `m_Resolutions` (`+0x108`) ;
- contrôle que le joueur existe (`+0x2b48[p] != -1`, `+0x20[p] != 0`) ;
- borne les votes par la faveur (`180371500` : **toujours au moins 1**, même à 0 de faveur) ;
- exige le segment `WORLDCONGRESS_1` (`1803cf8f0`) ;
- range les votes (`1803d0a40` : `résolution+0x104[p]`, entier signé, le signe donne A ou B) ;
- débite la faveur (`18026c410(congrès du joueur, -coût, 1)`) et journalise (`1803cf9d0`).

`1803c0d40` : même chose pour les propositions et urgences (`WORLD_CONGRESS_DISCUSSION_VOTE`, par
vtable de la discussion, débit de faveur de type 2).

Le choix A/B et la cible arrivent par les setters de `18030d330` (`18030a100`…), pas par ces deux
fonctions.

### 2. Une voix d'office au dépouillement

`1803c5f50(résolution)`, appelée pour chaque résolution par la fin de session `1803bf880` : **pour
chaque ID de `+0xfa0` (majeurs vivants) qui n'a pas voté** (`résolution+0x104[p] == 0`), elle pose
1 voix et le choix 1 (`résolution+4[p] = 1`), puis appelle la vtable de cible (`+0xb8`). Un spectateur
qui ne vote pas donne donc quand même une voix au choix A. Refuser ses votes dans `1803c0ba0` ne
suffit pas.

Plus loin, `1803c5f50` départage les égalités par la faveur des votants (`congrès du joueur+0xc0`).
Un spectateur sans voix n'y entre plus.

### 3. Rétribution après la résolution

`1803bff40(congrès, résolution)`, pour chaque ID de `+0xfa0` :
- s'il a voté pour le choix gagnant : bonus de faveur selon l'agenda (`18026cfb0`), et une entrée
  `LOC_DVP_TOOLTIP_AGREEING_RESOLUTIONS` sur son objet de victoire diplomatique (`+0x710`) ;
- sinon : remboursement d'une partie du coût (`18026c410(…, coût, 9)`, la moitié si `1803c7d20`).

Avec 0 voix et le choix 0, le spectateur ne gagne rien, à condition que la voix d'office (point 2)
lui soit retirée.

### 4. Cible de résolutions

`DLC/Expansion2/Scripts/WorldCongress.lua` : les cibles « joueur » de `WC_Validate_BorderControl`,
`MigrationTreaty`, `ArmsControlTreaty`, `TradeTreaty` et `DiploVictory` sont tirées de
`PlayerManager.GetAliveMajors()`. On peut donc désigner un spectateur, par exemple pour la victoire
diplomatique. C'est du Lua de gameplay, exécuté partout : à filtrer avec le correctif (POL).

À vérifier : le dépouillement des propositions et urgences (`1803bb830`, `1803bfe50`, vtables)
accorde-t-il aussi une voix d'office, et qui compte parmi les membres d'une urgence ? Les votes d'un
spectateur y sont déjà refusés (`1803c0d40`).

## Tourisme

### Les données (`Player_Culture`, objet joueur `+0x6f0`)

| décalage | contenu |
|---|---|
| `+0x490[p]` | tourisme cumulé vers le joueur `p` (virgule fixe 24.8), `p < 22` |
| `+0x598` | culture cumulée (24.8) |
| `+0x5e8` (données en `+0x5f8`) `[p]` | touristes venus de `p` (`GetTouristsFrom`, `1800b3390`) |
| `+0x5a0` / `+0x5b0` | domestiques restants (« staycationers ») |
| `+0x648` | touristes par case (`GetTouristsAt`, `18027ff30`) |

Paramètres (`DAT_180b8aa70`) : `+0x6d0` = `TOURISM_CULTURE_PER_CITIZEN` (100), `+0x6e0` =
`TOURISM_TOURISM_TO_MOVE_CITIZEN` (200, Gathering Storm).

### La formule

`18027ce10(culture de A, joueur P)` : touristes que A prend à P.

```
N        = nombre de majeurs ayant vécu (PlayerManager+0x10d0)
dom(P)   = (culture cumulée de P) / 100
t(A, P)  = (tourisme cumulé de A vers P / 200) / N
demande  = somme de t(X, P) pour X majeur vivant, X ≠ P, qui a rencontré P   (18027e960)
si dom(P) < demande :  t(A, P) = t(A, P) * dom(P) / demande
```

Divisions entières, dans cet ordre. `1802a6be0` est le test « a rencontré » (diplomatie,
objet joueur `+0x6e0`).

- `180282cb0(culture de A, P)` range `t(A, P)` dans `+0x5e8[P]` et retire l'écart aux domestiques de
  P. Appelée par `18027a0e0`, `18037cee0`, `18074eac0` (bombe de tourisme), `18087bb70`.
- `1802805b0` : `GetTouristsTo`, somme des `+0x5f8[X]` sur les majeurs vivants (`+0xb50`).
- `18027d150` : répartition des touristes sur les cases.
- Victoire culturelle : `REQUIREMENT_TOURISTS_EXCEED_STAYCATIONERS` (`Victories.xml`), enregistré en
  `1808806e0`. L'évaluation passe par une vtable (`180a8a560` ou `180a929a0`), non décompilée.

### Où un spectateur pèse

1. **Diviseur `N`** : chaque spectateur compte parmi les majeurs ayant vécu. À 6 joueurs et
   2 spectateurs, `N` vaut 8 au lieu de 6 : chacun prend 25 % de touristes en moins, et la victoire
   culturelle recule d'autant. C'est l'effet principal.
2. **Spectateur visé ou visiteur** : sans ville ni culture, il n'a ni touristes domestiques à céder
   (seuls les domestiques de P peuvent partir, d'où le plafond), ni tourisme à exercer. Rien à
   corriger.
3. **Victoire culturelle** : face à un spectateur, la condition (plus de touristes que ses domestiques)
   est remplie d'office, puisqu'il en a 0. Pas de blocage ; l'évaluation n'est pas décompilée.

Seul `N` change donc. L'infobulle des touristes (`1800b33c0`) et `CultureAi` ne lisent pas
`+0x10d0` : seules `18027ce10` et `18027e960` le font dans `Player_Culture`.

## Le correctif

Proxy de GameCore, [proxy/src/spectateurs.rs](../proxy/src/spectateurs.rs), et deux blocs Lua posés
par l'accroche `luaL_loadbuffer` (`proxy/src/ui.rs`). **Correctif POL, sans réglage** : tout ne
s'applique que si `BgdPolActifs` rend 1, sinon le jeu d'origine partout, scripts compris.

| site | prologue recopié | rôle |
|---|---|---|
| `18027ce10` | 7 octets, puis `mov rax,[rip+…]` refait en `movabs rax,[GlobalParameters]` | touristes que A prend à P, réécrite avec `N` sans spectateurs |
| `18027e960` | 16 octets | demande sur P, réécrite avec le même `N` |
| `1803c0ba0` | 15 octets | vote sur une résolution : 0 pour un spectateur |
| `1803c0d40` | 15 octets | vote sur une proposition : 0 pour un spectateur |
| `1803c5f50` | 15 octets | dépouillement encadré : spectateur à 1 voix et choix 0 pendant l'appel (ni voix d'office, ni décompte), puis 0 et 0 (rien à la rétribution) |

Les deux formules sont celles de l'assembleur, divisions et multiplication 32 bits comprises : sans
spectateur, le résultat est celui du jeu. Le choix A/B d'un spectateur peut être rangé par
`18030d330` même quand son vote est refusé : l'encadrement du dépouillement l'efface.

**Scripts**, qui lisent `Civ6RolloverProxy_Pol()` (posée par le proxy dans l'état Lua qui les
compile, 1 si les détours sont posés et les correctifs POL actifs) :
- `WorldCongress.lua` (gameplay, exécuté partout) précédé de
  [ui_congres_cibles.lua](../proxy/src/ui_congres_cibles.lua) : les cinq validateurs à cible joueur
  appellent `BGD_MajorsCibles()`, les majeurs vivants sans les spectateurs ;
- `WorldCongressPopup.lua` suivi de [ui_congres.lua](../proxy/src/ui_congres.lua), pour un
  spectateur seulement :
  - son tour du congrès est soumis (`WORLD_CONGRESS_SUBMIT_TURN` puis fin de tour) dès qu'il est
    actif, une fois par segment : la session ne l'attend plus ;
  - la fenêtre reste ouverte à la fin de son tour pendant le vote, et le bouton du congrès la
    rouvre (lecture seule) au lieu de l'écran d'attente ;
  - boutons de vote sans effet, navigation libre jusqu'au récapitulatif, bouton « Fermer », mention
    « consultation seule » sous la description ;
  - ses données de vote sont remises à zéro au début de chaque segment de vote, comme la fin de tour
    le fait pour un votant.

**Le journal** (`RolloverProxy.log`) : `spectateurs : détours posés (tourisme, votes, dépouillement)`,
`spectateurs : tourisme divisé par N' majeurs au lieu de N (k spectateurs)` quand le compte change,
les 20 premiers votes refusés et dépouillements corrigés de chaque partie, `ui : Civ6RolloverProxy_Pol
posée`, et une ligne par script patché.

**Tests** :

```bash
cd proxy && cargo test --release -- spectateurs congres
```

```bash
lua proxy/tests/test_congres.lua
```

`check_hooks.py` vérifie la pose et la restauration des cinq détours sur la vraie DLL. Rien n'a
encore été essayé en partie.

## Le niveau de civilisation

Les correctifs ci-dessus laissent le spectateur classé majeur : il figure dans les listes de majeurs,
et notamment dans celles que lit le Lua (`PlayerManager.GetAliveMajorIDs`). BSM le détecte ainsi.

Le getter `180169cd0` rend le niveau d'un joueur. Pour un spectateur, le proxy rend
`CIVILIZATION_LEVEL_SPECTATOR` (`d67f2ab1`, le niveau défini par BSM) au lieu de `FULL_CIV` : il sort
des listes de majeurs, des conditions de victoire et du score.

Deux points qui ne se déduisent pas :

- l'objet reçu par ce getter **n'est pas le CvPlayer** : c'est un wrapper de `0x20` octets où l'ID du
  joueur est à `+0x00`. À `+0xD8` on lit un pointeur partagé, identique pour tous les joueurs ;
- la pose vole **14 octets** : `sub rsp,28h`, `mov rcx,[rcx+10h]` et `mov edx,[rip+0A2393Eh]`. La
  troisième est relative à RIP : copiée telle quelle dans le trampoline, elle lirait ailleurs. Le
  trampoline la refait en absolu (`mov r11,imm64` / `mov edx,[r11]`).

Les octets du niveau sont vérifiés à part : s'ils manquent, les cinq détours ci-dessus restent posés et
le journal le dit. S'il manque ceux des cinq autres, le niveau n'est pas posé non plus (il lit la même
base). Comme eux, il est retiré au déchargement du proxy.

### Verrouillé derrière POL

Le niveau change la simulation, et il ne marche qu'avec le `Data/Spectator.lua` du proxy : celui de
BSM ne cherche les spectateurs que parmi les majeurs, il ne les trouverait plus. Les deux sont un seul
correctif POL, qui ne dépend que d'états communs à toutes les machines de la partie :

- `Data/Spectator.lua` (script de gameplay) n'est substitué que si les correctifs POL sont actifs à
  sa compilation. Ils sont fixés par le verdict de l'hôte dans le salon, avant le chargement, y
  compris pour qui rejoint en cours de partie ; aucun réglage local (`ui_spectateur`,
  `ui_spectateur_fichiers`) n'y touche ;
- le détour ne rend `CIVILIZATION_LEVEL_SPECTATOR` que si ce script a été compilé dans la partie
  (drapeau remis à zéro à chaque `DllCreateGameContext`), si les correctifs POL sont actifs, et si le
  leader du joueur est `LEADER_SPECTATOR`. Le journal note le premier joueur ainsi reclassé.

Les scripts d'interface repris (`CityStates`, ruban, icône, popup du Congrès) sont écrits pour des
spectateurs hors des majeurs : ils ne sont substitués qu'avec les correctifs POL eux aussi. Sans POL,
une partie avec BSM se comporte comme avant cette reprise. `ui_spectateur=off` ne coupe que ces
scripts d'interface, sur la seule machine qui le règle.

Sur le popup du Congrès de BSM, le proxy ajoute le retour au vote (`ui_congres_vote.lua`), comme au
popup du jeu, mais pas la lecture seule (`ui_congres.lua`) : ce popup gère déjà le spectateur.

## Refaire

Décompilations : `analyzeHeadless ~/Tools/ghidra-projects civ6xp2 -process
GameCore_XP2_FinalRelease.dll -noanalysis -readOnly -postScript DumpAddrs.java <dossier> <adresses…>`.
Les décalages viennent de `ScanDisp.java` (`5f8`, `10d0`) et `CallersOf.java`.
