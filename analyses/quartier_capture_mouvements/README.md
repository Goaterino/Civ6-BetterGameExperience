# Zone de déplacement absente sur un quartier ennemi occupé

**Corrigé dans le proxy** (2026-11-06, interface seule, sans réglage) : `patch_selected_unit` dans [proxy/src/ui.rs](../../proxy/src/ui.rs), bloc [ui_mouvements.lua](../../proxy/src/ui_mouvements.lua) ajouté à `SelectedUnit.lua`. Confirmé en partie par l'utilisateur le 2026-11-06.

Analyse statique (2026-11-06), bug 6. Une unité posée sur un campement ennemi (PV à 0, occupé
après l'avoir vidé) n'affiche pas sa zone de déplacement : pas de bordure bleue autour des cases
atteignables. Analyse seule, rien n'est implémenté.

## En bref

- **Cause : GameCore**, pas l'interface ni un mod. Le parcours en largeur qui calcule les cases
  atteignables (`UnitManager.GetReachableMovement`) applique la règle « entrer dans un quartier
  ennemi qui a des points de vie arrête l'unité » **à la case de départ** aussi. L'unité y commence
  avec 0 mouvement, aucune case voisine n'est atteignable, la fonction Lua rend `nil` et
  `RealizeMoveRadius` ne dessine rien.
- La règle porte sur `Districts.HitPoints` (la valeur de la base, 100 ou 200), pas sur les PV
  actuels du quartier : PV à 0 ne change rien.
- **L'unité peut bouger** (très probable, lu dans le code, à confirmer en jeu) : le vrai
  déplacement passe par l'A* de `Formation_Movement`, qui n'a pas ce test. Le survol d'une case
  (chemin et nombre de tours) et le clic droit marchent.
- Correctif proposé : **interface seule** (aucune désynchro), bloc Lua ajouté à la fin de
  `SelectedUnit.lua`, qui refait la zone avec `UnitManager.GetMoveToPathEx` quand le jeu ne rend
  rien sur un quartier ennemi. Bug pur : pas de réglage.

## Quels scripts sont chargés

- `Base/Assets/UI/WorldView/SelectedUnit.lua` : `RealizeMoveRadius`, seul appelant de
  `GetReachableMovement` dans le jeu et les mods.
- `DLC/Expansion2/UI/Replacements/SelectedUnit_Expansion2.lua` (`ReplaceUIScript` du contexte
  `SelectedUnit`, `Expansion2.modinfo:468`) : fait `include("SelectedUnit")` et ne remplace que
  `RealizeGreatPersonLens`. `RealizeMoveRadius` reste celle du jeu de base.
- Aucun mod du Workshop ne remplace `SelectedUnit` : BBG, BBG Expanded, BBG BETA, BSM, MPH ne
  l'ont pas. MPH remplace `InGame.xml` mais garde
  `<LuaContext ID="SelectedUnit" FileName="WorldView/SelectedUnit"/>` (`3307282026/ui/ingame.xml:29`).
- BSM remplace `WorldInput` (`worldinput_spec.lua`), qui ne touche pas à la zone de déplacement.

## Côté interface : rien d'anormal

`SelectedUnit.lua:28` `RealizeMoveRadius(kUnit)` :

```lua
if not ( kUnit:GetMovesRemaining() > 0 ) then return; end          -- :35
...
if not kUnit:HasMovedIntoZOC() then
    kMovePlots = UnitManager.GetReachableMovement( kUnit );          -- :53
    kZOCPlots  = UnitManager.GetReachableZonesOfControl( kUnit, true );
...
if kMovePlots ~= nil and table.count( kMovePlots ) > 0 then          -- :101
    UILens.SetLayerHexesArea( m_MovementRange, eLocalPlayer, kMovePlots );
```

Aucun test sur le propriétaire de la case ni sur un quartier. Si `HasMovedIntoZOC` était vrai, la
zone se réduirait à la case de l'unité ; ce drapeau (`unité+3000 → +0x1b0`, `1803a4020`) concerne
l'entrée en ZOC pendant le tour, il ne reste pas vrai au tour suivant. Le vide vient de
`GetReachableMovement`.

## Côté GameCore

Projet Ghidra `civ6xp2`, `GameCore_XP2_FinalRelease.dll`.

### La fonction Lua rend `nil` sur une liste vide

`1806c2060` (`GetReachableMovement`, `Cache_Lua_IUnitManager`) : appelle `1800d0df0`
(`Cache_Unit_Manager.cpp`) puis ne pousse une table que si la liste a au moins un élément.
Sinon elle rend 0 valeur : `nil` en Lua. Même chose pour `GetReachableZonesOfControl` (`1806c25e0`
→ `1800d11f0`). Les deux lisent l'unité vivante de GameCore et lancent le parcours en largeur
`1801a6a80(formation, liste, 0, nbTours=1, 0x11)` (`Formation_Movement.cpp`, « Breadth First
search must be for 1 or 2 turns ONLY »).

### Le test de validité du parcours

`1801a6a80` installe `1801a6e00` comme test de validité d'un nœud (`+0x58`), `1801a6350` comme
copie du coût dans le nœud (`+0x68`). `1801a6e00(…, parent=param_2, nœud=param_3, …, données=param_6)` :

```c
FUN_1801a7150(...)                       // coût : +0x1d4 = mouvements restants à l'arrivée,
                                         // +0x1dc = tour (départ : parent nul → tour 1, PM pleins)
plot   = case du nœud
type   = 18005e720(plot->district)       // district+0xf0, index dans DistrictInfo, -1 si aucun
if (type != -1 && 1802a86c0(diplo(propriétaire unité), district+0x1c)   // en guerre (supposé)
    && DistrictInfo[type]+0x58           // HitPoints non NULL
    && DistrictInfo[type]+0x5c >= 1)     // HitPoints
    arrêt = vrai
else si unités sur la case : arrêt si une unité de combat d'un joueur en guerre (18015a2b0)
if (arrêt) { données+0x1d0 += données+0x1d4; données+0x1d4 = 0; }   // plus de mouvement
if (parent != 0 && ...) contrôle de ZOC du parent (1801a7e90)
if (tour <= nbTours ...) return 1801a9f00(...)   // pour parent nul : 1
```

`+0x58`/`+0x5c` de `DistrictInfo` sont lus dans le chargeur `1807bafa0` (requête de `180759440`,
colonne 22 = `HitPoints`) : `+0x58 = (colonne non NULL)`, `+0x5c = HitPoints`. Quartiers
concernés (base + DLC) : `DISTRICT_CITY_CENTER` 200, `DISTRICT_ENCAMPMENT`, `DISTRICT_IKANDA`,
`DISTRICT_OPPIDUM`, `DISTRICT_THANH` 100. BBG ne change pas ces valeurs. Un centre-ville ennemi
ne peut pas être occupé sans être pris (il devient alors à nous) : en pratique, ce sont les
campements et leurs remplaçants.

**Le défaut : rien ne dispense le nœud de départ (`param_2 == 0`).** Pour une unité sur un
campement ennemi, le départ reçoit 0 mouvement. Chaque voisin passe alors par `1801a7150` avec un
parent à 0 PM : `tour + 1 = 2` > `nbTours = 1`, refusé. La liste est vide, la fonction Lua rend
`nil`, l'interface ne dessine rien. La règle vise l'entrée dans le quartier (une unité qui y entre
s'y arrête), pas la sortie.

`1802a86c0` (`Player_Diplomacy.cpp`) : test entre deux joueurs, en guerre d'après l'usage (`1801a9a40`
pose le drapeau `0x51|4` « quartier hostile » quand il répond oui, `|2` sinon). Supposé, pas vérifié
par une chaîne.

### Le vrai déplacement n'a pas ce test

`1801a6e00` n'est installé que par les deux parcours en largeur `1801a6a80` et `1801a6830` (appelés
par le cache de l'interface `1800d0df0`, `1800d0f70`, `1800d11f0` et par l'IA `1804bb840`,
`1804c52d0`). L'A* du déplacement, monté par `1801a7840` (`Formation_Movement.cpp`), installe
`1801a9f00` comme test de validité, avec `1801a9060`, `1801a9000`, `1801a97b0`… mais pas
`1801a6e00`. Pour le départ, `1801a7150` donne les PM pleins et `1801a9f00` répond 1. Rien dans ce
chemin ne remet les mouvements à 0 au départ. C'est cet A* que suit `UnitManager.GetMoveToPathEx`
(survol, `WorldInput.lua:961`) et l'ordre `MOVE_TO`.

Donc : **seul l'affichage manque** (lu dans le code, à confirmer en jeu). La même erreur fausse
aussi les parcours en largeur de l'IA quand une de ses unités est sur un de nos campements : la
corriger dans GameCore pour tout le monde changerait la simulation (POL).

## Correctif proposé : interface seule

Bloc [ui_mouvements.lua](../../proxy/src/ui_mouvements.lua), à ajouter à la fin de `SelectedUnit.lua` (il lit la
locale `m_MovementRange` du fichier). Il enveloppe la fonction globale `RealizeMoveRadius` :

1. appelle celle du jeu ;
2. si l'unité a des mouvements, n'est pas entrée en ZOC, n'est ni espion ni marchand, et que
   `GetReachableMovement` ne rend rien (ou seulement sa case) ;
3. et qu'elle est sur un quartier dont `GameInfo.Districts[...].HitPoints >= 1`, d'un joueur avec
   qui son propriétaire est en guerre (`CityManager.GetDistrictAt`, `IsAtWarWith`) ;
4. alors elle teste les cases autour avec `UnitManager.GetMoveToPathEx` : une case est retenue si
   le chemin y arrive (`plots[n] == case`) et que son dernier tour vaut 1. Portée max
   `min(10, 4 × PM)` (1/4 de PM par case de chemin de fer). D'abord anneau par anneau avec arrêt
   après deux anneaux vides ; depuis le 2026-11-08, par front avec réutilisation des chemins, cache
   et budget (section « Lag sur un campement posé non terminé » plus bas). Les cibles d'attaque
   (`GetReachableTargets`) sont ajoutées comme le fait le jeu ;
5. redessine `m_MovementRange` avec cette zone.

La zone suit exactement l'A* du survol : elle ne montre jamais une case que le clic droit
refuserait. Les appels `RealizeMoveRadius(...)` du fichier passent par le nom global : la nouvelle
fonction est prise. `SelectedUnit_Expansion2.lua` ne la redéfinit pas.

Dans le proxy, à la manière de `patch_production_panel` : une entrée de `FIXES` dans
`proxy/src/ui.rs` pour `"SelectedUnit.lua"`, qui ajoute le bloc à la fin (contrôle : la présence de
`function RealizeMoveRadius(` et de `local m_MovementRange`, refus si le marqueur
`Civ6RolloverProxy (src/ui_mouvements.lua)` est déjà là). Aucun mod ne remplace ce fichier : une
seule version à couvrir. Correctif de bug pur : pas de clé `read_config`, pas d'onglet.

Limites :
- la couche ZOC (`m_MovementZoneOfControl`, cases rouges) n'est pas refaite : elle vient du même
  parcours, vide lui aussi. Les indicateurs d'attaque à distance restent ceux du jeu ;
- coût : un A* par case testée, à la sélection et à chaque changement de PM, seulement dans ce cas.
  Pour 2 PM sur terrain plat, environ 40 à 60 chemins.

### Lag sur un campement posé non terminé, et optimisation (2026-11-08)

Un joueur a signalé un pic de lag au clic sur une unité posée sur un campement ennemi **posé mais
pas terminé**. Ni le bloc ni GameCore (`1801a6e00`, relu) ne regardent si le quartier est terminé :
seul compte `Districts.HitPoints` de la base. Le repli tourne donc aussi sur un chantier, où l'on
entre librement (pas de PV, pas de défense), avec tous ses PM.

Mesure en jeu (lignes `BGD mouvements:` de `Lua.log`, ancien algorithme) : un clic = un repli ;
1 PM : 36 chemins, zone de 5, ~10 ms ; 2 PM : 60 chemins, zone de 10, ~6,5 ms ; 3 PM : 90 chemins,
zone de 16, ~13 ms ; arrêt toujours sur deux anneaux vides, la fonction du jeu ~0,1 ms. Le coût est
celui des chemins : un A* par case du disque, multi-tours, en échec pour les cases inatteignables
(qui explorent alors toute la zone connexe), sans réutilisation ni cache. Une route ou une voie
ferrée qui part de la case empêche l'arrêt : jusqu'à 330 chemins.

Le bloc calcule maintenant la même zone ainsi :
1. **exploration par front** depuis la case de départ : seuls les voisins d'une case retenue (ou
   traversée par un chemin de ce tour, ou occupée par une unité, donc peut-être traversable) sont
   testés, dans la même portée `min(10, 4 × PM)` ;
2. **réutilisation des chemins** : les cases intermédiaires d'un chemin atteintes au tour 1 et sans
   unité dessus sont retenues sans nouveau chemin ; avec une unité, elles restent sources du front ;
3. **cases infranchissables écartées sans chemin** : `IsImpassable()` sans aménagement (tunnel,
   Qhapaq Ñan) ni route, d'après la vraie case de la carte, brouillard compris. Rien d'autre n'est
   filtré (eau, embarquement, unités navales : dans le doute, le chemin tranche) ;
4. **cache** de la zone, clé (joueur, unité, case, PM restants, tour, époque). L'époque avance quand
   une unité bouge, apparaît, disparaît, est téléportée, embarque ou devient visible, et à une
   déclaration de guerre, une paix, une route ou un aménagement. `LensLayerOn` et les
   `UnitMovementPointsChanged` sans changement sont servis par le cache ;
5. **budget** : 50 ms par calcul (horloge du proxy), 200 chemins sans elle ; au-delà, la zone trouvée
   (toujours juste, mais incomplète) est dessinée et la ligne de mesure porte `budget=1`.

Maquette hors jeu (`lua proxy/tests/test_ui_mouvements.lua`, plus court chemin réel sur une carte
hexagonale) : zone identique à l'ancien algorithme sur 16 cas (plaine, route, rail, côte avec et
sans embarquement, montagnes avec tunnel, brouillard, couloir avec unités amies). Chemins et nœuds
explorés, ancien → nouveau : plaine 2 PM 60/1 890 → 36/702 ; rail 3 PM 330/96 671 → 132/9 578 ;
route 4 PM 330/64 917 → 138/9 729 ; côte 3 PM 90/23 652 → 57/8 381 ; montagnes 3 PM 90/27 709 →
47/1 175 ; couloir 3 PM 216/155 006 → 21/252.

### Zones de contrôle et cases d'eau (2026-11-08, après le test en jeu)

Mesure en jeu de la version optimisée : 3 PM sur un campement ennemi non terminé, 36 à 48 chemins
et 4 à 12 ms par calcul neuf, puis 100 appels servis par le cache (41,5 ms cumulés pour 200 appels).
Deux défauts signalés.

**Zones de contrôle (cases rouges) absentes.** `GetReachableZonesOfControl` (`1806c25e0` →
`1800d11f0`) lance le même parcours en largeur `1801a6a80` (drapeaux `0x800`) puis garde les cases
où `180378e20` répond oui : vide aussi sur un quartier ennemi. Règle relue dans `180378a70` : une
case est sous zone de contrôle si une **case voisine** (autre que celle d'où l'on vient) porte :
- un quartier d'un joueur en guerre qui exerce une zone de contrôle (`1801928a0` : `HitPoints` de
  la base ≥ 1, quartier **terminé** (`+0x3f8`), **non pillé** (`+0x458`)) ; un campement non
  terminé n'en exerce donc pas, contrairement à la règle du parcours en largeur ;
- ou une unité d'un joueur en guerre, vivante, du **même domaine**, qui exerce une zone de
  contrôle (`1803a30f0`, colonne `Units.ZoneOfControl`), **visible** du joueur, **sans rivière**
  entre les deux cases (le bord de rivière coupe la zone de contrôle des unités, pas des quartiers).

Le bloc applique cette règle (la case de départ n'en exerce pas, comme `param_6` de `180378a70`
pour le premier pas) : la couche `m_MovementZoneOfControl` est redessinée comme le jeu (cases de la
zone sous zone de contrôle, visibles, plus les indicateurs d'attaque), et le front ne repart pas
d'une case sous zone de contrôle. Si un chemin du jeu traverse une telle case dans le même tour
(unité qui ignore les zones de contrôle), le front reprend partout. L'A* du jeu applique lui-même
les zones de contrôle (`1801a7e90` dans le parcours) : la zone reste la sienne.

**Cases d'eau proposées sans Construction navale.** `GetMoveToPathEx` (`1806c1240` → `1800d0790` →
A* `1801aa680`, drapeaux `0x4000`/`0x4010`) suit le pathfinder du joueur, qui ne connaît que ce que
le joueur a vu : une case **non révélée** y est prise pour franchissable. Une côte non révélée à
portée devient alors une case « atteignable ce tour » et un chemin peut passer par elle. Ces
chemins sont faux, et la réutilisation des chemins les propageait aussi. Le bloc lit maintenant la
vraie case (`Map`) et écarte sans chemin, par paliers d'accès :
- infranchissable sans aménagement (tunnel, Qhapaq Ñan) ni route ;
- eau pour une unité terrestre qui ne peut pas embarquer : `Technologies`/`Civics` avec `EmbarkAll`
  ou `EmbarkUnitType` = son type, acquis par le joueur (`GetTechs():HasTech`,
  `GetCulture():HasCivic`), ou dans le doute (`GlobalParameters.UNLOCK_EMBARKATION`, trait du joueur
  dont un modificateur parle d'embarquement : Maoris…) ;
- océan si aucun modificateur `*VALID_TERRAIN*` n'est acquis (technologie, dogme, trait, ou
  personnage illustre recruté : Cartographie, Norvège, Maoris, Leif Erikson…) ;
- terre sans quartier pour une unité navale.

Un chemin qui passe par une case écartée est compté faux (`faux=` dans la ligne de mesure) : ni sa
cible ni ses cases ne sont retenues. Les accès sont relus à chaque tour, technologie ou dogme.

**Zone 16 puis 12 pour la même unité.** Le premier calcul (`LensLayerOn`) et le suivant
(`selection`) ont la même clé sauf l'époque : entre les deux, une unité a bougé, est apparue ou
est devenue visible (tours simultanés). La zone de l'A* change avec les unités (cases occupées,
zones de contrôle), d'où l'écart ; c'est aussi pourquoi l'époque fait partie de la clé. Les cases
d'eau non révélées comptaient aussi dans ces zones.

Maquette : 26 cas identiques à l'ancien algorithme avec un A* exact, dont côte sans et avec
embarquement, trait d'embarquement, océan sans et avec Cartographie, côte non révélée (l'A* du jeu
la propose, la zone n'en garde aucune case), zones de contrôle (avec rivière, unité qui les ignore,
campement terminé au départ).

Écart possible avec l'ancien algorithme, en mieux : l'ancien s'arrêtait après deux anneaux sans
case où s'arrêter, et pouvait manquer des cases plus loin (au-delà d'un passage occupé par nos
unités). Le front les trouve. Hypothèse à confirmer en jeu : une case traversée au tour 1 par un
chemin du survol, sans unité, est une case où l'A* accepte de s'arrêter.

### Variante GameCore, exacte et sans désynchro

Si la zone Lua déçoit : dispenser le nœud de départ du test, **seulement pendant les appels de
l'interface**. Détourner `1800d0df0` et `1800d11f0` (cache de l'interface, appelés seulement par les
fonctions Lua `GetReachableMovement` / `GetReachableZonesOfControl`) pour lever un drapeau local au
fil, et l'entrée de `1801a6e00` : si le drapeau est levé et `param_2 == 0`, rendre
`1801a9f00(param_1, …, param_6)` (même suite que l'original pour un départ, sans le test du quartier).
Le drapeau est nécessaire car l'IA appelle aussi `1801a6a80` ; sans lui ce serait un correctif POL.
Avantages : couche ZOC comprise, aucun A* en plus. Inconvénient : adresses liées au build.

## À vérifier en jeu

- Unité sur un campement ennemi avec des PM : survoler une case voisine montre un chemin en 1 tour,
  et le clic droit la déplace (confirme que seul l'affichage manque).
- Avec le bloc : la bordure apparaît et colle aux chemins du survol ; rien ne change ailleurs.
- Même chose sur un ikanda (Zoulous) ou un oppidum (Gaule).

## Outillage

Décompilation : `analyzeHeadless <projets> civ6xp2 -process GameCore_XP2_FinalRelease.dll
-noanalysis -readOnly -postScript DumpAddrs.java <sortie> 1806c2060 1800d0df0 1801a6a80 1801a6e00
1801a7150 1801a7840 1801a9f00 1807bafa0`, et `CallersOf.java` pour les appelants de `1801a6e00`.

## Réglage de la mesure (2026-11-08)

La mesure reste dans le bloc, coupée par défaut : `ui_mesure_mouvements=on|off` (off), option avancée
(section Diagnostic) de l'onglet BetterGameExperience, présente aussi en release. Le proxy fait précéder
le bloc de `local BGD_MOUV_MESURE = true|false;` ; avec false, la zone est calculée de la même façon
mais aucune ligne `BGD mouvements:` ni bilan n'est écrit dans `Lua.log`.
