# Temps de chargement d'une partie

Question : où part le temps entre le lancement d'une partie (depuis le salon, à la reprise d'une
sauvegarde, au remap de MPH) et le moment où le joueur a la main ? Et que peut-on en gagner ?

État au 2026-11-16 : une capture dédiée (reprise, remap, nouvelle partie) ventilée phase par phase
jusqu'aux fonctions de l'exe et de GameCore. Leviers classés à la fin de
[la capture du 2026-11-16](#capture-du-2026-11-16), aucun encore essayé.

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

1. **`MaxJobThreads` plus haut** : rien à coder. Le terrain (24 s de CPU) et la tâche des assets se
   répartissent sur le pool, et 6 threads sur 20 en laissent 14 inutilisés pendant le chargement.
   Mais 6 avait été choisi pour réduire l'attente active du pool pendant les passages de tour
   ([rollover](../../rollover/README.md)). À mesurer : même séquence avec `MaxJobThreads -1` ou 12.
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
  - `appelants` : qui appelle un offset de l'exe.

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
