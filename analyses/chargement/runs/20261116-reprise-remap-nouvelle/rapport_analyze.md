# Chargements : 20261116-211647-reprise-remap-nouvelle

## Chargements (LoadGameViewState.log)

| n° | début | durée (s) | GAMECORE_LOAD_SAVEFILE | CONFIGURE_CONTENT | GAMECORE_UNSERIALIZE_GAMESTATE | GAMECORE_INITIALIZATION_DONE | WAIT_FOR_PLAYERS | PostLoadTime | GAMECORE_NEW_GAME |
|---:|---|---:|---:|---:|---:|---:|---:|---:|---:|
| 1 | 21:17:03 | 26 | 0.00 | 5.09 | 5.99 | 1.71 | 9.55 | 3.32 |  |
| 2 | 21:17:58 | 28 |  | 5.57 |  | 1.49 | 9.35 | 2.48 | 8.48 |
| 3 | 21:19:37 | 27 |  | 5.27 |  | 1.43 | 6.16 | 1.74 | 6.38 |

## Chargement n° 1, 21:17:03

- Instantané réseau : `Net RECV (6) :NetSnapshot : m_snapShotDataSize=3007547` à 21:17:03, 0 s avant le début du chargement.
- Fin de la mise en file des messages (le joueur a la main) : 21:17:29, 26 s après le début.

Chargement de 21:17:03 à 21:17:29 (26 s, à la seconde près). Les sous-phases à 0 s ne sont pas chronométrées par le jeu : leur instant (à la seconde) montre où le temps passe.

| phase | durée (s) | à (s) |
|---|---:|---:|
| **GAMECORE_LOAD_SAVEFILE** | 0.00 | 0 |
| **CONFIGURE_CONTENT** | 5.09 | 5 |
| ↳ Unserialize | 0.00 | 7 |
| ↳ InitializeCache | 0.00 | 7 |
| ↳ PostGameInitialization | 0.00 | 7 |
| ↳ PostInitialize | 0.00 | 7 |
| ↳ UpdateCache | 0.00 | 7 |
| ↳ Block on asset load | 0.00 | 11 |
| ↳ RestoreVisualState | 0.00 | 11 |
| **GAMECORE_UNSERIALIZE_GAMESTATE** | 5.99 | 11 |
| ↳ GameViewState::LoadUI | 0.00 | 13 |
| ↳ Post-GameCore asset load | 0.28 | 13 |
| **GAMECORE_INITIALIZATION_DONE** | 1.71 | 13 |
| ↳ GameViewState::StartupGame | 9.55 | 23 |
| ↳ StartupGame asset load | 0.00 | 23 |
| **WAIT_FOR_PLAYERS** | 9.55 | 23 |
| **PostLoadTime** | 3.32 | 26 |
| *non attribué (bornes à la seconde)* | 0.3 | |

Étapes de Modding.log autour du chargement (heure recalée) :

| heure | t / début (s) | étape | jusqu'à la suivante (s) |
|---|---:|---|---:|
| 21:16:15.391 | -47.6 | Configuring game content | 1.398 |
| 21:16:16.789 | -46.2 | Successfully reconfigured game. | 15.221 |
| 21:16:32.010 | -31.0 | Configuring game content | 1.047 |
| 21:16:33.057 | -29.9 | Successfully reconfigured game. | 15.695 |
| 21:16:48.752 | -14.2 | Configuring game content | 0.906 |
| 21:16:49.658 | -13.3 | Successfully reconfigured game. | 13.790 |
| 21:17:03.448 | 0.4 | Stage: Loading a save | 0.001 |
| 21:17:03.449 | 0.4 | Stage: Configure Content | 5.096 |
| 21:17:08.545 | 5.5 | Successfully reconfigured game. | 0.000 |
| 21:17:08.545 | 5.5 | Stage: Unserializing Game State |  |

CONFIGURE_CONTENT (Modding.log, à la milliseconde), 5.10 s, après « Loading a save », avant « Unserializing Game State » :

| t (s) | étape | jusqu'à l'étape suivante (s) |
|---:|---|---:|
| 0.207 | Performing pre configure processing. | 0.004 |
| 0.211 | Reloading Gameplay DLL. | 0.058 |
| 0.269 | Reverted Localization database to its original state. | 0.000 |
| 0.269 | Rebuilding gameplay database. | 0.100 |
| 0.369 | Reverted Gameplay database to its original state. | 0.000 |
| 0.369 | Updating gameplay database with embedded GameCore data. | 0.011 |
| 0.380 | Updated gameplay database with GameCore data. | 0.000 |
| 0.380 | UpdateContent - Importing implicit files, pre components | 0.165 |
| 0.545 | Modding Framework - Applying Components | 3.181 |
| 3.726 | Modding Framework - Finished Apply Components | 0.000 |
| 3.726 | Applied all components of enabled mods. | 0.000 |
| 3.726 | UpdateContent - Importing implicit files, post components | 0.021 |
| 3.747 | Localization database has been updated.  Caching data. | 0.311 |
| 4.058 | Initializing Gameplay DLL. | 0.000 |
| 4.058 | Saving a debug version of the gameplay database. | 0.084 |
| 4.142 | Performing post configure processing. | 0.229 |
| 4.371 | Reloading icons. | 0.065 |
| 4.436 | Reloading colors. | 0.660 |
| 5.096 | Successfully reconfigured game. | 0.000 |

Composants les plus lents (453 composants, 3.16 s au total) :

| composant | s |
|---|---:|
| Expansion2_CoreText (UpdateText) | 0.803 |
| Expansion2CoreContent (UpdateDatabase) | 0.405 |
| BBG_TextUpdate (UpdateText) | 0.173 |
| Expansion2Core (UpdateDatabase) | 0.113 |
| BBG_Base (UpdateDatabase) | 0.108 |
| Expansion1_MajorText (UpdateText) | 0.107 |
| BBG_XP2 (UpdateDatabase) | 0.069 |
| GranColombiaMayaText (UpdateText) | 0.056 |
| MUU_LocalizedText (UpdateText) | 0.046 |
| database (UpdateDatabase) | 0.043 |
| NewAction (UpdateDatabase) | 0.042 |
| Expansion2MajorContent (UpdateDatabase) | 0.040 |
| BabylonText (UpdateText) | 0.036 |
| KublaiKhanVietnamText (UpdateText) | 0.032 |
| UpdateDatabase (UpdateDatabase) | 0.032 |

Fichiers les plus lents (1099 fichiers, 2.81 s, dont 1.41 s de textes) :

| fichier | s |
|---|---:|
| Text/Expansion2_Translations_Text.xml | 0.609 |
| Data/Expansion2_NamedPlaces.xml | 0.117 |
| Data/Expansion2_RemoveData.xml | 0.107 |
| Text/Expansion1_Translations_Major_Text.xml | 0.075 |
| Text/GranColombia_Maya_Translations_Text.xml | 0.044 |
| Data/NWOC_Civilizations_Major.sql | 0.041 |
| Text/Babylon_Translations_Text.xml | 0.030 |
| lang/spanish.xml | 0.029 |
| SteelThunderUnits/Data/MOARUniqueUnits.sql | 0.028 |
| sql/_utils.sql | 0.028 |
| Text/KublaiKhan_Vietnam_Translations_Text.xml | 0.027 |
| Text/Byzantium_Gaul_Translations_Text.xml | 0.024 |
| lang/french.xml | 0.023 |
| lang/chinese.xml | 0.022 |
| Data/Expansion2_Modifiers.xml | 0.021 |

Interface (UserInterface.log) : 129 fichiers Lua, 0.95 s d'exécution du corps des fichiers, 1.43 s d'initialisation des contrôles. Les gestionnaires d'événements appelés ensuite (LoadGameViewStateDone…) ne sont pas chronométrés par le jeu.

| fichier ou contexte | s |
|---|---:|
| contrôles de InGame | 1.290 |
| ../../../Base/Assets/UI/Civilopedia/CivilopediaScreen.lua | 0.147 |
| ram Files (x86)/Steam/steamapps/workshop/content/289070/3307282026/ui/InGame.lua | 0.111 |
| ../../../DLC/Expansion2/UI/Additions/MapLabelManager.lua | 0.054 |
| contrôles de InGameTopOptionsMenu | 0.050 |
| ../../../Base/Assets/UI/Options.lua | 0.032 |
| apps/workshop/content/289070/2865001760/ui/replacements/greatpeoplepopup_bbg.lua | 0.026 |
| ../../../DLC/Expansion2/UI/Replacements/ResearchChooser_Expansion1.lua | 0.022 |
| ../../../Base/Assets/UI/Popups/MapPinPopup.lua | 0.021 |
| Steam/steamapps/workshop/content/289070/1916397407/UI/ReportScreen_Spectator.lua | 0.021 |
| contrôles de MinimapPanel | 0.020 |
| ../../../DLC/Expansion2/UI/Replacements/CityPanel_Expansion2.lua | 0.019 |

## Trace WPR

- Fenêtre : 21:17:02 → 21:17:30 (28 s).
- civilizationvi_dx12.exe (pid 16580) : 86806 échantillons à 1 ms, soit 86.8 s de CPU ; 86607 avec pile.

### Images (présentations DXGI)

- 2300 images de 21:17:02 à 21:17:29 ; intervalle de synchro : 1 (2300) (0 : sans synchro verticale).

| période | images | durée (s) | images/s | p50 (ms) | p95 (ms) | p99 (ms) | max (ms) | > 33 ms | > 100 ms | bloqué dans Present, moyenne / p95 (ms) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| hors passage | 2299 | 28.0 | 82 | 10.0 | 21.0 | 37.3 | 1436 | 35 | 8 | 4.4 / 8.9 |

### GPU (relevés à la seconde)

Moyenne / p95 sur la période. Les compteurs « jeu » ne portent que sur le processus du jeu, « autres » sur tous les autres (bureau, navigateur…).

| mesure | hors passage (28 s) |
|---|---:|
| jeu 3D (%) | 11.1 / 52.5 |
| jeu Copy (%) | 0.0 / 0.0 |
| jeu VideoDecode (%) | 0.0 / 0.0 |
| autres 3D (%) | 0.2 / 0.3 |
| autres Copy (%) | 2.5 / 19.4 |
| autres VideoDecode (%) | 0.0 / 0.0 |
| nvidia fréquence GPU (MHz) | 1066.6 / 2820.0 |
| nvidia fréquence mémoire (MHz) | 3277.9 / 9001.0 |
| nvidia puissance (W) | 26.4 / 80.0 |
| nvidia température (°C) | 38.5 / 46.0 |
| nvidia utilisation carte (%) | 25.5 / 66.0 |

### Threads

Concentration : part des échantillons sur les 4 adresses les plus fréquentes du thread. Tranche : CPU moyen par mise sur le processeur. Sortie : raison la plus fréquente de sortie du processeur.

| thread | nom | CPU (ms) | occupation | dont GameCore (ms) | noyau | mises sur le CPU | tranche (ms) | sortie | concentration | adresse la plus fréquente | point d'entrée |
|---:|---|---:|---:|---:|---:|---:|---:|---|---:|---|---|
| 8444 |  | 21629 | 77 % | 713 | 18 % | 32295 | 0.67 | cession (yield) 69 % | 15 % | `noyau` | `civilizationvi_dx12.exe+0xe5278d` |
| 9548 |  | 13561 | 48 % | 0 | 84 % | 848067 | 0.02 | attente (UserRequest) 100 % | 17 % | `noyau` | `civilizationvi_dx12.exe+0x99a252` |
| 8748 |  | 11464 | 41 % | 4 | 27 % | 141521 | 0.08 | 27 56 % | 10 % | `noyau` | `civilizationvi_dx12.exe+0xe5278d` |
| 29552 |  | 11269 | 40 % | 7 | 26 % | 142385 | 0.08 | 27 56 % | 11 % | `civilizationvi_dx12.exe+0x9bbf27` | `civilizationvi_dx12.exe+0xe5278d` |
| 30380 |  | 10563 | 38 % | 3 | 24 % | 143842 | 0.07 | 27 55 % | 13 % | `civilizationvi_dx12.exe+0x9bbf27` | `civilizationvi_dx12.exe+0xe5278d` |
| 14936 |  | 10269 | 37 % | 5 | 24 % | 145090 | 0.07 | 27 55 % | 9 % | `civilizationvi_dx12.exe+0x9bbf27` | `civilizationvi_dx12.exe+0xe5278d` |
| 25156 |  | 7320 | 26 % | 86 | 30 % | 285371 | 0.03 | verrou ou variable de condition (WrAlertByThreadId) 88 % | 29 % | `ntdll.dll+0x1bcaa` | `civilizationvi_dx12.exe+0xea3607` |
| 20448 |  | 165 | 1 % | 0 | 16 % | 1626 | 0.10 | attente (UserRequest) 82 % | 7 % | `civilizationvi_dx12.exe+0xd23e01` | `civilizationvi_dx12.exe+0xcf2547` |
| 17512 |  | 97 | 0 % | 80 | 6 % | 136 | 0.71 | interruption 74 % | 8 % | `gamecore_xp2_finalrelease.dll+0x1aef05` | `civilizationvi_dx12.exe+0x99a252` |
| 12664 |  | 73 | 0 % | 0 | 5 % | 178 | 0.41 | attente (UserRequest) 84 % | 74 % | `civilizationvi_dx12.exe+0xc7e7c5` | `civilizationvi_dx12.exe+0xcde2eb` |
| 6456 |  | 71 | 0 % | 0 | 4 % | 154 | 0.46 | interruption 69 % | 14 % | `bink2w64.dll+0x1a1fa` | `bink2w64.dll+0x35c02` |
| 15264 |  | 65 | 0 % | 0 | 51 % | 2802 | 0.02 | attente (UserRequest) 100 % | 22 % | `noyau` | `xaudio2_9.dll+0x343e7` |
| 1600 |  | 37 | 0 % | 0 | 97 % | 972 | 0.04 | attente (UserRequest) 86 % | 19 % | `noyau` | `nvwgf2umx.dll+0xeba2aa` |
| 11076 |  | 30 | 0 % | 0 | 87 % | 2725 | 0.01 | 4 98 % | 27 % | `noyau` | `sdk_legacy_led_x64.dll+0x78bdf` |
| 29292 |  | 27 | 0 % | 0 | 4 % | 108 | 0.25 | interruption 58 % | 19 % | `bink2w64.dll+0x169d0` | `bink2w64.dll+0x35c02` |
| 30916 |  | 24 | 0 % | 0 | 92 % | 2778 | 0.01 | attente (UserRequest) 98 % | 25 % | `noyau` | `nvwgf2umx.dll+0xeba2aa` |
### Zones du profileur de l'exe (proxy de démarrage, `profil_zones=on`)

- 137663 zones de 259 noms sur 28.0 s.
- `Civ6App_UpdateFrame` : 2300 images.

Temps réel passé dans chaque zone, propre (sans ses zones filles), tous threads.

| zone | nombre | total (ms) | propre (ms) | moyenne (ms) | max (ms) | threads |
|---|---:|---:|---:|---:|---:|---|
| `Civ6App_UpdateFrame` | 2300 | 21656 | 11857 | 9.416 | 1430.6 | 25156 |
| `Civ6App_UpdateNetwork` | 2287 | 5099 | 5099 | 2.230 | 41.2 | 25156 |
| `Civ6App_SteamAPI_RunCallbacks` | 2301 | 4333 | 4333 | 1.883 | 37.3 | 25156 |
| `UIManager_Update` | 2301 | 1496 | 1494 | 0.650 | 1395.8 | 8748, 14936, 25156 … |
| `Graphics_SubmissionIncFrame` | 2300 | 1470 | 1470 | 0.639 | 60.5 | 25156 |
| `SerialEvent::AfterGameplayContentChange` | 1 | 653 | 653 | 652.786 | 652.8 | 8444 |
| `UILoadLock` | 2301 | 614 | 614 | 0.267 | 613.6 | 25156 |
| `WorldView_PostLoadUpdate` | 155 | 558 | 558 | 3.597 | 113.1 | 25156 |
| `Civ6App_HandleGameStateSwap` | 2300 | 325 | 325 | 0.141 | 257.7 | 25156 |
| `lua_RegisterDatabase` | 1 | 221 | 221 | 220.799 | 220.8 | 8444 |
| `UIUpdateJob_Render` | 2301 | 205 | 205 | 0.089 | 17.1 | 8748, 14936, 25156 … |
| `LuaContext::WorldViewIconsManager` | 2840 | 179 | 179 | 0.063 | 30.5 | 25156 |
| `LuaContext::CityBannerManager` | 3362 | 162 | 162 | 0.048 | 18.0 | 25156 |
| `GameViewState_UpdateCache` | 1 | 82 | 82 | 82.138 | 82.1 | 25156 |
| `WorldView_Render_Models` | 46 | 81 | 81 | 1.764 | 3.8 | 25156 |
| `AppLuaScriptSystem_EndFrame` | 2300 | 67 | 67 | 0.029 | 1.4 | 25156 |
| `LuaContext::UnitFlagManager` | 885 | 52 | 52 | 0.059 | 1.5 | 25156 |
| `Civ6App_EOSAPI_RunCallbacks` | 2301 | 51 | 51 | 0.022 | 0.1 | 25156 |
| `LuaContext::MapLabelManager` | 45 | 47 | 47 | 1.049 | 45.3 | 25156 |
| `WorldView_Render_VFX` | 46 | 44 | 44 | 0.962 | 9.6 | 25156 |
| `WorldView_Render_Submission` | 45 | 37 | 37 | 0.828 | 14.4 | 25156 |
| `Civ6App_AudioUpdateWait` | 2301 | 36 | 36 | 0.016 | 0.9 | 25156 |
| `UnitContext_Update` | 46 | 35 | 35 | 0.760 | 31.1 | 25156 |
| `SerialEvent::LensUpdateArea` | 122 | 35 | 35 | 0.285 | 0.6 | 25156 |
| `GameCoreEvent::UnitAddedToMap` | 263 | 79 | 34 | 0.301 | 1.6 | 25156 |
| `LuaGarbageCollectJob_Collect` | 21968 | 32 | 32 | 0.001 | 0.1 | 8444, 8748, 14936 … |
| `SerialEvent::GameCoreEventPublishComplete` | 1672 | 71 | 29 | 0.042 | 18.0 | 25156 |
| `GameCoreEvent::DistrictAddedToMap` | 374 | 61 | 25 | 0.164 | 1.5 | 25156 |
| `SequenceEvent::LoadScreenClose` | 1 | 19 | 19 | 19.137 | 19.1 | 25156 |
| `GameViewState_Tick` | 29 | 245 | 19 | 8.463 | 100.0 | 25156 |

CPU attribué par les échantillons, par CPU propre (86806 sur les threads qui ont des zones ; 78537 hors de toute zone).

| zone | CPU propre (ms) | CPU inclusif (ms) |
|---|---:|---:|
| `Civ6App_UpdateNetwork` | 1863 | 1863 |
| `Civ6App_UpdateFrame` | 1621 | 4671 |
| `Civ6App_SteamAPI_RunCallbacks` | 1587 | 1587 |
| `SerialEvent::AfterGameplayContentChange` | 645 | 645 |
| `WorldView_PostLoadUpdate` | 549 | 549 |
| `Civ6App_HandleGameStateSwap` | 300 | 300 |
| `lua_RegisterDatabase` | 222 | 222 |
| `UIUpdateJob_Render` | 199 | 199 |
| `LuaContext::CityBannerManager` | 159 | 159 |
| `UIManager_Update` | 84 | 85 |
| `LuaContext::WorldViewIconsManager` | 84 | 84 |
| `WorldView_Render_Models` | 83 | 83 |
| `GameViewState_UpdateCache` | 82 | 82 |
| `Civ6App_EOSAPI_RunCallbacks` | 52 | 52 |
| `AppLuaScriptSystem_EndFrame` | 50 | 50 |
| `LuaContext::MapLabelManager` | 47 | 47 |
| `SerialEvent::GameCoreEventPublishComplete` | 45 | 90 |
| `LuaContext::UnitFlagManager` | 45 | 45 |
| `WorldView_Render_Submission` | 40 | 40 |
| `GameCoreEvent::UnitAddedToMap` | 36 | 78 |
| `UnitContext_Update` | 36 | 36 |
| `SerialEvent::LensUpdateArea` | 35 | 35 |
| `LuaGarbageCollectJob_Collect` | 27 | 27 |
| `Civ6App_AudioUpdateWait` | 27 | 27 |
| `WorldView_Render_VFX` | 26 | 26 |
| `GameCoreEvent::DistrictAddedToMap` | 24 | 60 |
| `GameViewState_Tick` | 19 | 246 |
| `GameCoreEvent::DistrictBuildingRestore` | 18 | 18 |
| `ModelContext_updateVises` | 15 | 15 |
| `AsyncUpdateManager_Update` | 14 | 14 |


### Modules, tout le processus (temps propre)

| module | CPU (ms) | % |
|---|---:|---:|
| civilizationvi_dx12.exe | 41806 | 48.2 |
| noyau | 28831 | 33.2 |
| ntdll.dll | 6725 | 7.7 |
| databasedll_finalrelease.dll | 2508 | 2.9 |
| ucrtbase.dll | 1234 | 1.4 |
| havokscript_finalrelease.dll | 982 | 1.1 |
| gamecore_xp2_finalrelease.dll | 898 | 1.0 |
| vcruntime140.dll | 807 | 0.9 |
| nvwgf2umx.dll | 777 | 0.9 |
| steamclient64.dll | 604 | 0.7 |
| kernelbase.dll | 413 | 0.5 |
| tier0_s64.dll | 340 | 0.4 |

## Chargement n° 2, 21:17:58

- Instantané réseau : `Net RECV (6) :NetSnapshot : m_snapShotDataSize=3007547` à 21:17:03, 55 s avant le début du chargement.
- Fin de la mise en file des messages (le joueur a la main) : 21:18:26, 28 s après le début.

Chargement de 21:17:58 à 21:18:26 (28 s, à la seconde près). Les sous-phases à 0 s ne sont pas chronométrées par le jeu : leur instant (à la seconde) montre où le temps passe.

| phase | durée (s) | à (s) |
|---|---:|---:|
| **CONFIGURE_CONTENT** | 5.57 | 6 |
| ↳ Block on asset load | 0.00 | 6 |
| ↳ InitializeNewGame | 8.48 | 14 |
| **GAMECORE_NEW_GAME** | 8.48 | 14 |
| ↳ GameViewState::LoadUI | 0.00 | 16 |
| ↳ Post-GameCore asset load | 0.31 | 16 |
| **GAMECORE_INITIALIZATION_DONE** | 1.49 | 16 |
| ↳ GameViewState::StartupGame | 9.35 | 25 |
| ↳ StartupGame asset load | 0.00 | 25 |
| **WAIT_FOR_PLAYERS** | 9.35 | 25 |
| **PostLoadTime** | 2.48 | 28 |
| *non attribué (bornes à la seconde)* | 0.6 | |

Étapes de Modding.log autour du chargement (heure recalée) :

| heure | t / début (s) | étape | jusqu'à la suivante (s) |
|---|---:|---|---:|
| 21:17:03.448 | -54.6 | Stage: Loading a save | 0.001 |
| 21:17:03.449 | -54.6 | Stage: Configure Content | 5.096 |
| 21:17:08.545 | -49.5 | Successfully reconfigured game. | 0.000 |
| 21:17:08.545 | -49.5 | Stage: Unserializing Game State | 50.049 |
| 21:17:58.594 | 0.6 | Stage: Configure Content | 0.002 |
| 21:17:58.596 | 0.6 | Configuring game content | 0.922 |
| 21:17:59.518 | 1.5 | Successfully reconfigured game. | 4.644 |
| 21:18:04.162 | 6.2 | Successfully reconfigured game. | 0.007 |
| 21:18:04.169 | 6.2 | Stage: Initializing a new game |  |

Interface (UserInterface.log) : 159 fichiers Lua, 1.14 s d'exécution du corps des fichiers, 1.87 s d'initialisation des contrôles. Les gestionnaires d'événements appelés ensuite (LoadGameViewStateDone…) ne sont pas chronométrés par le jeu.

| fichier ou contexte | s |
|---|---:|
| contrôles de InGame | 1.067 |
| contrôles de FrontEnd | 0.323 |
| contrôles de MainMenu | 0.267 |
| ../../../Base/Assets/UI/Civilopedia/CivilopediaScreen.lua | 0.147 |
| ram Files (x86)/Steam/steamapps/workshop/content/289070/3307282026/ui/InGame.lua | 0.091 |
| iles (x86)/Steam/steamapps/workshop/content/289070/3307282026/ui/StagingRoom.lua | 0.080 |
| contrôles de HostGame | 0.064 |
| contrôles de InGameTopOptionsMenu | 0.045 |
| ../../../DLC/Expansion2/UI/Additions/MapLabelManager.lua | 0.043 |
| m Files (x86)/Steam/steamapps/workshop/content/289070/3307282026/ui/MainMenu.lua | 0.041 |
| ../../../Base/Assets/UI/Options.lua | 0.027 |
| ../../../Base/Assets/UI/Options.lua | 0.026 |

## Trace WPR

- Fenêtre : 21:17:57 → 21:18:27 (30 s).
- civilizationvi_dx12.exe (pid 16580) : 74098 échantillons à 1 ms, soit 74.1 s de CPU ; 73903 avec pile.

### Images (présentations DXGI)

- 2569 images de 21:17:58 à 21:18:26 ; intervalle de synchro : 1 (2569) (0 : sans synchro verticale).

| période | images | durée (s) | images/s | p50 (ms) | p95 (ms) | p99 (ms) | max (ms) | > 33 ms | > 100 ms | bloqué dans Present, moyenne / p95 (ms) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| hors passage | 2568 | 28.3 | 91 | 9.9 | 21.7 | 34.3 | 1208 | 29 | 4 | 3.6 / 7.9 |

### GPU (relevés à la seconde)

Moyenne / p95 sur la période. Les compteurs « jeu » ne portent que sur le processus du jeu, « autres » sur tous les autres (bureau, navigateur…).

| mesure | hors passage (30 s) |
|---|---:|
| jeu 3D (%) | 7.9 / 66.6 |
| jeu Copy (%) | 0.0 / 0.0 |
| jeu VideoDecode (%) | 0.0 / 0.0 |
| autres 3D (%) | 0.3 / 0.4 |
| autres Copy (%) | 1.9 / 11.4 |
| autres VideoDecode (%) | 0.0 / 0.0 |
| nvidia fréquence GPU (MHz) | 698.5 / 2820.0 |
| nvidia fréquence mémoire (MHz) | 1953.7 / 9001.0 |
| nvidia puissance (W) | 23.5 / 55.0 |
| nvidia température (°C) | 44.9 / 51.0 |
| nvidia utilisation carte (%) | 32.3 / 82.0 |

### Threads

Concentration : part des échantillons sur les 4 adresses les plus fréquentes du thread. Tranche : CPU moyen par mise sur le processeur. Sortie : raison la plus fréquente de sortie du processeur.

| thread | nom | CPU (ms) | occupation | dont GameCore (ms) | noyau | mises sur le CPU | tranche (ms) | sortie | concentration | adresse la plus fréquente | point d'entrée |
|---:|---|---:|---:|---:|---:|---:|---:|---|---:|---|---|
| 8444 |  | 23167 | 77 % | 3169 | 10 % | 23927 | 0.97 | cession (yield) 73 % | 7 % | `gamecore_xp2_finalrelease.dll+0x5ca1d0` | `civilizationvi_dx12.exe+0xe5278d` |
| 9548 |  | 15198 | 51 % | 0 | 84 % | 947815 | 0.02 | attente (UserRequest) 100 % | 18 % | `noyau` | `civilizationvi_dx12.exe+0x99a252` |
| 25156 |  | 9524 | 32 % | 26 | 30 % | 445949 | 0.02 | verrou ou variable de condition (WrAlertByThreadId) 81 % | 35 % | `ntdll.dll+0x1bcaa` | `civilizationvi_dx12.exe+0xea3607` |
| 29552 |  | 6654 | 22 % | 0 | 13 % | 158330 | 0.04 | cession (yield) 94 % | 10 % | `civilizationvi_dx12.exe+0xe50fd2` | `civilizationvi_dx12.exe+0xe5278d` |
| 8748 |  | 6426 | 21 % | 0 | 13 % | 160427 | 0.04 | cession (yield) 94 % | 10 % | `civilizationvi_dx12.exe+0xe50fd2` | `civilizationvi_dx12.exe+0xe5278d` |
| 14936 |  | 6352 | 21 % | 0 | 12 % | 160476 | 0.04 | cession (yield) 93 % | 9 % | `civilizationvi_dx12.exe+0xe50fd2` | `civilizationvi_dx12.exe+0xe5278d` |
| 30380 |  | 6280 | 21 % | 0 | 11 % | 153321 | 0.04 | cession (yield) 94 % | 10 % | `civilizationvi_dx12.exe+0xe50fd2` | `civilizationvi_dx12.exe+0xe5278d` |
| 20448 |  | 168 | 1 % | 0 | 7 % | 1632 | 0.10 | attente (UserRequest) 88 % | 11 % | `civilizationvi_dx12.exe+0xd23e01` | `civilizationvi_dx12.exe+0xcf2547` |
| 12664 |  | 58 | 0 % | 0 | 0 % | 193 | 0.30 | attente (UserRequest) 87 % | 74 % | `civilizationvi_dx12.exe+0xc7e7c5` | `civilizationvi_dx12.exe+0xcde2eb` |
| 15264 |  | 53 | 0 % | 0 | 55 % | 3010 | 0.02 | attente (UserRequest) 100 % | 11 % | `noyau` | `xaudio2_9.dll+0x343e7` |
| 1600 |  | 52 | 0 % | 0 | 85 % | 1446 | 0.04 | attente (UserRequest) 89 % | 12 % | `noyau` | `nvwgf2umx.dll+0xeba2aa` |
| 11076 |  | 34 | 0 % | 0 | 79 % | 2906 | 0.01 | 4 99 % | 21 % | `noyau` | `sdk_legacy_led_x64.dll+0x78bdf` |
| 10132 |  | 25 | 0 % | 0 | 76 % | 2905 | 0.01 | 4 99 % | 24 % | `noyau` | `civilizationvi_dx12.exe+0xa8d79` |
| 30916 |  | 24 | 0 % | 0 | 96 % | 2988 | 0.01 | attente (UserRequest) 98 % | 25 % | `noyau` | `nvwgf2umx.dll+0xeba2aa` |
| 29560 |  | 24 | 0 % | 0 | 54 % | 363 | 0.07 | attente (UserRequest) 82 % | 21 % | `msvcrt.dll+0x7b0e3` | `winmm.dll+0x88017` |
| 6464 |  | 12 | 0 % | 0 | 92 % | 960 | 0.01 | 4 98 % | 33 % | `noyau` | `eossdk-win64-shipping.dll+0x4cf67c` |
### Zones du profileur de l'exe (proxy de démarrage, `profil_zones=on`)

- 121184 zones de 214 noms sur 30.0 s.
- `Civ6App_UpdateFrame` : 2569 images.

Temps réel passé dans chaque zone, propre (sans ses zones filles), tous threads.

| zone | nombre | total (ms) | propre (ms) | moyenne (ms) | max (ms) | threads |
|---|---:|---:|---:|---:|---:|---|
| `Civ6App_UpdateFrame` | 2569 | 24740 | 11122 | 9.630 | 3128.7 | 25156 |
| `Civ6App_UpdateNetwork` | 2569 | 6644 | 6644 | 2.586 | 91.2 | 25156 |
| `Civ6App_SteamAPI_RunCallbacks` | 2569 | 5886 | 5886 | 2.291 | 37.3 | 25156 |
| `GameViewState_Tick` | 59 | 3194 | 3094 | 54.137 | 3090.3 | 25156 |
| `Graphics_SubmissionIncFrame` | 2569 | 2294 | 2294 | 0.893 | 125.1 | 25156 |
| `UIManager_Update` | 2570 | 1206 | 1205 | 0.469 | 1159.2 | 8444, 8748, 14936 … |
| `SerialEvent::FinishedGameplayContentConfigure` | 2 | 293 | 293 | 146.528 | 293.1 | 25156 |
| `lua_RegisterDatabase` | 1 | 212 | 212 | 212.496 | 212.5 | 8444 |
| `UIUpdateJob_Render` | 2570 | 152 | 152 | 0.059 | 19.0 | 8444, 8748, 14936 … |
| `WorldView_PostLoadUpdate` | 155 | 91 | 91 | 0.590 | 6.5 | 25156 |
| `Civ6App_EOSAPI_RunCallbacks` | 2569 | 49 | 49 | 0.019 | 0.1 | 25156 |
| `AppLuaScriptSystem_EndFrame` | 2569 | 47 | 47 | 0.018 | 0.5 | 25156 |
| `WorldView_Render_VFX` | 73 | 39 | 39 | 0.528 | 14.1 | 25156 |
| `SerialEvent::AfterGameplayContentChange` | 1 | 38 | 38 | 37.779 | 37.8 | 8444 |
| `LuaContext::MapLabelManager` | 26 | 36 | 36 | 1.399 | 34.3 | 25156 |
| `SerialEvent::LensUpdateArea` | 102 | 36 | 36 | 0.356 | 0.6 | 25156 |
| `LuaContext::DiplomacyRibbon` | 249 | 35 | 35 | 0.141 | 3.9 | 25156 |
| `WorldView_Render_Submission` | 73 | 31 | 31 | 0.431 | 11.5 | 25156 |
| `SerialEvent::GameCoreEventPublishComplete` | 1695 | 48 | 28 | 0.029 | 1.1 | 25156 |
| `Civ6App_HandleGameStateSwap` | 2569 | 25 | 25 | 0.010 | 15.5 | 25156 |
| `LuaGarbageCollectJob_Collect` | 13281 | 22 | 22 | 0.002 | 0.9 | 8444, 8748, 14936 … |
| `Civ6App_AudioUpdateWait` | 2569 | 22 | 21 | 0.008 | 1.5 | 25156 |
| `LocalMachineEvent::PlayerInfoChanged` | 1 | 20 | 20 | 20.376 | 20.4 | 25156 |
| `WorldView_Render_Models` | 73 | 20 | 20 | 0.275 | 0.5 | 25156 |
| `WorldView_Render` | 73 | 114 | 18 | 1.561 | 24.8 | 25156 |
| `ModelContext_updateVises` | 73 | 13 | 13 | 0.182 | 1.4 | 25156 |
| `WorldView_RenderFOWJob` | 73 | 13 | 13 | 0.180 | 0.3 | 8444, 8748, 14936 … |
| `GameCoreEvent::UnitAddedToMap` | 66 | 22 | 13 | 0.338 | 0.5 | 25156 |
| `WorldView_Render_FOWMesh` | 73 | 11 | 11 | 0.157 | 1.4 | 8444, 8748, 14936 … |
| `LuaContext::UnitFlagManager` | 463 | 11 | 11 | 0.024 | 0.2 | 25156 |

CPU attribué par les échantillons, par CPU propre (74098 sur les threads qui ont des zones ; 64260 hors de toute zone).

| zone | CPU propre (ms) | CPU inclusif (ms) |
|---|---:|---:|
| `Civ6App_UpdateNetwork` | 2542 | 2542 |
| `Civ6App_SteamAPI_RunCallbacks` | 2304 | 2304 |
| `Civ6App_UpdateFrame` | 1848 | 6511 |
| `GameViewState_Tick` | 1728 | 1819 |
| `SerialEvent::FinishedGameplayContentConfigure` | 291 | 291 |
| `lua_RegisterDatabase` | 213 | 213 |
| `UIUpdateJob_Render` | 132 | 132 |
| `WorldView_PostLoadUpdate` | 80 | 80 |
| `Civ6App_EOSAPI_RunCallbacks` | 62 | 62 |
| `UIManager_Update` | 42 | 42 |
| `AppLuaScriptSystem_EndFrame` | 40 | 40 |
| `SerialEvent::AfterGameplayContentChange` | 38 | 38 |
| `SerialEvent::LensUpdateArea` | 37 | 37 |
| `LuaContext::MapLabelManager` | 36 | 36 |
| `LuaContext::DiplomacyRibbon` | 35 | 35 |
| `SerialEvent::GameCoreEventPublishComplete` | 34 | 54 |
| `LuaGarbageCollectJob_Collect` | 31 | 31 |
| `WorldView_Render_Submission` | 31 | 31 |
| `Civ6App_HandleGameStateSwap` | 22 | 22 |
| `WorldView_Render_Models` | 22 | 22 |
| `Civ6App_AudioUpdateWait` | 19 | 19 |
| `WorldView_Render` | 19 | 97 |
| `WorldView_Render_VFX` | 17 | 17 |
| `Graphics_SubmissionIncFrame` | 14 | 14 |
| `GameCoreEvent::UnitAddedToMap` | 13 | 22 |
| `WorldView_RenderFOWJob` | 13 | 13 |
| `LuaContext::LaunchBar` | 12 | 12 |
| `LuaContext::UnitFlagManager` | 11 | 11 |
| `ModelContext_updateVises` | 10 | 10 |
| `WorldView_RenderAOJob` | 10 | 10 |


### Modules, tout le processus (temps propre)

| module | CPU (ms) | % |
|---|---:|---:|
| civilizationvi_dx12.exe | 29921 | 40.4 |
| noyau | 21414 | 28.9 |
| ntdll.dll | 6615 | 8.9 |
| havokscript_finalrelease.dll | 5179 | 7.0 |
| gamecore_xp2_finalrelease.dll | 3195 | 4.3 |
| databasedll_finalrelease.dll | 3080 | 4.2 |
| ucrtbase.dll | 1210 | 1.6 |
| steamclient64.dll | 725 | 1.0 |
| nvwgf2umx.dll | 677 | 0.9 |
| vcruntime140.dll | 610 | 0.8 |
| tier0_s64.dll | 410 | 0.6 |
| kernelbase.dll | 322 | 0.4 |

## Chargement n° 3, 21:19:37

- Aucun instantané réseau dans la minute avant le chargement.
- Fin de la mise en file des messages (le joueur a la main) : 21:20:04, 27 s après le début.

Chargement de 21:19:37 à 21:20:04 (27 s, à la seconde près). Les sous-phases à 0 s ne sont pas chronométrées par le jeu : leur instant (à la seconde) montre où le temps passe.

| phase | durée (s) | à (s) |
|---|---:|---:|
| **CONFIGURE_CONTENT** | 5.27 | 6 |
| ↳ Block on asset load | 0.00 | 11 |
| ↳ InitializeNewGame | 6.38 | 18 |
| **GAMECORE_NEW_GAME** | 6.38 | 18 |
| ↳ GameViewState::LoadUI | 0.00 | 19 |
| ↳ Post-GameCore asset load | 0.31 | 19 |
| **GAMECORE_INITIALIZATION_DONE** | 1.43 | 19 |
| ↳ GameViewState::StartupGame | 6.16 | 25 |
| ↳ StartupGame asset load | 0.00 | 25 |
| **WAIT_FOR_PLAYERS** | 6.16 | 25 |
| **PostLoadTime** | 1.74 | 27 |
| *non attribué (bornes à la seconde)* | 6.0 | |

Étapes de Modding.log autour du chargement (heure recalée) :

| heure | t / début (s) | étape | jusqu'à la suivante (s) |
|---|---:|---|---:|
| 21:19:12.812 | -24.2 | Configuring game content | 2.048 |
| 21:19:14.860 | -22.1 | Successfully reconfigured game. | 8.877 |
| 21:19:23.737 | -13.3 | Configuring game content | 0.901 |
| 21:19:24.638 | -12.4 | Successfully reconfigured game. | 13.093 |
| 21:19:37.731 | 0.7 | Stage: Configure Content | 5.269 |
| 21:19:43.000 | 6.0 | Successfully reconfigured game. | 5.598 |
| 21:19:48.598 | 11.6 | Stage: Initializing a new game |  |

CONFIGURE_CONTENT (Modding.log, à la milliseconde), 5.27 s, après « Initializing a new game », avant « Initializing a new game » :

| t (s) | étape | jusqu'à l'étape suivante (s) |
|---:|---|---:|
| 0.241 | Performing pre configure processing. | 0.014 |
| 0.255 | Reloading Gameplay DLL. | 0.057 |
| 0.312 | Reverted Localization database to its original state. | 0.000 |
| 0.312 | Rebuilding gameplay database. | 0.132 |
| 0.444 | Reverted Gameplay database to its original state. | 0.000 |
| 0.444 | Updating gameplay database with embedded GameCore data. | 0.011 |
| 0.455 | Updated gameplay database with GameCore data. | 0.000 |
| 0.455 | UpdateContent - Importing implicit files, pre components | 0.169 |
| 0.624 | Modding Framework - Applying Components | 3.268 |
| 3.892 | Modding Framework - Finished Apply Components | 0.000 |
| 3.892 | Applied all components of enabled mods. | 0.000 |
| 3.892 | UpdateContent - Importing implicit files, post components | 0.021 |
| 3.913 | Localization database has been updated.  Caching data. | 0.305 |
| 4.218 | Initializing Gameplay DLL. | 0.001 |
| 4.219 | Saving a debug version of the gameplay database. | 0.066 |
| 4.285 | Performing post configure processing. | 0.223 |
| 4.508 | Reloading icons. | 0.066 |
| 4.574 | Reloading colors. | 0.695 |
| 5.269 | Successfully reconfigured game. | 0.000 |

Composants les plus lents (452 composants, 3.25 s au total) :

| composant | s |
|---|---:|
| Expansion2_CoreText (UpdateText) | 0.898 |
| Expansion2CoreContent (UpdateDatabase) | 0.405 |
| BBG_TextUpdate (UpdateText) | 0.168 |
| Expansion2Core (UpdateDatabase) | 0.114 |
| BBG_Base (UpdateDatabase) | 0.111 |
| Expansion1_MajorText (UpdateText) | 0.106 |
| BBG_XP2 (UpdateDatabase) | 0.070 |
| GranColombiaMayaText (UpdateText) | 0.055 |
| MUU_LocalizedText (UpdateText) | 0.046 |
| NewAction (UpdateDatabase) | 0.044 |
| MUU_UpdateDatabase (UpdateDatabase) | 0.040 |
| Expansion2MajorContent (UpdateDatabase) | 0.040 |
| BabylonText (UpdateText) | 0.035 |
| KublaiKhanVietnamText (UpdateText) | 0.033 |
| UpdateDatabase (UpdateDatabase) | 0.033 |

Fichiers les plus lents (1098 fichiers, 2.86 s, dont 1.47 s de textes) :

| fichier | s |
|---|---:|
| Text/Expansion2_Translations_Text.xml | 0.649 |
| Data/Expansion2_NamedPlaces.xml | 0.116 |
| Data/Expansion2_RemoveData.xml | 0.105 |
| Text/Expansion1_Translations_Major_Text.xml | 0.075 |
| Text/GranColombia_Maya_Translations_Text.xml | 0.044 |
| SteelThunderUnits/Data/MOARUniqueUnits.sql | 0.040 |
| Text/Babylon_Translations_Text.xml | 0.029 |
| sql/_utils.sql | 0.029 |
| lang/spanish.xml | 0.027 |
| sql/Base/BBM_Wonders.sql | 0.026 |
| Text/KublaiKhan_Vietnam_Translations_Text.xml | 0.026 |
| Text/Byzantium_Gaul_Translations_Text.xml | 0.024 |
| Text/Indonesia_Khmer_Translations_Text.xml | 0.022 |
| Data/Expansion2_Modifiers.xml | 0.022 |
| lang/chinese.xml | 0.022 |

Interface (UserInterface.log) : 129 fichiers Lua, 0.83 s d'exécution du corps des fichiers, 1.12 s d'initialisation des contrôles. Les gestionnaires d'événements appelés ensuite (LoadGameViewStateDone…) ne sont pas chronométrés par le jeu.

| fichier ou contexte | s |
|---|---:|
| contrôles de InGame | 1.002 |
| ../../../Base/Assets/UI/Civilopedia/CivilopediaScreen.lua | 0.107 |
| ram Files (x86)/Steam/steamapps/workshop/content/289070/3307282026/ui/InGame.lua | 0.097 |
| contrôles de InGameTopOptionsMenu | 0.044 |
| ../../../DLC/Expansion2/UI/Additions/MapLabelManager.lua | 0.031 |
| ../../../Base/Assets/UI/Options.lua | 0.026 |
| ../../../DLC/Expansion2/UI/Replacements/ResearchChooser_Expansion1.lua | 0.021 |
| Steam/steamapps/workshop/content/289070/1916397407/UI/ReportScreen_Spectator.lua | 0.020 |
| ../../../Base/Assets/UI/Panels/ProductionPanel.lua | 0.018 |
| ../../../DLC/Expansion2/UI/Replacements/CityPanel_Expansion2.lua | 0.018 |
| 6)/Steam/steamapps/workshop/content/289070/3307282026/ui/Additions/VotePanel.lua | 0.017 |
| contrôles de MinimapPanel | 0.017 |

## Trace WPR

- Fenêtre : 21:19:36 → 21:20:05 (29 s).
- civilizationvi_dx12.exe (pid 16580) : 78456 échantillons à 1 ms, soit 78.5 s de CPU ; 78235 avec pile.

### Images (présentations DXGI)

- 2566 images de 21:19:36 à 21:20:04 ; intervalle de synchro : 1 (2566) (0 : sans synchro verticale).

| période | images | durée (s) | images/s | p50 (ms) | p95 (ms) | p99 (ms) | max (ms) | > 33 ms | > 100 ms | bloqué dans Present, moyenne / p95 (ms) |
|---|---:|---:|---:|---:|---:|---:|---:|---:|---:|---|
| hors passage | 2565 | 29.0 | 88 | 10.0 | 20.3 | 31.9 | 1120 | 20 | 4 | 4.0 / 7.9 |

### GPU (relevés à la seconde)

Moyenne / p95 sur la période. Les compteurs « jeu » ne portent que sur le processus du jeu, « autres » sur tous les autres (bureau, navigateur…).

| mesure | hors passage (29 s) |
|---|---:|
| jeu 3D (%) | 8.7 / 51.6 |
| jeu Copy (%) | 0.0 / 0.0 |
| jeu VideoDecode (%) | 0.0 / 0.0 |
| autres 3D (%) | 0.3 / 0.5 |
| autres Copy (%) | 1.9 / 13.7 |
| autres VideoDecode (%) | 0.0 / 0.0 |
| nvidia fréquence GPU (MHz) | 774.6 / 2550.0 |
| nvidia fréquence mémoire (MHz) | 1644.2 / 9001.0 |
| nvidia puissance (W) | 19.2 / 33.0 |
| nvidia température (°C) | 41.9 / 42.0 |
| nvidia utilisation carte (%) | 33.0 / 67.0 |

### Threads

Concentration : part des échantillons sur les 4 adresses les plus fréquentes du thread. Tranche : CPU moyen par mise sur le processeur. Sortie : raison la plus fréquente de sortie du processeur.

| thread | nom | CPU (ms) | occupation | dont GameCore (ms) | noyau | mises sur le CPU | tranche (ms) | sortie | concentration | adresse la plus fréquente | point d'entrée |
|---:|---|---:|---:|---:|---:|---:|---:|---|---:|---|---|
| 8444 |  | 23965 | 83 % | 2537 | 22 % | 46977 | 0.51 | cession (yield) 80 % | 21 % | `noyau` | `civilizationvi_dx12.exe+0xe5278d` |
| 27584 |  | 14443 | 50 % | 0 | 84 % | 896780 | 0.02 | attente (UserRequest) 99 % | 17 % | `noyau` | `civilizationvi_dx12.exe+0x99a252` |
| 14936 |  | 8496 | 29 % | 0 | 32 % | 186002 | 0.05 | cession (yield) 60 % | 16 % | `civilizationvi_dx12.exe+0x9bbf27` | `civilizationvi_dx12.exe+0xe5278d` |
| 8748 |  | 8417 | 29 % | 0 | 38 % | 187408 | 0.04 | cession (yield) 60 % | 15 % | `noyau` | `civilizationvi_dx12.exe+0xe5278d` |
| 29552 |  | 7894 | 27 % | 0 | 33 % | 187557 | 0.04 | cession (yield) 60 % | 16 % | `civilizationvi_dx12.exe+0x9bbf27` | `civilizationvi_dx12.exe+0xe5278d` |
| 30380 |  | 7462 | 26 % | 0 | 34 % | 166732 | 0.04 | cession (yield) 56 % | 12 % | `civilizationvi_dx12.exe+0x9bbf27` | `civilizationvi_dx12.exe+0xe5278d` |
| 25156 |  | 7061 | 24 % | 12 | 34 % | 355848 | 0.02 | verrou ou variable de condition (WrAlertByThreadId) 82 % | 37 % | `ntdll.dll+0x1bcaa` | `civilizationvi_dx12.exe+0xea3607` |
| 17512 |  | 168 | 1 % | 59 | 10 % | 514 | 0.33 | verrou ou variable de condition (WrAlertByThreadId) 83 % | 20 % | `civilizationvi_dx12.exe+0xf65990` | `civilizationvi_dx12.exe+0x99a252` |
| 20448 |  | 111 | 0 % | 0 | 13 % | 1418 | 0.08 | attente (UserRequest) 99 % | 12 % | `civilizationvi_dx12.exe+0xd4de5c` | `civilizationvi_dx12.exe+0xcf2547` |
| 15264 |  | 64 | 0 % | 0 | 50 % | 2907 | 0.02 | attente (UserRequest) 100 % | 14 % | `noyau` | `xaudio2_9.dll+0x343e7` |
| 12664 |  | 64 | 0 % | 0 | 2 % | 188 | 0.34 | attente (UserRequest) 90 % | 67 % | `civilizationvi_dx12.exe+0xc7e7c5` | `civilizationvi_dx12.exe+0xcde2eb` |
| 29292 |  | 57 | 0 % | 0 | 4 % | 106 | 0.54 | interruption 58 % | 12 % | `bink2w64.dll+0x1a17e` | `bink2w64.dll+0x35c02` |
| 6456 |  | 49 | 0 % | 0 | 2 % | 155 | 0.32 | interruption 71 % | 8 % | `bink2w64.dll+0x15cc9` | `bink2w64.dll+0x35c02` |
| 1600 |  | 43 | 0 % | 0 | 100 % | 1213 | 0.04 | attente (UserRequest) 87 % | 16 % | `noyau` | `nvwgf2umx.dll+0xeba2aa` |
| 29560 |  | 30 | 0 % | 0 | 60 % | 382 | 0.08 | attente (UserRequest) 76 % | 20 % | `winmm.dll+0x33a3f` | `winmm.dll+0x88017` |
| 10132 |  | 24 | 0 % | 0 | 88 % | 2805 | 0.01 | 4 98 % | 29 % | `noyau` | `civilizationvi_dx12.exe+0xa8d79` |
### Zones du profileur de l'exe (proxy de démarrage, `profil_zones=on`)

- 122460 zones de 275 noms sur 29.0 s.
- `Civ6App_UpdateFrame` : 2565 images.

Temps réel passé dans chaque zone, propre (sans ses zones filles), tous threads.

| zone | nombre | total (ms) | propre (ms) | moyenne (ms) | max (ms) | threads |
|---|---:|---:|---:|---:|---:|---|
| `Civ6App_UpdateFrame` | 2565 | 22339 | 11901 | 8.709 | 1114.3 | 25156 |
| `Civ6App_UpdateNetwork` | 2542 | 5500 | 5500 | 2.164 | 30.4 | 25156 |
| `Civ6App_SteamAPI_RunCallbacks` | 2566 | 4883 | 4883 | 1.903 | 31.4 | 25156 |
| `Graphics_SubmissionIncFrame` | 2566 | 2760 | 2760 | 1.076 | 54.3 | 25156 |
| `UIManager_Update` | 2566 | 1160 | 1159 | 0.452 | 1095.1 | 8444, 8748, 14936 … |
| `SerialEvent::AfterGameplayContentChange` | 1 | 686 | 686 | 686.004 | 686.0 | 8444 |
| `UILoadLock` | 2566 | 636 | 636 | 0.248 | 635.5 | 25156 |
| `Civ6App_HandleGameStateSwap` | 2565 | 492 | 492 | 0.192 | 473.9 | 25156 |
| `lua_RegisterDatabase` | 1 | 214 | 214 | 213.687 | 213.7 | 8444 |
| `UIUpdateJob_Render` | 2566 | 173 | 173 | 0.067 | 17.6 | 8444, 8748, 14936 … |
| `DoGameStateSaveRequest` | 1 | 92 | 92 | 92.386 | 92.4 | 17512 |
| `WorldView_PostLoadUpdate` | 119 | 76 | 76 | 0.638 | 4.3 | 25156 |
| `AppLuaScriptSystem_EndFrame` | 2565 | 55 | 55 | 0.022 | 0.5 | 25156 |
| `Civ6App_EOSAPI_RunCallbacks` | 2566 | 52 | 52 | 0.020 | 0.6 | 25156 |
| `LuaGarbageCollectJob_Collect` | 18174 | 35 | 35 | 0.002 | 1.0 | 8444, 8748, 14936 … |
| `LuaContext::MapLabelManager` | 36 | 26 | 26 | 0.715 | 23.1 | 25156 |
| `SerialEvent::LensUpdateArea` | 81 | 24 | 24 | 0.299 | 0.5 | 25156 |
| `WorldView_Render_Submission` | 57 | 24 | 24 | 0.417 | 7.7 | 25156 |
| `WorldView_Render_VFX` | 57 | 23 | 23 | 0.396 | 7.2 | 25156 |
| `Civ6App_AudioUpdateWait` | 2566 | 21 | 21 | 0.008 | 0.6 | 25156 |
| `WorldView_RenderFOWJob` | 57 | 20 | 20 | 0.352 | 7.1 | 8444, 8748, 14936 … |
| `SerialEvent::GameCoreEventPublishComplete` | 1869 | 34 | 18 | 0.018 | 0.3 | 25156 |
| `SequenceEvent::PostLoadBegin` | 1 | 15 | 15 | 15.000 | 15.0 | 25156 |
| `WorldView_Render_Models` | 57 | 12 | 12 | 0.203 | 0.5 | 25156 |
| `WorldView_Render` | 57 | 76 | 12 | 1.328 | 15.2 | 25156 |
| `MovieTick_Upload` | 45 | 11 | 11 | 0.249 | 0.3 | 25156 |
| `GameViewState_Tick` | 41 | 90 | 10 | 2.202 | 16.3 | 25156 |
| `WorldView_RenderAOJob` | 57 | 10 | 10 | 0.179 | 0.3 | 8444, 8748, 14936 … |
| `LuaContext::LoadScreen` | 3 | 9 | 9 | 3.165 | 8.1 | 25156 |
| `SerialEvent::PreTurnBegin` | 1 | 102 | 9 | 101.524 | 101.5 | 17512 |

CPU attribué par les échantillons, par CPU propre (78456 sur les threads qui ont des zones ; 70361 hors de toute zone).

| zone | CPU propre (ms) | CPU inclusif (ms) |
|---|---:|---:|
| `Civ6App_UpdateNetwork` | 2135 | 2135 |
| `Civ6App_SteamAPI_RunCallbacks` | 1897 | 1897 |
| `Civ6App_UpdateFrame` | 1812 | 4359 |
| `SerialEvent::AfterGameplayContentChange` | 684 | 684 |
| `Civ6App_HandleGameStateSwap` | 489 | 489 |
| `lua_RegisterDatabase` | 213 | 213 |
| `UIUpdateJob_Render` | 167 | 167 |
| `DoGameStateSaveRequest` | 80 | 80 |
| `UIManager_Update` | 58 | 58 |
| `AppLuaScriptSystem_EndFrame` | 58 | 58 |
| `Civ6App_EOSAPI_RunCallbacks` | 48 | 48 |
| `WorldView_PostLoadUpdate` | 48 | 48 |
| `LuaGarbageCollectJob_Collect` | 39 | 39 |
| `LuaContext::MapLabelManager` | 27 | 27 |
| `WorldView_Render_Submission` | 26 | 26 |
| `SerialEvent::LensUpdateArea` | 23 | 23 |
| `Civ6App_AudioUpdateWait` | 21 | 23 |
| `WorldView_RenderFOWJob` | 18 | 18 |
| `SerialEvent::GameCoreEventPublishComplete` | 17 | 27 |
| `WorldView_Render` | 13 | 63 |
| `WorldView_RenderAOJob` | 12 | 12 |
| `Graphics_SubmissionIncFrame` | 12 | 12 |
| `WorldView_Render_VFX` | 12 | 12 |
| `GameCoreEvent::UnitAddedToMap` | 10 | 14 |
| `GameViewState_Tick` | 9 | 88 |
| `ModelContext_updateVises` | 8 | 8 |
| `Civ6App_PublishGameCoreEvents` | 7 | 45 |
| `MovieTick_Upload` | 7 | 7 |
| `UnitContext_Update` | 7 | 7 |
| `GameViewState_UpdateCache` | 7 | 7 |


### Modules, tout le processus (temps propre)

| module | CPU (ms) | % |
|---|---:|---:|
| noyau | 31000 | 39.5 |
| civilizationvi_dx12.exe | 26629 | 33.9 |
| ntdll.dll | 8222 | 10.5 |
| havokscript_finalrelease.dll | 3228 | 4.1 |
| gamecore_xp2_finalrelease.dll | 2608 | 3.3 |
| databasedll_finalrelease.dll | 2406 | 3.1 |
| ucrtbase.dll | 827 | 1.1 |
| steamclient64.dll | 733 | 0.9 |
| nvwgf2umx.dll | 691 | 0.9 |
| vcruntime140.dll | 497 | 0.6 |
| kernelbase.dll | 484 | 0.6 |
| tier0_s64.dll | 354 | 0.5 |
