# Salon multijoueur : création, config, presets et changements d'emplacement

Trois questions :
1. quelle chaîne d'appels crée un salon, et sous quel format circule sa config, pour pouvoir plus
   tard l'automatiser (créer et régler un salon d'un clic, par exemple depuis un civdraft terminé) ;
2. quels réglages s'appliquent ou se transmettent mal aux autres joueurs ;
3. comment fonctionne, réseau compris, un changement d'emplacement (swap), en particulier pendant
   qu'un autre joueur rejoint.

Analyse statique du 2026-11-04 :
- `CivilizationVI_DX12.exe` 1.0.12.54 (951533), projet Ghidra `~/Tools/ghidra-projects/civ6exe`,
  base 0x140000000 ;
- Lua du front-end de Base et de **MPH** (Multiplayer Helper, Workshop 3307282026), qui remplace par
  `ImportFiles` `hostgame`, `stagingroom`, `setupparameters`, `gamesetuplogic`, `mainmenu` et
  `playertargetlogic`. Aucun autre mod installé, ni aucun DLC, ne remplace ces scripts ;
- journaux réels d'un salon du 2026-11-03 à 21:33 : hôte theflowgate, sauvegarde chargée, MPH preset
  9, 4 joueurs dont un swap de l'hôte. Copie dans `runs/20261103-lobby/` (non versionné).

Rien n'a encore été vérifié en jeu au-delà de ces journaux : voir [Protocole de test](#protocole-de-test).
GameCore n'intervient pas : tout le salon est dans l'exe et dans le Lua du front-end.

## Conclusions

1. **Le salon est piloté par l'hôte, mais rien ne l'impose côté réception.**
   - `Network.BroadcastGameConfig` ne fait **rien** chez un client.
   - `Network.BroadcastPlayerInfo(id)` ne part que pour son propre emplacement, ou depuis l'hôte.
   - À la réception, `NetGameConfig` et `NetPlayerInfo` sont appliqués **sans regarder l'émetteur**.
   - Un `NetPlayerInfo` recopie **tout** l'arbre de l'emplacement : civ, équipe, prêt, statut, nom
     réseau. Deux écritures concurrentes du même emplacement (l'hôte et le propriétaire, ou un
     message parti avant un swap) : **la dernière arrivée gagne, et l'ordre d'arrivée peut différer
     d'une machine à l'autre**.
2. **Le preset MPH est réappliqué par chaque machine, avec un état local** (`CheckPreset`,
   `hostgame.lua` l.958). `m_Preset` vaut -1 au démarrage de chaque machine, et `CheckPreset` tourne
   sur `GameConfigChanged` et à l'affichage de HostGame, y compris chez les clients (la staging room
   force l'affichage de HostGame, l.7284). Effets :
   - chez un client qui rejoint, les listes `EXCLUDE_NATURAL_WONDERS` / `EXCLUDE_CITY_STATES` et
     `CITY_STATE_COUNT` sont remplacées par celles du preset, ce qui écrase une personnalisation de
     l'hôte. Le `BroadcastGameConfig` qui suit est sans effet chez un client : la config du client
     **diverge en silence** de celle de l'hôte ;
   - cette divergence devient la vérité si ce client **devient hôte** (migration d'hôte dans le salon) ;
   - chez l'hôte, le preset écrase aussi les listes d'une config ou d'une sauvegarde chargée.
     Journal du 2026-11-03 : `CheckPreset() 9 -1` puis `Applied PPL Arena Settings` juste après le
     chargement de la sauvegarde.
3. **Un swap est une demande, que l'hôte exécute à sa prochaine mise à jour.**
   - `NetRequestChangePlayerID` part à tout le monde ; chaque machine note la demande dans
     `FNetPlayer+0x20`.
   - L'hôte l'accepte seulement si l'emplacement visé n'a pas de joueur réseau (ouvert, IA, fermé),
     ou si son occupant a demandé l'inverse (swap mutuel).
   - Il diffuse alors `NetChangePlayerId`, et **chaque machine** échange de son côté `NETWORK_NAME`,
     `READY_STATUS`, `MOD_READY_STATUS` et `SLOT_STATUS` des deux emplacements.
   - Une demande refusée **reste en attente sans limite**. Aucun événement Lua n'en signale l'échec.
4. **Swap pendant une arrivée : deux courses possibles** (à confirmer par le protocole) :
   - le nouveau venu reçoit la config complète de l'hôte à son enregistrement. Si l'hôte a déjà
     diffusé un `NetChangePlayerId` mais ne l'a pas encore appliqué chez lui (il le traite comme un
     message reçu, à la passe suivante), le nouveau venu reçoit la config **d'avant** le swap sans
     jamais recevoir le swap : il voit les deux emplacements inversés ;
   - un `NetPlayerInfo` émis avec l'ancien identifiant et arrivé après le swap est écrit dans le
     **mauvais emplacement**. Il y laisse le nom réseau et le statut `TAKEN` du joueur parti :
     emplacement fantôme.
5. **Créer un salon automatiquement est faisable en Lua seul** (voir [Automatisation](#pistes-pour-automatiser-la-création)) :
   - `Network.HostGame(serverType)`, puis les `GameConfiguration.SetValue` / `MapConfiguration.SetValue`
     et `PlayerConfigurations[id]:Set*` voulus ;
   - puis `Network.BroadcastGameConfig(true)` (les 64 emplacements) et un `BroadcastPlayerInfo(id)`
     par emplacement modifié, depuis l'hôte.

   La config est un arbre de `TypedVariantMap` dont les clés sont `~crc32(nom)`.

## Chaîne d'appels de la création d'un salon

### Côté Lua (MPH)

| étape | où | appels |
|---|---|---|
| 1 | `MainMenu` `OnInternet` / `OnLANGame` | `LuaEvents.ChangeMPLobbyMode(...)`, affiche `Lobby` |
| 2 | `Lobby.lua` `OnHostButtonClick` (l.340) | `LuaEvents.Lobby_RaiseHostGame()` |
| 3 | `hostgame.lua` `OnRaiseHostGame` (l.732) | `GameConfiguration.SetToDefaults(gameMode)`, `SetValue("RULESET", nil)` |
| 4 | `hostgame.lua` `OnShow` (l.441) | `CheckPreset()`, `RebuildPlayerParameters(true)`, `GameSetup_RefreshParameters()` : toutes les valeurs par défaut sont écrites, `MapSize_ValueChanged` fait `BroadcastGameConfig(true)` (`gamesetuplogic.lua` l.830) |
| 5 | réglages de l'hôte | `SetupParameters:SetParameterValue` → `Config_Write` (`GameConfiguration` / `MapConfiguration` / `PlayerConfigurations[id]:SetValue`) + cascade des `ConfigurationUpdates` (les 289 lignes de `MPH_PRESET` dans `Config.xml`), puis `BroadcastGameConfigChanges()` dans le pilote d'interface |
| 6 | `hostgame.lua` `OnConfirmClick` (l.534) → `HostGame` (l.555) | `Network.HostGame(serverType)` (`ServerTypeForMPLobbyType`) |
| 7 | événements moteur | `MultiplayerJoinRoomAttempt` → `JoiningRoom` ; `MultiplayerJoinRoomComplete` ; `MultiplayerJoinGameComplete` → `JoiningRoom_ShowStagingRoom` |
| 8 | `stagingroom.lua` `OnRaise` (l.7282) | `LuaEvents.StagingRoom_EnsureHostGame()` (affiche aussi HostGame : `OnShow`, donc `CheckPreset`), puis la staging room (`RealizeGameSetup`, `BuildPlayerList`) |

Variantes :
- **config `.Civ6Cfg`** : `OnLoadConfig` → `LoadGameMenu` → `Network.LoadGame(fichier, serverType)`
  → `Events.LoadComplete` → `SetToPreGame`, `RegenerateSeeds`, leaders remis à nil ;
- **sauvegarde** : `Lobby` `OnLoadButtonClick` → `SetToDefaults` → `Network.LeaveGame()`,
  `Network.LoadGame(...)`, puis la même suite qu'à l'étape 7.

Garde d'écriture `Config_CanWriteParameter` (`SetupParameters.lua` l.538) : en session, un client
n'écrit que ses propres paramètres de joueur ; l'hôte écrit ceux de la partie et de la carte, et ceux
des emplacements non humains. Les `GameConfiguration.SetValue` directs de MPH (`CheckPreset`,
draft par chat) contournent cette garde.

### Côté exe

Journal du 2026-11-03 (hôte, sauvegarde chargée) :

```
21:33:09  Attempting to host game.  ServerType: 2          NetworkManager::HostGame (14027de80)
21:33:09  Network Session Hosted. / PlayerConnected: iPlayerID = 0
21:33:10  JoinRoomComplete / MultiplayerJoinGameComplete, host finished hosting a game.
21:33:12  Net SEND (0): NetGameConfig (7 Ko)               premier BroadcastGameConfig, à soi seul
21:33:21  PlayerConnected 1 → NetWelcome, NetGameConfig complet (15 Ko) au seul joueur 1 (1402893f0),
          NetAppRegister(1) à tous
```

| Lua | liaison | ce qu'elle fait |
|---|---|---|
| `Network.HostGame(serverType)` | `14026bb50` → `14027de80` | journal `Attempting to host game`, création de la session (`14027e750`) avec le transport `140fe2238[serverType]` |
| `Network.BroadcastGameConfig([joueurs])` | `14026acc0` | **sans effet si l'on n'est pas l'hôte** (`14027eff0` = `IsGameHost`). Sinon `NetGameConfig` (`140265aa0`) à tous. Sans argument : `GAME_ROOT` et `MAP_ROOT` ; avec `true` : les 64 `PLAYER_n` en plus |
| `Network.BroadcastPlayerInfo([id])` | `14026ad60` | par défaut le joueur local ; part si `id` est le joueur local **ou** si l'on est l'hôte. `NetPlayerInfo` (`140265c90`) = l'arbre complet de `PLAYER_id` |
| `Network.RequestPlayerIDChange(id)` | `14026d1b0` → `14028ee60` | `NetRequestChangePlayerID(actuel = local, désiré = id)` à tous (`14080dcb0`) |
| `Network.GetChangePlayerID(id)` | `14026b190` | `FNetPlayer(id)+0x20` : la demande en attente, telle que **cette machine** la connaît |
| `Network.LaunchGame()` | `14026c850` → `140282e20` | hôte : `NetSnapshotRequest(raison 2)` à chaque joueur, puis chargement ou génération, et envoi de l'instantané |
| `Network.KickPlayer(id)` | `14026c690` | réservé à l'hôte |

La table `Network` (85 fonctions) est enregistrée par `14026d5a0`.

## Format de la config

**Arbre.** `TypedVariantMap` imbriquées, racines `GAME_ROOT`, `MAP_ROOT`, `PLAYER_0` à `PLAYER_63`.
Les valeurs sont `Int`, `UInt`, `UInt64`, `Bool`, `Char{…}` (chaînes, parfois du JSON de
localisation), des `TypedVariantMap` et des `TypedVariantVector`. Les listes Lua
(`EXCLUDE_NATURAL_WONDERS`…) deviennent des tables indexées 1, 2, 3…

**Clés.** Une clé est le hachage de son nom (`1407a2330`) : CRC32 réfléchi, polynôme `0xedb88320`,
valeur initiale `0xffffffff`, **sans XOR final**, soit `~zlib.crc32(nom)`. Vérifié sur
`MPH_PRESET` (1206098174 → 9 dans le journal) et `SpawnRecalculation`. L'exe ne connaît le nom que
des clés qu'il déclare. Les clés des mods (`MPH_PRESET`, `BSM_TOURNAMENT`, `BAN_*`, `SAILORGC_*`,
`BBM*`…) sont journalisées en `Unknown Variant Key(<hachage>)`. `gen_noms_cles.py` les renomme
d'après les fichiers du jeu et du Workshop : 153 des 179 clés du journal du 03/11
(`noms_cles.tsv`).

**Ce que voit Lua.** `GameConfiguration.GetValue(nom)` / `SetValue(nom, v)` hachent le nom.
`SetupParameters` lit et écrit par `ConfigurationId`, `Hash=1` stocke `DB.MakeHash(valeur)`, et
`Array=1` stocke une table de chaînes.

**Sections d'un `NetGameConfig`.** Le message emporte la liste des racines à appliquer (`msg+0x448`,
nombre en `msg+0x458`, remplie par `140100240`) : toujours `GAME_ROOT` et `MAP_ROOT`, plus les
`PLAYER_n` selon le mode (0 aucun, 1 les emplacements utilisés, 2 les 64). À la réception,
`1401035a0` ne rafraîchit que les racines listées.

**Journal tronqué.** `net_message_debug.log` coupe chaque ligne à 32 Ko, la taille du tampon passé à
`ToString`. Le `NetGameConfig` envoyé au nouveau venu dépasse 32 Ko en texte : on y perd la fin des
emplacements et toutes les clés de partie (dans le journal du 03/11, il s'arrête au milieu de
`PLAYER_12`). `hook_lobby.js` rappelle `ToString` avec un tampon de 4 Mo.

**Config des joueurs dans l'exe.** `140100a60()[0x5c]` : 64 configs de 0x40 octets, leur arbre en
`+0x18`.

| clé | lecture | écriture |
|---|---|---|
| `NETWORK_NAME` | `140115bd0` | `140117430` |
| `READY_STATUS` | `140115d30` | `140117680` |
| `MOD_READY_STATUS` | `140115b00` | `140117410` |
| `SLOT_STATUS` (OPEN 0, COMPUTER 1, CLOSED 2, TAKEN 3, OBSERVER 4) | `140115ed0` | `1401176a0` (ajuste aussi `HUMAN_REQUIRED`) |

## Propagation : les messages

Chaque message a une table virtuelle `[destructeur, ToString(this, tampon, taille), …, Process]`.
`Process` s'exécute à la réception sur chaque destinataire, l'émetteur compris : un message
« à tous » revient à soi.

| message | construit par | `Process` |
|---|---|---|
| `NetGameConfig` | `140265aa0` (Lua), `1402893f0` (au nouveau venu, les 64 emplacements), `140265940` (cloud) | `1402662d0` : applique les sections (`1401035a0`) puis émet `GameConfigChanged`. Aucun contrôle de l'émetteur |
| `NetPlayerInfo` | `140265c90` | `140266800` : recopie tout l'arbre `PLAYER_id` (`140116880`) puis émet `PlayerInfoChanged`. L'émetteur n'est contrôlé que pour PlayByCloud |
| `NetRequestChangePlayerID` | `14080dcb0` | `140810ae0` : `FNetPlayer(actuel)+0x20 = désiré`, rappel de session `+0x88` |
| `NetChangePlayerId` | `14080d370` | `14080f6a0` → `140812ca0` : déplace ou échange les `FNetPlayer`, met à jour les identifiants hôte et local, puis rappel `+0x98` = `PlayersSwapped` (`140274660`) |

**Libellés inversés dans le journal.** `NetChangePlayerId : newID=%d, oldID=%d` affiche l'**ancien**
identifiant sous `newID`. `newID=0, oldID=2` du 03/11 est un déplacement de 0 vers 2, cohérent avec
`NetRequestChangePlayerID : curID=0, desiredID=2` juste avant.

## Réglages qui s'appliquent ou se transmettent mal

Par ordre de gravité. **L1 à L4** sont tranchés par l'exe. **L5 à L9** viennent du Lua, avec leur
effet réel une fois connue la règle de l'exe.

**L1. Écritures concurrentes d'un emplacement : la dernière arrivée gagne, machine par machine.**
- **Cause.** `NetPlayerInfo` porte l'arbre entier de l'emplacement. L'hôte peut l'émettre pour
  n'importe quel emplacement, le propriétaire pour le sien.
- **Exemple.** L'hôte change l'équipe ou le statut d'un joueur pendant que celui-ci change de civ.
  - L'hôte reçoit le message du joueur après sa propre écriture : il garde la civ du joueur et perd
    son changement d'équipe.
  - Le joueur reçoit celui de l'hôte : il garde l'équipe de l'hôte et perd sa civ.
  - Un tiers suit l'ordre d'arrivée.
- **Pourquoi c'est sans issue.** Personne ne rediffuse pour réconcilier : chaque machine garde sa
  version jusqu'au prochain `BroadcastPlayerInfo` de cet emplacement.
- **Ce qui l'amplifie dans MPH :**
  - tout `GameConfigChanged` fait `SetLocalReady(false)` puis `BroadcastPlayerInfo()` sur chaque
    machine (`stagingroom.lua` l.465) ;
  - `GetNextID()` est appelé dans des `print` (l.1955, l.2037), ce qui ajoute des
    `BroadcastPlayerInfo` de l'hôte pour l'emplacement d'un humain distant (l.3020).

**L2. Preset MPH réappliqué localement** (conclusion 2).
- **Où.** `hostgame.lua` l.34, l.958-1031, l.1070.
- **Ce qui est sûr côté exe.** Le `BroadcastGameConfig` d'un client est sans effet : l'hôte et les
  autres clients ne voient pas la liste écrasée.
- **Effets :**
  - le client affiche une liste de merveilles et de cités-États différente de celle de l'hôte ;
  - le client prend le pouvoir avec sa version en cas de migration d'hôte ;
  - côté hôte, le preset écrase la liste d'une config ou d'une sauvegarde chargée (journal du 03/11).
- **Défauts annexes du même code :**
  - passer d'un preset 5, 8, 9 ou 10 à un autre ne vide pas `EXCLUDE_CITY_STATES` ;
  - le preset 7 n'a pas de branche ;
  - `OnUpdateUI()` est appelé sans argument et ne fait rien.
- **Hypothèse non vérifiée.** La génération de la carte repart de l'instantané de l'hôte : la carte
  elle-même ne devrait pas diverger.

**L2b. Preset « PPL Team » (9) : nombre de cités-États remis à 0.**
- **Ce que fait le preset.**
  - Ses `ConfigurationUpdates` (`Config.xml`) posent `MAP_SIZE = MAPSIZE_DUEL`.
  - `MapSize_ValueChanged` (`gamesetuplogic.lua` l.790) règle alors le nombre de cités-États sur
    l'intervalle de cette taille : 0 à 6, par défaut 3 (`MapSizes.xml`, idem BBM).
  - Puis `CheckPreset` écrit en dur `CITY_STATE_COUNT = 12` (`hostgame.lua` l.1017).
- **Pourquoi on tombe à 0.**
  - Au rafraîchissement suivant, `Parameter_SyncConfigurationValues` (`setupparameters.lua` l.1908)
    trouve 12 hors de l'intervalle [`MAP_MIN_MINOR_PLAYERS`, `MAP_MAX_MINOR_PLAYERS`] = [0, 6].
  - `CityStateCount` n'a pas de `DefaultValue` (`SetupParameters.xml` l.18) : la valeur retombe au
    minimum, **0**, et l'hôte l'écrit.
- **Même effet sur une carte Tiny** (maximum 10) : 12 cités-États y sont impossibles. La partie du
  2026-11-03 avait justement `MPH_PRESET=9`, `MAP_SIZE=MAPSIZE_TINY` et `CITY_STATE_COUNT=12`.
- **Le preset 8** écrit 3, qui tient dans l'intervalle Duel. Les autres presets n'écrivent pas ce
  nombre : ils ne font que le ramener à la valeur par défaut de leur taille de carte (12 en
  Standard, 15 en Large), à chaque fois que la taille est réécrite.
- **Remarque.** `DB.MakeHash` est le même hachage que les clés de config : `MAPSIZE_TINY` =
  -601637951, la valeur du journal.

**L3. Demande de swap orpheline.**
- **Cause.** Une demande refusée (emplacement pris par un humain qui n'a rien demandé) reste dans
  `FNetPlayer+0x20`. Elle n'est effacée que par une nouvelle demande ou par l'exécution du swap
  (`140814000` remet -1 des deux côtés).
- **Effet.** Si l'occupant part plus tard, l'hôte exécute le swap à sa mise à jour suivante. Le
  joueur change d'emplacement sans l'avoir redemandé.
- **Ce qui rend la demande invisible :**
  - un joueur arrivé après la demande ne la connaît pas : `GetChangePlayerID` y renvoie -1 ;
  - aucun Lua n'affiche les demandes en attente.

**L4. `NetPlayerInfo` écrit dans le mauvais emplacement après un swap** (conclusion 4) :
emplacement fantôme, avec le nom réseau du joueur et le statut `TAKEN`.

**L5. Mode anonyme : ping-pong de `NICK_NAME` entre l'hôte et chaque joueur d'une autre langue.**
- **Où.** `stagingroom.lua` l.5940 (`Anonymise_ID`).
- **Mécanisme.**
  - Chaque machine calcule `Locale.Lookup(GetLeaderName()) .. " - " .. id`, l'écrit et
    rediffuse s'il diffère.
  - L'exe ne laisse partir que l'hôte, pour tous les emplacements, et chaque joueur pour le sien.
  - Langues différentes : l'hôte impose son nom, le joueur le réécrit dans sa langue, l'hôte le
    réécrit, et ainsi de suite, à chaque `PlayerInfoChanged`.
- **Effet de bord.** Chaque aller-retour recopie aussi le reste de l'emplacement (L1).

**L6. Commandes de chat exécutées par tous les récepteurs.**
- **Où.** `stagingroom.lua` l.4078-4167.
- **Cause.** `b_ishost` teste l'**émetteur** (`fromPlayer == GetGameHostPlayerID()`), pas la machine
  locale.
- **Effet de chaque commande chez les clients :**
  - `.broadcast_game` : un `BroadcastGameConfig()` sans effet ;
  - `.broadcast_player_0` : `NICK_NAME="paf"` écrit localement sur l'emplacement 0, diffusion sans
    effet ;
  - `.spec` : l'hôte devient `LEADER_SPECTATOR` localement, puis `Network.LaunchGame()`. Ce
    lancement est sans effet chez un client d'après `140282e20`, à confirmer ;
  - `.next` : un `GetNextID()` par machine.
- **Conséquence.** Les configurations locales divergent.

**L7. Écritures locales jamais diffusées.**
- **Clés concernées :**
  - `MOD_*_ID`, réécrites à chaque tick par `OnTick` (branché sur `GameCoreEventPublishComplete`,
    l.7439) ;
  - `BAN_1..6`, remises à `LEADER_NONE` par `QuickRefresh` ;
  - `BAN_n`, `MAP_SCRIPT`, `world_age` et `temperature`, recopiés par chaque machine depuis le chat
    du draft.
- **Effet.** Chaque machine tient sa propre copie, qui ne se resynchronise qu'au prochain
  `BroadcastGameConfig` de l'hôte.

**L8. `SpawnRecalculation` mis à false au hasard, localement.**
- **Où.** `hostgame.lua` l.419-438.
- **Pourquoi la condition est toujours vraie :**
  - `m_InSession` ne passe jamais à true ;
  - le test vise BBS `…66540`, alors que BBM est `…66542`.
- **Effet.** Chaque affichage de HostGame en session a 10 % de chances d'écrire
  `SpawnRecalculation=false`, sur n'importe quelle machine. Chez un client, la valeur n'est pas
  diffusée.

**L9. Arrivée pendant le draft MPH.**
- **Où.** `stagingroom.lua` l.4452.
- **Ce que fait l'arrivée :**
  - `g_phase` est remis au défaut sur chaque machine ;
  - `g_disabled_slot_settings` reste à true chez les anciens joueurs (swap, exclusion et type
    d'emplacement bloqués par une fenêtre), mais pas chez le nouveau venu.
- **Autre défaut.** `OnMultiplayerHostMigrated` passe une variable globale `playerID` indéfinie
  (l.4644).

## Swap de joueurs : chaîne complète

```
Lua (demandeur)  OnSlotType(id, "Swap") → OnSwapButton (stagingroom.lua l.4723)
                 GetChangePlayerID(local) == id ? → annulation (-1)
                 Network.RequestPlayerIDChange(id)                       14026d1b0 → 14028ee60
réseau           NetRequestChangePlayerID(cur, desired) → tous, soi compris
chaque machine   Process 140810ae0 : FNetPlayer(cur)+0x20 = desired ; rappel de session +0x88
hôte, à chaque   140847260 → 140813f10 → 140814000 (pour chaque FNetPlayer) :
mise à jour        hôte ? demande ≠ -1 ?
                   cible sans FNetPlayer, ou cible.+0x20 == demandeur (swap mutuel) ?
                   rappels de session +0x50 (autorisé ?) pour les deux ids, +0x38 (validation)
                   → +0x20 = -1 des deux côtés, NetChangePlayerId(de, vers) → tous
chaque machine   Process 14080f6a0 → 140812ca0(de, vers) :
                   cible occupée : 140813d60 échange les deux FNetPlayer ; sinon déplacement
                   mise à jour des identifiants hôte et local
                   rappel +0x98 = PlayersSwapped (140274660) :
                     échange NETWORK_NAME, READY_STATUS, MOD_READY_STATUS des deux PLAYER_n
                     hors partie : échange SLOT_STATUS (avec la règle OBSERVER), et
                       CIVILIZATION_LEVEL_FULL_CIV pour celui qui devient humain (1401172a0)
                     en partie : événement 0xdd63e281 vers GameCore (changement à chaud)
                     cloud : rattache l'emplacement PlayByCloud (sinon « PlayersSwapped Error »)
Lua (tous)       aucun événement propre au swap : PlayerInfoChanged seulement
```

Ce qui ne bouge pas : civ, chef, équipe, couleur et le reste de l'emplacement. Les joueurs changent
d'emplacement, l'emplacement garde ses réglages.

**Au 2026-11-03.** Le joueur 1 se connecte à 21:33:21 :
1. l'hôte lui envoie la config complète, encore avec l'hôte en 0 ;
2. dans la même seconde, l'hôte demande l'emplacement 2 (`curID=0, desiredID=2`, à 0 et 1) ;
3. l'hôte l'accepte et diffuse le déplacement ;
4. il ne l'applique chez lui qu'après avoir reçu deux `NetPlayerInfo` du joueur 1 :
   `RECV (0) NetChangePlayerId` arrive après `RECV (1) NetPlayerInfo`.

Ici l'ordre est sain : config, puis swap. Le joueur suivant (21:33:46) reçoit `m_iHostID=2`.

**Pendant une arrivée, ce qui peut mal tourner :**
1. **Config d'avant le swap, swap jamais reçu** (conclusion 4) :
   - l'hôte diffuse `NetChangePlayerId` aux joueurs présents ;
   - un joueur s'enregistre avant que l'hôte n'ait traité son propre exemplaire ;
   - `1402893f0` lui envoie la config telle qu'elle est encore chez l'hôte ;
   - le nouveau venu ne reçoit pas le swap, parti avant lui : il voit les deux emplacements
     inversés jusqu'aux prochains `NetPlayerInfo` des intéressés.
2. **Message d'avant le swap appliqué après** (L4).
3. **Emplacement visé attribué au nouveau venu.** La demande reste orpheline (L3).
4. **Version MPH sous l'ancien identifiant.** MPH envoie sa version (`SendVersion`) dans
   `OnSwapButton`, avant que le swap ait lieu. Après le swap, l'hôte la range sous l'ancien
   identifiant (`RefreshStatusID`), et le nouvel emplacement a un statut de mod nul.

## Pistes pour automatiser la création

Tout passe par l'API Lua, côté hôte :
1. `GameConfiguration.SetToDefaults(gameMode)` ;
2. `Network.HostGame(serverType)` ;
3. attendre `MultiplayerJoinGameComplete` ;
4. écrire les réglages (`GameConfiguration.SetValue`, `MapConfiguration.SetValue`,
   `PlayerConfigurations[id]:SetLeaderTypeName` / `SetTeam` / `SetSlotStatus`) ;
5. `Network.BroadcastGameConfig(true)`, puis `Network.BroadcastPlayerInfo(id)` pour chaque
   emplacement modifié.

**Où le faire.** Un script d'interface du front-end, ajouté par le proxy de démarrage comme l'onglet
BetterGameDLLs, peut s'en charger. Il recevrait le résultat du civdraft par un fichier ou un
`LuaEvents`.

**Précautions :**
- passer par `SetupParameters` (`SetParameterValue`) plutôt que par des `SetValue` bruts, pour que
  les `ConfigurationUpdates` (presets) s'appliquent et que la revalidation des domaines ne défasse
  rien au rafraîchissement suivant ;
- **ne pas** laisser `CheckPreset` de MPH tourner après coup, ou fixer `MPH_PRESET` avant les listes ;
- diffuser une fois tout écrit, et attendre que chaque joueur ait rejoint avant d'écrire
  l'emplacement d'un humain. Sinon son propre `NetPlayerInfo` écrase celui de l'hôte (L1) ;
- les clés de mod se nomment comme dans leur XML de configuration ; `noms_cles.tsv` relie hachage
  et nom.

## Outils

| fichier | rôle |
|---|---|
| `hook_lobby.js` / `hook_lobby.py` | hook Frida, sur **chaque machine** du test. Journalise `HostGame`, `BroadcastGameConfig` / `BroadcastPlayerInfo` (avec « suis-je l'hôte »), `RequestPlayerIDChange`, `LaunchGame`, la réception de chaque message, la décision de l'hôte, l'application du swap, `PlayersSwapped` (emplacements avant/après), l'envoi de la config au nouveau venu, les arrivées et les départs. Écrit le texte complet de chaque `NetGameConfig` / `NetPlayerInfo` reçu dans `configs/`. Entrée : photo des 16 premiers emplacements. Octets vérifiés avant d'accrocher ; rien n'est modifié |
| `parse_netconfig.py` | décode un `NetGameConfig` / `NetPlayerInfo` en JSON (ligne N du journal, ou ligne 1 d'un fichier de `configs/`), ou compare deux messages |
| `gen_noms_cles.py` / `noms_cles.tsv` | noms des clés `Unknown Variant Key` d'un journal |
| `ghidra/*.java` | scripts Ghidra headless utilisés (copies de `~/Tools/ghidra-scripts`) |

```bash
.venv/Scripts/python.exe -I analyses/lobby/hook_lobby.py
```

```bash
.venv/Scripts/python.exe -I analyses/lobby/parse_netconfig.py analyses/lobby/runs/<dossier>/net_message_debug.log
```

```bash
.venv/Scripts/python.exe -I analyses/lobby/gen_noms_cles.py analyses/lobby/runs/<dossier>/net_message_debug.log
```

**Essai à vide du 2026-11-04** (salon Internet hébergé seul, `runs/essai-solo-2/`, non versionné).
Tous les sites s'accrochent, et la chaîne du swap est captée de bout en bout, conforme à l'analyse :
1. `lua_request_player_id_change` 0 → 4 ;
2. `recu_request_change` ;
3. `hote_decision accepte=true` (l'emplacement 4, une IA, n'a pas de joueur réseau) ;
4. `recu_change_id` ;
5. `firewire_change_id` ;
6. `app_players_swapped` : TAKEN/nom passent de 0 à 4, COMPUTER de 4 à 0, et l'identifiant local
   devient 4.

Le `NetGameConfig` capté fait 23 780 caractères et se décode en entier. Autres constats :
- les `BroadcastGameConfig` faits avant `HostGame` (hors session) sont bien sans effet ;
- **à l'hébergement, l'hôte diffuse 76 `NetPlayerInfo` en une seconde**, quatre par emplacement IA
  ou ouvert (6 à 23). C'est autant d'écritures complètes d'emplacement qui peuvent en croiser une
  autre (L1). L'essai ne dit pas d'où elles viennent : `PlayerSetupLogic`, MPH, ou les deux.

L'identifiant d'un `FNetPlayer` est en `+0x14` (`140f78900`, fonction partagée par repliement de
code, que Ghidra nomme `HksCompilerSettings::_isHksGlobalMemoTestingMode`). Il ne demande pas le proxy de GameCore, et peut tourner en même temps que
`netcode/hook_reconnect.py` : les sites communs (`PlayerConnected`, `PlayerLeft`) ne sont que
journalisés.

**Méthode Ghidra.** REA arrive à son délai d'expiration en décompilant GameCore, et ghidra-mcp n'est
pas installé sur cette machine. L'exe a donc été analysé en headless :

```bash
analyzeHeadless ~/Tools/ghidra-projects civ6exe -import CivilizationVI_DX12.exe -max-cpu 24
```

Cette analyse prend 5 min avec `GHIDRA_HEADLESS_MAXMEM=16G`. Ensuite, chaque script se lance avec
`-process CivilizationVI_DX12.exe -noanalysis -readOnly -postScript <script> <arguments>`. Les sorties
sont dans `~/Tools/ghidra-projects/lobby_dump/` (non versionné).

## Protocole de test

**Kit pour les joueurs.** `kit/faire_kit.ps1` écrit `Bureau\kit-test-salon.zip` : le hook,
`kit/test_salon.ps1` et la fiche joueur `kit/FICHE.md`. `test_salon.ps1 -Nom <pseudo>` fait trois
choses :
1. il crée un environnement Python à part, dans `%LOCALAPPDATA%\Civ6LobbyTest`, avec frida 17.25.2 ;
2. il attend le jeu et lance le hook ;
3. à `fin`, une fois le jeu fermé, il zippe sur le Bureau le journal du hook, les configs et les
   journaux du jeu.

`-Verifier` contrôle seulement l'installation.

**Matériel :**
- 3 machines au moins : l'hôte H et les clients A et B, avec un C pour les tests d'arrivée ;
- Civ6 en **DX12**, mêmes mods (MPH, BSM, BBG) ;
- horloges Windows synchronisées ;
- `hook_lobby.py` lancé sur chaque machine avant d'ouvrir le salon ;
- à la fin, `netcode/collect_logs.ps1 -Extra analyses\lobby\runs\<dossier>` sur chaque machine,
  **avant** de relancer le jeu.

**T1, preset chez un client qui rejoint (L2).**
1. H crée un salon, choisit le preset 1 (CWC), puis ajoute à la main 2 merveilles à
   `EXCLUDE_NATURAL_WONDERS`.
2. A rejoint.
3. Sur les deux machines : photo (Entrée) et capture de la liste des merveilles exclues.

Le dernier `configs/*recu_game_config*` de A doit avoir la liste de H, et son `Lua.log` doit écrire
`CheckPreset() 1 -1` puis `Applied CWC Settings`. Variante : H quitte le salon, A devient hôte, et
l'on regarde la liste chez B.

**T2, swap pendant une arrivée (conclusion 4).**
1. B est prêt à rejoindre.
2. A demande un emplacement IA ou ouvert ; B clique sur Rejoindre dans la même seconde. Recommencer
   10 fois, en variant l'ordre.
3. Chez B, comparer `recu_change_id` et `hote_config_au_nouveau` (frida.log de H), ainsi que la photo
   des emplacements de A et B après stabilisation, contre celle de H.

Si un essai donne un B sans `recu_change_id` et une config reçue où A est encore à son ancien
emplacement, c'est la course 1.

**T3, demande orpheline (L3).**
1. A demande l'emplacement de B, humain, qui ne répond pas. Le hook de H doit écrire
   `hote_decision accepte=false` une fois.
2. B quitte le salon.

Attendu d'après le code : `hote_decision accepte=true` et `NetChangePlayerId` sans nouvelle action
de A.

**T4, écritures concurrentes (L1).**
1. H change l'équipe de A, pendant que A change de civ, dans la même seconde (compter à voix haute).
2. Photo sur H, A et B.

Comparer, dans les `configs/*recu_player_info*` de chaque machine, l'ordre d'arrivée et l'état final
de `PLAYER_<A>` (`TEAM`, `LEADER_TYPE_NAME`).

**T5, mode anonyme et langues différentes (L5).** H en français, A en anglais, mode anonyme MPH.
Compter les `recu_player_info` de l'emplacement de A pendant 30 s sans rien toucher : plus d'un par
seconde signale le ping-pong.

**T6, rien de cassé sans swap.** Créer un salon, faire rejoindre A et B, régler, lancer. Les photos
des trois machines doivent concorder : c'est la référence.

## Pour la suite

- Confirmer, pour une **nouvelle** partie, que l'instantané de l'hôte remplace la config des clients
  au lancement : la carte ne devrait alors jamais diverger, même avec L2. Il faut regarder la
  génération dans `140282e20` / `14016c2f0`.
- Identifier les rappels de session `+0x38` et `+0x88`. Leur table n'est pas celle de
  `NetSessionCallbacks` (`140fe1960`) : c'est l'objet `FNetSession+0x18` (file `Session CB`,
  `140816a30`).
- Corrections possibles, toutes côté Lua (proxy de démarrage ou correctif à proposer à MPH) :
  - `CheckPreset` réservé à l'hôte et appelé seulement quand `MPH_PRESET` change ;
  - garde `Network.IsGameHost()` sur les commandes de chat ;
  - `Anonymise_ID` réservé au propriétaire de l'emplacement ;
  - affichage des demandes de swap en attente (`GetChangePlayerID`) et bouton d'annulation.
