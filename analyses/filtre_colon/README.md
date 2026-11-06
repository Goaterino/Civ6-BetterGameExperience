# Lentille du colon qui « se casse »

**Corrigé dans le proxy** (2026-11-06, interface seule, sans réglage) : `patch_minimap_panel` dans
[proxy/src/ui.rs](../../proxy/src/ui.rs), blocs [ui_colon.lua](../../proxy/src/ui_colon.lua) (sous
`pcall`) et [ui_colon_fin.lua](../../proxy/src/ui_colon_fin.lua). Ce sont les correctifs A à D proposés
plus bas, avec une différence pour D : la sélection d'une unité recolorie tout de suite
(`SetWaterHexes`), au lieu de lever `m_refreshLens`, que seul `GameCoreEventPublishComplete` traite. Pas
encore essayé en partie.

## Symptôme

Avec un colon sélectionné, ou avec la lentille « Colon » de la mini-carte, la lentille se casse dans
certains contextes. Elle ne colorie plus que les cases dans le brouillard (révélées mais pas
visibles), ou des cases très loin de la capitale. Les autres cases, proches et visibles, n'ont plus
aucune couleur. Passer à la lentille d'attrait puis revenir à celle du colon répare en général.

(Une première version de cette analyse partait d'un autre symptôme, des cases proches coloriées
comme fondables. Ce défaut existe aussi : il est rangé dans
[Défaut secondaire](#défaut-secondaire--bit-fondable-périmé-dans-le-cache).)

Analyse statique du 2026-11-06 : scripts Lua de Base, XP1 et XP2, mods du Workshop, blocs Lua du
proxy, `GameCore_XP2_FinalRelease.dll` (projet Ghidra `civ6xp2`) et `CivilizationVI_DX12.exe`
(projet `civ6exe`). Les décompilations sont dans [ghidra/](ghidra/). Le script Ghidra
[outils/DumpRange.java](outils/DumpRange.java) décompile une plage d'adresses.

## Conclusion

**Ce n'est pas la liste de cases qui est fausse : c'est la couche qui est vidée sans être
recoloriée.** Les cases du brouillard qui gardent une couleur sont des restes ; les cases visibles
montrent la couche telle qu'elle est, c'est-à-dire vide.

1. **Cause principale (vérifiée dans le Lua du jeu).** `OnCityAddedToMap` (`MinimapPanel.lua:969`)
   appelle `UILens.ClearLayerHexes(Hex_Coloring_Water_Availablity)` à chaque `CityAddedToMap`. Cet
   événement arrive pour chaque ville ajoutée à la carte, par n'importe quel joueur. Il arrive aussi
   pour une ville hors de vue : `CityBannerManager` crée une bannière pour toutes les villes
   (`CityBannerManager.lua:2618`, XP2) et ne regarde la visibilité qu'ensuite. Le code ne demande
   pas de recalcul (`m_refreshLens` n'est pas levé). Avec un colon sélectionné, la couche reste
   allumée, car on n'est pas en `VIEW_MODAL_LENS`. Elle reste donc vide jusqu'au prochain
   `PlotVisibilityChanged` du joueur local. En multijoueur, en début de partie, des villes sont
   fondées sans cesse par les autres joueurs : c'est justement là qu'on se sert de la lentille.
2. **Pourquoi le brouillard reste colorié (supposé, cohérent avec tout le reste).** La couche ne
   connaît pas la visibilité. `SetLayerHexesColoredArea` (`140196c50`) envoie toutes les cases au
   rendu sans tri, et `ClearLayerHexes` (`140197f10`) les retire sans tri non plus. La différence
   vient donc du rendu du brouillard. Une case révélée mais pas visible est affichée telle qu'elle
   était quand on l'a vue pour la dernière fois, couleur de lentille comprise ; elle ne voit pas
   l'effacement. Les cases visibles montrent l'état réel de la couche. Cela explique aussi « loin de
   la capitale » : ce sont surtout des zones de brouillard.
3. **Pourquoi l'attrait répare (vérifié dans l'exe).** `UILens.SetActive` (`140199450`) fait
   ceci pour chaque couche qui quitte la lentille : `ClearLayerHexes`, puis une commande
   « éteindre » (type `0x1e`). À l'arrivée dans la nouvelle lentille, il envoie une commande
   « allumer » (`0x1d`). Le retour à la lentille du colon fait donc arriver `LensLayerOn` à
   `MinimapPanel.OnLensLayerOn`, qui appelle `SetWaterHexes` et recolorie toute la couche. N'importe
   quel recalcul suffirait. Le jeu n'en fait simplement pas après `OnCityAddedToMap`.
4. **Autre façon de vider la couche : une erreur Lua dans `SetWaterHexes`.** La fonction commence
   par `ClearLayerHexes`. Si une erreur survient ensuite, la couche reste vide, avec le même
   symptôme. Dans le jeu d'origine, rien ne peut échouer là. En revanche, l'appel
   `BGD_FiltrerColon` ajouté par le proxy se trouve entre l'effacement et le coloriage, sans
   `pcall`. Voir [Correctif](#correctif-proposé).

Aucun mod ne joue : aucun ne remplace `MinimapPanel`, `SelectedUnit` ou `ModalLensPanel`, et aucun
ne touche à cette couche. Voir [Mods](#mods). Les blocs Lua du proxy ne touchent pas non plus à
cette couche, sauf `ui_colon.lua`. `ui_mouvements.lua` ne touche qu'à `Movement_Range`.

## Toutes les façons de vider la couche

Ces appels vident la couche de l'eau, ou l'éteignent, dans les scripts chargés :

| où | quand | recoloriée ensuite ? |
|---|---|---|
| `MinimapPanel.lua:564` `SetWaterHexes` | à chaque recalcul | oui, dans la même fonction (sauf erreur) |
| `MinimapPanel.lua:489` `OnLensLayerOff` | couche éteinte, hors lentille modale | sans objet, la couche est éteinte |
| **`MinimapPanel.lua:974` `OnCityAddedToMap`** | **chaque ville ajoutée, tout joueur** | **non (jeu d'origine)** |
| `SelectedUnit.lua:430` `OnLocalPlayerChanged` | changement de joueur (hotseat) | couche éteinte |
| `SelectedUnit.lua:207`, `UnitPanel.lua:2646/2762/2964`, `UnitCaptured.lua:29` | colon désélectionné, fondation, suppression, capture | couche éteinte |
| exe, `SetActive` (`140199450`) | la couche quitte la lentille active | éteinte ; `LensLayerOn` au retour |

Le recalcul du jeu n'a lieu qu'à deux moments. Le premier est `OnLensLayerOn`. Le second est
`OnFlushChanges` (`GameCoreEventPublishComplete`), seulement si `m_refreshLens` a été levé par
`PlotVisibilityChanged` et si `m_WaterAvailabilityLensOn` est vrai. Ce dernier ne suit que les
événements `LensLayerOn`/`LensLayerOff`.

### Détails de l'exe (`CivilizationVI_DX12.exe`)

Liaisons Lua de `UILens` (enregistrées par `14019a4a0`) ; toutes passent par le gestionnaire
`DAT_14230f760+0x450` :

| fonction Lua | liaison | cœur |
|---|---|---|
| `SetLayerHexesColoredArea` | `14019c310` | `140196c50` |
| `ClearLayerHexes` | `14019ab50` | `140197f10` |
| `ClearHex` / `ClearHexes` | `14019a9e0` / `14019aa50` | `140197c90` / `140197e00` |
| `ToggleLayerOn` / `Off` | `14019c8c0` / `14019c890` | `140199b30` / `1401999e0` |
| `SetActive` | `14019b990` | `1401993b0` -> `140199450` |

- Les commandes partent dans une file de 2 × 4 Mo (`DAT_141b0b380`). Si elle est pleine, la commande
  est perdue sans message. Un coloriage prend 1 Ko par bloc de 64 cases : la file ne déborde pas en
  temps normal.
- Chaque commande porte l'identifiant de la couche. Rien, côté exe, ne filtre par visibilité ou
  par propriétaire.
- `ToggleLayerOn` n'envoie la commande « allumer » que si la couche n'était pas déjà affichée (bit
  posé par l'utilisateur, ou couche de la lentille active, et permise par la lentille). Si la couche
  est déjà affichée, il n'y a donc pas de `LensLayerOn`, et pas de recalcul.
- `140196c50` **remplace** la liste de cases mémorisée pour la couche par celle de l'appel en
  cours. Après `SetWaterHexes`, elle ne contient donc plus que `NoSettlePlots`.
  `ClearLayerHexes`, selon un drapeau de la couche (`entrée+4`), envoie soit un effacement complet,
  soit un effacement case par case de cette liste. Dans le second cas, il n'effacerait que les
  cases rouges. Je n'ai pas trouvé d'où vient ce drapeau. Le symptôme rapporté (tout est effacé)
  laisse penser que c'est l'effacement complet.

## Correctif proposé

Interface seule, sans désynchronisation. Ce sont des bugs purs, donc pas de réglage. Tout se passe
dans `patch_minimap_panel` (`MinimapPanel.lua`).

### A. À garder : `m_refreshLens = true` dans `OnCityAddedToMap` (déjà fait)

C'est le correctif de la cause principale. Après `CityAddedToMap`, le jeu publie ses événements puis
envoie `GameCoreEventPublishComplete`. `OnFlushChanges` appelle alors `SetWaterHexes` si la couche
est allumée. Avec la lentille modale, `OnCityAddedToMap` repasse en `SELECTION`, ce qui éteint la
couche (sauf colon sélectionné) : `m_WaterAvailabilityLensOn` passe à faux, et rien n'est
recalculé pour rien.

### B. À modifier : protéger `BGD_FiltrerColon` par `pcall`

Je n'ai pas trouvé d'appel qui puisse échouer dans [ui_colon.lua](../../proxy/src/ui_colon.lua). Il
lit seulement le cache, et `GetArea` est déjà sous `pcall`. Mais il s'exécute après
`ClearLayerHexes` : une seule erreur (une API qui manque dans un contexte BSM, un joueur sans
`GetCities`…) reproduirait exactement le symptôme, à chaque recalcul. Remplacer la ligne insérée
`FILTRE` par :

```lua
	do local ok, a, b, c, d = pcall(BGD_FiltrerColon, FullWaterPlots, CoastalWaterPlots, NoWaterPlots, NoSettlePlots);
	if ok and a ~= nil then FullWaterPlots, CoastalWaterPlots, NoWaterPlots, NoSettlePlots = a, b, c, d; end end
```

En cas d'échec, on garde les listes du jeu.

### C. À ajouter : recalcul plus robuste dans `OnFlushChanges`

`m_WaterAvailabilityLensOn` ne suit que les événements. Si un `LensLayerOn` manque, la couche reste
allumée et vide. On peut se fier à l'état réel de la couche. Dans `OnFlushChanges`
(`MinimapPanel.lua:985`), remplacer :

```lua
		if(m_WaterAvailabilityLensOn)then
```

par :

```lua
		if(m_WaterAvailabilityLensOn or UILens.IsLayerOn(m_HexColoringWaterAvail))then
```

### D. Facultatif : recolorier quand on resélectionne un colon

`SelectedUnit.lua:198` appelle `ToggleLayerOn` sur une couche peut-être déjà affichée. Dans ce cas,
il n'y a ni `LensLayerOn` ni recalcul (voir les détails de l'exe). Plutôt que de patcher
`SelectedUnit`, ajouter à la fin de `MinimapPanel.lua`, pour accéder aux `local` :

```lua
Events.UnitSelectionChanged.Add(function(p, u, i, j, k, bSel)
	if bSel and UILens.IsLayerOn(m_HexColoringWaterAvail) then m_refreshLens = true; end
end);
```

Avec A, C et D, le joueur a toujours un moyen simple de réparer : resélectionner le colon.

### Le filtre de distance (`BGD_FiltrerColon`) : à garder

Le défaut qu'il corrige est réel (voir plus bas). Le filtre ne fait qu'ajouter aux cases interdites
des cases que la règle interdit aussi. Il n'utilise que les villes dont la case est révélée au joueur.
Coût : environ 49 cases par ville. Une seule réserve : pour le test de masse terrestre, si
`Plot:GetArea()` n'existe pas côté interface, il se rabat sur le continent. Une case à 3 de distance,
de l'autre côté d'un bras de mer mais sur le même continent, passerait alors en rouge à tort. Ce cas
est rare. Avec B, il ne peut pas aggraver le nouveau symptôme.

## À vérifier

1. **Reproduire la cause principale.** Colon sélectionné, en multijoueur ou contre l'IA. Attendre
   qu'un autre joueur fonde une ville, même hors de vue, sans bouger ses unités. Sans correctif, les
   cases visibles perdent leur couleur et le brouillard la garde. Avec A, la couche se recolorie
   aussitôt.
2. **Restes dans le brouillard.** Après la casse, déplacer une unité pour révéler une case du
   brouillard. Si la case perd sa couleur en devenant visible, l'explication 2 est confirmée.
3. **Erreurs Lua.** Chercher `MinimapPanel` dans `Lua.log` (activer `EnableLuaLogging`), ou dans le
   journal du proxy.
4. **Le défaut secondaire**, avec [diagnostic.lua](diagnostic.lua) : voir à la fin.

---

# Défaut secondaire : bit « fondable » périmé dans le cache

Défaut toujours vrai, mais ce n'est pas lui qui casse la lentille. Les cases neutres à 2 ou 3
cases d'une ville fondée après leur dernière mise à jour dans le cache sont coloriées comme
fondables (vert, ou gris pour « sans eau »). Elles devraient être rouges. Le filtre
`BGD_FiltrerColon` le corrige.

## Le chemin dans le jeu d'origine

### Qui allume la lentille

- Colon sélectionné : `SelectedUnit.lua:198` appelle
  `UILens.ToggleLayerOn(Hex_Coloring_Water_Availablity)` si l'unité a `FoundCity` et si le joueur a
  déjà au moins une ville. Ce n'est pas un mode de lentille : la lentille reste `Default`.
- Bouton de la mini-carte : `ToggleWaterLens` (`MinimapPanel.lua:298`) fait
  `UILens.SetActive("WaterAvailability")`.
- Dans les deux cas, `Events.LensLayerOn` arrive à `MinimapPanel.OnLensLayerOn`
  (`MinimapPanel.lua:465`), qui appelle `SetWaterHexes`.

### Le coloriage : `SetWaterHexes` (`MinimapPanel.lua:558-590`)

```lua
FullWaterPlots, CoastalWaterPlots, NoWaterPlots, NoSettlePlots = Map.GetContinentPlotsWaterAvailability();
...
UILens.SetLayerHexesColoredArea( m_HexColoringWaterAvail, localPlayer, NoSettlePlots, DisgustingColor );
```

Quatre listes, quatre couleurs (`Colors.xml:1248-1267`) :

| liste | couleur | légende (`ModalLensPanel.lua:52`) |
|---|---|---|
| `FullWaterPlots` | `COLOR_STANDARD_GREEN_MD,128` | eau douce |
| `CoastalWaterPlots` | `COLOR_STANDARD_GREEN_LT,128` | côte |
| `NoWaterPlots` | `COLOR_STANDARD_WHITE_MD2,128` (gris) | sans eau |
| `NoSettlePlots` | `COLOR_STANDARD_RED_MD,128` | trop proche d'une ville |

Pour les Mayas, les trois premières listes prennent toutes la couleur grise.

Le Lua ne fait aucun tri : tout le classement est fait par GameCore.

Le recalcul a lieu à l'allumage, puis à chaque `GameCoreEventPublishComplete` qui suit un
`PlotVisibilityChanged` (`MinimapPanel.lua:532`, `:982`).

Les expansions ne changent rien. `MinimapPanel_Expansion1.lua` et `MinimapPanel_Expansion2.lua`
incluent le fichier de base et ne redéfinissent ni `SetWaterHexes`, ni `OnLensLayerOn`, ni
`OnCityAddedToMap`. `SettlerWarningIconManager.lua` (crues, volcans) et
`SettlerInfluenceIconManager.lua` (loyauté) ajoutent des icônes sur la même couche, sans couleur.

### `Map.GetContinentPlotsWaterAvailability` côté interface : `FUN_1806a2070`

Il y a deux liaisons de ce nom : `180017b30` (`Lua_IMap.cpp`, scripts de règles) et `1806a2070`
(`Cache_Lua_IMap.cpp`, interface). L'interface utilise la seconde. Assembleur dans
[ghidra/1806a2070_asm.txt](ghidra/1806a2070_asm.txt). Pour chaque case du cache :

```
si (visible ou révélée pour le joueur local) et continent >= 0 :      // eau exclue
    si (case+0x50 & 1)                                                // bit « fondable » du cache
       ou (propriétaire == joueur local et CanFoundCity(local, x, y) == 1) :   // vérif. en direct
        eau douce ? -> Full ; sinon côte ? -> Coastal ; sinon -> NoWater
    sinon -> NoSettle
```

Seule la mer (continent négatif) et les cases jamais révélées sont absentes des quatre listes.
Toutes les autres cases reçoivent une couleur.

La version des règles (`180017b30`) fait le test en direct pour toutes les cases :
`CanFoundCity(-1, x, y)`. Elle ne garde que les cases visibles en ce moment.

### `CanFoundCity` : `FUN_180144810(joueur, x, y)` (`City_Placement.cpp`)

Elle renvoie 0, donc case interdite, dans ces cas :
- la case a le drapeau `+0x3a & 8` ;
- la case a une caractéristique sans le bit « fondable » ;
- un district est déjà sur la case (`18005e720`) ;
- la case est possédée : par n'importe qui si `joueur == -1`, ou par un autre que l'équipe du joueur ;
- le terrain est interdit (`180060820`, terrains 15-16, plus un drapeau du terrain) ;
- un test de ressource ou d'objet échoue (`vtable+0x50` d'un gestionnaire) ;
- une ville se trouve dans un rayon `R = GlobalParameters+300` (`CITY_MIN_RANGE`, 3), et
  `ville+0x10 == case+0x10` ou `distance < R`.

Le dernier test interdit donc une distance ≤ 3 sur la même masse terrestre, et une distance ≤ 2
de l'autre côté de l'eau. Je suppose que le champ `+0x10` est l'identifiant de masse terrestre
(c'est cohérent avec la règle du jeu) ; ce n'est pas vérifié.

`CITY_MIN_RANGE` vaut 3 (`GlobalParameters.xml:98`). Aucun mod installé ne le change : j'ai cherché
dans BBG 7.5.1, BBG BETA, BBG Expanded, BSM, MPH, DMT et les autres. Le `BBM_MIN_DISTANCE` de Better
Balanced Map (3179425402) règle l'écart entre positions de départ, pas la fondation.

### Le bit du cache : `FUN_1800127e0`

Elle remplit les drapeaux du cache d'une case :

```c
iVar3 = FUN_180144810(-1, x, y);
*param_2 = *param_2 & 0xfe | (iVar3 == 1);       // bit 0 = case+0x50 & 1
```

Elle a trois appelants :
- `18000f3b0`, qui copie une case dans le flux d'événements du cache quand arrive l'événement
  `0x77d19dc2` (une case a changé) ;
- `180012bd0` et `180012e90`, qui recopient une case du cache (création, application de l'événement).

Il n'y a pas d'autre écrivain. Le bit n'est donc juste qu'à la dernière modification de la case elle-même.
Fonder une ville change le propriétaire de la case de la ville et de l'anneau 1, et ces cases sont
recopiées. Les cases à 2 ou 3 de distance, hors frontières, ne bougent pas : leur bit reste celui
d'avant la ville, donc « fondable ». Je n'ai pas trouvé de mise à jour en rayon autour d'une
nouvelle ville ; ce point est à confirmer en partie (voir plus bas).

Même chose dans l'autre sens : une ville rasée laisse ses voisines en rouge.

`Map.GetContinentPlotsLoyalty` (`180712220`), qui affiche les chiffres de loyauté sur la même
lentille, a exactement la même condition. Il hérite donc du même défaut.

### Le rendu : `UILens.SetLayerHexesColoredArea` (`14019c310`, `140196c50` dans l'exe)

Elle pousse les cases dans la file de rendu, par blocs de 64, avec la couleur et l'asset de la couche.
Elle mémorise ensuite la liste de cases pour la couche. Je n'ai trouvé aucun filtre par propriétaire,
par visibilité ou par distance. Je n'ai pas étudié le rendu lui-même.

### Effacement sans recalcul : `OnCityAddedToMap` (`MinimapPanel.lua:969`)

```lua
function OnCityAddedToMap(playerID, cityID, x, y)
    if UI.GetInterfaceMode() == InterfaceModeTypes.VIEW_MODAL_LENS then
        UI.SetInterfaceMode(InterfaceModeTypes.SELECTION);
    end
	UILens.ClearLayerHexes(m_HexColoringWaterAvail);
end
```

Avec un colon sélectionné, la couche reste allumée (on n'est pas en `VIEW_MODAL_LENS`), mais elle est
vidée. `m_refreshLens` n'est pas levé : rien ne la recolorie avant le prochain changement de
visibilité. En multijoueur, cela arrive dès qu'un autre joueur fonde une ville.

## Mods

| mod | fichier en cause | effet |
|---|---|---|
| BBG 7.5.1, BBG BETA | `unitpanel_bbg.lua` remplace `UnitPanel` | ne touche pas à la lentille |
| BSM | remplace `UnitPanel`, `WorldInput`, `PlotToolTip`… | ne touche pas à la lentille |
| MPH | `ingame.xml` | charge `MinimapPanel` normalement (`ingame.xml:49`) |
| DMT | `mappinmanager_dmt.lua` | ne lit que `MapPinListButton` de `MinimapPanel` |

Aucun mod ne remplace `MinimapPanel.lua`, `SelectedUnit.lua` ou `ModalLensPanel.lua`, ni les couleurs
`COLOR_*_APPEAL`. La version chargée est donc celle du jeu : `MinimapPanel_Expansion2.lua` (XP2),
qui inclut `MinimapPanel_Expansion1.lua`, qui inclut `Base/Assets/UI/MinimapPanel.lua`.


## Vérifier le défaut secondaire

Lancer [diagnostic.lua](diagnostic.lua) dans FireTuner, une ville fondée depuis quelques tours à côté
de cases neutres. Attendu si le bit est périmé : des `F`/`C`/`N` à distance 2 ou 3, là où la règle
donne `X`. Après une sauvegarde et un rechargement (le cache est reconstruit), ces cases doivent passer
en `X`. Le champ `+0x10` testé par `180144810` est supposé être la masse terrestre ; ce n'est pas
vérifié.
