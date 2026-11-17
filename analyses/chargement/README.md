# Temps de chargement d'une partie

Question : où part le temps entre le lancement d'une partie (depuis le salon, à la reprise d'une
sauvegarde, au remap de MPH) et le moment où le joueur a la main ? Et que peut-on en gagner ?

État au 2026-11-17 : une capture dédiée (reprise, remap, nouvelle partie) ventilée phase par phase
jusqu'aux fonctions de l'exe et de GameCore (2026-11-16). Le reverse des quatre postes est fait
(base de données, assets, génération de la carte, terrain), ainsi que celui des 3 s avant le remap.
Le plan d'action complet est dans [Plan d'action](#plan-daction). Les leviers 1 à 4 et 7 sont codés
(voir [Implémentation du 2026-11-17](#implémentation-du-2026-11-17)), pas encore mesurés.

## Capture du 2026-11-16

Résultats dans [runs/20261116-reprise-remap-nouvelle](runs/20261116-reprise-remap-nouvelle) :
- les journaux du jeu (`journaux.zip`) ;
- le rapport d'`analyze.py --chargement` (`rapport_analyze.md`) ;
- les sorties de [piles.py](piles.py) : `chrono.txt`, `zones_par_phase.txt`, `phases_*.txt`,
  `chaine_terrain.txt`, `appelants_pathfileexists.txt`.

La trace WPR (12 Go), son cache (5,5 Go) et le fichier de zones (54 Mo) restent sur la machine de
mesure, dans `rollover/captures/20261116-211647-reprise-remap-nouvelle`, non versionné. Hôte seul

(`Goaterino`), build diag de BGE 0.11.0 avec `actif=on` et `profil_zones=on`. Mods : BBG, BBS, BSM,
MPH, BBM. Carte : Pangée de BBS. Machine : 20 threads logiques, `MaxJobThreads 6`. Trois chargements
à la suite :

| n° | quoi | total (s) | CONFIGURE_CONTENT | UNSERIALIZE ou NEW_GAME | INITIALIZATION_DONE | StartupGame | PostLoadTime |
|---:|---|---:|---:|---:|---:|---:|---:|
| 1 | reprise (sauvegarde de 3,0 Mo) depuis le menu | 26 | 5,09 | 5,99 | 1,71 | 9,55 | 3,32 |
| 2 | remap de MPH | 28 (+3 avant) | 5,57 | 8,48 | 1,49 | 9,35 | 2,48 |
| 3 | nouvelle partie depuis le menu | 27 | 5,27 | 6,38 (12 s entre les lignes) | 1,43 | 6,16 | 1,74 |

Méthode :
- découpage par phases avec `chargement.py` ;
- piles CPU (WPR à 1 ms) du thread de chargement (une tâche du pool, toujours le même thread) et
  des threads de tâches ;
- fonctions de l'exe identifiées dans le projet Ghidra `civ6exe1105` (même empreinte que l'exe
  installé).

Le thread principal ne fait qu'afficher l'écran de chargement (91 images/s), il n'est presque jamais
sur le chemin critique. Le thread 9548 (`steamclient`, la moitié d'un cœur en continu) vient du banc
hébergé seul, comme dans [rollover](../../rollover/README.md#résultats-de-la-partie-du-2026-11-03-2043--0017).

### Où part le temps

- **Remap : aucun instantané réseau.** Seuls les `NetGameConfig` (nouvelles graines) circulent.
  Chaque machine génère elle-même la carte et charge en parallèle des autres : au remap, un client
  n'a pas de transfert à attendre.
  **Correction (2026-11-17)** : la capture a été faite avec l'hôte seul. En réalité, MPH fait ensuite
  dire `.mph_ui_snap` à l'hôte (`VotePanel.lua`), et les clients chargent alors un instantané de la
  partie de l'hôte. Dans la PR #12 de yodha (`remap_host_load`), l'hôte recharge aussi sa propre
  partie avant les clients (voir [Croisement avec la PR #12](#croisement-avec-la-pr-12-de-yodha)).
- **Avant le remap : 3 s de Lua sur le thread principal.** Elles viennent de
  `Serialization, restarting game` (`140166f60`), après le `REMAP - HOST IS RESTARTING` de MPH et
  avant le début de l'écran de chargement. Aucune trace dans `Lua.log`. C'est sans doute la
  fermeture des contextes de l'interface, ou un ramasse-miettes Lua complet : non identifié.
- **CONFIGURE_CONTENT (5,1 à 5,6 s, les trois fois)** : la base de jeu est reconstruite, sur un seul
  thread. Le CPU va à `databasedll` (SQLite, 2 s), au tas de `ntdll` (1 s) et à l'exe (0,7 s). Le
  remap la refait aussi, alors que la liste des mods n'a pas changé.
- **UNSERIALIZE (reprise, 6 s)** :
  - environ 1,8 s de désérialisation dans GameCore ;
  - puis environ 4 s d'attente (`Block on asset load`) : le thread de chargement attend le pool
    (3,4 s dans le noyau depuis `+e4d3c3`, l'attente du planificateur de tâches).

  Pendant ce temps, les tâches `SharedDataLoad` des quatre threads de tâches passent environ 4 s de
  CPU dans `PathFileExistsW` (`140996ae0`, appelée depuis `14077a630`, `BLP_SharedDataCache.cpp`).
  Pour chaque entrée des paquets BLP, elles testent l'existence d'un fichier et lisent sa date,
  pour le rechargement à chaud des assets (message « Couldn't read file meta-data … during
  hotload »). Il n'y a qu'environ 950 fichiers `.blp` : les fichiers testés sont sans doute des
  fichiers détachés, absents d'une installation Steam, et `ArtDef.log` ne signale rien. Le coût est
  le même au chargement 3 (4,4 s de CPU), et presque nul au remap (0,1 s) : les assets y sont déjà
  en mémoire. **Hypothèse à confirmer** en comptant les appels et en relevant les chemins.
- **GAMECORE_NEW_GAME (remap, 8,5 s)** : génération de la carte sur un seul thread.
  - Environ 3 s dans le Lua du script de carte (`havokscript`), qui pilote tout.
  - 2,9 s dans `TerrainBuilder.AnalyzeChokepoints` (`Lua_Utility` → `MapAnalysis`), que la Pangée
    de BBS appelle trois fois (`pangaea.lua` lignes 86, 113 et 156).
  - Le reste dans `Map_Generator` et `Region_Builder`.

  Au chargement 3, les 4 s des assets s'y ajoutent : 12 s entre les lignes du journal.
- **StartupGame (6 à 9,5 s) : la construction du terrain 3D**, entièrement refaite à chaque
  chargement (`140628f80` → `14060c5e0`, `TerrainContext`). Le jeu la chronomètre dans
  `SynthesisLog.log` : `SynthesizeTerrain` dure 6,4, 6,0 et 4,1 s, dont `ProcessTiles` 5,5, 5,1 et
  3,4 s. Ce sont les passes de la carte (`HillsPassJob`, `MountainHeightJob`, `RiverPassJob`,
  `HeightFromHexGridPassJob`, `TransitionHeightBlendJob`…), réparties sur le pool. Somme des passes
  pour la reprise : environ 24 s de CPU, soit un facteur 4,4 seulement, faute de threads :
  `MaxJobThreads 6` sur une machine à 20 threads.
- **INITIALIZATION_DONE (1,5 s)** : surtout du Lua d'interface (`havokscript`, 1 s).

### Leviers, du moins cher au plus cher

1. ~~**`MaxJobThreads` plus haut**~~ : **écarté (2026-11-17)**. 6 avait été choisi pour réduire
   l'attente active du pool pendant les passages de tour ([rollover](../../rollover/README.md)), et le
   passage de tour reste prioritaire sur le chargement.
2. **Le test d'existence des assets** (environ 4 s par chargement depuis le menu, toutes les
   machines) : d'abord compter les appels et relever les chemins avec un détour de diagnostic sur
   `PathFileExistsW` dans le proxy de démarrage. Si ce sont des fichiers détachés absents, une
   réponse en cache (dossier absent, donc fichier absent) est locale et sans risque de désynchro.
3. **Ne pas reconstruire la base au remap** (5 s, toutes les machines) : il faut garder une copie de
   la base après CONFIGURE_CONTENT et la remettre quand la liste des mods et la config sont les
   mêmes. C'est un gros travail de reverse, et le levier le plus rentable pour le remap.
4. **Les 3 s avant le remap** : identifier ce qui tourne en Lua pendant `restarting game`.
5. **`AnalyzeChokepoints`** (2,9 s au remap) : une version plus rapide dans GameCore. Elle devrait
   donner exactement le même résultat, puisque chaque machine génère la carte, avec ou sans BGE.
   Risqué pour un gain moyen.

## Reverse du 2026-11-17

Toutes les adresses sont celles de Ghidra : projet `civ6exe1105` pour l'exe (`1401…`), `civ6xp2` pour
GameCore (`1801…`). Les comptes d'échantillons viennent de l'extrait de la capture du 2026-11-16
(`extrait.pkl`, 1 échantillon = 1 ms de CPU), et les durées de `Modding.log`.

### 1. La base de données (CONFIGURE_CONTENT, 5,1 à 5,6 s)

#### Les trois bases

| base | où | « remise à l'état d'origine » |
|---|---|---|
| gameplay | SQLite en mémoire partagée `file:gameplay-N?mode=memory&cache=shared` (`1400bfe20`, N incrémenté à chaque remise) | `1400c0280` ferme la connexion et en ouvre une nouvelle (`1400bfea0`). Elle recopie `Base/Assets/Database/Gameplay.sqlite` par l'API de sauvegarde (vtable +0xc0) si le fichier existe. Sinon, elle reconstruit la base depuis `Base/Assets/Gameplay/Data/` (`1400bf5c0`). Le fichier n'existe pas dans une installation Steam : 0,10 s. |
| localisation | `:memory:` (`1400c1c50`), puis `SAVEPOINT Origin` | `1400c1f60` : `ROLLBACK TO Origin` si le compteur de modifications (vtable +0x30) a bougé. |
| configuration | base du salon | reconstruite par `140247fd0` |

La base de localisation contient les **12 langues**. Dans la copie du salon, `DebugLocalization.sqlite`
compte 15 229 textes par langue, soit 182 748 lignes. Les mods ajoutent eux aussi leurs textes dans
toutes les langues (les 11 de BBG, par exemple).

#### Déroulé de `UpdateContent` (`14024d340`) au chargement 1

| t (s) | étape | fonction | durée (s) |
|---:|---|---|---:|
| 0,00 | configuration du salon (passe rapide ici, voir plus bas) | `140247fd0` | — |
| 0,00 | listes d'actions « appliquées » et « cibles », écrites ligne à ligne dans `Modding.log` (2 × 580 lignes) | `14024bbd0`, `140b3fb40` | 0,21 |
| 0,21 | pré-configuration, déchargement puis rechargement de la DLL de GameCore (donc du proxy) | `140246e00`, `14024bb40` | 0,06 |
| 0,27 | remise de la localisation, puis de la base de gameplay | `1400c1f60`, `1400c0280` | 0,10 |
| 0,37 | données embarquées de GameCore | `14024e340` | 0,01 |
| 0,38 | fichiers implicites, avant les composants | `14024ec50` (6, 5) | 0,17 |
| 0,55 | **application des composants** (savepoint `AppModdingFramework_UpdateDatabase`) | `140b3d8f0` | **3,18** |
| 3,73 | fichiers implicites, après les composants | `14024ec50` (1, 2) | 0,02 |
| 3,75 | mise en cache de la localisation (`StringDictionary::NotifyDatabaseUpdated`) | `1400a2e40` | 0,31 |
| 4,06 | initialisation de la DLL de GameCore | `14024b690` | 0,00 |
| 4,06 | copie de débogage de la base vers `Cache/DebugGameplay.sqlite` (13,7 Mo) | `1400bfc10` | 0,08 |
| 4,14 | post-configuration | `140246d40` | 0,23 |
| 4,37 | rechargement des icônes | `140500ab0` | 0,07 |
| 4,44 | **rechargement des couleurs** : base `:memory:` ColorManager recréée depuis `ColorManager.sql`, copie de `Colors` et `PlayerColors`, XML d'accessibilité | `1400fd580` | **0,66** |

Ventilation de l'application des composants, par type d'action (identique aux trois chargements) :

| type | s | composants |
|---|---:|---:|
| UpdateText | 1,60 à 1,72 | 51 |
| UpdateDatabase | 1,40 à 1,42 | 161 |
| ImportFiles | 0,05 | 33 |
| UpdateLogitechARX, UpdateIcons, UpdateAudio, ReplaceUIScript, UpdateArt, UpdateColors, AddGameplayScripts, AddUserInterfaces | 0,10 à 0,14 en tout | 248 |

95 % du temps va donc aux écritures en base (textes et données). Le répartiteur `140b3d8f0` trouve le
gestionnaire de chaque action dans une table (`+200`), indexée par un hachage FNV-1a du type. Chaque
gestionnaire a son `BeginApplyComponents` (vtable +8) et son `EndApplyComponents` (+0x18).

#### Le jeu a déjà un raccourci, qui ne sert jamais

`UpdateContent` sait sauter toute la reconstruction (« Game configuration matches target
configuration. » → `14024dd00` puis `140247880`), à deux conditions :
- la liste des actions en jeu déjà appliquées (`+0x208`) est la liste cible ;
- le compteur de modifications de la base de localisation n'a pas bougé depuis la dernière mise à
  jour (`+0x23c`).

Au remap, la première condition est remplie : les listes appliquée et cible sont identiques (le
`diff` des 580 lignes ne trouve qu'un avertissement de tri). Le jeu reconstruit pourtant tout, à
cause d'un ping-pong entre les deux configurations qui se partagent la base de localisation :
1. `UpdateContent` met `+0x238 = -1` en sortant. Cela invalide la configuration du salon.
2. Au chargement suivant, `140247fd0`, appelée en tête d'`UpdateContent`, voit `+0x238` différent du
   compteur de la localisation. Elle reconstruit la configuration (0,92 s au remap) et fait un
   `ROLLBACK TO Origin` sur la localisation.
3. Le compteur de la localisation a bougé, donc `+0x23c` ne correspond plus : la reconstruction du
   gameplay repart de zéro (4,4 s).

Ce raccourci n'est donc jamais pris quand une partie était déjà chargée. Ce chemin n'est pas testé
par le jeu lui-même, et il ne recharge pas la DLL de GameCore.

#### Cache de la base sur disque (signature)

Principe : après une reconstruction normale, on garde la base de gameplay finale et l'apport des mods
à la localisation. Au chargement suivant, si la signature est la même, on les remet en place au lieu
de rejouer les composants qui écrivent en base.

**Signature** (manifeste JSON, haché en SHA-256 sur sa forme canonique). La proposition de départ
(mods, versions et version de BGE) est nécessaire mais ne suffit pas :
- version du format du cache, `RELEASE` et `VERSION` de BGE ;
- empreinte des binaires : exe, `GameCore_XP2_FinalRelease.dll` (ses données embarquées entrent
  dans la base) et `DatabaseDLL_FinalRelease.dll`, par taille et horodatage PE ;
- **liste ordonnée des actions en jeu cibles** (mod, version du mod, composant, type). Elle intègre
  les modes de jeu et les critères, qui changent les actions sans changer la liste des mods ;
- empreinte (taille et date) des fichiers lus par les actions UpdateDatabase, UpdateText, UpdateIcons
  et UpdateColors, ainsi que de `Base/Assets/Gameplay/Data`. Une mise à jour du Workshop ne change
  pas toujours la version du mod, et quelques milliers de `stat` coûtent environ 20 ms.

**Stockage** : `%LOCALAPPDATA%\Civ6RolloverProxy\cache_bdd\<hachage>\` contient `manifest.json`,
`gameplay.sqlite` et `localisation_delta.sqlite`. On garde les quatre plus récents.

**Écriture (défaut de cache)** :
- base de gameplay : au point « Saving a debug version » (`1400bfc10`), le jeu fait déjà une
  sauvegarde SQLite complète de la base finale, et il suffit d'en faire une seconde vers le cache ;
- localisation : à la remise (`1400c1f60`), on copie la base d'origine en mémoire. À la fin, le delta
  est calculé en SQL : lignes ajoutées ou modifiées (`EXCEPT`), et clés supprimées.

Le surcoût n'est payé qu'au défaut, environ 0,2 à 0,3 s.

**Lecture (succès)** :
1. `1400c0280` : passer le chemin du cache à `1400bfea0`. Le jeu sait déjà charger un
   `Gameplay.sqlite` depuis un fichier, par sauvegarde SQLite vers la base en mémoire.
2. Sauter `14024e340` (données embarquées), déjà dans l'instantané.
3. Dans le répartiteur, neutraliser les gestionnaires qui écrivent en base (UpdateDatabase,
   UpdateText, et UpdateIcons et UpdateColors après vérification). Garder ceux qui ont d'autres
   effets : ImportFiles, ReplaceUIScript, AddGameplayScripts, AddUserInterfaces, UpdateArt,
   UpdateAudio, UpdateLogitechARX et ModArt.
4. Localisation : après le `ROLLBACK TO Origin` du jeu, appliquer le delta (`INSERT OR REPLACE`
   puis `DELETE`), dans le savepoint `Origin`. La remise suivante reste ainsi correcte.

**Avant de coder, il reste à reverser** :
- le foncteur `param_3[7]`, appelé avec la base de gameplay juste avant les composants. Il vient de
  la machine d'états du chargement, `140084850`. S'il écrit en base selon la config de la partie, il
  doit entrer dans la signature ou être rejoué ;
- les objets gestionnaires (vtables) de chaque type d'action, pour les neutraliser un par un ;
- ce qu'écrivent UpdateIcons et UpdateColors en jeu.

**Vérification** : un mode de diagnostic `cache_bdd=verifier` reconstruit normalement, puis compare
un hachage du contenu de chaque table (`SELECT * ORDER BY rowid`) à celui du cache. À faire tourner
sur de nombreux chargements avant d'activer le saut.

**Multijoueur** : le contenu est par construction celui d'une reconstruction normale, donc pas de
désynchro, même face à un joueur sans BGE. C'est un réglage (`cache_bdd=on|off`, `install.ps1`,
onglet), activé par défaut.

**Gain attendu, à confirmer par la mesure** : au succès, CONFIGURE_CONTENT passe de 5,1 s à environ
2,0 s, **soit environ −3 s par chargement depuis le menu**. Restent :
- couleurs : 0,66 s ;
- cache de la localisation : 0,31 s ;
- post-configuration : 0,23 s ;
- journal des listes : 0,21 s ;
- actions hors base : 0,2 s ;
- fichiers implicites : 0,19 s ;
- remise depuis le fichier : 0,1 à 0,15 s ;
- copie de débogage : 0,08 s ;
- DLL : 0,06 s.

Les 5 s espérées ne sont pas atteignables par le seul cache. On peut s'en approcher avec trois
compléments :
- couleurs : recalculer seulement si les tables `Colors` et `PlayerColors` ou l'option daltonisme
  ont changé, soit −0,6 s ;
- copie de débogage : la sauter au succès, soit −0,08 s ;
- journal : ne plus écrire les listes d'actions dans `Modding.log`, soit −0,15 s environ.

#### Remap : briser le ping-pong

Au remap (même partie, mêmes mods), il suffit de laisser le jeu prendre son propre raccourci. Dans
`140247fd0` appelée depuis `UpdateContent`, on renvoie « configuration identique » quand toutes ces
conditions sont réunies :
- listes de mods identiques ;
- aucun mod manquant ;
- `+0x240 != 1` ;
- `+0x238 == -1`, posé par la mise à jour précédente ;
- compteur de la localisation égal à `+0x23c`, donc localisation intacte depuis.

La localisation reste alors dans l'état « gameplay ». C'est déjà l'état du jeu en partie, puisque la
remise du gameplay retire de toute façon les textes de configuration des mods. Le raccourci du
gameplay est ensuite pris tout seul : **−5,3 s au remap** (0,9 + 4,4).

~~Risque à valider : ce chemin ne recharge pas la DLL de GameCore.~~ **Écarté (2026-11-17)**, voir
[Croisement avec la PR #12](#croisement-avec-la-pr-12-de-yodha). Ce raccourci ne recharge pas
GameCore, et la PR #12 montre que c'est précisément ce qui désynchronise. Au remap, on passe donc par
le cache, en gardant le déchargement et le rechargement de GameCore.

#### Croisement avec la PR #12 de yodha

La PR #12 (ouverte, branche `yodha-rejoin-desync`) reprend le « GameCore neuf à chaque partie » de la
PR #6, qui a été fermée.

**Le constat de yodha.** Le proxy de GameCore charge l'original par `LoadLibrary` et ne le libère
jamais. Quand l'exe décharge GameCore (« Unloading Gameplay DLL » : `1400f8fb0`, `FreeLibrary` du
proxy), l'original reste donc en mémoire, avec l'état de la partie précédente : compteur d'ID de
GameEffects et autres statiques. En jeu, les objets sont alors numérotés autrement que sur les
autres machines : l'hôte différait de tous les clients à chaque fin de tour après un remap.

**Son correctif** (`chargeur.rs`, réglage `fresh_gamecore`) : au `LoadLibrary` suivant de GameCore
par l'exe, il libère l'original de la partie précédente. Les conditions : un proxy a été remis
depuis la dernière libération, le proxy n'est plus chargé, et l'original l'est encore.

Dans le déroulé d'`UpdateContent`, ce `LoadLibrary` est « Reloading Gameplay DLL » (`14024bb40`).
**Le correctif de yodha ne se déclenche donc que si le jeu passe par la reconstruction complète.**

Conséquences pour la base de données :
1. **Le raccourci natif est à proscrire.** Il saute le déchargement et le rechargement de GameCore,
   donc le correctif de yodha, et même sans BGE il garderait les statiques de la partie précédente.
   C'est sans doute pour cela que le jeu fait en sorte de ne jamais l'emprunter. Le levier « briser
   le ping-pong » (n° 4 du plan) est retiré.
2. **Le cache de la base reste compatible**, à condition de suivre le chemin complet :
   - déchargement de GameCore, `LoadLibrary`, donc libération de l'original par `chargeur.rs` ;
   - initialisation de la DLL (`14024b690`) ;
   - le cache ne remplace que les écritures en base.

   La base est dans l'exe : un GameCore neuf la relit entièrement (« Initializing Gameplay DLL »,
   puis `InitializeCache`).
3. **Point à vérifier en plus** : l'étape des données embarquées (`14024e340`) appelle le GameCore
   tout juste chargé (vtable +0x50 avec la connexion). Au succès du cache, on comptait la sauter
   puisque ses données sont dans l'instantané. Il faut d'abord s'assurer qu'elle n'initialise rien
   dans GameCore. Sinon, il faut l'exécuter contre une base jetable ou tolérer les doublons.
4. **Mesure à refaire** : la capture du 2026-11-16 a été faite sans la PR #12. « Reloading Gameplay
   DLL » (0,06 s) n'y rechargeait que le proxy, et l'original restait résident. Avec le correctif,
   l'original (environ 20 Mo) est vraiment déchargé et rechargé, et ses statiques réinitialisées. Ce
   surcoût est inévitable (c'est le comportement sans BGE), mais il faut le mesurer.
5. **Le remap gagne plus qu'estimé.** Avec `remap_host_load` :
   - l'hôte fait deux chargements : la nouvelle partie générée, puis sa propre partie rechargée ;
   - chaque client fait un chargement de l'instantané ;
   - les trois passent par `UpdateContent` avec la même liste d'actions que la partie précédente,
     donc trois succès de cache sur le chemin critique.

   Hypothèse à confirmer : le chargement d'instantané passe bien par « Stage: Configure Content »,
   comme une reprise.
6. **Reconstruction à la demande** (levier 4 refait, voir plus bas) : on recharge GameCore à chaque
   partie, mais on ne reconstruit la base que si elle a changé.
7. **La génération de la carte au remap** n'est faite que par l'hôte : les clients chargent son
   instantané. Pour le remap, la réécriture exacte de `1805c9bb0` (M1) n'est donc pas une condition
   de synchro. Elle le reste tant qu'on ne sait pas si une nouvelle partie lancée depuis le salon
   fait générer la carte à chaque machine (à vérifier avec une capture à deux machines,
   `NetSnapshot` dans `net_message_debug.log`).

#### Reconstruction à la demande : séparer GameCore, gameplay et localisation

**Où vit ce que corrige yodha.** L'état d'une partie (objets, compteurs d'ID de GameEffects,
caches) est dans la mémoire de GameCore : ses statiques et son tas. La sauvegarde le sérialise. La
base de gameplay ne contient que les règles (unités, modificateurs…), en lecture seule pendant la
partie. Dans GameCore, on trouve 1 309 requêtes `SELECT`. Les seules écritures sont :
- les données embarquées (`GameEffects`, `GameEffectArguments`, `Types`), écrites par `14024e340` ;
- l'export de carte du WorldBuilder (`Map`, `Plots`, `Units`…), qui va dans un autre fichier.

Recharger GameCore et reconstruire la base sont donc deux choses indépendantes. Le jeu les lie dans
`UpdateContent`, mais rien ne l'oblige.

**Le retour au menu ne remet pas le contenu à zéro.** `14024cea0` (« complete rollback to vanilla »)
n'est appelée qu'en cas d'échec d'`UpdateContent`. Au chargement 3, la liste « appliquée » était
encore celle du remap. Elle ne différait de la cible que parce que les mods de la nouvelle partie
n'étaient pas les mêmes (No More AI World Wonders, composants BBCC). Avec les mêmes mods, la base de
gameplay construite reste donc valable en mémoire d'une partie à l'autre de la session, y compris en
passant par le menu.

Ce que le salon invalide, c'est seulement la **localisation**. La configuration du salon la remet à
`Origin` et y charge ses propres textes, d'où le ping-pong.

Trois invalidations indépendantes, au lieu d'un tout-ou-rien :

| quoi | à refaire quand | coût |
|---|---|---:|
| GameCore | toujours : déchargement, `LoadLibrary` (libération de l'original par `chargeur.rs`), initialisation (`14024b690`) | à mesurer avec la PR #12 |
| base de gameplay | liste d'actions cible différente, ou compteur de modifications de la base différent de celui relevé en fin de construction. Le jeu se sert déjà de ce compteur (`+0x1c`, « Gameplay database unchanged, no rollback needed ») | 1,4 s (UpdateDatabase) + 0,1 s (remise) |
| localisation | compteur de la localisation différent de `+0x23c` (le salon l'a remise à `Origin`) | 1,6 s si on rejoue UpdateText, environ 0,2 s si on réapplique un delta gardé en mémoire |

Déroulé visé dans `UpdateContent` quand la liste d'actions est la même :
1. pré-configuration (`140246e00`), déchargement et rechargement de GameCore, comme aujourd'hui ;
2. base de gameplay : rien si le compteur n'a pas bougé ;
3. localisation :
   - rien si elle est intacte : en jeu, quand on empêche la configuration du salon de la remettre
     à `Origin` (sans toucher au raccourci natif) ;
   - sinon, `ROLLBACK TO Origin` puis le delta gardé en mémoire depuis la dernière construction ;
4. actions hors base (ImportFiles, ReplaceUIScript, AddGameplayScripts, AddUserInterfaces…) : à
   rejouer (0,2 s), car la pré-configuration vide les listes `+0x290` et `+0x2a8` ;
5. mise en cache de la localisation (si elle a changé), initialisation de GameCore,
   post-configuration (couleurs recalculées seulement si elles ont changé).

Gains attendus, à mesurer :
- **chargement dans la même session sans repasser par le menu** (deux chargements de l'hôte et
  celui des clients au remap, rechargement de tout le monde de la PR #12, reconnexion) : environ
  −4,5 s sur CONFIGURE_CONTENT, configuration du salon comprise ;
- **depuis le menu avec les mêmes mods** : environ −3 à −3,5 s (base intacte, delta de
  localisation) ;
- **premier chargement après le lancement du jeu** : rien en mémoire, c'est là que sert le cache
  disque par signature. Il devient un complément de ce levier, plus le levier principal.

À vérifier avant de coder :
- la sémantique exacte du compteur (vtable +0x30 de la connexion, `DatabaseDLL_FinalRelease.dll`) ;
- qu'aucun mod n'écrit dans la base de gameplay en partie (le compteur le détecterait de toute façon) ;
- les effets de `14024e340` dans un GameCore neuf, pour savoir si on peut s'en passer quand la base
  est gardée. Les données embarquées y sont déjà, et GameCore pourrait n'y faire qu'écrire.

### 2. Les assets (Block on asset load, environ 4 s depuis le menu)

Chaîne :
- `140a9b650` et `140aaf900` appellent `140a49300`, qui appelle `1407791e0`. Pour chaque donnée
  partagée demandée, `1407791e0` crée une tâche :
  - `SharedDataLoad` (`1407786d0`) si l'entrée n'a pas encore de propriétaire ;
  - `SharedDataUpdateOwnership` (`14077a630`) sinon ;
- puis le thread de chargement attend la fin des tâches (`EXP_JobManager_Wait`, en aidant le pool).

Les deux tâches parcourent **toutes les racines SHARED_DATA** (`job+0x60`, 0x220 octets par racine).
Pour chacune :
- elles composent le chemin (`1407edda0`) ;
- elles appellent `PathFileExistsW` (`140996ae0`), puis, si le fichier existe,
  `GetFileAttributesExW` (`140996960`) ;
- elles inscrivent la racine dans l'entrée (`140777a70`, un ajout dans un ensemble : sans effet si
  elle y est déjà) ;
- elles retiennent la date la plus récente.

La tâche de propriété **refait ce parcours complet pour chaque propriétaire supplémentaire**, alors
que le résultat ne change pas. Échantillons : environ 8 200 sous `14077a630`, contre 66 sous
`1407786d0`.

Sur disque, il y a 25 racines SHARED_DATA dans l'installation (12 000 fichiers) et 30 dans le
Workshop. La plupart des tests portent donc sur des fichiers absents : environ 55 racines × chaque
donnée demandée. Au remap, les entrées sont déjà en mémoire et le coût tombe à 0,1 s.

Leviers, tous locaux et sans effet sur la synchro :
- **A1, diagnostic** : compter les appels, les noms distincts et les succès dans les deux tâches.
- **A2, cache du contenu des racines** : lister chaque racine une fois par chargement
  (`FindFirstFileExW` avec `FIND_FIRST_EX_LARGE_FETCH`) dans une table de hachage nom →
  `WIN32_FIND_DATA`. Elle répond à l'existence et aux attributs (mêmes champs que
  `WIN32_FILE_ATTRIBUTE_DATA`). On patche les quatre appels de `14077a630` et `1407786d0` depuis le
  proxy de démarrage. Lister 12 000 fichiers coûte quelques dizaines de ms. **Gain attendu : −3,5 à
  −4 s** sur la reprise et la nouvelle partie depuis le menu.
- **A3** : ne refaire le parcours d'une entrée qu'une fois par lot (résultat identique). Après A2,
  c'est un complément.

### 3. La génération de la carte (GAMECORE_NEW_GAME, remap et nouvelle partie)

`TerrainBuilder.AnalyzeChokepoints` : `1800223e0` → `1805d8d30` → `1805d16c0` (Region_Builder).
`1805d16c0` remet à zéro les régions des cases, triangule (`1805c4570`), puis remplit les régions
case par case. La Pangée de BBS l'appelle trois fois (`pangaea.lua` lignes 86, 113 et 156), toutes
les cartes de BBS au moins une fois. Les trois appels encadrent des changements du terrain, et
`StampContinents` lit le premier : ils ne sont pas redondants. En nouvelle partie, le moteur ne
l'appelle pas lui-même (`18060feb0` n'est pas sur ce chemin).

La triangulation de Delaunay est faite par division et fusion (`1805cbc60` → `1805cb9c0` →
`1805c3570`). **82 % des échantillons sont du temps propre de `1805c9bb0`**, la « fermeture éclair »
de la fusion (4 139 sur 5 077) :
- pour trouver les voisins du point de base, elle parcourt **tous les points** de la moitié, et pour
  chacun **toutes les arêtes** : O(points × arêtes) à chaque pas ;
- chaque suppression d'arête décale le vecteur des arêtes et décrémente les indices de tous les
  triangles.

Leviers :
- **M1, réécrire `1805c9bb0` dans le proxy de GameCore** avec une table d'adjacence (ensemble de
  bits des voisins du point de base, construit à partir du même vecteur d'arêtes). Le résultat doit
  être identique au bit près :
  - même ordre de parcours (index croissant) ;
  - mêmes calculs en `f32`, dans le même ordre (constantes `DAT_1809d18b0`, `DAT_1809d62dc`,
    `DAT_180a09038`) ;
  - mêmes suppressions.

  Un mode de diagnostic exécute l'original sur une copie et compare les vecteurs (points, arêtes,
  triangles) sur de nombreuses graines. **Gain attendu : environ −2 s par génération de carte.** Coût
  élevé : 740 lignes, et le résultat doit être exact (chaque machine génère la carte).
- **M2, `AI_ChokePoint.csv`** : `1805c9750` (graphe de Voronoï) écrit tout le graphe dans le canal de
  journal `0x94ee4ab3` (`DAT_180b8aa90`, vtable +0x50) après chaque étape, et à chaque tour de la
  boucle de fusion des cellules. Cela fait 11,6 Mo, 241 000 lignes et 763 échantillons, soit environ
  0,35 s par génération. ~~Renvoyer un canal nul pendant la génération ne change rien au jeu.~~ **Faux** :
  la fusion des cellules (`1805cb5e0`) ne tourne que si ce canal existe (`test rdi,rdi`), un canal
  nul changerait la carte. On neutralise donc l'écriture d'une cellule (`1805c8fe0`), seul appelant
  `1805c9750`.
  **Gain : −0,35 s, coût faible.** Les autres `AI_*.csv` écrits pendant les tours sont un sujet du
  passage de tour.
- Le Lua du script de carte (environ 3 s, `havokscript`) relève de BBS et sort du code de BGE : pas
  retenu.

### 4. Le terrain 3D (StartupGame)

D'après `SynthesisLog.log`, `SynthesizeTerrain` dure 6,4 s au chargement 1, dont :
- `ProcessTiles` : 5,5 s, 18 passes réparties sur le pool, environ 24 s de CPU ;
- `SetupGlobalData` : 0,9 s ;
- `GenerateStructures` : 0,18 s.

Aucune fonction ne dépasse 3 % des échantillons (feuille la plus chargée : `e50fd2`, 860 sur 32 653).
Il n'y a pas de fonction mal écrite évidente : l'optimisation fonction par fonction ne paierait pas.

Pistes, non retenues pour l'instant :
- **T1, cache disque du terrain synthétisé**, indexé par un hachage de la carte et des réglages
  graphiques. Il ne servirait qu'aux reprises de la même carte, et demanderait de reverser
  `TerrainContext` : 66 Mo de géométrie, plus l'export. Coût très élevé.
- **T2, plus de threads pendant le seul chargement**, sans toucher à `MaxJobThreads` en partie. Il
  faudrait reverser le gestionnaire de tâches. À reconsidérer seulement si le reste ne suffit pas.

### 5. Les 3 s avant le remap

Pendant les 3,5 s qui précèdent l'écran de chargement, 2 712 échantillons du thread principal sont
dans `havokscript`. 2 340 d'entre eux passent par `140980470`, appelée **à la destruction de chaque
contexte d'interface** (`8f03fd` → `4d91ce` → `982759` → `9804bd`) :

```c
lua_cpcall(L, FUN_140981820, contexte);   // nettoyage du contexte
lua_gc(L, LUA_GCCOLLECT, 0);              // ramasse-miettes COMPLET de tout l'état Lua partagé
```

Le ramasse-miettes complet est donc refait autant de fois qu'il y a de contextes détruits. Les
feuilles (`havokscript+3c0c6`, `+56fc7`…) sont le marquage et le balayage.

- **G1** : pendant une destruction en lot (redémarrage ou sortie), remplacer ce `lua_gc` par rien,
  puis faire un seul ramasse-miettes complet à la fin. Cela ne concerne que l'interface, sans effet
  sur la synchro. **Gain attendu : environ −2 s au remap.** Le même schéma vaut probablement pour le
  retour au menu (à mesurer). Coût faible à moyen : il faut repérer le début et la fin du lot, à
  partir des appelants de `1404d91b0` et `140982730`.

## Implémentation du 2026-11-17

Après la fusion de la PR #12 dans `main` (local, pas encore poussée). Chaque levier a sa clé dans
`rollover_proxy.cfg`, son paramètre dans `proxy/install.ps1` et sa ligne dans l'onglet (section
Performances, options avancées), textes dans les 11 langues. Les fonctions de l'exe sont retrouvées
par motifs d'octets, uniques dans le DX12 et le DX11 du 2026-11-05 (vérifié hors jeu sur les deux
exes) ; un motif absent ou ambigu laisse le levier non installé, avec une ligne dans
`RolloverProxy.log`.

| levier | clé (défaut) | où | ce qui est fait |
|---|---|---|---|
| A2 | `cache_assets` (on) | `startup_proxy/src/assets.rs` | les trois appels de `PathFileExistsW` des tâches `BLP_SharedDataCache` (`14077a772`, `140777cc1`, `140778884`) répondent depuis un listage de chaque dossier (`FindFirstFileExW`, grands lots), valable 20 s. Chemins non ASCII ou avec joker : système |
| G1 | `gc_en_lot` (on) | `startup_proxy/src/ramasse_miettes.rs` | `1408f0710` (arrêt d'un contexte, entrée +0x1a8) est remplacée par la même chose avec un compteur de profondeur ; pendant un arrêt, le `lua_gc(COLLECT)` de `140980470` est noté au lieu d'être fait, et un seul est fait à la sortie de l'arrêt le plus externe |
| 4 (gameplay) | `bdd_gardee` (on) | `startup_proxy/src/bdd.rs` | voir ci-dessous |
| 4 (textes) | `textes_gardes` (on) | `startup_proxy/src/bdd.rs`, `localisation.rs` | voir ci-dessous |
| M2 | `journal_chokepoints` (off) | `proxy/src/journal_chokepoints.rs` | `1805c8fe0` (écriture d'une cellule du graphe de Voronoï dans `AI_ChokePoint.csv`) remplacée par une fonction vide ; le canal reste, car la fusion des cellules en dépend. Test sur la vraie DLL |
| M1 | `delaunay_rapide` (on) | `proxy/src/delaunay.rs` | réécriture de `1805c9bb0` (voisins de la base en un parcours, suppressions en une passe, mêmes calculs `f32` et mêmes comparaisons). Garde-fous : empreinte FNV-1a de `1805c9bb0` et `1805c8d80`, constantes. `verifier` fait tourner les deux et garde l'originale. Banc sur la vraie DLL : 600 000 fusions et 1 200 triangulations entières identiques ; carte de 3 582 points en 15 ms au lieu de 1 874 ms |

**Base gardée.** `UpdateContent` (`14024d340`) est détourné à l'entrée (relevé de fin), et cinq de
ses appels le sont aussi, ceux-là seulement (le retour à vanilla `14024cea0` reste intact) : remise
de la localisation (`1400c1f60`), remise du gameplay (`1400c0280`), données embarquées (`14024e340`),
composants (`140b3d8f0`), copie de débogage (`1400bfc10`).
- À la fin d'un `UpdateContent` réussi : connexion de la base de gameplay, compteur de la vtable
  (+0x30), `SELECT total_changes()` et liste des actions appliquées (`+0x208`).
- Au suivant, si les quatre sont identiques et que le gestionnaire UpdateDatabase est trouvé (nœud
  de type CRC-32 « UpdateDatabase » dans la liste `+0xc8` du cadre) : pas de remise, pas de données
  embarquées, gestionnaire remplacé le temps des composants par un neutre (Begin, Apply et End
  rendent vrai), pas de copie de débogage. GameCore est toujours déchargé, rechargé et initialisé.
- Vérifié avant de coder : le foncteur `param_3[7]` est une fonction vide
  (`CallAppLostFocusHandler`, un simple `ret`) ; les données embarquées (GameCore +0x50,
  `180045ed0`) ne font que des `INSERT` depuis des registres statiques ; UpdateColors et
  UpdateIcons inscrivent des fichiers auprès de leurs gestionnaires (`1400fad90`, `1404ffce0`), sans
  écrire dans la base de gameplay ; les Begin et End d'UpdateDatabase et d'UpdateText ne font que
  mémoriser puis oublier la connexion ; le proxy de GameCore n'écrit rien en base.

**Textes gardés.** Après la remise de la localisation à `Origin` dans `UpdateContent` :
- chargement complet : des déclencheurs temporaires (créés après `Origin`, donc retirés par le
  prochain `ROLLBACK TO Origin` s'ils ne l'ont pas été avant) notent la clé de chaque ligne de
  `LocalizedText` écrite, et toute écriture dans les sept autres tables. Après les composants,
  l'état final de chaque clé notée est lu (ligne, ou absence) et gardé en mémoire avec la liste
  d'actions et une empreinte de la table (nombre de lignes, longueur cumulée) ;
- chargement suivant, mêmes actions, même connexion : suppressions puis `INSERT OR REPLACE` des
  lignes gardées, par requêtes préparées, puis comparaison de l'empreinte. Si elle diffère, retour à
  `Origin` et UpdateText est fait normalement. Sinon, UpdateText est neutralisé comme UpdateDatabase,
  et la remise rend « reverted » pour que le jeu remette les textes en cache ;
- pas de delta si un mod change le schéma ou écrit hors de `LocalizedText`.

API des bases du jeu employée (lue dans l'exe et GameCore) : connexion +0x18 préparer, +0x20
exécuter, +0x30 compteur, +0xa0 revenir à un savepoint ; requête +0x8 libérer, +0x10 exécuter,
+0x20 avancer, +0x38 réinitialiser, +0x90 lier un texte, +0xf0 lire un texte. Les retours de
« lier » et « exécuter » ne sont jamais testés par le jeu : le rejeu ne s'y fie pas et vérifie
l'empreinte.

**Non faits, et pourquoi** :
- levier 5, cache disque par signature : il ne sert qu'au premier chargement après le lancement.
  Il demande d'écrire et relire une base complète par l'API de sauvegarde, plus le delta des
  textes. À reprendre une fois la base gardée mesurée ;
- levier 6, couleurs : le salon recharge lui aussi les couleurs (`140247fd0` → `1400fd580`), avec
  celles de la configuration. Sauter le rechargement en jeu laisserait ces couleurs-là. Écarté ;
- journal des listes d'actions (0,2 s) : gardé, il sert au diagnostic des mods.

À vérifier à la première capture (`RolloverProxy.log`) : lignes `database:` et `localization:` à
chaque chargement (« built » puis « kept », « captured » puis « replayed ») ; `Modding.log` doit
montrer « Gameplay database unchanged, no rollback needed. » au deuxième chargement, et aucune
erreur de composants ; partie identique (pas de désynchro au remap à deux machines).

### Première mesure (2026-11-17, `20261117-132539-apres-leviers-3`)

Hôte seul, PR #12 comprise : reprise d'une sauvegarde du tour 131, remap MPH (l'hôte charge la
nouvelle partie puis sa propre copie), puis nouvelle partie depuis le salon, avec une action de mod
de moins (504 au lieu de 505), donc base reconstruite.

| chargement | 2026-11-16 | 2026-11-17 | CONFIGURE_CONTENT | autre écart notable |
|---|---:|---:|---|---|
| reprise (premier de la session) | 26 s | 24 s | 5,09 → 6,48 s (capture des textes, voir plus bas) | UNSERIALIZE 5,99 → 2,65 s (A2) |
| remap, nouvelle partie | 28 s | 22 s | 5,57 → 3,26 s (base et textes gardés) | GAMECORE_NEW_GAME 8,48 → 5,88 s (M1, M2) |
| remap, copie de l'hôte (PR #12) | — | 20 s | 3,45 s (gardés) | |
| nouvelle partie depuis le salon | 27 s | 24 s | 5,27 → 6,62 s (reconstruite, capture) | GAMECORE_NEW_GAME 6,38 → 4,21 s |

- Triangulation : 17 à 18 ms par passe (1 900 à 2 000 points), 0 écart, 0 ignorée.
- Assets : 863 000 tests d'existence servis par 196 listages, aucun renvoyé au système.
- G1 : 125 collectes complètes remplacées par une à la fermeture de l'interface.
- G1, restreint ensuite au thread principal : au lancement d'une partie depuis le salon, les
  contextes du salon sont arrêtés sur le thread de chargement pendant que le thread principal fait
  tourner l'interface. Avec la collecte reportée, il appelait le `OnTick` de `StagingRoom.lua`
  déjà nettoyé (46 à 134 erreurs « function expected instead of nil » par capture depuis G1, 0
  avant), et une fois il a écrit dans de la mémoire libérée de HavokScript (crash du 2026-11-17,
  `20261117-145241-vsync-levier`, nouvelle partie depuis le salon, VSync levée : 2,5 fois plus
  d'images dans cette fenêtre).
- Deux crashs en cours de route, corrigés : la collecte différée de G1 était faite hors du verrou de
  l'état Lua (`140dc32d0` / `140dc3ef0`) ; la liaison de texte (+0x90) prend un 4ᵉ argument, la
  longueur (`-1`), qui manquait au rejeu des textes.
- Textes : la première version notait les clés par déclencheurs (+0,7 s pendant les composants,
  +0,54 s de lecture) et le rejeu prenait 0,95 s, car UpdateText réécrit 184 000 des 197 000
  lignes. Remplacée après cette mesure : lecture de toute la table triée par clé, rejeu par vidage
  puis réinsertion dans l'ordre (hors jeu, l'insertion triée coûte 40 % de moins). À remesurer.

### Ventilation du reste (2026-11-17, même capture)

Outils : `piles.py chrono`, `zones` et `rythme` (nouveau), et
[rollover/waits.py](../../rollover/waits.py) pour les attentes d'un thread.

**Le rythme des images, calé sur la VSync, freine deux passages : environ 3 à 4,5 s par chargement.**
La machine a `VSync 1` (GraphicsOptions.txt), en fenêtre sans bordure, et `LoadGameTickIntervalInMS 0`.
Pendant tout le chargement, le thread principal affiche 90 à 100 images par seconde et attend dans
`Present` (dxgi), à l'image suivante de l'écran.

- **StartupGame, après le terrain (2 à 3 s).** Le chargement 3 en est l'exemple : de 13:26:56 à
  13:26:59, ni le CPU ni le GPU du jeu ne travaillent (3D à 1 à 4 %). La machine à états du
  chargement (`140084850`, état 9) appelle StartupGame (`14007f5f0`) en un seul appel, sur un thread
  de chargement. Ce thread crée les textures des caches du terrain (`140c6cca0` :
  `BaseColorSpecTerrainCache`, `FOWColorTerrainCache`, …) par `ID3D12Device::CreateCommittedResource`
  (vtable +0xd8, appelée en `140969b50`). Sur 4 s, environ 2 000 créations attendent dans le noyau
  (WrResource) au total 2,9 s, et c'est le thread principal qui les réveille à la sortie de son
  `Present`. La création d'une ressource passe après la présentation bloquée par la VSync.
- **PostLoadTime (1,6 à 3 s).** `140089aa0` attend que `WorldView_PostLoadUpdate` (`14054e960`) n'ait
  plus rien à faire. Celle-ci remplit le cache de vue du terrain (`14060ff30`), une part à chaque
  image : 149 à 155 étapes à chaque chargement complet, une par image, soit au moins 1,5 s à
  100 images par seconde. Le reste de la phase : `SerialEvent::FinishedGameplayContentConfigure`
  (0,3 à 0,4 s) et deux images longues (0,2 et 0,1 s).

Levier, fait (`vsync_chargement`, on par défaut, DX12 seulement,
[startup_proxy/src/vsync.rs](../../startup_proxy/src/vsync.rs)) : `140973470` appelle
`Present(VSync ? 1 : 0, drapeaux)`, et l'appel `call [rax+0x40]` (`1409734da`) passe par le proxy,
qui met l'intervalle à 0 entre `LoadGameViewState::Begin()` (`140083a20`) et `::End()`
(`140083d30`), seulement si le jeu demandait 1. Sans VSync, ou en jeu, l'appel est inchangé.
Les drapeaux restent ceux du jeu (0 avec la VSync) : pas de `DXGI_PRESENT_ALLOW_TEARING`. Le code
de `::Begin()` existe deux fois à l'identique (un autre état) : la bonne cite sa chaîne.

**Mesure avec `VSync 0` (`20261117-141910-sans-vsync`, même scénario)** : le thread principal passe
à 250 à 290 images par seconde pendant le chargement.

| chargement | VSync 1 | VSync 0 | StartupGame (WAIT_FOR_PLAYERS) | PostLoadTime |
|---|---:|---:|---|---|
| reprise (premier de la session) | 24 s | 25 s | 9,88 → 9,77 s | 3,04 → 2,81 s |
| remap, nouvelle partie | 22 s | 21 s | 9,79 → 7,93 s | 2,33 → 2,07 s |
| remap, copie de l'hôte | 20 s | 18 s | 11,14 → 8,21 s | 2,41 → 2,00 s |
| nouvelle partie depuis le salon | 24 s | 21 s | 7,68 → 5,85 s | 1,60 → 1,31 s |

- StartupGame gagne 1,8 à 2,9 s, sauf au premier chargement de la session, qui ne gagne rien
  (pas encore expliqué).
- PostLoadTime ne gagne que 0,25 à 0,4 s : ses 150 étapes affichent maintenant le monde, et chaque
  image coûte du rendu (70 à 120 images par seconde), pas une attente.
- Les textes, nouvelle méthode : le rejeu porte sur toute la table (360 480 lignes), et non plus sur
  les 184 428 lignes réécrites par UpdateText. L'étape « Reloading Gameplay DLL », qui le contient,
  passe de 1,00 à 1,58-1,64 s au remap : c'est une régression d'environ 0,6 s par rechargement. Au
  premier chargement, le CPU de la configuration baisse (6,8 → 5,6 s), mais sa durée monte
  (6,48 → 7,65 s) : le thread attend, pour une raison non identifiée.

**Le reste n'offre pas de gros gain :**
- *Contrôles d'`InGame` (1,13 à 1,4 s, une seule image de `UIManager_Update`)* : d'après
  `UserInterface.log`, ce sont surtout les corps des 123 fichiers Lua de l'interface (0,9 s). Aucun
  ne domine : `CivilopediaScreen.lua` 0,15 s, `InGame.lua` 0,1 s, `MapLabelManager.lua` 0,04 s.
  L'enregistrement des contextes ne coûte que 0,1 s.
- *`SerialEvent::AfterGameplayContentChange`* : 0,81 s quand la base est reconstruite (reprise 1),
  42 ms quand elle est gardée (chargement 3). La base gardée règle déjà ce point.
- *Lua d'`INITIALIZATION_DONE`* : c'est la même image d'`UIManager_Update` que ci-dessus.

### Leviers ajoutés le 2026-11-17 après-midi (non mesurés)

| levier | réglage | fichier | quoi |
|---|---|---|---|
| VSync levée pendant le chargement | `vsync_chargement` (on) | `startup_proxy/src/vsync.rs` | voir plus haut ; DX12 seulement |
| Premier chargement du remap sans terrain | `remap_leger` (on) | `startup_proxy/src/remap_leger.rs`, `remap.lua` | bit 2 de StartupGame (World Builder) pour le chargement qui suit l'armement Lua de l'hôte, détail dans [remap_premier_chargement.md](remap_premier_chargement.md) ; garde-fou : l'hôte ne joue jamais une partie sans terrain |
| Ouvriers du pool qui s'endorment plus tôt | `pool_cessions` (16 ; 100 = jeu) | `startup_proxy/src/pool.rs` | étape 1 de [pool_taches.md](pool_taches.md) |
| Fin d'attente d'un lot sans attente active | `pool_attente_fin` (on) | `startup_proxy/src/pool.rs` | étape 2 de [pool_taches.md](pool_taches.md) |

Relus par un agent avant installation. Ce qu'il a fait corriger :
- la copie d'`Origin` des textes est tenue en mémoire, pas dans une table SQL temporaire ;
- l'attente de fin de lot cède comme le jeu quand il n'y a plus d'ouvrier mais que l'emplacement 0 est pris ;
- l'armement du remap allégé ne sert qu'au chargement qui suit directement ;
- un seul remap complet en dernier recours ;
- pas de filet `LocalPlayerTurnBegin` pour la fenêtre de pause.

Restent à mesurer :
- le délai de secours de 1 ms de `WaitOnAddress` si le minuteur système est à 15,6 ms (fenêtre masquée) ;
- un second `RequestSnapshot` possible au repli.

Le nombre d'ouvriers n'est pas changé : il sera réestimé une fois le coût de l'attente mesuré au passage de tour.

**Mesure (`20261117-154532-leviers-4`, VSync 1, tous les leviers)** :

| chargement | matin (`apres-leviers-3`) | leviers-4 | StartupGame | CONFIGURE_CONTENT |
|---|---:|---:|---|---|
| reprise (premier de la session) | 24 s | 22 s | 9,88 → 8,67 s | 6,48 → 6,12 s |
| remap, nouvelle partie de l'hôte | 22 s | **11 s** | 9,79 → 0,27 s (sans terrain), PostLoadTime 0,43 s | 3,26 → 3,14 s |
| remap, copie de l'hôte | 20 s | 16 s | 11,14 → 7,86 s | 3,45 → 3,12 s |
| nouvelle partie depuis le salon | 24 s | 21 s | 7,68 → 5,34 s | 6,62 → 5,83 s |

Le remap complet de l'hôte passe de 42 à 27 s. Le rejeu des textes par delta reprend les 0,6 s de la
version « toute la table ». Deux défauts vus dans cette capture, corrigés ensuite :
- le chargement sans terrain finit avant que `OnRefresh` de MPH (qui attend 1 s) remette
  `GAME_HOST_IS_JUST_RELOADING` à « N » : le garde-fou de `remap.lua` a demandé le rechargement
  avec « Y » dans la sauvegarde, et la copie a redemandé un instantané (10 s de plus pour les
  clients). Le garde-fou fait maintenant ce que fait `OnRefresh` avant la demande ;
- l'arrêt des 30 contextes du salon tourne bien sur le thread principal, et les erreurs
  `StagingRoom.lua` restaient (208) : G1 ne reporte plus que l'arrêt de l'interface d'une partie
  (entre la fin d'un chargement et le premier arrêt d'au moins 20 collectes).

## Plan d'action

Objectif : réduire au maximum, dans le code, le temps de chargement de toutes les machines. Toutes
les mesures se font avec la même séquence (reprise, remap, nouvelle partie), deux fois chacune.

**Phase 0, instrumentation (diag, ne part pas en release)** :
1. Compteurs des tâches d'assets (A1), dans le proxy de démarrage.
2. Zones du profil pour `UpdateContent` et ses étapes, `1805d16c0` et `140980470`.
3. Reverse restant pour le cache de la base :
   - foncteur `param_3[7]` ;
   - vtables des gestionnaires ;
   - UpdateIcons et UpdateColors ;
   - effets de `14024e340` dans GameCore.
4. Capture avec la PR #12 de yodha (GameCore neuf, rechargement de l'hôte au remap), pour
   remplacer les durées de référence.

**Phase 1, gains rapides à faible risque** :

| n° | quoi | où | gain attendu | quand |
|---|---|---|---:|---|
| 1 | cache du contenu des racines SHARED_DATA (A2) | proxy de démarrage (exe) | −3,5 à −4 s | depuis le menu |
| 2 | un seul ramasse-miettes par destruction en lot (G1) | proxy de démarrage (exe) | ≈ −2 s | remap (et retour au menu ?) |
| 3 | `AI_ChokePoint.csv` coupé (M2) | proxy de GameCore | −0,35 s | génération de carte |

**Phase 2, base de données** :

| n° | quoi | où | gain attendu | quand |
|---|---|---|---:|---|
| 4 | reconstruction à la demande : GameCore toujours rechargé, base de gameplay et localisation refaites seulement si elles ont changé (pas de raccourci natif) | exe | ≈ −4,5 s dans la session, ≈ −3 s depuis le menu | tous les chargements sauf le premier, dont 3 au remap avec la PR #12 |
| 5 | cache disque de la base par signature, d'abord en mode `verifier` : pour le premier chargement après le lancement | exe | ≈ −3 s | premier chargement de la session |
| 6 | couleurs recalculées seulement si elles changent, journal des listes et copie de débogage allégés | exe | ≈ −0,8 s | tous |

**Phase 3, algorithme** :

| n° | quoi | où | gain attendu | quand |
|---|---|---|---:|---|
| 7 | réécriture exacte de `1805c9bb0` (M1), avec comparaison A/B en diag avant activation | proxy de GameCore | ≈ −2 s | génération de carte |

**Phase 4, à évaluer plus tard** : cache du terrain (T1), threads pendant le chargement (T2),
filtrage des langues dans UpdateText (utile seulement aux défauts de cache).

Chaque fonctionnalité a son réglage (clé de `read_config`, `install.ps1`, onglet), sauf les
correctifs purs. Les modes de vérification passent derrière la feature `diag`.

**Effet cumulé estimé**, à confirmer par la mesure (les gains sont sur le chemin critique, donc
s'additionnent à peu près) :

| scénario | aujourd'hui | leviers | estimé |
|---|---:|---|---:|
| reprise depuis le menu | 26 s | 1, 4 ou 5, 6 | ≈ 18 s |
| remap de MPH, hôte seul, sans la PR #12 | 28 s + 3 s | 2, 3, 4, 7 | ≈ 22 s |
| nouvelle partie depuis le menu | 27 s | 1, 3, 4 ou 5, 6, 7 | ≈ 17 s |

Il reste surtout le terrain (5 à 6,5 s) et le Lua des scripts de carte et de l'interface.

Avec la PR #12, le remap n'est pas mesuré : l'hôte enchaîne deux chargements, puis les clients
chargent. Le cache de la base y joue trois fois sur le chemin critique, et le ramasse-miettes en lot
deux fois chez l'hôte. Il faut une capture à deux machines avec la PR, avant et après les leviers.

## Base de référence : reprise au tour 123 (2026-11-13)

Journaux de l'hôte `OJR1UG7` dans [double_load/runs/20261113](../double_load/runs/20261113), lus par
[rollover/chargement.py](../../rollover/chargement.py). Quatre chargements de la même partie, en
réseau, avec BBG, BBS, BSM, MPH et BGE.

| début | total (s) | CONFIGURE_CONTENT | UNSERIALIZE | INITIALIZATION_DONE | WAIT_FOR_PLAYERS (StartupGame) | PostLoadTime |
|---|---:|---:|---:|---:|---:|---:|
| 18:18:28 | 18 | 4,92 | 3,88 | 1,47 | 5,44 | 1,45 |
| 18:35:58 | 17 | 5,35 | 4,08 | 1,42 | 5,00 | 1,23 |
| 18:39:18 | 25 | 5,47 | 3,78 | 1,47 | 10,85 | 2,93 |
| 18:52:57 | 24 | 5,76 | 3,95 | 1,46 | 9,30 | 2,81 |

Les deux premiers chargements reprennent une sauvegarde de 0,55 et 0,61 Mo, les deux derniers une de
2,9 Mo. Avec la sauvegarde cinq fois plus grosse, StartupGame double, alors que les autres phases ne
bougent presque pas.

Côté client, s'y ajoute le transfert de l'instantané avant le début du chargement : 2,9 Mo à environ
265 Ko/s, soit **11 s** (voir [double_load](../double_load/README.md#capture-du-2026-11-13)).

- **CONFIGURE_CONTENT** (5,8 s, `Modding.log` à la milliseconde) : la base de jeu est remise à
  l'état d'origine puis reconstruite à partir de tous les mods, à chaque chargement, même quand
  la liste des mods n'a pas changé :
  - 3,6 s pour appliquer les 451 composants, dont 1,6 s de textes :
    - `Expansion2_CoreText` à lui seul prend 1,03 s, dont 0,75 s pour
      `Expansion2_Translations_Text.xml` ;
    - les 11 langues de BBG sont chargées, alors que le joueur n'en lit qu'une ;
  - 0,82 s pour recharger les couleurs ;
  - 0,33 s de mise en cache de la localisation ;
  - 0,24 s de traitement après la configuration.
- **GAMECORE_UNSERIALIZE_GAMESTATE** (3,9 s) : les sous-phases que le jeu chronomètre sont à 0. Leurs
  instants montrent environ 2 s entre `UpdateCache` et `Block on asset load`.
- **StartupGame** (5 à 11 s) : le corps des 129 fichiers Lua de l'interface s'exécute en 0,8 s, et
  les contrôles d'`InGame` s'initialisent en 1,05 s. Ensuite, le journal n'a plus aucune ligne pendant
  9,4 s. Il reste à savoir ce qui occupe ce temps : gestionnaires Lua (bandeaux, drapeaux, étiquettes)
  ou construction du monde. Le temps double entre le 2ᵉ et le 3ᵉ chargement, sans explication pour
  l'instant.

## Le remap de MPH

`VotePanel.lua` (MPH) appelle `Network.RestartGame()` : c'est une nouvelle partie, pas une reprise.
La carte est générée par le script de BBS (`perfectworld6.lua`) et les départs sont placés. Deux
questions restent ouvertes, auxquelles la première capture doit répondre :
- le chargement passe-t-il par les mêmes phases de `LoadGameViewState.log` ?
- chaque machine génère-t-elle la carte elle-même, ou l'hôte envoie-t-il un instantané
  (`NetSnapshot` dans `net_message_debug.log`) ?

## Outils

- [rollover/chargement.py](../../rollover/chargement.py) : ventilation du dernier chargement à partir
  des journaux seuls :
  - phases de `LoadGameViewState.log` ;
  - étapes, composants et fichiers de `Modding.log` ;
  - contextes de `UserInterface.log` ;
  - instantané réseau et fin de la mise en file des messages.

  ```bash
  python -I rollover/chargement.py "<dossier Logs du jeu ou dossier de capture>"
  ```
- [rollover/capture.ps1](../../rollover/capture.ps1) `-Chargement` : capture WPR (CPU avec piles), lancée
  avant le premier chargement et arrêtée par Entrée une fois la main revenue après le dernier. Elle copie aussi
  `LoadGameViewState.log`, `UserInterface.log`, `Database.log`, `GameCore.log` et `SynthesisLog.log`
  (durées de construction du terrain).
- [rollover/analyze.py](../../rollover/analyze.py) `--chargement` : la ventilation ci-dessus, plus
  l'analyse de la trace (threads, modules, zones de l'exe), pour chaque chargement de la capture.

- [piles.py](piles.py) : les piles CPU d'une capture, chargement par chargement. `extrait` garde une
  fois pour toutes les échantillons autour des chargements (`extrait.pkl`, dans la capture). Ensuite,
  sans relire la trace :
  - `chrono` : chronologie seconde par seconde ;
  - `zones` : zones de l'exe par phase et par thread ;
  - `phases` : modules et fonctions par phase, et ce qui appelle le noyau ;
  - `chaine` : chemins d'appel de l'exe sur une fenêtre ;
  - `appelants` : qui appelle un offset de l'exe ;
  - `rythme` : images, temps de présentation et CPU par phase, pour voir si une phase attend la
    VSync ou calcule.

  Les offsets de l'exe se décompilent dans le projet Ghidra `civ6exe1105`.

  ```bash
  .venv/Scripts/python.exe -I analyses/chargement/piles.py extrait rollover/captures/<dossier>
  .venv/Scripts/python.exe -I analyses/chargement/piles.py phases rollover/captures/<dossier> <threads>
  ```

## Protocole de mesure

Préparation, une fois :
1. Installer un build local des deux proxys : la feature `diag`, active par défaut, est nécessaire
   pour le profil des zones.
2. `profil_zones=on` (onglet BetterGameExperience, section Diagnostic, ou
   `proxy/install.ps1 -ProfilZones on`), puis relancer le jeu.

Une capture peut enchaîner plusieurs chargements. Dans un PowerShell administrateur, jeu ouvert au
menu principal :

```powershell
powershell -ExecutionPolicy Bypass -File rollover\capture.ps1 -Name reprise-remap-nouvelle -Chargement
```

1. reprise d'une partie avancée depuis le salon ;
2. remap depuis le menu MPH ;
3. retour au menu principal (pas au bureau : l'exe réécrit ses journaux à chaque lancement), puis
   nouvelle partie depuis le salon ;
4. une fois la main revenue, Entrée dans le PowerShell.

Puis, pour avoir le détail de chaque chargement, l'un après l'autre :

```bash
.venv/Scripts/python.exe -I rollover/analyze.py --chargement rollover/captures/<dossier>
```

| scénario | comment | ce qu'on cherche |
|---|---|---|
| `remap` | partie réseau hébergée seul, mêmes mods qu'en vrai, remap depuis le menu MPH | phases d'une nouvelle partie, durée du script de carte, base reconstruite ou non |
| `salon` | lancement d'une nouvelle partie depuis le salon, mêmes réglages | comparaison avec le remap |
| `reprise` | reprise d'une sauvegarde avancée (tour 100 ou plus) depuis le salon | les 9 s de StartupGame, le trou d'UNSERIALIZE |
| `remap-off` | `remap` avec `actif=off` (BGE chargé mais sans détour ni interface) | ce que coûte BGE lui-même |

- Faire chaque scénario deux fois : le premier chargement après le lancement du jeu lit le disque à
  froid.
- Ajouter `-WithIO` à une des deux captures de `reprise`, pour voir si `Block on asset load` et
  StartupGame attendent le disque.
- Le côté client (transfert de l'instantané, chargement qui commence en retard) demande deux machines.
  Ce sera l'étape suivante, une fois le côté hôte compris.
