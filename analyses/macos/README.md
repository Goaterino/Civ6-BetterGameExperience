# Portage macOS

État au 2026-11-11 : binaires du jeu inspectés, essai de faisabilité écrit, pas encore lancé. Aucun
code du proxy n'est porté.

## Ce que livre le jeu sous macOS

Relevé sur le dépôt Steam 26407122 (port Aspyr, version 1.0.12.54), par lecture des en-têtes
Mach-O, des tables de symboles et des chaînes.

| Fichier | Rôle |
|---|---|
| `MacOS/Civ6_Exe` | lanceur Aspyr (`com.aspyr.civ6.steam`), qui démarre le jeu |
| `MacOS/Civ6_Exe_Child` | le jeu |
| `MacOS/GameCore_XP2.dll`, `MacOS/GameCore_Base.dll` | GameCore : des bundles Mach-O, malgré l'extension |

Les quatre sont universels, x86_64 (macOS 10.14 au moins) et arm64 (macOS 11 au moins). Chaque
adresse de détour est donc à retrouver deux fois.

- **Signature.** Le lanceur et le jeu sont sous hardened runtime, avec trois droits :
  `allow-dyld-environment-variables`, `disable-library-validation` et `allow-jit`. Ils n'ont ni
  `disable-executable-page-protection` ni `allow-unsigned-executable-memory`. Les bundles GameCore
  sont signés sans hardened runtime.
- **Bibliothèques.** HavokScript, Localization et la base de données sont liés dans
  `Civ6_Exe_Child` : il n'y a plus de `HavokScript_FinalRelease.dll` dont détourner un import.
  L'exe exporte 443 symboles nommés (`hks_*`, `luaL_*`, `lua_*`, `Localization::*`, `Database::*`),
  ceux que GameCore lui demande. `luaL_loadbuffer` n'en fait pas partie. Le reste est sans symboles.
- **Chargement de GameCore.** L'exe importe `dlopen`, `dlsym` et `dlclose`, et contient les noms
  `GameCore_XP2.dll` et `DllCreateGameContext`. Le bundle n'a que deux exports,
  `DllCreateGameContext` et `DllDestroyGameContext`, contre 157 sous Windows. Ses 360 imports du
  jeu sont résolus dans l'exe au chargement (`dynamic_lookup`).
- **Dylibs livrées.** L'exe demande `@loader_path/libsteam_api.dylib`, `@rpath/libtbb.dylib`,
  `@rpath/libtbbmalloc.dylib`, `@executable_path/libBink2Mac*.dylib` et, en x86_64,
  `@rpath/libEOSSDK-Mac-Shipping.dylib`. Ses rpath sont `@executable_path` puis
  `@executable_path/../Frameworks`.
- **Binaires Windows du dépôt macOS.** Un vieux build (mai 2020), différent du jeu Windows
  actuel. Il est livré avec `GameCore_XP2_FinalRelease.map`, qui nomme toutes les fonctions de ce
  GameCore-là : une aide pour reconnaître les fonctions du GameCore macOS, qui n'a pas de symboles.

## Ce qui change par rapport à Windows

| Sous Windows | Sous macOS |
|---|---|
| `winmm.dll` posée à côté de l'exe | `DYLD_INSERT_LIBRARIES` dans les options de lancement Steam, que les droits du jeu autorisent ; ou une dylib posée devant `libtbb.dylib` si celle du jeu est dans `Frameworks` (à vérifier) |
| accroche de `LoadLibrary` dans la table d'imports | accroche de l'import `dlopen` de l'exe |
| 157 exports relayés | 2 exports relayés |
| accroche de l'import `luaL_loadbuffer` | fonction interne de l'exe, à retrouver par signature et à détourner |
| lecture XML retrouvée par signature | idem, signature à refaire par architecture |
| une trentaine de `Site` dans GameCore, adresses fixes | adresses à retrouver dans un binaire sans symboles, par architecture |
| `VirtualProtect` puis saut de 14 octets | patch du code sous hardened runtime : **à établir**, voir l'essai |

Le Lua et le XML de l'interface, les réglages et le salon multijoueur ne dépendent pas de la
plateforme.

## Essai de patch sous hardened runtime

Tout le portage des détours dépend d'une question : un processus signé comme le jeu peut-il
modifier son propre code ? [essai_patch](essai_patch) y répond sans le jeu ni Mac, sur les
machines macOS de GitHub Actions.

- [hote.c](essai_patch/hote.c) tient le rôle de `Civ6_Exe_Child` : une fonction dans son code, et
  un bundle chargé par `dlopen` puis `dlsym`.
- [gamecore.c](essai_patch/gamecore.c) tient le rôle de `GameCore_XP2.dll`.
- [injecte.c](essai_patch/injecte.c) est la dylib injectée par `DYLD_INSERT_LIBRARIES`. Elle essaie :
  - de la mémoire exécutable pour les trampolines (`MAP_JIT`, ou une page RW passée RX) ;
  - l'accroche de l'import `dlopen` de l'exe ;
  - le patch d'une fonction de l'exe, puis d'une fonction du bundle, par cinq méthodes
    (`mprotect`, `mach_vm_protect` avec `VM_PROT_COPY`, page RWX, copie posée par `mach_vm_remap`,
    la même depuis une page `MAP_JIT`).

  Chaque tentative tourne dans un processus fils : un refus qui tue le processus devient un verdict.
- [essai.sh](essai_patch/essai.sh) compile le tout pour une architecture, signe l'hôte de cinq
  façons et rassemble les verdicts en tableau.

| Variante de l'hôte | Signature | Ce qu'elle montre |
|---|---|---|
| `jeu` | hardened runtime + les trois droits du jeu | le cas à trancher |
| `nu` | sans hardened runtime | témoin : ce que permet le système sans restriction |
| `strict` | hardened runtime sans droit | l'injection doit être refusée : l'essai applique bien les règles |
| `sans_protection` | droits du jeu + `disable-executable-page-protection` | ce que ce droit ajouterait |
| `memoire_libre` | droits du jeu + `allow-unsigned-executable-memory` | idem |

Le workflow [macos-essai.yml](../../.github/workflows/macos-essai.yml) lance l'essai sur Apple
Silicon en arm64, sur Apple Silicon en x86_64 (Rosetta) et sur un Mac Intel. Il se déclenche à la
main ou quand l'essai change ; le tableau est dans le résumé de chaque job. Sur un Mac :

```bash
bash analyses/macos/essai_patch/essai.sh
```

Limite : l'hôte est signé ad hoc, le jeu par un certificat Developer ID. Les règles du hardened
runtime tiennent aux drapeaux et aux droits de la signature, les mêmes ici ; l'essai dans le vrai
jeu restera à faire sur un Mac.

Lecture des résultats de la variante `jeu` :

- **un patch du code passe** : `patch.rs` se porte tel quel, avec la méthode qui a passé ;
- **aucun ne passe, mais l'accroche de `dlopen` et `MAP_JIT` passent** : il reste les accroches par
  pointeurs (imports de l'exe et de GameCore, tables virtuelles). Les détours au milieu du code de
  GameCore demandent alors un autre moyen, par exemple charger une copie de GameCore déjà modifiée
  à la place de l'original, ce que l'accroche de `dlopen` permet ;
- **x86_64 sous Rosetta passe et arm64 non** : lancer le jeu en x86_64 sur Apple Silicon devient
  une solution de repli.

## Plan

1. Lancer l'essai et en tirer la méthode de détour.
2. Chargeur : dylib injectée qui accroche `dlopen` et charge le proxy à la place de
   `GameCore_XP2.dll`. `win.rs`, `patch.rs` et `exe.rs` passent derrière une couche par plateforme.
3. Interface : retrouver `luaL_loadbuffer` et la lecture XML dans l'exe macOS. Tous les correctifs
   Lua, l'onglet d'options et le salon suivent.
4. Correctifs de GameCore : reporter les `Site` par architecture, les correctifs POL d'abord
   (ils conditionnent le multijoueur entre Mac et Windows).
5. Distribution : installation sous macOS, signature ad hoc des dylibs (obligatoire en arm64),
   compilation et tests sur une machine macOS de la CI, journal et réglages hors de `%LOCALAPPDATA%`.

Reste à obtenir d'un joueur sous macOS : la liste des fichiers du `.app`
(`find "Civilization VI.app" -type f`), pour savoir où sont les dylibs livrées, et son
architecture.
