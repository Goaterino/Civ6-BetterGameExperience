# Départ de tour simultané

Objectif : quand tous les joueurs ont le proxy, tous les humains commencent leur tour au même
instant, et le finissent donc au même instant. Plus de *first move* pour la machine rapide, plus de
*last move* pour la lente. L'écart est reporté sur le temps de chargement ; seul le ping reste.

État (2026-11-05) : implémenté, testé hors jeu, **pas encore essayé en partie**. Voir
[Implémentation](#implémentation) et [Reste à vérifier en partie](#reste-à-vérifier-en-partie).

## Le problème, dans les journaux

Partie du 2026-10-29 (`netcode/runs/20261029-reconnexion`, tour 78 → 79) :

| | hôte | client |
|---|---|---|
| `GameSyncComplete`, `GameUpdate InitialTurnProcessing=1` | 13:00:39 | 13:00:39 |
| `Player 0 set TurnActive 1` et `Player 1 set TurnActive 1` | **13:00:47** | **13:00:49** |

- Le passage de tour est calculé par chaque machine. L'hôte cadence le tour des IA : un
  `GameUpdate From=0` par IA, que le client attend. Un client ne peut donc pas devancer l'hôte, il
  peut seulement prendre du retard.
- L'activation des tours humains est **locale** : aucun message réseau, chaque machine la fait quand
  sa simulation y arrive. Ici, 2 s d'écart ; avec un processeur faible, bien plus.

## Ce que fait la synchro de tour de MPH (et pourquoi on la coupe)

Option `CPL_SYNCTURN` (« Synchronisation du tour »), `ui/Additions/TurnProcessing.lua` de MPH
(Workshop 3307282026) :

- à `Events.TurnEnd`, MPH marque chaque humain « PROCESSING » et ouvre un voile noir plein écran
  (`ConsumeMouse="1"`, toutes les touches avalées sauf Alt, Ctrl, Maj, effets en pause) ;
- il marque un joueur « READY » à `LocalPlayerTurnBegin` / `RemotePlayerTurnBegin`, et ferme le
  voile quand tous le sont (vérifié à chaque `GameCoreEventPublishComplete`).

Or ces deux événements naissent de l'activation **locale** du tour : chaque machine se libère dès
que sa propre simulation est arrivée, sans rien attendre des autres. La synchro ne synchronise
rien : elle masque l'écran pendant le passage de tour local. Le proxy la **désactive de force**
(voir [Neutraliser `CPL_SYNCTURN`](#neutraliser-cpl_syncturn)).

## Conception

1. **Prêt.** Quand le tour local a commencé et que l'interface a rattrapé la simulation (premier
   `GameCoreEventPublishComplete` après `LocalPlayerTurnBegin`), la machine envoie à l'hôte un
   message de chat caché `.bgd_pret <tour>`.
2. **Départ.** L'hôte, quand tous les humains connectés (lui compris) sont prêts pour ce tour,
   envoie à tous `.bgd_depart <tour>`. Délai maximal côté hôte ; un joueur déconnecté sort de la
   liste attendue.
3. **Attente.** Entre l'activation locale et le départ, le joueur voit tout et peut tout consulter,
   mais ses ordres sont **rejetés** (pas mis en file : une rafale au départ recréerait le first move)
   et son minuteur est gelé. Un bandeau discret dit qui on attend.
4. **Départ reçu.** Ordres débloqués, minuteur lancé à cet instant.

Activation : il faut que l'option de partie « No First/Last moves (BGD) » soit cochée **et** que tous
les humains aient le proxy (verdict de l'hôte, comme les correctifs POL), puisqu'un joueur sans proxy
ne serait pas bloqué.

### L'option de partie « No First/Last moves (BGD) »

C'est une option de la config de partie, choisie par l'hôte dans le salon comme les options de MPH,
et non un réglage de `rollover_proxy.cfg`. Seule l'option d'affichage des signaux du chat caché
(ci-dessous) est un réglage de BGD, dans l'onglet hors salon.

**Comment l'injecter.** Les options de partie viennent de la base `Configuration` :
`SetupParameters.lua` lit la table `Parameters` (et ses tables liées) par `DB.ConfigurationQuery`,
avec un cache vidé quand `DB.ConfigurationChanges()` change. MPH ajoute les siennes par une ligne
`Parameters` de son `Config.xml` (`CPL_SYNCTURN` : `Domain="bool"`, `ConfigurationGroup="Game"`,
`GroupId="GameOptions"`, `SupportsSinglePlayer="0"`, `SupportsHotSeat="0"`,
`SupportsPlayByCloud="0"`). Le proxy n'est pas un mod. Pistes, dans l'ordre :
1. le proxy de démarrage **est** `DatabaseDLL`, la couche SQLite du jeu : il peut insérer la ligne
   dans la base `Configuration` après chaque reconstruction de celle-ci (à chaque changement de
   mods). Il faut pour cela intercepter la connexion ou l'exécution, aujourd'hui de simples renvois
   PE ;
2. plus simple si cela marche : faire l'`INSERT` depuis le Lua injecté par
   `DB.ConfigurationQuery`, si elle accepte autre chose qu'un `SELECT` (à essayer) ;
3. en dernier recours, une case dessinée par le Lua injecté dans le salon, qui écrit la clé avec
   `GameConfiguration.SetValue` puis `Network.BroadcastGameConfig()`.

Le libellé (`Name`, `Description`) passe par `Locale.Lookup` : texte en clair, ou clé `LOC_`
ajoutée à la base de localisation.

**Un joueur sans le proxy dans le salon.**
- La valeur voyage dans `GAME_ROOT` comme toute clé de mod : la réception d'un `NetGameConfig`
  (`1402662d0`) recopie l'arbre sans contrôle, et `SetupParameters` ne parcourt que les lignes de
  *sa* base. Ce joueur ne voit pas l'option et ignore la clé : pas de plantage attendu. À
  confirmer par un essai avec le kit du salon (`analyses/lobby/kit`) : jusqu'ici, aucune machine
  n'a été observée recevant une clé de partie qu'elle ne déclare pas (les clés de mods arrivent
  chez des joueurs qui ont les mêmes mods), et il faut aussi vérifier que le jeu ne compare pas le
  contenu de la base `Configuration` entre machines.
- Il fait tomber le verdict : la barrière n'est pas activée, pour personne, et l'hôte le dit dans
  le chat.
- Cocher ou décocher l'option diffuse la config de partie et remet tout le monde « pas prêt »,
  comme toute option changée par l'hôte.
- Hôte sans le proxy : l'option n'existe pas chez lui, donc pas de barrière. La clé restée dans une
  sauvegarde ne suffit pas : il faut aussi le verdict du salon en cours.

**Un joueur sans le proxy qui arrive en cours de partie.** La barrière ne touche pas la simulation :
elle rejette des ordres avant leur envoi et gèle un minuteur local. L'hôte peut donc la couper pour
tous à tout moment, sans désynchro (`.bgd_` de fin), au lieu de l'attendre. Il ne compte de toute
façon que les joueurs qui se sont annoncés, et ce joueur verrait les signaux `.bgd_` en clair dans
son chat tant qu'elle reste active.

### Canal : chat caché

`Network.SendChat` envoie un `NetChat` de 61 à 68 octets en un morceau ; un
`Network.BroadcastPlayerInfo` envoie toute la config du joueur, 5,1 Ko en 5 morceaux, et déclenche
`PlayerInfoChanged` dans une dizaine de contextes d'interface (journaux du 2026-10-29). Le chat
n'est pas non plus sauvegardé avec la partie : pas de valeur périmée au rechargement.

MPH s'en sert déjà : `.mph_ui_seed_m…` / `.mph_ui_seed_g…` (`MPHOptions.lua`), cachés par
`chatpanel_MPH.lua` qui jette tout message commençant par `.mph`. Le proxy fera de même pour
`.bgd_` en enveloppant `OnChat` de `ChatPanel` ; plus tard, si un autre mod de chat les affichait, en
les jetant dans l'exe à la réception (fonction qui journalise `NetChat : … said`).

**Option « afficher les signaux du chat caché »** (demandée le 2026-11-05), désactivée par défaut :
si elle est cochée, les messages `.bgd_` ne sont pas jetés et apparaissent dans la fenêtre de chat,
pour suivre le dispositif en partie. Réglage local, propre à chaque joueur : il ne change que
l'affichage. Comme tout réglage du proxy : clé lue par `read_config`, option dans
`proxy/install.ps1`, entrée dans `PROXY_REGLAGES` (`startup_proxy/src/options_proxy.lua`), avec le
compte de réglages de `test_options_proxy.lua` à mettre à jour. Si le filtrage passe un jour dans
l'exe, l'option doit le court-circuiter aussi.

## Envoi des ordres (GameCore_XP2)

Tout message sortant de GameCore passe par **`1800366b0`** (`Messages_Manager.cpp`), qui journalise
`GameCore SEND: %s` dans `net_message_debug.log`, sérialise le message, puis le remet à l'exe par
`(*DAT_180b8aa80)->vtable[0x70]`. L'ordre local n'est exécuté qu'à son retour du réseau
(`SEND` puis `RECV (1)` dans les journaux) : un message jeté avant cet appel n'existe nulle part,
pas de désynchro.

Disposition d'un message : `+0x00` vtable, `+0x08` identifiant de type (u32, sérialisé en tête),
`+0x10` nom (`char*`). Identifiants relevés dans les constructeurs :

| type | identifiant | constructeur | pendant l'attente |
|---|---|---|---|
| `UnitCommand` | `0x493830af` | `18003a2b0` | rejeter |
| `UnitOperation` | `0x8c0f3f36` | `18003a640` | rejeter |
| `CityCommand` | `0x346b0a13` | `180033c40` | rejeter |
| `CityOperation` | `0x7ce9adc2` | `180034070` | rejeter |
| `DiplomacyAction` | `0xa797ad1c` | `180034f80` | rejeter (déclarations de guerre) |
| `DiplomacySession` | `0x08a6cf5f` | `180035570`, `1800355e0` | rejeter |
| `DiplomacyStatement` | `0xa4172391` | `180035ab0` | rejeter, à tester : l'appelant `1801887a0` tient un état « Adding to pending » |
| `PlayerOperation` | `0xc45de12f` | `180038990` | rejeter si `ePlayer` (`+0x18`) est le joueur local, sauf `EXECUTE_SCRIPT` (opération en `+0x20`, `InsertMode` en `+0x24`) |
| `CivicsCommand` | `0xe57882af` | `1800345d0`, `180034630` | à décider : choix de recherche ou de dogme, sans enjeu d'ordre |
| `GameTurnComplete` | `0x1bdb4f5c` | `180035ea0` | laisser passer |

Laisser passer aussi tout le reste : `GameUpdate` (l'hôte cadence les IA par lui), `PlayerTurnComplete`
(finir son tour pendant l'attente ne donne rien), et les `EXECUTE_SCRIPT` que BBG envoie pour les IA
au début du tour (`ePlayer=2…`, hôte et client). Les demandes venues de l'exe passent par le
répartiteur `180590880` (par exemple `0x2cc9a6b5` → `PlayerTurnComplete`, raison `0xb6fa1284`).

Le proxy de GameCore pose donc un détour sur `1800366b0` : en attente, si le type est dans la liste,
il ne fait rien (et le journalise) ; sinon il appelle l'original.

## Minuteur de tour (exe, `CivilizationVI_DX12.exe`)

Projet Ghidra `~/Tools/ghidra-projects/civ6exe`. Complète
[analyses/congres](../analyses/congres/README.md#exe-minuteur-et-annulation) et
[nuke](../nuke/README.md).

Le gestionnaire (objet `this`, enregistré par `1400f55a0` / `1400f7e80`) tient deux chronos sur
`QueryPerformanceCounter` (`140998430`, conversion en secondes par `140998220`) :

| chrono | départ | cumul | en marche |
|---|---|---|---|
| A | `+0x18` | `+0x20` (double) | `+0x28` |
| B | `+0x30` | `+0x38` (double) | `+0x40` |

- **`1400f4df0`** (temps écoulé) : `cumul + (en marche ? maintenant − départ : 0)`, sur A ou B
  selon une méthode virtuelle (`+0x40`) de l'objet de `DAT_14230d690`. C'est lui que lisent la fin de
  tour automatique (`1400f8170`), `UI.IsTurnTimerElapsed` et `Events.TurnTimerUpdated` (`1400f5860`).
- **`1400f6300`** (rappel de `GameCoreEvent::PlayerTurnActivated`, emplacement `1412fe1d8`) :
  remet B à zéro et en marche ; remet aussi A si le drapeau `+0x11` est levé et que le joueur
  activé passe le test `1401165b0`. Puis appelle `1400f8590`. **Le minuteur part donc à
  l'activation locale du tour**, d'où l'écart.
- **`1400f8590`** (pause) : si la partie est en pause (`140102d80`) ou le minuteur est
  `TURNTIMER_NONE` (`0xa5196dab`), arrête A et B en ajoutant le temps couru au cumul ; sinon les
  relance (départ = maintenant).

Pour le proxy de l'exe :
- détour sur `1400f8590` : en attente, toujours prendre la branche « arrêt » (minuteur gelé, il ne
  bouge plus à l'écran et la fin de tour automatique ne part pas) ; capturer `this` au passage ;
- au départ : pour A et B, cumul = 0, départ = `QueryPerformanceCounter`, en marche = 1.

La durée limite (`1400f4d50`) reste celle de la machine : `TURN_TIMER_TIME` local, ou la valeur de
GameCore en `TURNTIMER_DYNAMIC`.

## Neutraliser `CPL_SYNCTURN`

MPH relit `GameConfiguration.GetValue("CPL_SYNCTURN")` à chaque événement (`Refresh_Data`,
`OnTurnEnd`, `OnTick`, tours début et fin), et `OnTick` se retire de lui-même quand la valeur est
fausse. `GameConfiguration.SetValue` en jeu est local à la machine ; MPH l'emploie déjà ainsi pour
`TURN_TIMER_TIME`.

Le Lua injecté à la fin de `InGame.lua` (`startup_proxy/src/partie.lua`, qui vise aussi le
`ingame.lua` de MPH) fera, au chargement, sur chaque machine :
`GameConfiguration.SetValue("CPL_SYNCTURN", false)`, puis `LuaEvents.InGame_CloseTurnProcessing()`
au cas où le voile serait ouvert. Le minuteur intelligent de MPH (`CPL_SMARTTIMER`, calculé à
`TurnEnd`) n'en dépend pas et continue de marcher.

## Fin de tour simultanée (même option)

Le départ simultané ne règle que les first moves. Pour les last moves, ce qui compte est l'instant
où chaque machine envoie son `TurnComplete` : **chacune n'envoie que celui de son joueur local**,
quand **son** minuteur est écoulé (`1400f8170`), et le tour ne se termine qu'une fois tous reçus.
La machine dont le minuteur expire en dernier garde donc une fenêtre où ses ordres passent encore.

Le minuteur compte en temps réel (`QueryPerformanceCounter`, pas image par image) : un PC lent ne
le ralentit pas. Ce qui lui donne plus de temps : un départ plus tardif (réglé par le départ
simultané), une durée différente (`TURN_TIMER_TIME` est local, et MPH l'écrit sur chaque machine
sans la diffuser), une pause locale (branche « arrêt » de `1400f8590`), ou une remise à zéro plus
tardive. Plutôt que recaler les horloges, **l'hôte décide de la fin** :

1. Quand le minuteur de l'hôte arrive à zéro (`Events.TurnTimerUpdated(écoulé, limite)`, écoulé ≥
   limite), il envoie `.bgd_fin <tour>` en privé à chaque humain attendu (même canal caché).
2. À réception, si le tour local est encore actif, `Civ6RolloverProxy_FinForcer()` ajoute 10⁶ s au
   cumul des deux chronos de l'exe. Le temps écoulé dépasse la limite : l'exe envoie lui-même le
   `TurnComplete` à l'image suivante, par son chemin normal, et refuse d'annuler la fin de tour
   (`1401c9c10`). Une attente du départ encore en place est levée.
3. Au début du tour suivant (`LocalPlayerTurnBegin`), `Civ6RolloverProxy_FinLever()` retire ces
   10⁶ s des chronos que le jeu n'a pas remis à zéro : aucun ne garde la trace de la fin forcée.
4. Chaque machine garde son propre minuteur : la première échéance l'emporte.

L'hôte part à T0, les autres à T0 + ping/2 ; sa fin arrive chez eux à T0 + durée + ping/2, pile à
la fin de leur propre durée. L'écart se réduit au ping, comme au départ.

- **Armement.** L'hôte n'envoie la fin qu'après avoir vu son minuteur sous la limite pendant le
  tour : une mise à jour encore écoulée du tour précédent, au début du nouveau, ne le termine pas.
  Un nouveau segment (congrès) remet le minuteur à zéro et le réarme.
- **Pas d'ordres retenus après la fin.** Entre la fin d'un tour et le début du suivant, la machine
  envoie encore les réponses du joueur à la diplomatie des IA : les retenir bloquerait ces
  sessions. Le `TurnComplete` de l'exe suffit.
- **Conditions** : la même option de partie (`BGD_DEPART_SIMULTANE`, « No First/Last moves
  (BGD) ») que le départ, correctifs POL actifs (tous les humains ont BetterGameDLLs à la même
  version), et minuteur de l'exe détourné. Sans minuteur de tour (limite 0), rien ne se passe.
  L'hôte l'annonce au chargement, dans le même message que le départ. La ligne locale « fin du
  tour : le minuteur de l'hôte est écoulé » n'apparaît qu'avec le réglage `depart_signaux`.

## Implémentation

| où | quoi |
|---|---|
| [proxy/src/depart.rs](../proxy/src/depart.rs) | détours de l'envoi (`1800366b0`) et de la pause du minuteur (signature, DX12 et DX11) ; fonctions natives `Civ6RolloverProxy_Depart*`, `Civ6RolloverProxy_FinForcer` et `Civ6RolloverProxy_FinLever` ; réglage `depart_signaux` |
| [proxy/src/ui_depart.lua](../proxy/src/ui_depart.lua) | à la fin de `InGame.lua` : `CPL_SYNCTURN` coupé, protocole prêt / départ, fin envoyée par l'hôte |
| [proxy/src/ui_depart_chat.lua](../proxy/src/ui_depart_chat.lua) | à la fin de `ChatPanel.lua` : signaux `.bgd_` cachés, lignes locales `BGD - …` |
| [startup_proxy/src/option_partie.lua](../startup_proxy/src/option_partie.lua) | à la fin de `SetupParameters.lua` : ligne `BGD_DEPART_SIMULTANE` insérée dans `Parameters` (approche n° 2) |

Choix faits à l'implémentation :
- **Début de l'attente** : à `LocalPlayerTurnBegin`, quand le tour des IA est fini. Commencer plus
  tôt retiendrait aussi les actions diplomatiques des IA envers le joueur, que sa machine envoie.
  Au chargement d'une partie, le signal « prêt » attend la fermeture de l'écran de chargement.
- **Qui est attendu** : les humains vivants, connectés, pas spectateurs, et qui annoncent la même
  version dans leur config de joueur (`BGD_VERSION`). Un humain sans BetterGameDLLs n'est pas
  attendu : il ne serait pas bloqué de toute façon.
- **Délais** : l'hôte part sans les retardataires après 60 s et les nomme (ligne locale) ; sans
  départ reçu, le proxy lève l'attente au premier ordre tenté après 120 s.
- **`CivicsCommand`** (choix de recherche ou de dogme) passe : son ordre n'a pas d'enjeu.
- **Annonce** : au chargement, l'hôte écrit dans le chat, pour tous, si le départ simultané est
  actif, ou pourquoi il ne l'est pas.
- **Prévenir le joueur** : au premier ordre retenu du tour, une ligne locale « ordres retenus »,
  puis « c'est parti » au départ.

## Reste à vérifier en partie

- ~~`DB.ConfigurationQuery` accepte-t-il l'`INSERT` (approche n° 2) ?~~ Oui, vérifié en jeu le
  2026-11-05 : l'option apparaît dans les options de partie. Reste à voir sa valeur chez les clients.
- Un joueur sans le proxy dans le salon : la clé inconnue ne le gêne pas, et le jeu ne compare pas
  le contenu de la base `Configuration` entre machines.
- Rejet sans effet de bord côté interface, surtout `DiplomacyStatement` (état « pending ») et les
  ordres d'unité (chemin affiché, unité marquée en mouvement).
- `PlayerOperations.EXECUTE_SCRIPT` correspond bien à l'opération en `+0x20` (sinon les scripts de
  BBG du joueur seraient retenus : journal `PlayerOperation retenu`).
- Les chronos gelés : le minuteur ne bouge pas pendant l'attente, puis part de son temps complet.
  Lequel des deux chronos sert en partie standard (méthode `+0x40`).
- `ContextPtr:SetUpdate` du contexte InGame est libre (aucun script du jeu ni de MPH ne s'en sert).
- Écart réel avec un processeur faible (horodatages `TurnActive 1` de `net_message_debug.log`).
- Fin simultanée : `Events.TurnTimerUpdated` continue chez l'hôte après sa propre fin de tour, et
  le `TurnComplete` part bien à réception de `.bgd_fin` (journal de l'exe `Turn Timer Sending
  TurnComplete, turnTimeLimit: …, elapsedTurnTime: …`, qui dira aussi si les durées diffèrent
  d'une machine à l'autre). Le congrès : le minuteur repart-il de zéro au changement de segment ?
