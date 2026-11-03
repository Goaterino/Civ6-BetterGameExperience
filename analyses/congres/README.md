# Congrès mondial : propositions divergentes, tours passés, votes ignorés

Analyse préliminaire (2026-11-02), statique seulement : scripts du jeu (Gathering Storm), MPH 1.8.0,
BBG 7.5.1, décompilation de `GameCore_XP2_FinalRelease.dll` (projet Ghidra `civ6xp2`) et de
`CivilizationVI_DX12.exe` (projet `civ6exe`). Aucune partie avec congrès n'a encore été capturée :
tout ce qui est marqué **hypothèse** reste à confirmer par les mesures de la fin du document.

Les trois symptômes rapportés en multijoueur (tours simultanés) :
1. certains joueurs voient un triplet de résolutions différent de celui de l'hôte ;
2. certains joueurs voient leur tour passé contre leur gré, sans avoir pu voter ;
3. certains joueurs voient leur vote ignoré, comme s'ils n'avaient pas voté.

Hachages : un type Civ6 se hache en `~CRC32(nom)` (`DB.MakeHash`). Exemples utiles :
`TURNSEG_WORLDCONGRESS_1` = `b52a667b`, `TURNSEG_WORLDCONGRESS_2` = `2c2337c1`,
`TURNMODE_SIMULTANEOUS` = `fd4af2d1`, `TURNTIMER_NONE` = `a5196dab`, `TURNTIMER_DYNAMIC` = `29a4dc64`,
`WORLD_CONGRESS_RESOLUTION_VOTE` = `b0cfd998`, `WORLD_CONGRESS_SUBMIT_TURN` = `35239b93`.

## Le congrès est un segment de tour

`DLC/Expansion2/Data/Expansion2_TurnPhases.xml` ajoute trois segments, joués **après** le segment
normal (`TURNSEG_SINGLEPHASE`, `PhaseOrder` 0) du même tour :

| segment | `PhaseOrder` | rôle |
|---|---|---|
| `TURNSEG_WORLDCONGRESS_1` | 1 | session ordinaire : résolutions et discussions |
| `TURNSEG_WORLDCONGRESS_2` | 2 | session extraordinaire : urgences |
| `TURNSEG_WORLDCONGRESS_RESOLUTION` | 3 | résultats |

Les trois ont `AllowTurnUnready="false"` et `TimeLimit_Base="60"`. Un joueur entre dans un segment
comme dans un tour : il est « actif », il le termine par `PlayerTurnComplete`, et le segment se clôt
quand tous les humains l'ont terminé.

## Flux d'appels d'une session ordinaire

### GameCore (simulation, exécutée par chaque machine en lockstep)

| adresse | rôle |
|---|---|
| `18005b680(h)` | vrai si le type du segment courant vaut `h` (`DAT_180b617c0` → ligne `TurnSegments`, `+0x60` → `+0x18`) |
| `1803be2e0` | **début de session** (segment `WORLDCONGRESS_1`) : tirage des résolutions, puis `1803c7d90` pour chacune |
| `1803c7d90` | options de chaque résolution, **par joueur** : appelle l'événement Lua `WC_Validate_*` de la résolution pour chaque joueur et range la liste `ResolutionOptions` |
| `18030d330` | exécuteur des `PlayerOperation` (appelé par le message `PlayerOperation`, `180038c80`) |
| `1803c0ba0` | `WORLD_CONGRESS_RESOLUTION_VOTE` : votes et coût en faveur |
| `18030a100`… `18030bc10` | même opération, choix A/B et cible, un setter par type de cible (`PlayerType`, ressource, district…) |
| `1803c0d40` | `WORLD_CONGRESS_DISCUSSION_VOTE` (propositions, urgences) |
| `18030d330`, branche `35239b93` | `WORLD_CONGRESS_SUBMIT_TURN` : écrit le tour courant en `+0x2cc` du congrès du joueur (lève le blocage de fin de tour) |
| `18030d330`, branche `1b75025a` | `WORLD_CONGRESS_LOOKED_AT_AVAILABLE` : idem en `+0x2c8` |
| `1803bf880` | **fin de session** (toujours dans `WORLDCONGRESS_1`) : `1803c0190`, puis dépouillement `1803c5f50` de chaque résolution et application |
| `180039920` | message `PlayerTurnComplete` |
| `180039fb0` | message `PlayerTurnUnready` |
| `1802f76a0` | `CanUnreadyTurn` |

L'état du congrès est l'archive synchronisée `WorldCongress` (constructeur `1803b9bb0`) :
`m_iAccumulateStartVotes`, `m_iLastNormalSession`, `m_iLastSpecialSession`, `m_iEstablishedTurns`,
`m_Resolutions` (`+0x108`, données en `+0x118`, nombre en `+0x128`, éléments de `0x218` octets),
`m_ResolutionsForInjection`, `m_Discussions` (`+0x158`), `m_LastResolutionsOfTypes` (`+0x180`).
Une divergence de ces champs entre machines apparaît donc au contrôle de synchro de fin de tour
(`AutoArchive=WorldCongress` dans `net_message_debug.log`, voir [docs/netcode](../../docs/netcode/README.md)).

### Interface (Lua, locale à chaque machine)

`DLC/Expansion2/UI/Additions/` :
1. `Events.WorldCongressStage1` → `WorldCongressIntro.lua` affiche l'intro ; sa fermeture (bouton,
   Échap, Entrée **ou l'action `EndTurn`**) lance `LuaEvents.WorldCongressIntro_ShowWorldCongress`.
2. `WorldCongressPopup.lua` : `OnShowFromIntro` → `SetupWorldCongress(stage, true)` → `SetStage` →
   `SetPhase(1)` → `PopulateResolutions` lit `Game.GetWorldCongress():GetResolutions(joueur local)`
   (lecture du **cache d'interface**, `Cache_Lua_IWorldCongress.cpp`, `1800ac5b0`, pas de GameCore).
   **Si l'interface ne trouve ni résolution ni proposition (`HasChoices()` faux), `SetPhase(1)` envoie
   aussitôt `WORLD_CONGRESS_SUBMIT_TURN` puis `ACTION_ENDTURN`.**
3. `OnAccept` envoie, dans l'ordre : un `WORLD_CONGRESS_RESOLUTION_VOTE` par résolution qui a des
   votes (type, nombre, choix A/B, indice de cible `target - 1`), un `WORLD_CONGRESS_DISCUSSION_VOTE`
   par proposition, `WORLD_CONGRESS_SUBMIT_TURN`, puis `ACTION_ENDTURN`. Il affiche ensuite
   `WorldCongressBetweenTurns.lua`, qui n'a aucun bouton pour revenir au vote.
4. `Events.LocalPlayerTurnEnd` → `OnLocalPlayerTurnEnd` **efface les votes en cours et ferme la
   fenêtre**.

`ActionPanel_Expansion2.lua` bloque la fin de tour automatique pendant le congrès par
`m_CongressIsInSession`, rafraîchi seulement dans `LateInitialize` et `Events.TurnBegin`.

### Exe (minuteur et annulation)

| adresse | rôle |
|---|---|
| `1400f8170` | minuteur : si le temps du **joueur local** est écoulé, journalise `Turn Timer Sending TurnComplete, turnTimeLimit: %u, elapsedTurnTime: %f` et envoie son `PlayerTurnComplete` |
| `1400f4d50` | durée limite : calculée par GameCore (segment) si `TURNTIMER_DYNAMIC`, sinon la valeur **locale** de `TURN_TIMER_TIME` |
| `1400f4df0` | temps écoulé, horloge locale de la machine |
| `1400f77f0` | `SendTurnComplete` (refusé si le joueur n'est pas actif localement) |
| `1401c9c10` | `AppRequestTurnUnready` : refuse une demande venue de l'interface (`d60b7e54`) si le temps du joueur est écoulé, accepte sinon |

Chaque machine n'envoie de `TurnComplete` que pour son joueur local : l'hôte ne force jamais la fin
du tour d'un autre.

## Bug 1 : triplet différent de celui de l'hôte

### Comment le triplet est tiré (`1803be2e0`)

1. Les résolutions de la session précédente sont recopiées depuis `m_Resolutions`, puis la liste est
   vidée.
2. Candidats : toutes les lignes de `Resolutions`, filtrées par l'ère (`EarliestEra`/`LatestEra`
   contre l'ère du monde), puis par l'événement Lua **`GameEvents.CanUseResolutions`**
   (`Scripts/WorldCongress.lua` : religion mondiale s'il existe une religion, victoire diplomatique
   si elle est activée, luxe s'il y a des luxes sur la carte). Le masque est initialisé à « tout
   permis » : **si l'appel Lua échoue sur une machine, elle garde toutes les résolutions**.
3. Les candidats sont répartis en deux listes selon le bit 0 de `+0xb8` du type de résolution.
4. Première liste : 3 tirages sans remise par l'aléa synchronisé (`1800e4190`, aléa de l'archive en
   `DAT_180b8aa68+0xe0`, étiquette `"World Congress Resolutions"`). Le premier tiré est écarté s'il
   figurait à la session précédente ; on garde les deux premiers acceptés.
5. Seconde liste : un tirage de plus. D'où **trois résolutions**.
6. `1803c7d90` calcule ensuite, pour chaque résolution et chaque joueur, la liste des cibles possibles
   par les gestionnaires Lua `WC_Validate_*`.

L'aléa est synchronisé : deux machines ne tirent autrement que si leurs **entrées** diffèrent
(candidats, résultat Lua, état de l'aléa, liste de la session précédente).

### Hypothèses, de la plus à la moins probable

- **H1a, affichage périmé.** L'interface ne lit pas GameCore mais son cache. Si la fenêtre se remplit
  avant que le cache ait reçu la nouvelle session, le joueur voit l'ancien triplet, ou rien (voir
  H2b). Rien ne diverge dans la simulation ; le contrôle de synchro ne signale rien.
- **H1b, simulation divergente.** Une machine tire un autre triplet. Causes possibles :
  - `CanUseResolutions` qui échoue ou répond autrement sur une seule machine (erreur Lua, gestionnaire
    absent, état Lua différent après une reconnexion) ;
  - un tirage de l'aléa synchronisé fait par une seule machine plus tôt dans le tour (aucun trouvé dans
    MPH ni BBG : leurs tirages sont dans des scripts de gameplay, exécutés partout) ;
  - un revenant qui a rechargé la partie : on sait déjà que `AI::CultureAi.m_resolutionAnalyzers`,
    l'analyse des résolutions par l'IA, diffère après un rechargement
    ([docs/netcode](../../docs/netcode/README.md#test-solo-du-2026-10-29--état-en-mémoire-contre-état-rechargé)).

  Signe distinctif : `AutoArchive=WorldCongress` dans les écarts de fin de tour, et des votes ignorés
  (bug 3) pour qui a voté sur une résolution que l'hôte n'a pas.
- **H1c, mêmes résolutions, cibles différentes.** Les listes de cibles viennent de Lua, par machine.
  `WC_Validate_LuxuryBan` met la liste des luxes en cache au **premier** appel de la session Lua, et ne
  la recalcule jamais : un joueur qui a rechargé la partie la calcule à un autre moment. Le vote
  envoie un **indice** dans cette liste (`target - 1`) : un même indice désigne alors deux luxes
  différents selon la machine.

## Bug 2 : tour passé contre son gré

Il n'y a aucun contrôle de segment dans `PlayerTurnComplete` (`180039920`) : le message ne porte que
`ePlayer` et une raison, ni tour ni segment, et n'est refusé que si le joueur a déjà fini ou si le jeu
est en pause. Tout `TurnComplete` qui arrive pendant le congrès termine donc le congrès du joueur.
Sources possibles :

- **H2a, minuteur local.** Le minuteur de l'exe (`1400f8170`) envoie le `TurnComplete` du joueur local
  quand **son** temps est écoulé, avec sa valeur **locale** de `TURN_TIMER_TIME` (MPH règle le type
  `TURNTIMER_STANDARD`). Or MPH (`ui/Additions/TurnProcessing.lua`) écrit cette valeur sur chaque
  machine, sans la diffuser :
  - 60 s tant que tous les humains n'ont pas commencé leur tour (`OnTick`), puis la valeur calculée ;
  - le minuteur de 240 s prévu pour le congrès (`SmartTimer`) ne s'applique jamais :
    `WORLD_CONGRESS_STAGE_1` et `WORLD_CONGRESS_STAGE_2` n'y sont pas définis (`nil`), la comparaison
    avec `Game.GetCurrentTurnSegment()` est toujours fausse. Le congrès hérite du minuteur du tour,
    calculé à `Events.TurnEnd`.

  Reste à savoir si le temps écoulé repart de zéro au début du segment du congrès : l'exe remet le
  minuteur à zéro sur des événements internes (`1400f55a0` enregistre les rappels), sans qu'on sache
  encore lesquels correspondent à un changement de segment. Si ce n'est pas le cas, un joueur qui a
  fini son tour tard arrive au congrès avec un minuteur déjà écoulé.
- **H2b, envoi automatique de l'interface.** `SetPhase(1)` termine le tour du joueur s'il ne voit
  aucun choix, par exemple si le cache est vide ou périmé à l'ouverture (H1a).
- **H2c, fin de tour automatique.** Pour un joueur qui a l'option « Fin de tour automatique »,
  `CheckAutoEndTurn` envoie `ACTION_ENDTURN` dès qu'aucun blocage n'existe. Le garde-fou du congrès,
  `m_CongressIsInSession`, n'est relu qu'à `Events.TurnBegin`, c'est-à-dire au début du tour, avant
  le segment du congrès : il vaut probablement `false` pendant la session. Il suffit alors que le
  blocage `ENDTURN_BLOCKING_WORLD_CONGRESS_SESSION` manque un instant (avant son ajout au début du
  segment) pour que le tour parte. Cela expliquerait que seuls « certains » joueurs soient touchés.
- **H2d, `TurnComplete` en double.** Le dernier joueur à finir le tour normal envoie deux
  `TurnComplete` (double appui sur Fin de tour, ou clic au moment où son minuteur expire). Le premier
  clôt le segment normal ; le second, traité après le changement de segment, clôt son congrès.
- **H2e, raccourci `EndTurn` pendant l'intro.** L'intro ferme sur l'action `EndTurn`, que
  l'ActionPanel reçoit aussi (`OnInputActionTriggered` → `DoEndTurn`). Sans blocage à cet instant,
  c'est un `ACTION_ENDTURN`.

## Bug 3 : vote ignoré

`1803c0ba0` ne retient un vote que si **toutes** ces conditions tiennent, sinon il l'ignore sans rien
dire :
- le type de résolution figure dans `m_Resolutions` de **cette** machine ;
- le segment courant est encore `WORLDCONGRESS_1` (`1803cf8f0`) ;
- le joueur est valide.

Le nombre de votes est ensuite borné par ce que la faveur du joueur permet **au moment de
l'exécution** (`180371500`), la faveur est débitée, et `1803d0a40` **remplace** le nombre de votes
du joueur sur cette résolution (un nombre négatif devient 0). Le choix A/B et la cible passent par
les setters `18030a100`… avec l'indice de cible envoyé par l'interface.

Causes possibles :
- **H3a** : le vote porte sur une résolution que la machine n'a pas (H1a, H1b) ;
- **H3b** : le tour du joueur a été passé avant qu'il valide (bug 2). `OnLocalPlayerTurnEnd` efface
  alors ses votes et ferme la fenêtre : rien n'est envoyé. Si les votes partent quand même, ils
  arrivent hors du segment et sont ignorés ;
- **H3c** : faveur plus basse côté GameCore que ce qu'affiche le cache : votes rabotés, jusqu'à zéro ;
- **H3d** : indice de cible qui désigne une autre cible sur une autre machine (H1c) : le vote compte,
  mais pas pour ce que le joueur a choisi ;
- **H3e** : votes des IA calculés à partir de `m_resolutionAnalyzers`, qui diffère après un
  rechargement : le résultat de la session peut différer d'une machine à l'autre.

**Preuve directe disponible sans outil** : chaque vote retenu est journalisé par `1803cf9d0` dans
`Logs/World_Congress.csv` (`tour, VOTES, résolution, joueur, votes`). Une ligne absente pour un joueur
qui a voté = vote rejeté par GameCore ; une ligne présente chez l'hôte et absente chez un client =
simulation divergente.

## Sortir de l'état « tour passé, a voté »

### Ce qui bloque aujourd'hui

| couche | condition |
|---|---|
| données | `AllowTurnUnready="false"` sur les trois segments du congrès |
| GameCore, `18005b640` | lit ce drapeau pour le segment courant (bit 2 de `+0x50` de la ligne `TurnSegments`). Appelé par `18005d870` dans la demande locale (`180036e90`, type `d60b7e54`) et dans le traitement du message `PlayerTurnUnready` (`180039fb0`, type `0a99df6c`) |
| GameCore, `1802f76a0` | `CanUnreadyTurn` : joueur plus actif, humain et vivant, segment en `TURNMODE_SIMULTANEOUS`, pas de drapeau `DAT_180b8ae30[joueur]`, et `DAT_180b8ab20` faux (sans doute « fin de tour de jeu en cours », c'est-à-dire plus aucun joueur actif) |
| exe, `1401c9c10` | refuse une demande de l'interface si le temps du joueur est écoulé |
| interface | aucun bouton dans `WorldCongressBetweenTurns` ; la fenêtre de vote ne sait pas se rouvrir après `OnAccept` |

`Player:CanUnreadyTurn()` (Lua) appelle `1802f76a0` **sans** regarder `AllowTurnUnready` : pendant le
congrès il peut répondre vrai alors que la demande sera refusée.

Les conditions voulues (temps restant, au moins un autre joueur encore actif) sont donc déjà celles
du jeu ; seul le drapeau du segment et l'interface manquent.

### Piste de mise en œuvre

1. **Lever le drapeau** pour `WORLDCONGRESS_1` et `WORLDCONGRESS_2` : détour de `18005b640` dans le
   proxy GameCore, ou modification de la ligne `TurnSegments` au chargement. Le message
   `PlayerTurnUnready` est traité par toutes les machines, qui consultent chacune leur drapeau :
   **correctif POL** (tous les joueurs avec le proxy, même version), sinon désynchro.
2. **Faveur.** Revoter appelle de nouveau `1803c0ba0`, qui débite la faveur une seconde fois et
   remplace le nombre de votes. Deux options :
   - n'autoriser le retour qu'à un joueur qui n'a envoyé aucun vote dans cette session (le cas du
     bug 2) : rien à changer dans la simulation ;
   - ou rembourser, dans un détour de `1803c0ba0`, le coût des votes déjà comptés pour ce joueur
     avant d'appliquer les nouveaux (simulation modifiée, POL).
3. **Interface**, par le proxy (accroche de `luaL_loadbuffer`, comme `ui_merveilles.lua`) :
   - un bouton « Revenir au vote » dans `WorldCongressBetweenTurns`, actif si `CanUnreadyTurn()`, le
     temps restant et le drapeau le permettent, qui envoie `ACTION_UNREADYTURN` ;
   - sur `Events.LocalPlayerTurnUnready`, rouvrir `WorldCongressPopup` à l'étape en cours :
     `m_HasAccepted` et `m_CurrentStage` sont des locales du script, il faut donc patcher le script
     lui-même (le remettre à `SetupWorldCongress(stage, true)`).
4. Réglage dans l'onglet BetterGameDLLs si c'est un choix de préférence ; s'il n'est vu que comme
   la correction du bug 2, pas de réglage (voir la règle des correctifs POL).

## Corrections envisageables, par bug

| bug | correction | nature |
|---|---|---|
| 2 (H2a) | minuteur du congrès réellement appliqué et identique partout : corriger les constantes de MPH (`DB.MakeHash` dans `TurnProcessing.lua`), ou le faire depuis le proxy en patchant ce script | interface, local |
| 2 (H2b) | ne jamais terminer le tour automatiquement dans `SetPhase(1)` ; relire `GetResolutions` après un délai si le cache est vide | interface, local |
| 2 (H2c) | relire `IsInSession()` dans `AllowAutoEndTurn_Expansion` au lieu de la variable figée | interface, local |
| 2 (H2d, H2e) | refuser dans le détour de `PlayerTurnComplete` un `TurnComplete` arrivé dans les premières secondes d'un segment de congrès pour un joueur qui n'y a encore rien fait ; ou rendre l'annulation possible (section précédente) | POL |
| 3 (H3a, H3b) | dépend des bugs 1 et 2 ; au minimum, prévenir le joueur quand son vote n'apparaît pas dans `GetResolutions` | interface |
| 1 (H1c) | ne plus mettre en cache la liste des luxes de `WC_Validate_LuxuryBan` (script de gameplay : mod ou patch, identique pour tous) | POL |

## Mesures à faire

À la prochaine partie qui passe un congrès (ou depuis une sauvegarde du tour qui précède une
session), copier **avant de relancer le jeu**, sur l'hôte et sur chaque joueur touché,
`%LOCALAPPDATA%\Firaxis Games\Sid Meier's Civilization VI\Logs\` :
- `World_Congress.csv` : votes retenus, par machine (bug 3) ;
- `net_message_debug.log` : `Turn Timer Sending TurnComplete` et ses valeurs (H2a),
  `PlayerTurnComplete` en double autour du changement de segment (H2d),
  `AppRequestTurnUnready`, et `AutoArchive=WorldCongress` dans les écarts (H1b) ;
- `Lua.log` : erreurs Lua des scripts du congrès (H1b), `MPH Track Local State` ;
- une capture d'écran du triplet chez chaque joueur, avec l'heure.

Instrumentation à ajouter au proxy, en mode diagnostic :
- journaliser à la sortie de `1803be2e0` le contenu de `m_Resolutions` et le résultat de
  `CanUseResolutions`, et, pour chaque `WORLD_CONGRESS_*` reçu, le segment courant et la décision de
  `1803c0ba0` ;
- côté interface, journaliser ce que `GetResolutions` renvoie à l'ouverture de la fenêtre, et chaque
  passage par l'envoi automatique de `SetPhase(1)`.

Points statiques encore ouverts :
- l'ordre exact des événements au passage du segment normal au congrès (`TurnBegin`,
  `LocalPlayerTurnBegin`, ajout du blocage), et l'événement qui remet le minuteur de l'exe à zéro ;
- le sens de `DAT_180b8ae30[joueur]` et de `DAT_180b8ab20` dans `CanUnreadyTurn` ;
- si les listes de cibles par joueur (`+0x208` de chaque résolution) font partie de l'archive
  synchronisée.
