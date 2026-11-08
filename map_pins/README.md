# Lag des épingles de carte en multijoueur

Symptôme rapporté : en multijoueur, quand un autre joueur pose (ou déplace, ou supprime) une épingle,
tout le monde subit un gel. Analyse statique du 2026-10-30, sur `CivilizationVI_DX12.exe` (projet Ghidra
`~/Tools/ghidra-projects/civ6exe`, base 0x140000000), les scripts Lua de Base et deux mods du Workshop
présents ici : Detailed Map Tacks (DMT, `2428969051`) et MPH Core (`3307282026`). Rien n'a encore été
mesuré en partie : voir [Tests en direct](#tests-en-direct).

Les décompilations sont dans `~/Tools/ghidra-projects/pin_dump` (graines par chaînes),
`pin_dump2`, `pin_vt` et `pin_rx` (adresses précises). REA n'a pas pu ouvrir l'exe : son import Ghidra
dépasse le délai de démarrage (330 s) sur 21 Mo. Le projet déjà analysé a donc été relu en
`-noanalysis -readOnly` avec `DumpByStrings.java`, `DumpAddrs.java`, `RefsTo.java` et `CallersOf.java`.

## Conclusion

**Le coût est dans l'interface Lua de chaque machine qui reçoit, pas dans le réseau ni dans le C++.**
Une épingle n'a pas de message à elle. `Network.BroadcastPlayerInfo()` renvoie **toute** la
configuration du joueur. Chez chaque destinataire, cela déclenche `Events.PlayerInfoChanged`, et
`MapPinManager.lua` **détruit puis reconstruit toutes les épingles de tous les joueurs**. Ce travail
croît comme le carré du nombre d'épingles par joueur. DMT, s'il est installé, le multiplie encore par un
`loadstring` du cache entier du joueur, deux fois par épingle.

Les causes, par ordre de poids supposé :

1. **DMT** (si installé) : `2 × Σ n_p` compilations Lua d'une chaîne qui grossit avec `n_p`, à chaque
   `PlayerInfoChanged`, émis par n'importe qui ;
2. **Base** : `MapPinFlag.Refresh` appelle une dizaine de fois `GetMapPins()`, qui construit à chaque
   fois une table neuve de **toutes** les épingles du propriétaire. Soit environ `10 × Σ n_p²` objets
   Lua alloués par reconstruction, plus le ramasse-miettes ;
3. **Base** : `MapPinListPanel` détruit et recrée la liste complète, même fermée. `DiplomacyRibbon`
   reconstruit les portraits ;
4. **Amplificateurs** : DMT et MPH appellent aussi `BroadcastPlayerInfo` pour leurs propres données
   (rendements DMT à chaque début de tour local, votes, contrôle des déconnexions, pause MPH). Chaque appel
   relance les points 1 à 3 chez tous les autres.

## Chemin complet

### Envoi

- `MapPinPopup.lua` `OnOk` / `OnDelete`, `MapPinManager.lua` `OnMapPinFlagRightClick` (suppression) :
  modification de `PlayerConfigurations[local]`, puis `Network.BroadcastPlayerInfo()`. La pose
  (`WorldInput.PlaceMapPin`) ne diffuse rien : elle ouvre le popup, qui crée l'épingle
  (`GetMapPin(x, y)`), et seul son OK diffuse. (`WorldInput.lua:1255` est `TogglePause`.)
- `14026ad60` (`lBroadcastPlayerInfo`, enregistré par `14026d5a0`) : construit un `NetPlayerInfo`
  (`140265c90`, vtable `140fdf088`) pour le joueur local, ou pour l'argument si l'on est hôte. Il
  l'envoie par `140277410`.
- `140265c90` sérialise **toute** la configuration du joueur dans le message (`1401177a0` → méthode
  `+0x178` de la carte de paramètres). Cela comprend toutes ses épingles et toutes les valeurs posées par
  `SetValue`, y compris celles des mods.

### Réception (`140266800`, `NetPlayerInfo` vtable `+0x20`)

1. `140116880` : remplace la configuration du joueur par celle reçue (méthode `+0x178`). Puis
   `1401162b0` reconstruit le cache C++ des épingles (vecteur d'entrées de 32 octets, une par nœud
   `MapPins`, via `140114d90`, `GameCore_Configuration_MapPin.cpp`). C'est linéaire et peu coûteux.
2. `14011e190` : met en file un événement de type `0x52` (`PlayerInfoChanged`) avec l'identifiant du
   joueur. Il n'y a qu'un événement par message.
3. `140298690` (`UploadCloudPlayerConfiguration`) n'est appelé qu'en partie PlayByCloud
   (`140102e00` compare un paramètre de partie à `0x10ff95a6`). Ailleurs, une ligne de journal au plus.

L'événement est ensuite distribué à deux sortes d'abonnés :

- **C++** : `140289780`, branché par `1402762d0` depuis `140293410`. Il appelle `1402975b0` et
  `1402955f0`, qui ne font quelque chose que si un état change, puis `140295fc0`. Ce dernier, chez
  l'hôte seulement, réécrit `MapName`, `MapDisplayName` et `MapSize` dans les données du lobby. C'est
  du travail constant et petit ;
- **Lua** : chaque `Events.PlayerInfoChanged.Add` de chaque contexte chargé (pont `LuaEvent`, adresse
  du registre `141b061e0 + slot × 0x3000`).

### Abonnés Lua en partie (Base)

| fichier | gestionnaire | travail |
|---|---|---|
| `MapPinManager.lua` | `OnPlayerInfoChanged` → `Refresh()` | `ResetInstances()`, puis recrée un drapeau pour **chaque épingle de chaque humain vivant**, quel que soit le joueur qui a changé |
| `Popups/MapPinListPanel.lua` | `BuildMapPinList()` | `DestroyAllChildren()` puis une entrée par épingle des 0..`MAX_MAJOR_CIVS`, tri, recalcul de taille ; même panneau fermé |
| `DiplomacyRibbon.lua` | `UpdateLeaders` | reconstruit le bandeau des dirigeants |
| `Popups/ChatPanel.lua`, `Popups/MapPinPopup.lua`, `EndGame/EndGameMenu.lua` | `PlayerTarget_OnPlayerInfoChanged` | listes déroulantes des destinataires |
| `Panels/PausePanel.lua` | `CheckPausedState` | léger |

MPH remplace `mappinlistpanel.lua`, `pausepanel.lua` et `endgamemenu.lua` sans changer ce schéma.

### Pourquoi `Refresh()` est quadratique

`MapPinFlag.GetMapPin()` fait `PlayerConfigurations[id]:GetMapPins()[self.m_pinID]`. Or `lGetMapPins`
(`140110f80`) crée **à chaque appel** une table neuve et y pousse un objet par épingle du joueur
(`14010f720` + `rawseti`). La construction d'un drapeau (`Initialize` → `Refresh` → `UpdateCurrentlyVisible`,
`SetFlagUnitEmblem`, `SetColor`, `UpdateName`, `UpdatePosition` → `SetPosition`…) appelle `GetMapPin()`
une dizaine de fois. Une reconstruction coûte donc de l'ordre de `10 × Σ_p n_p²` objets Lua, en plus des
appels d'interface (texte, couleur, position). Pour 8 joueurs à 60 épingles, cela fait environ 290 000
objets à chaque épingle posée (estimation, non mesurée).

### Detailed Map Tacks

- `mappinmanager_dmt.lua` surcharge `MapPinFlag.Refresh` : après la version de base, `UpdateYields`
  puis `UpdateCanPlace`.
- Chacune appelle `GetMapPinSubject(owner, x, y)` → `LoadMapPinSubjectTable` →
  `playerConfig:GetValue("DMT_MapPinSubjects")` → `deserialize` = **`loadstring(chaîne)()`**.
- La chaîne est le cache entier du propriétaire : une entrée par épingle DMT, avec `YieldString`,
  `YieldToolTip` et `CanPlaceToolTip` localisés.
- Résultat : à chaque reconstruction, `2 × (nombre total d'épingles)` compilations de chunks Lua, dont
  la taille croît avec le nombre d'épingles du propriétaire. Les épingles des autres joueurs comptent
  aussi, puisque leur cache voyage dans leur configuration.
- Ce même cache gonfle chaque `NetPlayerInfo` du joueur : il part en entier à chaque broadcast, même
  pour une simple épingle.
- `BroadcastMapPinSubjects` (appelé par `UpdatePinYields`) rediffuse la configuration :
  - à chaque `LocalPlayerTurnBegin` si un rendement a changé ;
  - à chaque amélioration, quartier ou caractéristique modifiés près d'une épingle ;
  - à chaque suppression automatique d'épingle.

  Chaque rediffusion relance une reconstruction complète chez tous les autres.

## Mesures du 2026-10-30

Partie à deux humains (joueur local 5, ami 0), DX12, mods BSM et MPH, **sans DMT**. Chacun pose
environ 25 épingles (de 2+2 à 27+28). Session `map_pins/runs/20261030-230735`, horloge du proxy (QPC).

**Côté natif (Frida)**
- 159 `NetPlayerInfo` reçus : 108 du joueur 5, puisque chaque broadcast local revient aussi à son
  émetteur, et 51 du joueur 0. Il y a eu 81 `BroadcastPlayerInfo` locaux.
- **Environ 2 à 3 `PlayerInfoChanged` par épingle**, donc autant de reconstructions complètes chez
  chaque joueur. Le second arrive de 60 ms à 2 s après le premier, avec la même taille. L'origine
  n'est pas établie : le jeu de base ne diffuse qu'une fois, au OK du popup. Il pourrait s'agir de
  renvois par le réseau ou par l'hôte, ou de modifications faites dans la foulée (déplacement,
  renommage). Ces doublons identiques sont de toute façon sautés par `ui_epingles=on` (ci-dessous).
- Taille : environ 5,4 Ko de configuration de base, plus environ 130 octets par épingle (9 Ko au
  maximum ici).
- Traitement C++ de la réception : 0 à 2 ms. Le natif est hors de cause, comme prévu.

**Côté Lua, par `PlayerInfoChanged`** (moyenne par tranche du nombre total d'épingles)

| épingles | MapPinManager | MapPinListPanel (MPH) | UpdateLeaders (BSM) | total par événement |
|---:|---:|---:|---:|---:|
| < 10 | 0,4 ms | 1,0 ms | 3,4 ms | ~5 ms |
| 20–29 | 1,1 ms | 2,7 ms | 3,3 ms | ~7 ms |
| 30–39 | 1,9 ms | 4,3 ms | 3,7 ms | ~10 ms |
| 50–55 | 2,5 ms | 5,8 ms | 3,4 ms | 12 à 14 ms (max 23) |

- `GetMapPins` est bien appelé environ 7 fois par épingle dans `MapPinManager`, et environ 1,5 fois
  dans la liste : 387 et 79 appels à 55 épingles.
- MapPinManager et la liste croissent avec le nombre d'épingles ; la liste de MPH coûte deux fois plus
  que les drapeaux. Le bandeau des dirigeants coûte un fixe d'environ 3,4 ms par événement.

**Ce que cela dit.** À 55 épingles, une épingle coûte environ 3 × 13 ms, soit 40 ms de Lua sur
chaque machine. C'est un à-coup visible, pas un gros lag. L'extrapolation à plusieurs centaines
d'épingles reste à mesurer : elle n'a rien de linéaire, la liste et les drapeaux grossissent tous les
deux. Il reste trois pistes, non testées :
- beaucoup plus d'épingles ;
- DMT chez les joueurs touchés ;
- un coût hors Lua, au rendu ou à la mise en page des instances recréées par `ResetInstances`, que
  ces mesures ne voient pas. Il faudrait mesurer la durée des images (Present).

### Deuxième session : durée des images (compte historique, 2026-10-30, 23:21–23:26)

Session `map_pins/runs/20261030-232118`, DX12, BSM et MPH, **sans DMT**. Le joueur local est le 2
(0 → 17 épingles), l'ami le 0 (22 → 28 épingles).

- 78 `NetPlayerInfo` traités, 39 par joueur, dont 72 après le chargement. Il y a eu 20 broadcasts
  locaux.
- Lua par événement : MapPinManager 1,5 ms, liste MPH 9,7 ms (13 ms au plus), bandeau environ 3,5 ms.
  Soit environ 15 ms au total.
- Thread principal (celui qui traite `NetPlayerInfo`) : 80 à 220 `PeekMessage` par seconde.
- **Aucune image de 50 ms ou plus dans la seconde qui suit l'un des 72 `NetPlayerInfo` reçus après le
  chargement.** Les 4 gels de plus de 100 ms que relie le résumé tombent tous pendant le chargement de
  la partie (23:21:33 à 23:22:04).
- Les images longues hors chargement (jusqu'à 342 ms) n'ont aucun `NetPlayerInfo` dans la seconde et
  demie qui précède. Elles ont une autre cause.

**Conclusion provisoire.** Sur cette machine, avec deux joueurs et une cinquantaine d'épingles, poser
une épingle ne gèle pas l'image. Le coût Lua, environ 15 ms par diffusion, reste sous la durée d'une
image lente. Le lag rapporté ne vient donc pas, à cette échelle, de la reconstruction de l'interface.
Restent :
- **l'échelle** : 6 à 12 joueurs, plusieurs centaines d'épingles, machines plus lentes. Le coût croît
  avec le nombre d'épingles de **tous** les joueurs, et chaque épingle de chacun le déclenche 2 à 3 fois ;
- **le réseau** : chaque épingle envoie 2 à 3 fois toute la configuration du joueur (7 à 9 Ko, soit 7
  ou 8 morceaux de ~1 166 octets) à tous. Si ces messages passent dans le même canal fiable et ordonné
  que les commandes de jeu, ils peuvent retarder les ordres des autres joueurs. Ce serait un lag
  d'actions, pas d'images, et ces mesures ne le voient pas.

### Hypothèse : le broadcast bloque-t-il le jeu quand le ping est élevé ?

- **Statique.** `140277410` ne fait qu'appeler la méthode `+0xb8` de la session avec le drapeau fiable
  (`(msg, 1, 0)`). D'après [docs/netcode](../docs/netcode/README.md), l'envoi réel revient au thread de
  transport. Rien n'attend de réponse : `NetPlayerInfo` n'a pas d'accusé de réception que le thread
  principal attendrait.
- **Session du 23:21.** Les 20 broadcasts locaux partent tous du thread principal, et aucun n'est suivi
  d'une image de 50 ms ou plus dans la seconde. Le message revient à son émetteur en 10 ms, soit à
  l'image suivante. À ce ping (non mesuré), l'envoi ne bloque pas.
- **Ce qui pourrait changer avec un ping élevé.** Un verrou de la file de messages tenu par le thread
  de transport pendant un envoi lent ferait attendre le thread principal. Sinon, ce serait un retard des
  ordres de jeu derrière les 7 à 9 Ko de configuration dans le canal fiable, mais cela retarde les
  actions, pas l'image. Rien de tout cela n'est observé.
- **Test.** `hook_epingles.js` mesure désormais la durée de `BroadcastPlayerInfo` (`ms`). À refaire avec
  de la latence ajoutée sur une des machines, ou avec un joueur lointain : si la durée de l'appel ou les
  gels qui suivent grimpent avec le ping, l'hypothèse tient.

**Test avec 500 ms de latence ajoutée (clumsy, UDP et ICMP, dans les deux sens), 23:43–23:46.**
Session `map_pins/runs/20261030-234301`, de 46 à 75 épingles (0:29→49, 2:17→26).
- Les 10 `BroadcastPlayerInfo` locaux durent 0 à 1 ms. L'appel ne bloque pas.
- Aucune image de 50 ms ou plus dans la seconde qui suit les broadcasts. Après les 64
  `NetPlayerInfo` reçus, le pire gel est de 93 ms, et aucun n'atteint 100 ms.
- Rien de visuel ressenti en jeu.
- Le message qui revient à son émetteur arrive toujours en 10 ms : il ne passe pas par le réseau, ce
  qui ne dit donc rien de la latence. La latence ajoutée a été vérifiée par `ping`.

**L'hypothèse du ping est écartée**, du moins à deux joueurs. Le coût Lua continue de croître avec
les épingles : environ 21 ms par diffusion à 75 épingles, dont 13 ms pour la liste MPH ;
`GetMapPins` passe de 324 à 527 appels par reconstruction.

## Corrections : `ui_epingles=on` (proxy, actif par défaut)

Le lag n'a pas été reproduit, mais ce travail est inutile et ne coûte rien à retirer. Le proxy ajoute
[proxy/src/ui_epingles_opti.lua](../proxy/src/ui_epingles_opti.lua) à la fin de `MapPinManager.lua`
(et de ses versions DMT ou CQUI) et de `MapPinListPanel.lua` (base ou MPH), avant le bloc de mesure
s'il est actif. Rien ne change sur le réseau, et les autres joueurs n'ont rien à installer.

- **Pas de reconstruction quand rien n'a changé.** `OnPlayerInfoChanged` calcule une signature de ce
  que les drapeaux, ou la liste, affichent : pour chaque joueur concerné et chaque épingle, la
  position, le nom, l'icône et la visibilité pour le joueur local. Si elle n'a pas changé depuis la
  dernière reconstruction, on ne fait rien. Cela couvre les diffusions qui ne touchent aucune épingle
  (votes, pause et contrôle des déconnexions de MPH) et les doublons identiques. Le coût est un passage
  sur les épingles, au lieu de tout détruire et recréer.
- **Une lecture de `GetMapPins()` par joueur et par reconstruction.** Pendant `Refresh`,
  `MapPinFlag.GetMapPin` lit la table du joueur dans un cache vidé à la fin, même en cas d'erreur. On
  passe d'environ 7 lectures par drapeau à une par joueur, de `n²` à `n`. Hors `Refresh`, rien ne
  change.
- **La liste fermée attend son ouverture.** Le panneau est masqué par son conteneur dans
  MinimapPanel (`/InGame/MinimapPanel/MapPinListPanel`). Fermé, un changement pose une mise à jour par
  image (`ContextPtr:SetUpdate`), qui reconstruit la liste dès qu'il s'ouvre puis se retire. Si ce
  contrôle est introuvable, seule la signature s'applique.
- **Non retenu.** Le bandeau des dirigeants : savoir de façon fiable ce qui a changé dans la
  configuration est difficile, pour un gain de 3,5 ms. La double diffusion : le jeu de base ne diffuse
  qu'une fois. DMT, absent de nos parties : il faudrait mémoriser la table désérialisée tant que
  `GetValue` renvoie la même chaîne.

Tests hors jeu : `lua proxy/tests/test_ui_epingles_opti.lua`. Ils couvrent le saut, la reconstruction
après un nom, un ajout ou une visibilité changés, le cache, l'erreur dans `Refresh`, le double
chargement base puis mod, la liste fermée puis ouverte, et la mesure par-dessus. En jeu, `Lua.log`
doit montrer `EpinglesOpti <fichier> actif`, et le bloc de mesure doit voir moins de
reconstructions et moins d'appels à `GetMapPins`.

## Épingles aux bords de la carte : `ui_carte_entiere=on`

Une partie des épingles, et donc des diffusions, n'a qu'un but : en début de partie, des joueurs en
posent tout en haut et tout en bas de la carte pour débloquer la caméra et donner à la minimap sa
taille finale. L'exe borne en effet la caméra et la minimap au rectangle des cases révélées,
**épingles visibles du joueur comprises** (les siennes et celles qu'on lui partage). Le proxy étend
ce rectangle à la carte entière dès le premier tour, ce qui rend ces épingles inutiles. Analyse et
correctif : [minimap/README.md](../minimap/README.md).

## Tests en direct

### Les outils

- **Côté Lua, dans le proxy** : `ui_mesure_epingles=on` (`install.ps1 -UiMesureEpingles on`) ajoute
  [proxy/src/ui_epingles.lua](../proxy/src/ui_epingles.lua) à la fin de `MapPinManager.lua` (et de
  ses versions DMT ou CQUI), `MapPinListPanel.lua`, `DiplomacyRibbon*.lua` et `dmt_yieldcalculator.lua`.
  - Les gestionnaires de `PlayerInfoChanged` de ces contextes sont enveloppés, ainsi que
    `UpdateLeaders` et `UpdatePinYields` de DMT. Chaque appel externe écrit une ligne dans `Lua.log` :

    ```
    MesureEpingles  <heure>  <os.clock>  <contexte>  <racine>  arg=<joueur>  ms=<durée>  <détail>  epingles=<id:n,…>  dmt_octets=<id:taille,…>
    ```

  - Le détail donne, pour chaque fonction mesurée appelée dedans, le nombre d'appels et la durée :
    `Refresh`, `MapPinFlag.Refresh`, `BuildMapPinList`, `deserialize` (avec les octets compilés),
    `BroadcastMapPinSubjects`, et le nombre d'appels à `GetMapPins`.
  - `Network` est en lecture seule dans le jeu : les appels à `BroadcastPlayerInfo` ne se comptent
    que côté natif (Frida ci-dessous). Tout le bloc s'exécute sous `pcall`. S'il échoue, il écrit
    `MesureEpingles <fichier> echec <erreur>` et laisse le script hôte intact. Une première version, qui
    enveloppait `Network.BroadcastPlayerInfo`, empêchait MapPinManager de se charger.
  - Le joueur qui pose l'épingle mesure sa propre reconstruction ; il faut le proxy chez **les autres**
    pour mesurer la leur.
  - Test hors jeu du bloc : `lua proxy/tests/test_ui_epingles.lua`.
- **Côté natif, Frida** ([hook_epingles.js](hook_epingles.js) / [hook_epingles.py](hook_epingles.py),
  DX12 seulement) :
  - écrit chaque `BroadcastPlayerInfo` local ;
  - écrit chaque `NetPlayerInfo` reçu, avec sa taille en octets (lecture du flux) et la durée de son
    traitement C++ ;
  - **images** : écart entre deux `PeekMessageW/A` d'un même thread. La boucle principale vide la file
    de messages à chaque tour, et cela marche en DX11 comme en DX12. Chaque écart de 50 ms ou plus est
    écrit (`image_longue`), avec un bilan par thread toutes les 5 s (`images`). Le résumé relève, pour
    chaque `NetPlayerInfo` reçu, le gel le plus long du thread principal dans la seconde qui suit, et le
    compare aux images longues du reste de la session. Ce gel couvre tout ce qui se passe sur ce
    thread, y compris ce que le Lua ne voit pas (mise en page, rendu) ;
  - à l'arrêt, recopie les lignes `MesureEpingles` de `Lua.log` dans le dossier de la session et
    affiche un résumé par racine (appels, moyenne, maximum, total).

```bash
powershell -ExecutionPolicy Bypass -File proxy/install.ps1 -UiMesureEpingles on
```

```bash
.venv/Scripts/python.exe -I map_pins/hook_epingles.py
```

Résultats : `map_pins/runs/<date-heure>/` (`frida.log`, `lua.tsv`, `resume.txt`, non versionnés).

### Protocole

1. Proxy installé avec `-UiMesureEpingles on` sur la machine qui mesure ; jeu en DX12 pour la partie
   native.
2. Partie multijoueur à deux humains au moins, avec les mods habituels (DMT, MPH). Lancer
   `hook_epingles.py`.
3. **Série A, référence** : l'autre joueur pose, déplace puis supprime une épingle, dix fois de suite,
   avec peu d'épingles (moins de 10 par joueur).
4. **Série B** : on monte à 50, puis à 150 épingles par joueur (`WorldInput` en mode épingle, ou
   l'outil de liste) et on recommence. Si la durée croît comme le carré du nombre d'épingles, la cause
   2 est confirmée.
5. **Série C, sans DMT** (si possible) : mêmes nombres d'épingles. L'écart entre B et C chiffre la
   cause 1 ; `dmt_octets` et `deserialize=…o` donnent la taille compilée.
6. **Passage de tour** : finir deux ou trois tours et compter les `BroadcastPlayerInfo` et les
   `PlayerInfoChanged` sans aucune épingle posée (cause 4 : DMT, MPH).
7. Ctrl+C sur `hook_epingles.py`, puis archiver `Lua.log` et `net_message_debug.log`
   (`netcode/collect_logs.ps1`).

À regarder ensuite : la durée de `PlayerInfoChanged` dans `mappinmanager*.lua`, comparée à celle de
`mappinlistpanel.lua` et de `UpdateLeaders` ; le rapport `GetMapPins` / nombre d'épingles ; la taille
des `NetPlayerInfo` reçus. Si les durées Lua restent faibles alors que le gel est réel, il faudra une
capture WPR pendant une pose d'épingle (outils de [rollover](../rollover/README.md)).
