# Netcode de Civ6 : où il est, comment il marche

Premier repérage (2026-10-27), fait à partir des chaînes de `CivilizationVI.exe` /
`CivilizationVI_DX12.exe` (build 1.0.12.54, 951533), de `GameCore_Base_FinalRelease.dll` et des
journaux d'une vraie partie à deux humains (A Soviet Goat en hôte, theflowgate), le 2026-10-27
de 18:50 à 18:52. `FGameSyncManager` est ensuite décompilé : voir
[Le contrôle de synchro décompilé](#le-contrôle-de-synchro-décompilé).

## Où il se trouve

**Tout le réseau est dans l'exe**, pas dans GameCore. GameCore ne touche jamais une socket : il
produit et consomme des messages (`Messages_*`), que l'exe transporte.

| couche | sources (chemins `__FILE__` de l'exe) | rôle |
|---|---|---|
| Application | `Civ6\Src\App\Network\Civ6NetMessage.cpp`, `NetSessionCallbacks.cpp` | messages propres à Civ6 (`NetGameConfig`, `NetPlayerInfo`, `NetModStatus`, `NetGameCoreMessage`), callbacks de session vers Lua (`LocalMachineEvent::Multiplayer*`) |
| Session | `CivTech\Libs\FireWire\FNetSession.cpp`, `FNetPlayerManager.cpp`, `FNetPlayer.cpp`, `FSessionSetup.cpp`, `FNetRelayMgr.cpp`, `FPendingConnections.cpp`, `KickVoteManager.cpp`, `FSecureSession.cpp` | joueurs, hôte, relais, migration d'hôte, vote d'exclusion |
| Messages | `FNetMessage.cpp`, `FNetMessageFactory.h`, `FSystemMessages.cpp`, `FDynamicMessageQueue.cpp` | sérialisation, découpage en morceaux (~1 166 octets), `NetMessageBundle` |
| Synchronisation | `FGameSyncManager.cpp`, `FNetMessageSync.cpp`, `CivTech\Libs\AutoVariable\FAutoArchiveFactory.h` | contrôle de désynchro et réparation |
| Transport | `ENetTransportThread.cpp` + `FNetAccessENet.cpp` (LAN / IP directe), `FSteamTransportThread.cpp` + `Steam\FNetSessionSteam.cpp`, `FNetAccessENetSteam.cpp` (Steam), `EOS\FNetEOSTransportThread.cpp`, `FNetSessionEOS.cpp` (crossplay Epic) | un thread de transport par backend, avec keepalive et timeouts |
| Lobbys | `Steam\FNetLobbySteam*.cpp`, `EOS\FNetLobbyEOS*.cpp`, `FLANLobby.cpp`, `firaxislive\...\MatchMaking.cpp` (PlayByCloud) | découverte et entrée en partie |

APIs tierces vues : `SteamNetworking005` (l'ancienne API P2P de Steam, pas
`ISteamNetworkingSockets`), `SteamMatchMaking009`, ENet (avec ses messages de debug), EOS P2P et
EOS Lobby (`EOSSDK-Win64-Shipping.dll`).

Côté GameCore, les seuls points de contact sont les en-têtes `Core\Common\Messages\Messages_*.h`
(`UnitCommand`, `CityCommand`, `PlayerOperations`, `PlayerTurnComplete`, `GameTurnComplete`,
`GameUpdate`, `PlayerSnapshotStatus`…) et les notifications `LOC_NOTIFICATION_PLAYER_DESYNCED` et
`…_DISCONNECT`. Les objets de jeu déclarent leur état synchronisé comme `AutoVariable`
(`SetDeltaDesyncCallback`, `Variable Out Of Sync`).

## Comment il marche

**Lockstep de commandes, topologie en étoile autour de l'hôte.** Chaque machine fait tourner toute la
simulation. Seules les commandes des joueurs circulent, dans des `NetGameCoreMessage
(MsgHash, Size)`, et chaque machine les exécute dans le même ordre. L'hôte (joueur 0 ici) relaie
au besoin (`NetRelay`, `NET SEND … disconnected relaying player`).

**Le tour, dans le journal** (`net_message_debug.log`, tour 78) :
1. les deux humains jouent ; leurs ordres partent en `PlayerOperation` ;
2. chacun envoie `PlayerTurnComplete` ; quand `TurnActiveCount` tombe à 0, `GameTurnComplete` ;
3. chaque machine calcule **localement** le passage de tour et le tour des IA
   (un `GameUpdate` par IA, `From=0`, émis et reçu par soi-même) : aucun trafic réseau pendant le
   rollover ;
4. **contrôle de synchro** (`StartGameSyncCheck`) : chaque client envoie à l'hôte un
   `NetGameSyncCheck` avec un hachage par `AutoArchive` (9 334 à 9 373 archives dans cette partie) ;
5. l'hôte compare et répond `NetSyncSnapshot` : `numDeltaArchives=0` si tout concorde, sinon les
   seules archives fautives, en delta. En dernier recours, le client demande un instantané complet.

**La réparation est donc partielle et faite par l'hôte** : on ne recharge pas la partie, on recopie
les archives qui diffèrent. Ce qui n'est pas une `AutoVariable` (caches, réserves de l'interface)
n'est ni contrôlé ni réparé.

**Entrée en partie / rechargement** : l'hôte envoie un `NetSnapshot`, c'est-à-dire la sauvegarde
entière : 2,7 Mo en 2 337 morceaux ici. Les clients répondent `NetSnapshotProcessed`, puis
`GameCoreController … TurnSynchronizationBarrier`.

**Pannes de connexion** : keepalive envoyé par le thread de transport quand un lien approche du
timeout ; timeouts ENet (`ENET_PEER_TIMEOUT_MINIMUM/MAXIMUM`) ; `LMS_DISCONNECT timeout` (EOS) ;
migration d'hôte (`StartOrUpdateHostMigration`, liste de candidats, échec si aucun candidat n'a
d'instantané) ; vote d'exclusion avec `KICK_TIMEOUT`. Les threads de transport ignorent les
timeouts quand un débogueur est attaché.

## Le contrôle de synchro décompilé

Analyse Ghidra complète de `CivilizationVI_DX12.exe` (projet `~/Tools/ghidra-projects/civ6exe`,
base 0x140000000). Le script `~/Tools/ghidra-scripts/DumpByStrings.java` a exporté dans
`~/Tools/ghidra-projects/sync_dump/` les 15 fonctions qui citent les chaînes de synchro, avec leurs
appelants et appelés (245 fonctions). `DumpAddrs.java` décompile des adresses précises sans
réanalyser (`-process CivilizationVI_DX12.exe -noanalysis -readOnly`). Le `.bat` de Ghidra refuse
les chemins à parenthèses : l'exe analysé est une copie dans `~/Tools/ghidra-projects/bin`.

| adresse | rôle |
|---|---|
| `1407f8780` | `StartGameSyncCheck` : marque la synchro en cours (`+0x3c`). Un client envoie son contrôle ; l'hôte se contente d'envoyer `NetGameSyncProcessed` |
| `1407f80a0` | construit et envoie `NetGameSyncCheck` (client) |
| `140dca6b0` | hachage d'une archive |
| `1409903d0` | **FNV-1a 32 bits** (`0x811c9dc5`, `0x1000193`) |
| `1407f9810` | côté hôte : file des contrôles reçus (entrées de 0x68 octets), rejoués jusqu'à ce qu'ils puissent être traités |
| `1407f6120` | côté hôte : comparaison et réponse `NetSyncSnapshot` |
| `1407f3090` | côté client : application de `NetSyncSnapshot` (11,7 Ko) |
| `1407f8020` | client : demande d'instantané complet |
| `1407f8a80` | `GameSyncComplete` : appelle `140dca850` sur chaque archive (remise à zéro de l'état de suivi, probablement) |
| `1407f5e70` | migration d'hôte en pleine synchro : relance le contrôle |
| `1407f7300`, `1407f8d40` | enregistrement et retrait d'une archive dans `m_syncArchives` (table de hachage en `+0x18`, taille en `+0x20`) |

**Le hachage.** Une archive est une liste d'`AutoVariable` (vecteur en `+0x08..+0x10`). Chaque
variable fournit un hachage de 32 bits (méthode virtuelle `+0x38`). Le hachage de l'archive est le
FNV-1a 32 bits du tableau de ces hachages. Le client envoie, par archive, son identifiant 64 bits,
ce hachage et une liste d'indices de variables, tirée d'un arbre en `+0x38` de l'archive (`140dcb4f0`).
Ce sont probablement les variables marquées depuis le dernier contrôle ; c'est à confirmer.

**Côté hôte** (`1407f6120`), seulement si l'on est l'hôte (méthode virtuelle `+0x1c0` de la session) :
- un contrôle reste en file tant que le message n'a pas son drapeau `+0x38` ou que le joueur n'a pas
  le drapeau `+0x1f`. L'hôte attend sans doute d'avoir atteint le même point du tour ;
- pour chaque archive du client :
  - absente chez l'hôte : priorité 2, ordre de suppression ;
  - hachage différent, **ou hachage nul** (toujours traité comme un écart) : priorité 1, delta
    (`1407f8560`), qui reçoit la liste d'indices du client ;
- pour chaque archive de l'hôte absente chez le client : priorité 3, données de création
  (`1407f8430`) ;
- envoi de `NetSyncSnapshot` à ce joueur seulement.

**Côté client** (`1407f3090`) : les archives sont appliquées par niveau de priorité.
- Création, suppression, ou recopie complète d'une archive existante (`Syncing...`).
- Delta : `140dcab30` compare les variables reçues aux siennes, `140dcb610` recopie chaque variable
  fautive, puis les `DeltaDesyncCallback` enregistrées par le jeu sont appelées (`140dca610`,
  `140dca930`). Un rappel peut supprimer l'archive (`was removed by previous DeltaDesyncCallback`).
- Delta vide, archive inconnue pendant un delta, ou « aucune variable désynchronisée trouvée » :
  passage à l'**instantané complet** (`1407f8020`), c'est-à-dire la sauvegarde entière comme à
  l'entrée en partie.
- Sinon, le client répond `NetGameSyncProcessed` à l'hôte et à lui-même.

**Conséquences :**
- les écarts **sont déjà journalisés avec le nom de l'archive** dans `net_message_debug.log`, chez
  l'hôte (`data hash mismatch … AutoArchive=%s HostDataHash=%u RemoteDataHash=%u`) et chez le client.
  Pas besoin de Frida pour savoir ce qui diverge ;
- la réparation se fait variable par variable, et ce qui n'est pas une `AutoVariable` n'est jamais
  comparé ni recopié ;
- FNV-1a 32 bits sur environ 9 300 archives : une collision qui masquerait un écart reste très
  improbable à chaque contrôle.

## Reconnexion en cours de partie et boucles de désynchro

Le symptôme : un joueur se déconnecte en pleine partie et revient aussitôt (MPH met le jeu en pause).
Parfois, la partie entre ensuite dans une boucle de désynchro un tour sur deux.

### Ce que fait le jeu

Fonctions décompilées (`~/Tools/ghidra-projects/rejoin_dump/`) :

| adresse | rôle |
|---|---|
| `140272910` | `PlayerConnected` : vide l'instantané en cache, met à jour le joueur |
| `140273c40` | `PlayerLeft` : vide aussi le cache |
| `14083fac0` | `HandleSnapshots` (hôte) : pour chaque joueur qui attend un instantané, le demande à l'application |
| `140274f90` | demande d'instantané : état en `+0x30` (0 aucun, 1 sauvegarde en cours, 2 en cache). En cache, il est renvoyé tel quel ; sinon, une sauvegarde est lancée |
| `1402723a0`, `1402752a0` | sauvegarde terminée, envoi à la session ; le cache est gardé seulement si `DAT_142391ee8+0x3c` |
| `14026d200` | `Network.RequestSnapshot()` en Lua : `NetSnapshotRequest(joueur local, raison 0)`, comme la resynchro complète de `FGameSyncManager` (`1407f8020`) |

Le cache est vidé à chaque arrivée ou départ, donc un revenant ne reçoit pas un instantané d'avant
sa déconnexion. Mais l'instantané est une sauvegarde **asynchrone**, prise par GameCore au premier
point sûr après la demande. Tout ce qui modifie l'état entre la reconnexion et ce point en fait
partie ou non selon l'ordre d'arrivée.

**Asymétrie structurelle.** Au chargement d'une partie, l'hôte s'envoie l'instantané à lui-même
(`Net SEND (0, 1) … NetSnapshot` puis `Net RECV (0) :NetSnapshot`, raison 2) : toutes les machines
repartent du même état désérialisé. À une reconnexion, **seul le revenant recharge**. L'hôte et les
autres gardent leur état en mémoire, y compris ce que la sauvegarde ne contient pas (caches, mémoire
des IA, état Lua des mods). Si cet état non sauvegardé influe sur la simulation, le revenant diverge.
Le contrôle de fin de tour ne répare alors que les `AutoVariable`, sans la cause : la divergence
revient. C'est cohérent avec le remède habituel, « tout le monde quitte et recharge l'autosave »,
qui rétablit la symétrie.

### La reprise par l'IA

À la déconnexion, **l'IA prend immédiatement la main sur le joueur parti**. Elle bouge des unités,
donne des ordres, déplace parfois les gouverneurs (observé en partie).
- **Valeurs de `SlotStatus`** (enregistrement Lua, `1404d8440`) : OPEN=0, COMPUTER=1, CLOSED=2,
  TAKEN=3, OBSERVER=4.
- **`PlayerLeft`** (`140273c40`) fait passer l'emplacement de TAKEN à **COMPUTER** (`1401176a0`,
  clé de config `DAT_14230f068`) et baisse un second drapeau du joueur (clé `DAT_14230f07c`).
- **Dans `net_message_debug.log`, à la déconnexion de 18:52:35**, l'hôte :
  - diffuse `NetPlayerInfo` et `NetGameConfig`, la nouvelle config avec l'emplacement en COMPUTER ;
  - envoie à GameCore **`PlayerConnectionChange(ePlayer=1, m_bConnecting=0)`**, par le flux
    synchronisé. Il l'envoie **deux fois**, une par rappel `DelayedPlayerLeft`
    (`netPlayerRemoved` 1 puis 0) ;
  - puis `GamePaused=1`, la pause demandée par MPH, et `Player 1 set TurnActive 0` : le tour du joueur
    parti est clos dans la même seconde.

**Conséquences pour la synchro :**
- **Le message GameCore passe par le lockstep, la config non**, et GameCore décide d'après la config.
  Gestionnaire de `PlayerConnectionChange` dans GameCore_XP2 (`1800383e0`, `Messages_PlayerConnect.cpp`,
  projet Ghidra `~/Tools/ghidra-projects/civ6xp2`) :
  - **déconnexion** : si le statut lu **n'est ni OBSERVER ni déjà COMPUTER**, il fait tout le travail :
    - notification « déconnecté » ou « exclu » ;
    - statut mis à COMPUTER (`18016b840`) ;
    - événement `0x6d134254` et appel des abonnés de la liste `+0x1890` ;
    - si les drapeaux `DAT_180b8ab20..22` le permettent, relance de la mise à jour du jeu
      (`18063e2e0`), là où l'IA prend la main.

    **Sinon, il ne fait rien** ;
  - **reconnexion** : statut remis à TAKEN (sauf OBSERVER), notification, abonnés de la liste
    `+0x1878`, même relance ;
  - **le statut est lu et écrit dans la config du joueur** : `18016a2b0` lit et `18016b840` écrit,
    méthodes `+0x58` et `+0x130` de l'objet en `+0x10`, clé `DAT_180b8d668`. C'est la même forme que
    dans l'exe (`140115ed0` / `1401176a0`). Ce n'est pas un état propre à GameCore.

  **Hypothèse de bug, à vérifier.** Si GameCore et l'exe partagent bien cet objet de config :
  - sur l'**hôte**, `PlayerLeft` a déjà mis COMPUTER **avant** d'envoyer le message. GameCore voit donc
    COMPUTER et **saute la bascule** : pas de notification, pas d'abonnés, pas de relance ;
  - sur un **autre client**, c'est l'ordre d'arrivée qui décide : config `NetGameConfig` avant le
    message, bascule sautée ; message avant la config, bascule faite.

  Un même message synchronisé produirait donc des effets différents selon la machine : c'est
  exactement ce qui casse le lockstep. Pour le vérifier :
  - voir si l'hôte reçoit la notification « X Disconnected » ;
  - ou poser un point d'arrêt ou un hook Frida sur `1800383e0`, puis lire la valeur rendue par
    `18016a2b0`.
- **L'IA décide aussi avec sa mémoire interne**, qui n'est pas sauvegardée en entier. Elle n'existe
  que chez les machines qui ont vécu la reprise, pas chez le revenant qui recharge.
- **Après la reconnexion**, le joueur retrouve un tour où l'IA a déjà joué : PM dépensés, ordres
  donnés, gouverneurs déplacés.

### Ce que fait MPH (Multiplayer Helper 1.8.0, Workshop 3307282026)

Le but de `OnDrop` est justement d'**empêcher l'IA de jouer les unités** du joueur parti : il met
leurs PM à 0, puis les rend au retour. Il ne couvre que les PM : gouverneurs, production, recherche,
achats, promotions et diplomatie restent à l'IA.

- `ui/Additions/DropControl.lua`, sur `Events.MultiplayerPrePlayerDisconnected` :
  - l'hôte se met en pause (`SetWantsPause`, `BroadcastPlayerInfo`) ;
  - **chaque machine** appelle directement le script de gameplay (`ExposedMembers.LuaEvents`
    → `UICPLPlayerDrop`).
- `data/MP_helper.lua`, `OnDrop` : met à 0 les points de mouvement de toutes les unités du joueur
  parti (`UnitManager.ChangeMovesRemaining(unit, -99)`) et garde les valeurs dans `Drop_Data`, une
  table Lua en mémoire.
- Sur `Events.MultiplayerPlayerConnected`, chaque machine qui a vu partir le joueur appelle
  `OnConnect` → `RestoreUnits` : les mouvements sont rendus.

Vu dans `Lua.log` : `Ondrop: Saving Player 1's data` à 18:52:35, quand theflowgate est retourné au
menu par la commande `.manual_disconnect_main_menu`. `Ondrop` est passé avant toute action de l'IA :
9 unités sur 11 avaient encore 2 PM, et les 2 autres venaient d'être déplacées par le joueur
(`UNITOPERATION_MOVE_TO` à 18:52:21 et 18:52:26).

Sur une vraie coupure (crash, timeout), l'ordre n'est pas garanti. `OnDrop` part de l'événement
d'interface `MultiplayerPrePlayerDisconnected`, sur le fil principal. L'IA, elle, joue quand GameCore
traite `PlayerConnectionChange`, sur son propre fil. Selon la machine, l'IA peut donc avoir déjà
dépensé des PM avant `OnDrop` ou non. Les PM sauvegardés dans `Drop_Data`, puis rendus, diffèrent
alors d'une machine à l'autre.

**Pourquoi c'est dangereux pour la synchro :**
1. **Modifications hors du flux de commandes.** L'état de jeu est modifié localement par chaque
   machine, au moment où son interface reçoit l'événement, sans message réseau. Le lockstep suppose
   que toute modification passe par une commande exécutée partout au même point.
2. **Le revenant ne fait jamais `RestoreUnits`.** Son interface vient d'être chargée : sa liste
   `g_dropped_player_list` est vide, et son `Drop_Data` aussi, car non sauvegardé. Ses unités gardent
   les mouvements contenus dans l'instantané :
   - si l'hôte a pris l'instantané **avant** de rendre les mouvements, le revenant a des unités à 0
     et les autres non ;
   - si c'est **après**, tout concorde.

   C'est une course entre la sauvegarde asynchrone et l'événement d'interface, ce qui explique le
   « parfois ».
3. **Reprise sans le joueur** (`OnYesResume`, bouton de l'hôte) : `UICPLPlayerConnect` n'est appelé
   **que chez l'hôte**. Les mouvements sont rendus chez lui seul : désynchro certaine.
4. **Deuxième déconnexion plus tard.** Sur une machine qui a rechargé entre-temps, `Drop_Data` ne
   connaît pas les déconnexions précédentes. `OnConnect` pour un joueur absent de `Drop_Data` lève
   une erreur Lua (`Drop_Data[playerID].unit` sur `nil`). La restauration n'a pas lieu sur cette
   machine seulement.
5. À chaque reconnexion, toutes les machines font `SetTurnTimerType("TURNTIMER_NONE")` et
   `BroadcastGameConfig`.

Réserve : les mouvements sont remis à neuf au début de chaque tour. Cette divergence seule devrait se
résorber au tour suivant. Elle suffit à expliquer une désynchro au retour, pas forcément une boucle.
La boucle demande un état qui ne se recalcule pas. Candidats :
- la mémoire de l'IA qui a joué le joueur parti, présente chez l'hôte et absente chez le revenant ;
- le statut d'emplacement lu dans la config hors lockstep ;
- un effet de bord des unités immobilisées : ordres annulés, automatisation ;
- l'état non sauvegardé de l'asymétrie ci-dessus.

### Diagnostic à la prochaine occurrence

Les journaux sont réécrits à chaque lancement : il faut les copier **avant** de relancer le jeu, sur
l'hôte **et** sur le revenant (`%LOCALAPPDATA%\Firaxis Games\Sid Meier's Civilization VI\Logs\`) :
- `net_message_debug.log` : lignes `AutoArchive out of sync … AutoArchive=<nom>`. Les noms disent
  quel objet diverge, unités, villes, joueur. Comparer d'un tour à l'autre : est-ce toujours la même
  archive ?
- `Lua.log` : MPH écrit à chaque début de tour `OnGameTurnStarted: Turn N Local State: X`, un tirage
  du générateur aléatoire synchronisé. Si X diffère entre deux machines au même tour, l'aléa de jeu a
  divergé. Le premier tour où c'est le cas date la divergence. Y chercher aussi `Ondrop`,
  `OnConnect`, `RestoreUnits` et les erreurs Lua.
- `net_connection_debug.log` : horodatage de la reconnexion et de la sauvegarde d'instantané, pour
  savoir de quel côté de la course on était.

### Pistes de correction

1. **Côté MPH**, la plus simple et la plus sûre :
   - ne plus modifier l'état depuis l'interface. Le jeu est en pause, donc l'immobilisation des unités
     est sans doute inutile ;
   - sinon, la faire passer par une opération synchronisée émise par l'hôte seul
     (`UI.RequestPlayerOperation(…, EXECUTE_SCRIPT)`), et garder les mouvements dans une propriété
     sauvegardée (`Game:SetProperty`) plutôt qu'en mémoire Lua.
2. **Rétablir la symétrie après une reconnexion.** Chaque client peut appeler
   `Network.RequestSnapshot()` pour recharger l'état de l'hôte : cela aligne les clients entre eux,
   mais pas sur la mémoire de l'hôte. Reste à tester si l'hôte peut se renvoyer son propre instantané,
   comme au chargement (raison 2). Ce serait l'équivalent automatique du rechargement manuel.
3. **Côté moteur**, une fois les archives fautives connues, chercher l'état non sauvegardé qui les
   fait rediverger : le relier au fichier `.cpp` via la carte de GameCore
   ([docs/gamecore_xp2](../gamecore_xp2/README.md)).

### Test du 2026-10-29 : reconnexion réelle

Données : `netcode/runs/20261029-reconnexion/` (non versionné). On y trouve :
- `hote/` : archive de l'hôte, prise à 13:17 ;
- `client/` : archive de l'autre joueur, prise à 13:03, après la coupure et **avant** sa
  reconnexion. Elle ne couvre donc pas l'après-retour ;
- `frida/frida.log` : le hook de l'hôte.

Partie BBG 7.5.1 et MPH, tour 78. Joueurs : 0 Kongo (hôte), 1 Inde (l'autre joueur), 2 à 5 IA
majeures (Pologne, Sumer, Australie, Japon), puis les cités-États dès 6 (Cardiff, Caguana…).

**Déroulé (journal Frida et `net_message_debug.log` de l'hôte) :**

| heure | événement |
|---|---|
| 12:58:44, 13:00:39 | contrôles des tours 77 et 78 : aucun écart |
| 13:02:11 | coupure du joueur 1, `PlayerLeft` deux fois, deux `PlayerConnectionChange` |
| 13:02:11 | GameCore lit **TAKEN** et écrit COMPUTER ; le second message lit COMPUTER et ne fait rien |
| 13:02:11 | `GamePaused=1` ; MPH `Ondrop` met les PM du joueur 1 à 0 |
| 13:11:44 | retour : `PlayerConnected`, GameCore passe de COMPUTER à TAKEN ; MPH `OnConnect` / `RestoreUnits` |
| 13:11:57 | instantané demandé, sauvegardé et envoyé, **après** la restauration de MPH |
| 13:12:37 | reprise (`GamePaused=0`) |
| 13:13:14 | contrôle de fin du tour 79 : **38 archives fausses** |
| 13:14:33, 13:15:21 | contrôles des tours 80 et 81 : **6 archives fausses à chaque fois**, les mêmes |

**Archives fausses :**
- **fin du tour 79**, la première après le retour :
  - `AI::CultureAi` de **tous** les joueurs, 0 à 19 et 62 ;
  - `Visibility::Instance` des joueurs 0 à 12 ;
  - `Player::Units` des joueurs **2 à 7** ;
- **fins des tours 80 et 81** : `Player::Units 2-40-0` à `7-40-0`, et rien d'autre.
  - `3-40-0` et `5-40-0` ont **les mêmes hachages, côté hôte comme côté client, trois contrôles de
    suite** : la réparation ne les corrige pas ;
  - les autres changent d'un tour à l'autre, mais restent fausses.

**Verdict sur les hypothèses :**
1. **Bascule en IA sautée chez l'hôte : réfutée.** L'exe n'écrit pas le statut avant le message.
   GameCore lit TAKEN et fait la bascule. Seul le doublon du message est ignoré, ce qui est
   inoffensif.
2. **Course entre la sauvegarde et la restauration de MPH : pas déclenchée ici.** La restauration
   (13:11:44) précède la sauvegarde (13:11:57), et les unités du joueur 1 ne sont jamais fausses. Le
   risque reste quand l'ordre s'inverse, par exemple si l'hôte reprend sans le revenant.
3. **Asymétrie du rechargement : confirmée.** Le revenant a chargé la sauvegarde que l'hôte venait
   d'écrire. Pourtant, une fin de tour plus tard :
   - la mémoire culturelle de l'IA diffère pour **tous** les joueurs ;
   - les unités des IA (2 à 5) et des deux premières cités-États (6, 7) diffèrent aussi.

   L'hôte calcule avec un état en mémoire que la sauvegarde ne reproduit pas à l'identique, et l'IA
   décide autrement chez le revenant. C'est la fin de tour, avec l'automatisation tactique des
   unités (`Performing End-of-Tactical Phase Unit Automation`), qui fait diverger les unités.
4. **Boucle : confirmée, silencieuse.** Les contrôles suivants trouvent toujours les unités des
   joueurs 2 à 7 fausses. La réparation ne converge pas : l'hôte envoie un delta (6 archives), le
   client l'applique, ne demande pas d'instantané complet, et l'écart revient.

   Explication probable, tirée du code : le delta d'une archive ne contient que les variables
   **marquées modifiées** depuis le dernier contrôle, chez l'hôte ou chez le client (`140dcbe10`).
   Une variable qui a divergé puis n'a plus bougé n'est jamais renvoyée. Les hachages figés de
   `3-40-0` et `5-40-0` collent avec ça. Le journal du client après son retour le confirmerait : il y
   écrit `delta data mismatch`, `No desynced AutoVariables found` ou `Received empty Delta data`.

   Personne n'a vu de notification : ces réparations partielles sont muettes. Le symptôme visible
   (« désynchro un tour sur deux ») est sans doute ce qui arrive quand la divergence finit par
   déclencher une resynchro complète. Il ne s'est pas produit pendant ces 3 tours.

**Autre constat, BBG 7.5.1** (`scripts/bbg_uitogameplay.lua`) : ses scripts d'interface transforment
des événements locaux (`Events.UnitPromoted` …) en opérations synchronisées `EXECUTE_SCRIPT`. Comme
**chaque machine** réagit à l'événement, chaque machine envoie sa propre opération. À chaque tour,
hôte **et** client envoient ainsi des opérations pour les IA 2 à 7, en nombre parfois différent.
- Exemple : au tour 80, l'hôte en envoie 3 pour le joueur 5.
- Toutes passent par le lockstep : la synchro n'est pas rompue directement.
- Mais chaque correction est appliquée autant de fois qu'il y a de machines qui l'ont vue.
  `GameplayPromotionFixExp` modifie l'expérience des unités, ce qui n'est pas forcément sans effet
  quand on le refait.

### Test solo du 2026-10-29 : état en mémoire contre état rechargé

Données : `netcode/runs/empreinte-20261029-135935/`. Ce sont 10 empreintes (`netcode/empreinte.py`)
et l'archive des journaux dans `journaux/`. Partie multijoueur hébergée seule, aucune action jouée.

**Chronologie.** Un autosave `N` contient l'état de **fin du tour N−1**. Au chargement, le jeu fait
un contrôle de synchro sur cet état (« TurnSynchronizationBarrier »), puis traite le tour N. Chaque
chargement donne donc une empreinte de l'état chargé.

| empreinte | parcours | instant |
|---|---|---|
| 1, 2 | étape 0 (`AutoSave_0078` de la partie du 29) | chargé (fin 77), fin 78 |
| 3, 4, 5 | A : `AutoSave_0077` | chargé (fin 76), fin 77, fin 78 |
| 6, 7, 8 | B : `TestB_0078`, l'autosave écrit par A | chargé (fin 77), fin 78, fin 79 |
| 9, 10 | C : `AutoSave_0077` | chargé (fin 76), fin 77 |

**Comparaisons :**

| comparaison | sens | archives différentes |
|---|---|---|
| 3 contre 9 | deux chargements du même fichier | 6 : `Player::Units` 2 à 7, variable 13 |
| 4 contre 10 | même tour joué après ces deux chargements | les mêmes 6, rien d'autre |
| **4 contre 6** | **fin du tour 77 en mémoire, contre la même fin sauvegardée puis rechargée** | **36** : `AI::CultureAi` ×21 (variable 17), `Visibility::Instance` ×9 (variable 6), `Player::Units` ×6 (variable 13) |
| 5 contre 7 | un tour plus tard | 14 : `Visibility::Instance` ×8 (variable 6), `Player::Units` ×6 (variable 13) |

**Les trois variables**, nommées d'après leur enregistrement dans les constructeurs d'archive de
GameCore_XP2 (`180347bd0`, `18040e8c0`, `180355c10`, appels à `1809953d0` dans l'ordre) :
- **`AI::CultureAi.m_resolutionAnalyzers`** (variable 17, tous les joueurs) : l'analyse des
  résolutions du Congrès mondial par l'IA. Elle diffère juste après le chargement, puis l'IA la
  recalcule pendant le tour : un tour plus tard, elle concorde.
- **`Visibility::Instance.m_aExplicitLayers`** (variable 6, joueurs 0 à 5, 12, 15, 17) : les couches
  de visibilité explicites. Elles diffèrent au chargement et **encore un tour plus tard**.
- **`Player::Units.m_aiAddOnDistrictFounding`** (variable 13, joueurs 2 à 7, les IA) : vide (hachage
  de 0) pour tous les autres joueurs. Les joueurs 2 à 7 ont tous la même valeur, mais elle
  **change d'un chargement à l'autre du même fichier** : `914d5dd7` dans le parcours A, `51caacc7`
  dans l'étape 0 et les parcours B et C. Le **contenu est pourtant le même** : seul l'ordre
  d'itération change, voir plus bas.

**Tout le reste concorde**, unités, villes, trésor, aléa compris. Sans rechargement, une fin de tour
est reproductible (4 contre 10, à la variable 13 près). Et un tour joué après rechargement ne
déplace aucune unité autrement (5 contre 7).

**Lien avec la reconnexion réelle** (test précédent) :
- le premier contrôle après le retour signalait exactement ces trois familles : `AI::CultureAi` de
  tous les joueurs, `Visibility::Instance` de joueurs 0 à 12, `Player::Units` 2 à 7 ;
- la boucle des tours suivants ne portait plus que sur `Player::Units` 2 à 7, avec des hachages
  figés pour certains. Cela concorde avec `m_aiAddOnDistrictFounding` :
  - un état reconstruit à chaque chargement, que le revenant n'a pas comme l'hôte ;
  - que personne ne modifie ensuite, donc que le delta, limité aux variables modifiées, ne renvoie
    jamais.

  Pour en être sûr, il faudrait le journal du client, ou un relevé des variables chez les deux
  joueurs.

#### `m_aiAddOnDistrictFounding` : un hachage qui dépend de l'ordre des seaux

**Ce que c'est.** Ce sont les unités promises à un joueur pour la création d'un district. Elles
viennent de la table `MajorStartingUnits`, lignes `OnDistrictCreated="true"` (`Eras.xml`). Aux ères
antique, classique et médiévale, ce sont des lignes `AiOnly="true"` :
- un bâtisseur à partir de King ;
- un colon à partir d'Emperor ;
- dans les deux cas, plus `DifficultyDelta` = 0,5 par niveau au-dessus.

Ces lignes expliquent pourquoi **seules les IA** ont une valeur. La structure est une table de
hachage EASTL `district → (unité → nombre restant)`, dont les clés sont les hachages de type.

| adresse (GameCore_XP2) | rôle |
|---|---|
| `1802ef560` | fondation de la civilisation : pour chaque ligne de `MajorStartingUnits` liée à un district, `table[district][unité] += quantité` |
| `180192c20` | district achevé : fait apparaître les unités promises, puis `table[district][unité] -= 1` tant que le nombre n'est pas nul |
| `18034cb20` | `table[a][b] += n` (via `18034bad0`, l'`operator[]` qui insère une table vide si besoin) |
| `180350440` | remise à zéro de `Player::Units` : vide la table **sans réduire le nombre de seaux** (`18034e320`) |
| `18034b480` | écriture de `Player::Units` dans la sauvegarde ; la table passe par `180345a80` |
| `180351620` | hachage de synchro de la variable (méthode +0x38) : sérialise via `180345a80`, puis FNV-1a |
| `180345a80` / `180345970` | sérialisation des tables extérieure et intérieure : taille, puis éléments **dans l'ordre des seaux** |
| `1803462a0` / `180346140` | relecture (sauvegarde ou delta) : réinsère chaque élément, chaque fois en tête de chaîne |

**Preuve par les hachages.** La table vide hache en `4b95f515` = FNV-1a de quatre octets nuls :
le format est brut. Une recherche exhaustive (`DISTRICT_CITY_CENTER`, colon et bâtisseur, nombres de
−3 à 8, hachages ou indices) ne trouve qu'une solution par valeur :
- `914d5dd7` = `{CITY_CENTER: {SETTLER: 0, BUILDER: 0}}` ;
- `51caacc7` = `{CITY_CENTER: {BUILDER: 0, SETTLER: 0}}`.

Le contenu est identique : au tour 77, tous les colons et bâtisseurs promis ont été donnés, et les
entrées restent avec un nombre de 0. **Seul l'ordre d'itération diffère**, et le hachage de
synchro, qui suit cet ordre, aussi.

**Pourquoi l'ordre change.** La sérialisation suit l'ordre des seaux, et la position d'une clé dans
les seaux dépend de deux choses :
- **le nombre de seaux**, avec l'indice `clé % nombre de seaux`. Avec SETTLER = `d0649d28` et
  BUILDER = `11d57532` :
  - avec 1 à 3 seaux, les deux clés partagent un seau ;
  - avec 5 seaux, le colon passe en premier ;
  - avec 7 à 19 seaux, le bâtisseur passe en premier ;
- **l'ordre d'insertion** dans un seau partagé : la relecture réinsère en tête de chaîne, ce qui
  inverse l'ordre.

Or le nombre de seaux dépend de l'histoire de la table (insertions une à une à la fondation, contre
réinsertion en bloc au chargement), et pas seulement de son contenu. Deux machines au même état de
jeu peuvent donc avoir des hachages différents.

L'écart entre les deux chargements de `AutoSave_0077` (3 contre 9) montre que l'histoire du
processus compte aussi. Le mécanisme exact n'est pas encore établi : des seaux gardés au vidage ? un
champ de politique de redimensionnement non recopié ? `empreinte.js` relève désormais la disposition
de cette table (nombre de seaux, taille, ordre des éléments) dans une 5ᵉ colonne. Une prochaine
série d'empreintes le dira.

**Conséquence pour la reconnexion.** C'est une boucle de désynchro **sans divergence de jeu** :
- le revenant recharge, et sa table a une autre disposition que celle de l'hôte ;
- le contrôle signale `Player::Units` de chaque IA ;
- la réparation ne corrige rien, dans les deux cas possibles :
  - la variable n'est pas marquée modifiée, et le delta ne la renvoie pas ;
  - ou il la renvoie, et le client la relit, donc la réinsère et la réordonne à sa façon.

  Le hachage reste faux à chaque tour, ce qui donne les hachages figés du test réel.

Le jeu a le même contenu des deux côtés : la désynchro est fausse, mais le jeu la signale sans fin.
Seules les parties avec des IA en King ou plus, ou commencées après l'Antiquité, ont cette table
non vide.

**Pistes :**
1. **Données, sans DLL** : un mod qui retire de `MajorStartingUnits` les lignes
   `OnDistrictCreated = 1`, ou les remplace par des unités de départ classiques, `OnDistrictCreated = 0`.
   La table reste alors vide et le problème disparaît. Ça ne vaut que pour les **nouvelles**
   parties. L'IA perd son colon ou bâtisseur bonus à la fondation de ses premières villes, sauf à
   les lui donner autrement.
2. **Moteur** : rendre la sérialisation indépendante de l'ordre, en triant par clé dans `180345a80`
   et `180345970`. Il faudrait que **toutes** les machines aient le même GameCore corrigé, puisque
   les hachages se comparent. Les sauvegardes restent lisibles, la relecture acceptant n'importe quel
   ordre. Une alternative moins invasive : trier seulement dans le hachage `180351620`, mais le delta
   resterait réordonné.
3. Chercher les autres variables de table de hachage EASTL (même famille de sérialiseurs) : elles ont
   le même défaut dès qu'elles ont deux clés dans un seau partagé, ou des nombres de seaux qui
   diffèrent.
4. `m_resolutionAnalyzers` (vtable `1809fba80`) et `m_aExplicitLayers` (vtable `1809f1150`) sont
   d'autres types, un vecteur et un tableau de couches. Ce n'est pas le même mécanisme.
   `m_aExplicitLayers` diverge encore un tour plus tard, mais n'est pas revenu dans la boucle
   réelle : il faudra vérifier s'il se répare.

## Accords croisés : deux initiateurs (partie du 2026-11-03)

Partie à quatre humains, hôte le joueur 2, journaux du joueur 1 (machine locale) dans
`rollover/captures/20261103-204357-mp`. Sauvegardes : `Saves\Multi\BGD bug deal T28` et `T29`.

**Le symptôme.** Au tour 29, le joueur 1 et son allié, le joueur 2, ne peuvent accepter ni l'un ni
l'autre la proposition de l'autre : chacun se voit initiateur de l'accord. Il a fallu que l'un des
deux renvoie un accord complet pour débloquer la situation.

**Un accord normal, dans `net_message_debug.log`** (21:27, le joueur 2 propose et le joueur 1
accepte) :
1. le proposant envoie deux `DiplomacyStatement MAKE_DEAL` (ouverture, puis proposition), puis
   `DiplomacySession CLOSE` ;
2. l'autre rouvre la session (`DiplomacySession RESTORE, FromPlayer=<proposant>`) ;
3. il accepte : `DiplomacyDeal eActionType=1`, de lui vers le proposant, suivi d'un `MAKE_DEAL` et
   d'un `CLOSE`.

**Le tour 29** (21:30, vu du joueur 1) :

| heure | message |
|---|---|
| 21:30:31 | reçu de 2 : `MAKE_DEAL` 2 → 1 (2 ouvre l'écran d'accord) |
| 21:30:33 | envoyé par 1 : `MAKE_DEAL` 1 → 2 (1 ouvre aussi le sien) |
| 21:30:34 | envoyé par 1 : `MAKE_DEAL` 1 → 2 (proposition de 1) |
| 21:30:34 | reçu de 2 : `MAKE_DEAL` 2 → 1 (proposition de 2), puis les deux `CLOSE` |
| 21:30:41, 43 | 1 rouvre deux fois son propre écran (une ouverture, puis `CLOSE`), sans proposer |
| 21:30:46, 47 | 2 renvoie une proposition complète (ouverture et proposition) |
| 21:30:51 | 1 rouvre la session de 2 et accepte (`DiplomacyDeal eActionType=1`) : débloqué |

**Ce qu'on en déduit (revu le 2026-11-05, GameCore et écran d'accord).** Deux lectures
successives se sont révélées fausses : d'abord une divergence d'ordre entre machines, puis un blocage
par `HasPendingDeal`. Le journal suffit à écarter la seconde : en fermant l'écran avec une proposition
en attente, l'écran envoie `SendWorkingDeal(CLOSED)` (`DiplomacyDealView.lua`, lignes 631–633), qui
part en `DiplomacyDeal eActionType=8`. Aucun n'apparaît entre 21:30:34 et 21:30:51, donc
`HasPendingDeal(1, 2)` était faux à 21:30:42 et 21:30:44.

Ce qui bloque, c'est **la session diplomatique**, pas l'accord :
- ouvrir l'écran d'accord ouvre une session dont on est `FromPlayer`, avec un `MAKE_DEAL`
  d'ouverture. Proposer envoie la proposition puis ferme la session (`CLOSE`) ;
- `OnShow` lit l'initiateur dans la session ouverte (`FindOpenSessionID` → `FromPlayer`). Si c'est
  soi, `ms_LastIncomingDealProposalAction = PENDING`, et Accepter est caché (`cantAccept` vaut vrai
  dès que l'action n'est ni `PROPOSED`, ni `ADJUSTED`, ni `ACCEPTED`). Seul l'écran rouvert sur la
  session de l'autre (`RESTORE, FromPlayer=<autre>`) part de `PROPOSED` ;
- à 21:30:34, chacun avait sa propre session ouverte quand la proposition de l'autre est arrivée.
  Aucune session de l'autre n'a été mise en attente pour être rouverte. GameCore a pourtant rangé
  la proposition de l'autre dans l'accord entrant (voir plus bas) ;
- à 21:30:41 et 43, le joueur 1 rouvre l'écran depuis le dirigeant : nouvelle session à son nom,
  `PENDING`, pas d'Accepter. C'est le « chacun se voit initiateur » ;
- à 21:30:46–48, le joueur 2 repropose alors qu'aucune session n'est ouverte chez 1. Cette fois la
  session de 2 attend chez 1, qui la rouvre (`RESTORE, FromPlayer=2`) et accepte.

**Ce que fait GameCore** (`GameCore_XP2_FinalRelease.dll` d'origine, projet Ghidra `civ6xp2`,
base 0x180000000). Les fonctions Lua de `DealManager` sont enregistrées en `0x1806d1490`.

| rôle | adresse | ce qu'elle fait |
|---|---|---|
| `HasPendingDeal(a, b)` | `0x18058aae0` | cherche (a, b) dans une liste **locale**, non synchronisée, de paires (de, vers), puis l'état IA des deux joueurs |
| `SendWorkingDeal(action, de, vers)` | `0x18058bfb0` | valide l'accord sortant (sauf `CLOSED`), **ajoute** (de, vers) à la liste, puis : `ACCEPTED`, `REJECTED`, `CLOSED` et `REJECTED_PERMANENT` (masque `0x1106`) partent en message `DiplomacyDeal`. Toute action crée aussi un `MAKE_DEAL` dans la session |
| réception de `DiplomacyDeal` | `0x180035390` → `0x18058b920` | **retire** une fois (de, vers), copie l'accord du message dans le sortant (de, vers) et l'entrant (vers, de). Pour `ACCEPTED`, exécute l'accord (`0x18058a2e0`) après validation |
| réception d'un `MAKE_DEAL` chez le destinataire | `0x18018b420` → `0x18058b890` | retire (de, vers), copie l'accord proposé dans le sortant (de, vers) et l'entrant (vers, de) |
| fermeture de session | `0x18058b770` | retire la paire du dernier `MAKE_DEAL` de la session |

Il en ressort :
- **`HasPendingDeal` marque un envoi pas encore revenu**, plutôt qu'une proposition en attente de
  réponse. Chaque envoi ajoute la paire, et son retour (l'écho du message, la fermeture de session)
  la retire. `CLOSED` ajoute puis retire une entrée : il ne vide pas une entrée restée d'avant ;
- **GameCore ne refuse pas un `ACCEPTED`** parce que l'accepteur a lui-même un accord en attente.
  Ni l'envoi ni la réception ne regardent cette liste. L'acceptation exécute l'accord que porte le
  message, après une validation de son contenu (ressources, or, villes encore disponibles) ;
- la proposition de l'autre est donc bien dans l'accord entrant du joueur bloqué. Seul l'écran,
  ouvert sur sa propre session, ne propose pas de l'accepter.

**Correctif possible, dans l'interface seule.** À l'ouverture de l'écran sur sa propre session
(`ms_InitiatedByPlayerID == moi`), face à un humain, si une proposition de l'autre est restée sans
réponse :
- partir de `PROPOSED` au lieu de `PENDING` ;
- copier l'entrant dans le sortant (`CopyIncomingToOutgoingWorkingDeal`), comme `OnShow` le fait
  pour la session de l'autre. Accepter réapparaît.

Accepter envoie alors le `DiplomacyDeal ACCEPTED` ordinaire. GameCore l'exécute partout, même chez
un joueur sans proxy : aucun format de message ne change, et il suffit que celui qui accepte ait le
proxy.

Reste à savoir **si une proposition de l'autre est toujours sans réponse** : l'accord entrant peut
être ancien. La piste est d'écouter `Events.DiplomacyIncomingDeal(de, vers, action)` dans un contexte
toujours chargé, de retenir la dernière action reçue de chaque joueur, et de l'effacer à toute
réponse : notre `ACCEPTED`, `REJECTED` ou `CLOSED`, son `CLOSED`, ou l'exécution de l'accord. Il faut
vérifier en partie que l'événement part bien quand la session de l'autre a été écartée, comme à
21:30:34.

**Diagnostic en partie : `ui_diag_accords=on`.** Le proxy ajoute
[proxy/src/ui_diag_accords.lua](../../proxy/src/ui_diag_accords.lua) à la fin de
`DiplomacyDealView.lua`. Le bloc ne fait qu'écrire, dans `Lua.log`, des lignes `BGD accords …` :
- les événements qui touchent le joueur local : `entrant` (`DiplomacyIncomingDeal`), `declaration`
  (`DiplomacyStatement`, avec ses champs), `session fermee`, `accord execute` ;
- l'`ouverture` et la `fermeture` de l'écran d'accord ;
- à chaque ligne, l'état : session ouverte et son initiateur, `HasPendingDeal` dans les deux sens,
  nombre d'objets des accords entrant et sortant, et, écran ouvert, l'initiateur, la dernière action
  reçue et le bouton Accepter.

Test hors jeu : `lua proxy/tests/test_ui_diag_accords.lua`. Le test compile le vrai fichier suivi du
bloc, puis joue les événements dans un écran imité.

Protocole, à deux humains, depuis la sauvegarde `BGD bug deal T29` :
1. les deux ouvrent l'écran d'accord l'un avec l'autre, préparent un accord, et cliquent sur
   Proposer à moins d'une seconde d'écart (un décompte à voix haute suffit) ;
2. chacun rouvre l'écran depuis le dirigeant de l'autre, et note si Accepter est là ;
3. on garde `Lua.log` et `net_message_debug.log` de la machine qui a le proxy.

Ce qu'on cherche : une ligne `entrant <autre>><soi> PROPOSED` au moment des deux propositions,
écrite alors que `session=` montre sa propre session. Ensuite, à la réouverture, une ligne
`ouverture` avec `initiateur=<soi> derniere=PENDING accepter=cache` et un `entrant` non vide. Si la
ligne `entrant` manque, le correctif devra s'appuyer sur `declaration` (`DiplomacyStatement`).

MPH remplace aussi cet écran (`diplomacydealview_MPH.lua`) : le correctif doit s'appliquer
par-dessus, comme pour les autres mods d'interface repris.

Le joueur 2 (l'hôte) a quitté la partie une minute plus tard (21:31:49, migration d'hôte), et la
partie a été rechargée au tour 29. Rien n'indique de lien avec l'accord.

**Autre constat de la même session : désynchros réparées à chaque tour.** Dès le tour 2 de la
quatrième partie, l'hôte répare 1 à 12 archives par tour. Ce sont surtout les évaluations de l'IA
du joueur 1, c'est-à-dire du joueur local, qui est humain :
- `AI::Religious 1-…` ;
- `AI::CityBuild 1-…`, avec `m_districtEvaluations`, `m_buildingEvaluations` et
  `m_improvementEvaluations` ;
- les `AI::CultureAi` (`m_greatWorkManager`) des IA 4 à 9.

C'est sans doute l'interface locale qui demande des recommandations de production à l'IA et modifie
ses caches, sur la seule machine du joueur. Le proxy avait coupé ses correctifs : l'hôte ne l'avait
pas. À creuser à part.

## Le proxy GameCore et la synchro

Pendant la partie de 18:50, le proxy était chargé chez l'hôte (`RolloverProxy.log`, horodatage
1793123446 = 18:50:46) en modes `Coalesce` et `Quiet`, et a traité une cascade de 358 déclarations de
guerre à 18:51:40. Les contrôles de fin des tours 78 et 79 (18:51:38 et 18:52:05) ont répondu
`numDeltaArchives=0` : **aucun écart sur l'état synchronisé**.

Deux réserves :
- on ne sait pas si theflowgate avait le proxy. Si oui, deux exécutions identiques peuvent concorder
  même avec un changement de règles. Le test probant est hôte avec proxy, client sans ;
- le contrôle ne couvre que les `AutoArchive`.

## Pistes de stabilité

Par ordre d'intérêt pour ce projet :

1. **Se servir du contrôle de synchro comme banc de test du proxy.** Une machine avec proxy et une
   sans, sur la même sauvegarde, plusieurs tours ; tout `numDeltaArchives>0` signale une rupture du
   déterminisme. Le nom des archives fautives est déjà dans `net_message_debug.log`, des deux côtés.
   Les chaînes `testSync` et `Forcing Debug Resync` suggèrent un mode de test caché à retrouver.
2. **Désynchros répétées.** La décompilation le confirme : la réparation ne recopie que les
   `AutoVariable` fautives. Un état dérivé non archivé qui diverge refait donc diverger la partie au
   tour suivant. C'est une cause plausible des « désynchros à chaque tour » connues de Civ6. Et
   quand le delta échoue, le client retombe sur la sauvegarde complète.
3. **Scripts des mods.** Dans le tour 78, le client envoie des `PlayerOperation EXECUTE_SCRIPT` pour
   les joueurs 2 à 7, et des messages de chat techniques (`.mph_ui_seed_…`), venant d'un mod. Du Lua
   d'interface qui lit un état local (sélection, caméra, horloge) et l'injecte en opération de jeu
   est une source classique de désynchro. Identifier le mod et ce qu'il envoie.
4. **Entrée et reconnexion.** 2,7 Mo en morceaux de ~1,2 ko : reconnexion lente, d'autant plus que la
   partie avance. À mesurer avant d'agir (compression ? débit de `SteamNetworking005`, limité et
   déprécié par Valve).
5. **Fil principal et timeouts.** Le keepalive vit dans le thread de transport, donc un passage de
   tour long ne devrait pas couper un lien. Mais la sauvegarde automatique (~500 ms) et la boucle
   principale traitent les messages : à confirmer qu'aucun timeout applicatif (vote, migration,
   `NetSessionTimeUpdate`) ne dépend de ce fil.
6. **Remplacer le transport Steam** (`ISteamNetworking` → `ISteamNetworkingSockets`) : c'est la
   cause supposée de beaucoup de déconnexions, mais il faudrait réécrire une partie de FireWire dans
   l'exe sans symboles. Gros chantier, à garder pour plus tard.

## Pour la suite

- Nommer les threads de transport et `FNetSession` dans le projet Ghidra déjà analysé, avec la même
  méthode (`DumpByStrings.java` et une liste de mots-clés).
- Savoir quel objet de GameCore porte quelles archives : le nom d'archive écrit dans le journal
  (`140dca310` sur `archive + 0x58`) fait le lien.
- Journaux utiles : `%LOCALAPPDATA%\Firaxis Games\Sid Meier's Civilization VI\Logs\`
  `net_message_debug.log` (messages), `net_connection_debug.log` (liens, lobbys, sérialisation),
  `EOS.log`. `capture.ps1` copie déjà le premier.
