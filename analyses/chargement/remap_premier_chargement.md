# Remap : alléger le premier chargement de l'hôte

Analyse statique et lecture de capture du 2026-11-17, sans modification de code. Exe
`CivilizationVI_DX12.exe` du projet Ghidra `civ6exe1105` (identique au fichier installé), DX11
`CivilizationVI.exe` par recherche de motifs dans le fichier, GameCore `civ6xp2`. Capture
`rollover/captures/20261117-132539-apres-leviers-3` (hôte seul), chargements 2 (nouvelle partie) et 3
(copie de l'hôte).

## Conclusion

**Le jeu sait déjà démarrer une partie sans construire le terrain.** `GameViewState::StartupGame`
(`14007f5f0`) prend un mot de drapeaux (3ᵉ argument, rangé en `GameViewState+0xd80`). Le bit 2
saute la synthèse du terrain et toute la WorldView, et les autres méthodes de `GameViewState` le
respectent : mise à jour, étape d'après-chargement, fermeture. La machine à états du chargement ne
passe ce bit que pour le World Builder (`WORLD_BUILDER` dans la configuration de la partie). Sinon,
elle passe 0.

Le levier retenu : **au premier chargement de l'hôte au remap, et à lui seul, appeler StartupGame
avec le bit 2.** Rien d'autre ne change :
- l'interface InGame est chargée, donc `VotePanel` déclenche le rechargement comme aujourd'hui ;
- les gestionnaires Lua (BSM…) voient les mêmes événements ;
- la sauvegarde et l'instantané suivent le même chemin natif.

Gain estimé : **environ −11 s sur les 22 s du chargement 2**. StartupGame passe de 9,8 s à
quelques dixièmes, et PostLoadTime de 2,3 s à presque rien.

**La synchro n'est pas en jeu.** La PR #12 garantit que l'hôte et les clients partent tous d'une
partie désérialisée :
- l'hôte recharge sa sauvegarde S1 ;
- les clients reçoivent S2, prise après ce rechargement.

Ce levier ne change que la façon dont S1 est produite, pas le fait que tout le monde recharge. Les
risques sont des plantages pendant la courte vie de la partie sans terrain, et un repli qui
laisserait l'hôte jouer sans terrain. Les deux se traitent, voir plus bas.

**Le court-circuit complet est écarté.** Il s'agirait de sauvegarder juste après NEW_GAME, sans
jamais entrer dans `GameViewState`. Deux gardes natives l'interdisent :
- l'instantané n'est produit que si l'application est dans `GameViewState` ;
- un instantané reçu pendant un chargement est ignoré.

Le seul moyen de sortir de `LoadGameViewState` sans passer par `GameViewState` est la sortie
anticipée, qui quitte la session réseau. Ce court-circuit ne gagnerait qu'environ 1 à 2 s de plus
que le bit 2.

## 1. Le flux de la PR #12 côté hôte

Code : [remap.rs](../../startup_proxy/src/remap.rs) et [remap.lua](../../startup_proxy/src/remap.lua).
Le bloc est ajouté à `VotePanel.lua` de MPH, et le seul `Network.SendChat(".mph_ui_snap",-2,-1)`
de MPH devient `BGD_RemapSnap()`.

| quand (capture) | où | quoi |
|---|---|---|
| 13:26:18 | `VotePanel.lua` `OnLocalHostRestart` (ou `.mph_ui_remap_execute`) | `GAME_HOST_IS_JUST_RELOADING = "Y"`, `BroadcastGameConfig`, pause, `Network.RestartGame()` |
| 13:26:19 → 13:26:29 | chargement 2 : CONFIGURE_CONTENT 3,26 s, GAMECORE_NEW_GAME 5,88 s (carte BBS), INITIALIZATION_DONE 1,56 s | GameCore neuf, carte générée, scripts de gameplay (MP_helper, BSM, bbg_script…), puis interface InGame (123 fichiers Lua) |
| **13:26:30** (WAIT_FOR_PLAYERS, StartupGame en cours) | `VotePanel.OnRefresh` sur `Events.GameCoreEventPublishComplete`, **pendant le chargement** | `GAME_HOST_IS_JUST_RELOADING = "N"`, `BroadcastGameConfig`, puis `BGD_RemapSnap()` : `Civ6Proxy_RemapMark()` et `Network.RequestSnapshot()` → `NetSnapshotRequest PlayerID=6, Reason=0` envoyé à soi-même (`+0x1a` levé) |
| 13:26:30 → 13:26:41 | StartupGame 9,79 s, PostLoadTime 2,33 s | événements de départ vus par l'interface. BSM y applique des bonus d'écriture et met à jour sa liste par `ExposedMembers` (gameplay de l'hôte, sans réseau) |
| 13:26:41 | `LoadGameViewState::End`, l'application passe dans `GameViewState`, « Queuing Messages End » | `HandleSnapshots` (`140840260`) trouve `+0x1a` et lance « Creating new save request » (`140275d90`) |
| 13:26:42 | `1402731a0` « Snapshot save complete », `1402760a0` | `NetSnapshot` de 1,14 Mo (S1) envoyé à soi-même, `ProcessSnapshot` (`140275ad0`) lance le chargement (`14016d980`) |
| 13:26:42 → 13:27:02 | chargement 3 (copie) : 3,45 / UNSERIALIZE 1,47 / 1,60 / 11,14 / 2,41 | |
| 13:27:00 | `remap.lua` `Tick` (interface fraîche, `Civ6Proxy_RemapTake()` ≥ 0) | `.mph_ui_snap` : les clients demandent l'instantané. L'hôte en produit un nouveau (S2) depuis sa partie rechargée |

Ce que le premier chargement doit fournir à la sauvegarde S1 :
- l'état de GameCore après `InitializeNewGame` : carte, départs, propriétés posées par les scripts
  de gameplay au chargement ;
- la configuration (`GAME_HOST_IS_JUST_RELOADING = "N"`) ;
- les effets des gestionnaires d'interface qui modifient le gameplay de l'hôte pendant le
  chargement (BSM : `Events.DiplomacyMeet`, `Events.TechBoostTriggered`, puis
  `UIEvents.HostWritingUpdate` et `HostPopCheckList`, voir `Lua.log` lignes 927-1185).

Ces événements de départ ne sont **pas rejoués** au chargement 3, qui part d'une sauvegarde. S'ils
ne sont pas vus au chargement 2, ils sont perdus. C'est pourquoi le levier garde l'interface.

Rien du terrain 3D, du cache de vue (`WorldView_PostLoadUpdate`) ni des caches de textures n'entre
dans GameCore. La sauvegarde est demandée par `HandleSnapshots` aux conditions suivantes (vtable de
l'application `140fe2928`) :

| entrée | fonction | condition |
|---|---|---|
| `+0xc8` | `140273040` | l'état de la configuration vaut `0xCF36F2EE` |
| `+0xd0` | `140272450` | l'état courant de l'application (`app+0x5420`) est `app+0x14f8` (`GameViewState`) ou `app+0x14f0` |
| `+0xd8` | `140273060` | emplacement du joueur valide |
| `+0xe0` | `140275d90` | demande la sauvegarde |

C'est la condition `+0xd0` qui retient la sauvegarde jusqu'à la fin de `LoadGameViewState`.

## 2. La machine à états du chargement

`LoadGameViewState` : vtable `140fbc810`. Les méthodes :
- `Begin` `140083a20` ;
- `End` `140083d30` ;
- `Update` `140089aa0` (fil principal) ;
- `LoadUI` `140089670` : seulement le contexte `LoadScreen` ;
- `UnloadUI` `14008a1f0`.

Le fil de chargement `140083970` appelle `140084850` en boucle tant que l'état (`+0x238`) n'est pas
10, 11 ou 12.

| état | contenu |
|---|---|
| 0 → 1 → 2 | CONFIGURE_CONTENT (`14024d340`…) |
| 3 | options de journal, puis 6 (nouvelle partie) ou 7 (chargement) |
| 4, 5 | GAMECORE_LOAD_SAVEFILE(_PENDING) |
| 6 | GAMECORE_NEW_GAME (`1400840c0`, `InitializeNewGame`) → 8 |
| 7 | GAMECORE_UNSERIALIZE_GAMESTATE → 8 |
| 8 | GAMECORE_INITIALIZATION_DONE (`140117800`, `14024cb80`, `GameViewState::LoadUI` donc l'interface InGame, « Post-GameCore asset load ») → 9 |
| 9 | WAIT_FOR_PLAYERS : `drapeaux = IGameContext->vtbl[0x1b0]() ? 3 : 0`, puis `StartupGame(app->GameViewState, p2, drapeaux)` → 10 |
| 10 | fil principal, `Update` `140089aa0` : tant que `140080790` rend vrai (une étape de `WorldView_PostLoadUpdate` `14054e960` par image), PostLoadTime n'est pas fini ; ensuite état 0xc, puis `End` |
| 0xb | sortie anticipée : `14007eda0` et « leaving the network session » |

`IGameContext` vient de `DllCreateGameContext` (GameCore `180752d50`, vtable `180a101e8`).
L'entrée `+0x1b0` (`1805906c0` → `18060b5b0`) lit le booléen de configuration `WORLD_BUILDER`
(`180001c20`).

### Ce que le bit 2 saute (`GameViewState+0xd80 & 2`)

Toutes les références à `+0xd80` dans l'exe (ScanDisp) sont dans `GameViewState` :

| fonction | rôle | avec le bit 2 |
|---|---|---|
| `14007f5f0` StartupGame | | saute `14054e4d0` / `14060a9c0` (la WorldView), `140665d60` et **`1405531f0`, la synthèse du terrain** ; pose `+0xfd8 = 1` |
| `140080790` | étape d'après-chargement | rend 0 aussitôt : pas de `WorldView_PostLoadUpdate`, PostLoadTime fini à la première image |
| `140080240` | mise à jour de `GameViewState` (vtable `140fbbb40`) | saute `140665fb0` et la mise à jour de la vue |
| `14007d620`, `14007e000` | perte et reprise du focus | sautent `14054e670` / `14054e700` |
| `14007e390` | | `+0xfd8 = 1` |
| `14007eda0` | fermeture de `GameViewState` (appelée aussi au rechargement) | saute `140552660` et `140665cd0` (destruction du terrain), cohérent avec un terrain jamais construit |

Le bit 1 ne touche que la qualité des leaders (`1401ee080` au démarrage, `1401edea0` à la fermeture).

**Mesure dans la capture** (`piles.py chaine`, chargement 2, fil de chargement 27104, fenêtre de
WAIT_FOR_PLAYERS) : 7 130 ms échantillonnées sous StartupGame (retour `88eb0`), dont **7 055 sous
l'appel de `1405531f0`** (retour `7ffa9`). Il ne reste donc qu'environ 75 ms de travail propre hors
terrain. Le reste du temps mur (attentes WrResource de la création des caches `140c6cca0` derrière
`Present`) relève aussi du terrain.

## 3. Les variantes et leurs gains

Gains estimés sur le chargement 2 de la capture (22,8 s au total, phases 3,26 + 5,88 + 1,56 + 9,79
+ 2,33).

| variante | ce qui est sauté | gain | risque | verdict |
|---|---|---:|---|---|
| **A. Bit 2 à StartupGame** (hôte, premier chargement du remap) | terrain, WorldView, cache de vue, caches de textures, PostLoad | **≈ −11 s** (StartupGame −9,3, PostLoad −2,0 ; il reste SerialEvent `FinishedGameplayContentConfigure`, 0,3 s) | moyen-faible : chemin du World Builder, mais dans `GameViewState` avec l'interface InGame | **retenue** |
| A'. Drapeaux 3 (comme le World Builder) | en plus, l'initialisation des leaders | quelques centaines de ms au plus | un écran de rencontre au départ (`DiplomacyMeet`) ouvrirait la scène des leaders non préparée | non |
| B. Rythme sans VSync pendant l'écran de chargement (déjà analysé) | les attentes de `Present` | −2 à −3 s par chargement | faible | à garder pour le chargement 3 et les clients ; recoupe A au chargement 2 |
| C. Interface InGame non chargée au chargement 2 | corps des 123 fichiers Lua, contrôles d'InGame | ≈ −1,1 à −1,4 s en plus de A | élevé : plus de `VotePanel` (il faut un déclencheur natif), les événements de départ de BSM perdus (changement de gameplay) | non |
| D. Court-circuit complet après NEW_GAME ou INITIALIZATION_DONE | tout sauf configuration, NEW_GAME et sauvegarde | ≈ −1 à −2 s en plus de A | très élevé : contourner `140272450` (sauvegarde seulement en `GameViewState`) et `140169bf0` (instantané ignoré pendant un chargement), avec la sortie anticipée qui quitte la session | non |

Chargement 2 estimé avec A : environ 3,3 + 5,9 + 1,6 + 0,3 + 0,3 ≈ **11-12 s** au lieu de 22 s,
plus environ 1 s de sauvegarde (inchangée).

## 4. Design du correctif (variante A)

### Site à patcher

L'appel de StartupGame dans l'état 9 de `140084850`. On redirige le `call rel32` vers un relais
Rust (`motifs::rediriger_appel`), qui appelle l'original avec `drapeaux | 2` quand c'est armé.
Les drapeaux sont en `R8D`, l'objet `GameViewState` en `RCX` et le 2ᵉ argument en `EDX`. Le relais
est donc `extern "C" fn(gvs: *mut c_void, p2: u32, drapeaux: u32)`.

Motif (unique dans les deux exe), ancré sur l'appel `IGameContext +0x1b0` :

```
ff 90 b0 01 00 00 48 8b 8d ?? ?? ?? ?? 0f b6 f8 48 85 c9 74 06 48 8b 11 ff 52 08
40 84 ff 41 8b de 41 0f 45 dd e8 ?? ?? ?? ?? 48 8b 0d ?? ?? ?? ?? 44 8b c3 8b d6
48 8b f8 48 8b 89 ?? ?? 00 00 e8
```

| exe | début du motif | `call StartupGame` (motif + 0x40) | StartupGame |
|---|---|---|---|
| DX12 | `140088e6b` | `140088eab` (`e8 40 67 ff ff`) | `14007f5f0` |
| DX11 | `14008829b` | `1400882db` (`e8 90 66 ff ff`) | `14007e970` |

Contrôle facultatif, que le site visé respecte bien le bit (motif unique dans StartupGame) :
`41 f6 85 80 0d 00 00 02 75 ?? 48 8d 4d ?? e8 ?? ?? ?? ?? 0f b6 95 ?? ?? 00 00 48 8d 4d ?? e8`.
Il se trouve en DX12 `14007ff86` et en DX11 `14007f306`. S'il manque, ne pas armer.

### Condition d'activation

Seulement le premier chargement de l'hôte, quand la PR #12 va recharger la copie. Toutes ces
conditions doivent être vraies :

1. **Armement en Lua, juste avant `Network.RestartGame()` de MPH**, par une nouvelle native
   `Civ6Proxy_RemapLeger()`. Il y a deux appels actifs dans `VotePanel.lua` : `OnLocalHostRestart`
   (ligne 704) et le gestionnaire `.mph_ui_remap_execute` (ligne 674). Le second s'exécute sur
   **toutes** les machines qui reçoivent le message de l'hôte. La native n'arme donc que si :
   - `Network.GetLocalPlayerID() == Network.GetGameHostPlayerID()` ;
   - la partie est en réseau ;
   - `remap_host_load` est actif et `Civ6Proxy_RemapMark` / `Civ6Proxy_RemapTake` sont publiées :
     c'est la condition pour que le rechargement ait lieu.

   Le patch de `VotePanel.lua` est déjà fait par `remap::patch_vote_panel`. On y ajoute le
   remplacement de `Network.RestartGame()` par `BGD_RemapLeger(); Network.RestartGame()`, aux
   lignes non commentées, avec un compte attendu de 2.
2. **Consommation en Rust** au premier passage dans le relais, en une seule fois (même schéma que
   `ASKED` dans `remap.rs`, l'état reste dans la DLL de démarrage) :
   - armement de moins de 120 s ;
   - réglage actif.

   Un armement périmé est jeté. Le chargement de la copie qui suit (état 7) passe par le même site
   sans armement, donc avec les drapeaux du jeu.
3. **Réglage** : nouvelle clé `remap_host_light` (`on` par défaut une fois validée), dans
   `read_config`, `install.ps1` et l'onglet. Elle n'a d'effet que si `remap_host_load` est actif.

Facultatif, en défense : vérifier dans le relais que le chargement en cours est une nouvelle
partie. L'état 6 a été traversé, c'est l'unique passage de `1400840c0` depuis l'armement.

### Garde-fou : jamais d'hôte sans terrain en jeu

Le danger est que la partie allégée devienne la partie jouée, si le rechargement n'a pas lieu.
Aujourd'hui, `remap.lua` se replie en dire `.mph_ui_snap` tout de suite quand le chargement propre de
l'hôte ne démarre pas dans les 10 s. Avec le bit 2, il faut :
- une native `Civ6Proxy_RemapLegerActif()` qui dit si la partie courante a été démarrée allégée ;
- dans ce cas, au repli, **relancer `Network.RequestSnapshot()`** (deux ou trois tentatives) au
  lieu d'envoyer `.mph_ui_snap` ;
- en dernier recours, refaire `OnLocalHostRestart` sans armement : chargement complet, puis le
  chemin normal.

Le relais garde l'adresse de `GameViewState` démarrée allégée. Il peut journaliser une alerte si
cette instance reçoit plus de N images sans qu'un chargement commence.

Il y a aussi un effet visible : l'écran de chargement se ferme environ 1 s sur une vue sans terrain,
entre la fin du chargement 2 et le début du 3. C'est cosmétique. On peut le masquer plus tard, en
gardant `LoadScreen` affiché, si c'est gênant.

### Risques à lever par la mesure

- **Interface InGame sur une WorldView vide** : caméra (`UI.LookAtPlot` au `LoadScreenClose`),
  minicarte (`1401f35d0` reste appelée), étiquettes de carte, lentilles. Le World Builder démarre ces
  gestionnaires avec les mêmes drapeaux, mais avec sa propre interface. Il faut chercher les erreurs
  Lua et les plantages natifs pendant environ 1 s de vie.
- **Sauvegarde S1** : `UNSERIALIZE` restaure un « visual state » (`RestoreVisualState`), donc la
  sauvegarde en écrit sans doute un. Écrit sans terrain, il est peut-être vide ou différent. Pas de
  désynchro possible : l'hôte recharge S1, et les clients reçoivent S2, prise après ce rechargement.
  Mais il faut vérifier qu'il n'y a ni plantage ni brouillard de guerre visuel faux au chargement 3.
- **Ordre des événements de départ et de la sauvegarde** : le chargement finit environ 10 s plus tôt.
  Il faut vérifier que les écritures de BSM (« Writing boost Applied », « Pop CheckList ») ont
  toujours lieu **avant** « Snapshot save complete ». Sinon, elles s'appliquent à la partie jetée et
  sont perdues. Ce ne serait pas une désynchro, mais un écart de gameplay par rapport à aujourd'hui.
- **Fermeture** au début du chargement 3 : `14007eda0` lit `+0xd80`, encore à 2, et saute la
  destruction du terrain. C'est le comportement voulu, à confirmer sans fuite ni plantage.

## Plan de validation

1. **Hôte seul**, même scénario que `20261117-132539` (reprise, remap, nouvelle partie), DX12 puis
   DX11 :
   - dans `LoadGameViewState.log`, le chargement 2 a StartupGame < 1 s et PostLoadTime < 0,3 s ;
   - le chargement 3 est inchangé (StartupGame complet) ;
   - la ligne du relais « premier chargement de l'hôte sans terrain » est présente, une seule fois ;
   - aucune erreur Lua nouvelle dans `Lua.log` entre « Loading InGame UI » et « MPH Shutdown » ;
   - BSM écrit avant `OnLoadGameViewStateDone` ;
   - un second remap enchaîné, puis une nouvelle partie depuis le salon, ne sont pas allégés.
2. **Identité de S1, à graines fixées** : MPH permet d'imposer les graines (`.mph_ui_remap_map` et
   `.mph_ui_remap_rng`). Faire deux remaps avec les mêmes graines, `remap_host_light` coupé puis
   actif. Garder S1 (instantané reçu par l'hôte, ou sauvegarde de diagnostic) et comparer :
   - la partie GameCore doit être identique ;
   - seul le « visual state » peut différer.
3. **Deux machines** (`capture_double_load.ps1` sur les deux, hôte DX12 et client DX11, puis
   l'inverse), remap avec un client connecté :
   - dans `net_message_debug.log`, l'hôte a `NetSnapshotRequest` envers lui-même, puis `NetSnapshot`
     S1 envers lui-même, rechargement, `.mph_ui_snap`, puis `NetSnapshot` S2 vers le client ;
   - le client ne charge qu'une fois ;
   - jouer 10 tours sans désynchro (`sync_notes`, contrôles de synchro de GameCore) ;
   - comparer visuellement le terrain et le brouillard entre l'hôte et le client.
4. **Cas limites** :
   - `remap_host_load=off` : rien n'est armé ;
   - un client reçoit `.mph_ui_remap_execute` : rien n'est armé chez lui ;
   - `RestartGame` sans chargement : l'armement expire ;
   - échec simulé de `RequestSnapshot` : le repli relance, et l'hôte ne joue jamais sans terrain ;
   - hôte lent face à un client rapide : le double chargement de [double_load](../double_load/README.md)
     n'est pas aggravé.

## Adresses et outils

- Décompilations : `~/Tools/ghidra-projects/remap1_str` (StartupGame, `140084850` récent,
  `140840260`, `140275d90`, `1402731a0`), `remap2_dump` à `remap19_dump`, `remap18_d80.txt`
  (références à `+0xd80` et `+0xfd8`), `remap20_dis.txt` (désassemblage des sites). Côté GameCore :
  `remap11_dump` à `remap17_dump`.
- Motifs vérifiés par lecture directe des deux exe installés (script de lecture du PE en Python,
  sections `.text`).
- `piles.py appelants <capture> 88eb0`, puis `chaine <capture> 2 10.4 20.2 27104` pour la part du
  terrain dans StartupGame.
