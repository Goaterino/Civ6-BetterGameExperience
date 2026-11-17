# Portage macOS

État au 2026-11-11 : binaires du jeu inspectés, essai de faisabilité passé sur les machines macOS
de la CI, où SIP est désactivé : il reste à le refaire sur un Mac ordinaire. Aucun code du proxy
n'est porté.

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
| `VirtualProtect` puis saut de 14 octets | `mach_vm_protect` avec `VM_PROT_COPY` puis écriture : passe en CI, **à confirmer SIP actif**, voir l'essai |

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

## Résultats en CI (2026-11-11)

Passage [43027744636](https://github.com/Goaterino/Civ6-BetterGameExperience/actions/runs/43027744636),
macOS 26.7.2 sur Apple Silicon (arm64 et x86_64 sous Rosetta) et macOS 15 sur Intel. Variante `jeu` :

| Essai | arm64 | x86_64 (Rosetta) | x86_64 (Intel) |
|---|---|---|---|
| injection par `DYLD_INSERT_LIBRARIES` | OK | OK | OK |
| mémoire exécutable, `MAP_JIT` et page RW passée RX | OK | OK | OK |
| accroche de l'import `dlopen`, appelée pour `GameCore_XP2.dll` | OK | OK | OK |
| exe : `mprotect` RW, ou RWX | refusé | refusé | refusé |
| exe : `mach_vm_protect` avec `VM_PROT_COPY` | OK | OK | OK |
| exe : copie posée par `mach_vm_remap` | OK | écrit, sans effet | OK |
| bundle : `mprotect` RW puis RX | OK | OK | OK |
| bundle : `mprotect` RWX | refusé | OK | OK |
| bundle : `mach_vm_protect` avec `VM_PROT_COPY` | OK | OK | OK |
| signature du processus après les patchs | valide | valide | valide |
| bout en bout, fonctions de l'exe et du bundle détournées | oui | oui | oui |

**Réserve : SIP est désactivé sur ces machines** (`csrutil status`). La variante `strict`, sans
aucun droit, y accepte l'injection, alors qu'un Mac ordinaire la refuse : le témoin montre que la
CI n'applique pas les règles de signature d'un Mac de joueur. Ces résultats disent que le
mécanisme est juste, pas que le hardened runtime le laisse passer. Les quatre autres variantes
donnent d'ailleurs les mêmes verdicts que `jeu`.

Ce qui tient quand même :

- **`mach_vm_protect` avec `VM_PROT_COPY` est la méthode à retenir**, la seule à passer partout,
  pour l'exe comme pour le bundle. `mprotect` seul est refusé sur le code de l'exe même sans
  hardened runtime (variante `nu`) : c'est la protection maximale du segment, pas la signature.
- **Sous Rosetta, la copie posée par `mach_vm_remap` est écrite mais le code exécuté ne change
  pas** : Rosetta exécute sa traduction de l'ancien code. Méthode à écarter.
- **L'exécutable n'est pas toujours l'image 0 de dyld.** En arm64 sous macOS 26, l'image 0 est la
  dylib insérée ; l'exe se reconnaît à son type `MH_EXECUTE`. Le chargeur devra faire de même.
- En arm64, le bundle refuse une page RWX : écrire en RW, puis repasser en RX.

Reste à lancer l'essai sur un Mac ordinaire, SIP actif (la commande ci-dessus, sans le jeu, avec
les outils en ligne de commande de Xcode). La variante `strict` doit alors refuser l'injection, et
la variante `jeu` tranche.

## Résultats sur un Mac ordinaire (2026-11-16)

MacBook Apple Silicon, macOS 26.6.2, **SIP actif**. Cette fois la variante `strict` refuse
l'injection (« different Team IDs ») : le système applique les règles de signature d'un Mac de
joueur, et la variante `jeu` tranche. Elle démarre avec les drapeaux `0x22011301` en arm64
(`VALID HARD KILL ENFORCEMENT RUNTIME`) et `0x22010301` sous Rosetta (sans `ENFORCEMENT`).

| Essai, variante `jeu` | arm64 | x86_64 (Rosetta) |
|---|---|---|
| injection par `DYLD_INSERT_LIBRARIES` | OK | OK |
| `MAP_JIT` | OK | OK |
| page RW passée RX | **processus tué** | OK |
| accroche de l'import `dlopen`, appelée pour `GameCore_XP2.dll` | OK | OK |
| exe : `mprotect` RW, ou RWX | refusé | refusé |
| exe : `mach_vm_protect` avec `VM_PROT_COPY` | **processus tué** | OK |
| exe : copie anonyme posée par `mach_vm_remap` | processus tué | écrit, sans effet |
| exe : **copie `MAP_JIT` posée par `mach_vm_remap`** | **OK** | écrit, sans effet |
| bundle : `mprotect` RW puis RX | processus tué | OK |
| bundle : `mach_vm_protect` avec `VM_PROT_COPY` | processus tué | OK |
| bundle : copie `MAP_JIT` posée par `mach_vm_remap` | **OK** | écrit, sans effet |
| signature du processus après les patchs | valide | valide |
| bout en bout, fonctions de l'exe et du bundle détournées | **oui** | **oui** |

Conclusions, qui remplacent celles tirées de la CI :

- **Le portage des détours est possible**, dans les deux architectures, signature du jeu intacte.
- **arm64 : seule la copie `MAP_JIT` posée par `mach_vm_remap` passe.** On copie la page de code
  dans une zone `MAP_JIT`, on y écrit le patch (`pthread_jit_write_protect_np`), puis on pose la
  copie à l'adresse de la page d'origine. Toute page de code rendue inscriptible autrement tue le
  processus, et une page RW passée RX aussi : les trampolines vont en `MAP_JIT`.
- **x86_64 sous Rosetta : `mach_vm_protect` avec `VM_PROT_COPY`** (la copie posée par
  `mach_vm_remap` y est écrite, mais Rosetta exécute toujours sa traduction de l'ancien code). Un Mac
  Intel reste à mesurer ; la CI y donnait `mach_vm_protect`.
- Le patch choisit donc sa méthode par architecture, et vérifie l'effet (relire l'octet ne suffit
  pas sous Rosetta).

## Plan

1. ~~Refaire l'essai sur un Mac dont SIP est actif~~ : fait, voir plus haut.
2. ~~Chargeur~~ : **fait** (2026-11-16). `libBGE_Chargeur.dylib` ([startup_proxy/src/macos.rs](../../startup_proxy/src/macos.rs))
   accroche l'import `dlopen` de l'exe et ouvre `BGD_GameCore_XP2.dylib` ([proxy/src/macos.rs](../../proxy/src/macos.rs))
   à la place de `GameCore_XP2.dll` ; le proxy charge l'original et relaie ses deux exports par un saut
   (fonctions nues : registres et pile intacts). Repli sur l'original si le proxy manque ou échoue.
   Lecture Mach-O commune dans [proxy/src/macho.rs](../../proxy/src/macho.rs), journal dans
   `~/Library/Application Support/BetterGameExperience/RolloverProxy.log`. Sous Windows, chaque crate
   garde sa racine dans `lib_windows.rs`, inchangée. Construction : `bash macos_dylibs.sh`
   (universelles, signées ad hoc). Vérifié dans le vrai jeu (arm64, Civ6 Aspyr 1.4.6, partie
   Gathering Storm lancée) : chargeur injecté, proxy chargé à la place de GameCore, partie jouable.
   Aucun correctif encore : les étapes 3 et 4 posent les détours, avec la méthode de patch par
   architecture mesurée plus haut.
3. Interface : retrouver `luaL_loadbuffer` et la lecture XML dans l'exe macOS. Tous les correctifs
   Lua, l'onglet d'options et le salon suivent.
   **Accroches posées et vérifiées dans le jeu (arm64, 2026-11-16)** :
   - **`luaL_loadbuffer`** ([startup_proxy/src/macos_lua.rs](../../startup_proxy/src/macos_lua.rs)) :
     HavokScript est lié dans l'exe. Retrouvée depuis la chaîne `=(load)` (`luaB_load`), qui appelle
     `hks_load(L, réglages, lecteur, données, nom)` ; parmi ses cinq appelants, `hksL_loadbuffer(L,
     réglages, buff, taille, nom)` (lecteur `getS`), dont l'enveloppe à quatre arguments, appelée par
     le seul chargeur de scripts du moteur (`LuaSystem`), est `luaL_loadbuffer`. arm64 0x100985898,
     x86_64 0x100a1ed80, trouvées par signature. Le début est remplacé par un saut, l'accroche appelle
     `hksL_loadbuffer` elle-même. Dans le jeu : les 225 scripts de l'interface y passent
     (`C:\Emu\AppAssets\Base\Assets\UI\Options.lua` : le portage d'Aspyr émule les chemins Windows).
   - **Lecture XML** ([startup_proxy/src/macos_xml.rs](../../startup_proxy/src/macos_xml.rs)) : pas de
     `fopen` (le jeu ouvre par descripteur, puis `fdopen`). Retrouvée par une pile d'appels prise à
     l'ouverture d'`Options.xml` (accroches de diagnostic sur `read`, `mmap`, `fdopen`, variable
     `BGE_DIAG_FICHIERS`) : `load_file(doc, chemin wchar_t, options)` en 0x1005c57b8 lit le fichier dans
     le tampon du document (`doc + 8`) puis appelle `parse(doc, texte, options)` en 0x1005c5920. Les
     deux sont accrochées par trampoline (prologue de quatre instructions de pile) ; le texte est
     remplacé dans le tampon du document par `Platform::MemoryBuffer::GrowToFit`, exportée. Vérifié :
     `Options.xml` remplacé, analysé, écran d'options intact. x86_64 : signatures à relever.
   - L'exe garde les noms de ses exports (`Platform::OpenFile`, `MemoryBuffer::GrowToFit`…) :
     `objdump` les affiche dans le code désassemblé, une aide pour l'étape 4.
   - **Onglet BetterGameExperience** ([startup_proxy/src/macos_options.rs](../../startup_proxy/src/macos_options.rs)) :
     les textes d'`Options.xml` et d'`Options.lua` sont communs avec Windows
     ([options_texte.rs](../../startup_proxy/src/options_texte.rs)). Les fonctions natives du bloc
     Lua (lire, écrire, réinitialiser `rollover_proxy.cfg`, à côté du journal) sont publiées dans la
     table globale juste avant la compilation d'`Options.lua`, par les exports
     `hks_pushnamedcclosure` et `hks_obj_rawset` (table globale en `L + 0x70`) et `lua_pushlstring`,
     non exportée, trouvée par signature (0x10090f828). Une courte cale Lua leur donne les noms du
     bloc Windows. Vérifié dans le jeu (arm64, 2026-11-17) : onglet affiché, limiteur d'images
     ajouté, `Options.lua` compilé. x86_64 : signature de `lua_pushlstring` à relever.
   - Reste : les blocs Lua des correctifs d'interface et du salon.
4. Correctifs de GameCore : reporter les `Site` par architecture, les correctifs POL d'abord
   (ils conditionnent le multijoueur entre Mac et Windows).
5. Distribution : installation sous macOS, signature ad hoc des dylibs (obligatoire en arm64),
   compilation et tests sur une machine macOS de la CI, journal et réglages hors de `%LOCALAPPDATA%`.
   **Le lanceur est fait** ([updater/README.md](../../updater/README.md#macos)) : il pose les deux
   dylibs dans `~/Library/Application Support/BetterGameExperience/dylib`, sans toucher `Civ6.app`, et
   lance le jeu avec `DYLD_INSERT_LIBRARIES` par l'option de lancement Steam. Le chargeur à écrire
   (étape 2) s'appelle `libBGE_Chargeur.dylib`, trouve le proxy de GameCore dans `BGE_DYLIBS`, et doit
   ne rien faire dans `Civ6_Exe`, le lanceur d'Aspyr, qui reçoit aussi la variable.

Relevé sur un Mac M-series (macOS 26.6, build Steam 15296837, Civ6 Aspyr 1.4.6) : les dylibs du jeu
sont dans `Civ6.app/Contents/MacOS`, pas dans `Frameworks` ; `SteamDefaultPlayExecutable` est
`Contents/MacOS/Civ6_Exe_Child`.

Reste à obtenir d'un joueur sous macOS : le tableau de l'essai lancé sur son Mac, la liste des
fichiers du `.app` (`find "Civilization VI.app" -type f`), pour savoir où sont les dylibs livrées,
et son architecture.
