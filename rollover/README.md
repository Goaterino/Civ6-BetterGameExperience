# Mesure du passage de tour

Outils pour mesurer où part le temps entre la fin d'un tour et la reprise de la main au tour suivant.

- `capture.ps1` : capture WPR, à lancer en administrateur, jeu ouvert. Elle prend :
  - le CPU échantillonné à 1 ms avec piles, les changements de contexte, les E/S en option ;
  - les présentations DXGI, une par image (profil `Frames` de `civ6.wprp`) ;
  - à la seconde, la charge du GPU : compteurs « GPU Engine » par processus (`gpu_moteurs.csv`) et
    `nvidia-smi dmon` (`gpu_nvidia.txt` : puissance, utilisation, fréquences) ;
  - une copie des journaux du jeu, de `AppOptions.txt`, `GraphicsOptions.txt` et du
    `rollover_proxy.cfg` installé ;
  - le fichier des zones de l'exe de la session, si `profil_zones=on`
    ([startup_proxy](../startup_proxy/README.md#le-profil-des-zones-de-lexe-profil_zones)).

  `-Duration 60` arrête seul la capture au bout de 60 s, pour un scénario à durée fixe.
- `etl.py` : lecture directe du `.etl` par l'API ETW de Windows. Elle ne décode que les images, les
  threads, les échantillons et les piles, et lit 4 Go en 1 min 40 s, là où tracerpt produirait des
  dizaines de Go de XML.
- `analyze.py` : écrit `rapport.md` dans le dossier de capture.
  - Partie journaux : durées des passages de tour et délai par joueur.
  - Partie trace : temps d'image, GPU, threads, modules, arbre d'appels GameCore, fonctions et
    fichiers `.cpp`. Les noms viennent de [la carte des sources](../docs/gamecore_xp2/README.md).
  - Images et GPU : séparés entre passage de tour et hors passage.
  - Threads : en plus du CPU, les mises sur le processeur, la raison de sortie la plus fréquente
    (cession, attente, verrou), la part du noyau et le point d'entrée, qui regroupe les pools.
  - Zones de l'exe (`zones.py`), si la capture en a :
    - par nom de zone, le temps réel total et propre (sans ses zones filles) ;
    - le CPU des échantillons, attribué à la zone ouverte sur leur thread à cet instant.
  - Fenêtre : par défaut, le dernier passage de tour des journaux ; `--tout` pour la capture entière.
    Une capture sans passage de tour (scénario à durée fixe) est analysée en entier.

```bash
.venv/Scripts/python.exe -I rollover/analyze.py rollover/captures/<dossier>
```

Les captures (`captures/`) ne sont pas versionnées.

## Premier constat : tour 78 → 79 (capture du 2026-10-27)

Partie multijoueur locale avec un humain, 19 cités-États, les cités libres et les barbares, mod BBG.
La sauvegarde est `AutoSave_0078`. Le passage dure 9 s, comme dans la partie d'origine.

**Déroulé.** Les IA jouent l'une après l'autre avant que l'humain reprenne la main. La cité-État 6
prend à elle seule 4 s (12:51:48 → 12:51:52).

**Les threads.**
- **Thread GameCore :** occupé 69 % de la fenêtre, soit 6,9 s de CPU.
  - Dont 73 % dans la DLL de règles, 11 % dans le noyau et 3 % dans `Localization`, appelée depuis
    `Game_Eras`.
  - La base de données ne pèse presque rien (12 ms).
- **Autres threads :** une dizaine de threads de travail tournent à environ 30 % chacun, presque
  entièrement dans l'exécutable. C'est à identifier : attente active d'un pool de tâches, ou travail
  réel ?

**Points chauds du thread GameCore**, en temps inclusif. Les noms de `.cpp` précédés de « ? » sont
déduits des fonctions voisines.

| chaîne | temps | remarque |
|---|---:|---|
| IA diplomatique → `Diplomacy_Manager` → `Player_Diplomacy` `1802a7100` | 2,7 s | une seule action, pendant le tour de la cité-État 6 |
| ↳ cascade `Player_Diplomacy` ⇄ `Player_Influence` ⇄ `Player_EventManager` | | réentrante, sur au moins 4 niveaux |
| ↳ chaque niveau → `Cache_District_EventStream` → `Cache_City_Gold` → `Cache_City_*` → `City_Instance` → `District_Instance` → `Improvement_Yields` | | recalcul complet des rendements des villes |
| `GlobalAi` → `CityBuild` → `Value_Manager` → `Improvement_Yields` | 1,0 s | évaluation des constructions par l'IA |
| `Player_Units` (combat, files de production, gouverneurs, routes commerciales) | 0,8 s | |
| `Game::Processor::BeginTurn` | 0,5 s | |

**Fonction la plus coûteuse.** `180260720` (`Improvement_Yields.cpp`) consomme **1,9 s de temps
propre**, soit 28 % du thread. Elle est atteinte surtout par la cascade diplomatique.

**Piste principale.** Chaque événement diplomatique déclenche, à chaque niveau de la cascade, une
remise à jour immédiate des caches `Cache_*` des villes. Ces caches portent l'état que lit
l'interface, et leur remise à jour recalcule les rendements de toutes les villes concernées. Si ces
remises à jour étaient différées et regroupées, une seule par ville à la fin, la majeure partie des
2,7 s disparaîtrait. Il faudra le confirmer par décompilation : que fait `1802a7100`, et est-ce que
les caches `Cache_*` comptent dans l'état synchronisé ou servent seulement à l'affichage ?

## Après le regroupement : tour 78 → 79 en mode coalesce (capture du 2026-10-27, 16:21)

Même sauvegarde, proxy en mode `coalesce`. Le passage dure 6 s au lieu de 9 s. `analyze.py`
reconnaît maintenant l'original chargé sous le nom `_orig`.

**Le thread GameCore** passe de 6,2 s à 4,0 s dans l'arbre du passage de tour :

| chaîne | avant | après |
|---|---:|---:|
| cascade diplomatique (`Diplomatic` → … → `1802a1f90`) | 2,97 s | 0,77 s |
| `GlobalAi` → `CityBuild` → `Value_Manager` → `Improvement_Yields` | 1,0 s | 0,9 s |
| `Player_Units` (combat, files de production, gouverneurs, routes commerciales) | 0,8 s | 0,9 s |
| `Game::Processor::BeginTurn` | 0,5 s | 0,7 s |

**Prochaine cible : `180260720` (`Improvement_Yields.cpp`).** Elle consomme encore **1,1 s de temps
propre**, soit plus d'un quart du thread. On l'atteint par deux chemins : l'évaluation des
constructions par l'IA (`CityBuild`, 0,37 s) et le calcul des rendements de quartier
(`District_Instance` → `18025f570`, 0,83 s).

Décompilée, elle répond à la question « ce joueur peut-il construire cette amélioration ? » :
1. elle vérifie les prérequis de technologie et de dogme de l'amélioration ;
2. elle parcourt une liste de la base de règles (`base + 0xe40`, des entrées
   (nom d'amélioration en `+0x20`, trait en `+0x28`)) et compare **par `strcmp`** le nom de
   l'amélioration à celui de chaque entrée. Les 62 noms commencent tous par `IMPROVEMENT_`, donc
   chaque comparaison parcourt au moins 12 caractères ;
3. pour chaque entrée trouvée, elle cherche le trait dans les traits de la civilisation et du
   dirigeant du joueur, par deux recherches linéaires. `1802209a0`, qui retrouve la civilisation et le
   dirigeant, pèse encore 0,2 s de temps propre.

L'étape 2 ne dépend que de l'amélioration, et la base de règles ne change pas pendant la partie. On
peut donc la précalculer une fois : une table de l'amélioration vers ses traits remplace les
`strcmp`. La fonction reste une lecture pure. Remplacée par un détour qui rend le même booléen,
elle ne change pas l'état de jeu.

**Ce qui reste hors GameCore.** Un thread de l'exe tourne à 72 % (5 s), et une dizaine de threads de
travail à environ 35 % chacun. La sauvegarde automatique bloque toujours le thread principal environ
500 ms par tour.

## Les attentes du thread GameCore (même capture, changements de contexte)

`etl.py` décode maintenant les changements de contexte (CSwitch) et les réveils (ReadyThread) des
threads du jeu, avec leurs piles. Une pile arrive en deux événements : partie noyau, puis partie
utilisateur. `waits.py` les exploite.

**Ce qu'on voit.** Pendant le passage de tour (16:21:34,4 → 16:21:41,25), le thread GameCore
travaille 4,8 s et passe **2,06 s hors CPU** :
- **1,99 s en attente volontaire** (`Waiting / UserRequest`), en 61 fois, dont 26 de plus de 20 ms et
  une de 375 ms ;
- **il attend dans sa propre boucle** (`exe+0x999dbe`, appelé directement par le point d'entrée du
  thread) : il n'a plus de travail ;
- **c'est presque toujours le thread principal 8724 qui le réveille** (1,96 s sur 2,06), depuis sa
  boucle principale (`exe+0x6ff769` → `0x76c3a` → `0x77d65` → `0x999ce6`), pour lui confier le
  message suivant (`Messages_GameUpdate`).

Le passage de tour fonctionne donc **à l'alternance** : GameCore traite un message, puis attend que
le thread principal ait fini son tour de boucle (traitement des événements de jeu, interface Lua,
rendu) avant de recevoir le suivant.

**Ce que fait le thread principal pendant ces attentes** (1,94 s, occupé à 76 %), par branche de sa
boucle principale :

| branche (sous `exe+0x6ff769`) | temps | contenu |
|---|---:|---|
| `0x76c3a` → `0x7795f` (`0x80880`, `0x8073e`, `0x8066e`…) | ~620 ms | code de l'exe, plus GameCore (93 ms) et Localization : probablement le traitement des événements de jeu pour la vue |
| `0x76bf0` → `0x6e917` | ~300 ms | Lua (`havokscript`) et GameCore : scripts d'interface |
| `0x76c3a` → `0x77c5b` → `0x94e62f` | ~150 ms | rendu (pilote graphique) |
| `0x77cbc`, `0x77be7` | ~150 ms | appels système, attentes |
| Steam (`steam_api64`) | ~70 ms (+ 139 ms en feuille) | rappels Steam |

**Conséquences pour la suite :**
- Sur ce passage de tour, environ 30 % du temps réel ne se joue pas dans GameCore, mais dans le thread
  principal de l'exe. Accélérer GameCore ne réduit pas ces attentes.
- Pour les réduire, il faudrait alléger le travail du thread principal entre deux messages :
  - **réglages du jeu** : mouvements et combats rapides, moins d'animations ;
  - **mods d'interface** et leurs écouteurs Lua ;
  - ou **détourner des fonctions de l'exe**, ce que le proxy peut faire aussi : il est dans le même
    processus. Mais l'exe n'a pas de symboles, et il faudrait d'abord nommer `0x7795f` et ses
    branches.

## Les trois fronts remplacés : tour 78 → 79 (capture du 2026-10-27, 22:33)

Même sauvegarde, proxy en `mode=coalesce`, `portee=message`, `ameliorations=quiet`,
`rendements=fast`. La sauvegarde du tour 80 ne s'écarte de la référence `runs/mesure` que du bruit
habituel (38 587 octets en 22 zones, contre 38 641 entre deux parties non modifiées).

| capture | passage (s) | fenêtre (s) | CPU du processus (s) | thread GameCore, CPU (ms) | arbre `18004afc0` (ms) |
|---|---:|---:|---:|---:|---:|
| `tour79` (jeu non modifié) | 9 | 10 | 54,0 | 6 904 | 6 102 |
| `tour79-coalesce` | 6 | 7 | 39,6 | 4 478 | 4 027 |
| `tour79-quiet` (+ améliorations) | 5 | 6 | 33,9 | 3 217 | 3 121 |
| `tour79-remplacement` (+ diplo, portée message, rendements) | 4 | 5 | 27,8 | 2 424 | 2 174 |

Par chaîne, entre `tour79-quiet` et `tour79-remplacement` :

| chaîne | quiet | remplacement |
|---|---:|---:|
| cascade diplomatique (`18042dc40`) | 680 ms | 247 ms |
| attente des tâches de l'IA (`1809922c0`, sous le détour de `1804a16e0`) | 595 ms | 478 ms |
| dont `CityBuild` → `Value_Manager` | 278 ms | 156 ms |
| `Player_Units` (`180344a30`) | 553 ms | 321 ms |

**Le goulot est maintenant le thread principal de l'exe.** Sur la fenêtre de 5 s, le thread GameCore
passe 2,29 s hors CPU, dont 2,23 s dans sa boucle d'attente (`exe+0x999dbe`), réveillé par le thread
principal (`exe+0x6ff769` → `0x76c3a` → `0x77d65` → `0x999ce6`). La plus longue attente dure 473 ms.
GameCore ne calcule plus que la moitié du passage ; le reste attend le traitement des événements,
l'interface Lua et le rendu côté exe.

## Banc de comparaison : tours 78 → 80, proxy retiré / proxy complet (2026-10-27, 22:55 → 23:06)

Quatre captures WPR (`captures/*-bench-*`), en alternance : sans proxy (`-Uninstall`), puis proxy en
`mode=coalesce`, `portee=message`, `ameliorations=quiet`, `rendements=quiet`, deux fois chacun (sauvegardes
du tour 80 dans `runs/bench-*`). Chaque
capture couvre les deux passages, de `AutoSave_0078` jusqu'à la main au tour 80. `analyze.py` mesure
maintenant chaque passage à la milliseconde, d'après l'activité du thread GameCore (section « Passages
mesurés dans la trace »).

| passage | sans proxy (1 / 2) | avec proxy (1 / 2) | moyenne sans → avec |
|---|---:|---:|---:|
| 78 → 79 | 9 483 / 9 327 ms | 4 245 / 4 586 ms | 9 405 → 4 416 ms (−53 %) |
| 79 → 80 | 5 923 / 6 124 ms | 4 977 / 4 154 ms | 6 024 → 4 566 ms (−24 %) |
| **total** | 15 406 / 15 450 ms | 9 222 / 8 740 ms | **15 428 → 8 981 ms (−42 %)** |
| CPU du thread GameCore | 10 547 / 10 606 ms | 4 275 / 4 193 ms | −60 % |
| CPU du processus | 81,4 / 82,1 s | 48,5 / 44,9 s | −43 % |

Dans la passe 1 avec proxy, le tour 79 a été fini dans la seconde où la main est revenue : la
répartition entre les deux passages y est approximative (à une demi-seconde près), pas le total. La
sauvegarde automatique reste à environ 492 ms par tour dans les deux configurations.

**État de jeu.** Deux passes de même configuration diffèrent de 2 763 octets (sans proxy) et
2 747 (avec). Entre configurations, l'écart est d'environ 38 700 octets, dont 35 971 suivent
l'alternance. Ce n'est pas le proxy : une zone de 56 Kio (avec l'ordre de deux valeurs répété six
fois) bascule d'une passe à l'autre, dans l'ordre chronologique, quelle que soit la configuration. Sur
les passes de l'après-midi, le test par groupes le montre : 35 973 octets suivent la parité des passes
(`count`, `coalesce2` / `coalesce`, `count2`) et 0 le mode (`count`, `count2` / `coalesce`,
`coalesce2`). Cause probable, non vérifiée : `AutoSave_0078` est une sauvegarde de fin du tour 77 que
le jeu réécrit à chaque chargement, si bien que chaque passe part de celle réécrite par la précédente.
Un redémarrage du jeu une fois sur deux ne l'explique pas ici : les quatre passes du banc ont toutes
redémarré le jeu. Copie fixe de départ pour les prochains bancs : `runs/depart/AutoSave_0078.Civ6Save`
(celle écrite par la dernière passe avec proxy), à remettre dans `Saves\Multi\auto` avant chaque passe.

## Banc de référence : charge CPU et GPU sur un tour complet

Premier étage de l'analyse de ce qui s'exécute pendant un tour. Le but est d'avoir une référence
reproductible avant d'aller chercher plus fin (profileur intégré de l'exe, comptage d'appels).

**Sauvegarde de départ.** `runs/depart/AutoSave_0078.Civ6Save` est copiée sous
`Saves\Multi\BGD banc T78.Civ6Save`, hors du dossier `auto` : le jeu ne la réécrit plus au
chargement, et les autres parties ne l'écrasent plus.

**Déroulé**, partie multijoueur locale comme pour les bancs précédents, mêmes réglages graphiques :
1. Charger `BGD banc T78` et attendre d'avoir la main.
2. Passage de tour : `capture.ps1 -Name banc-passage`, finir le tour sans rien jouer, Entrée une fois
   la main revenue au tour 79.
3. Tour du joueur, au repos : sans toucher à la caméra ni à la souris,
   `capture.ps1 -Name banc-repos -Duration 60`.
4. Tour du joueur, en mouvement : `capture.ps1 -Name banc-camera -Duration 60`, en faisant défiler la
   carte au clavier pendant toute la capture.
5. `analyze.py` sur chaque dossier.

**Ce qu'on suit** : durée du passage, CPU du processus, temps d'image (p50, p99) et images par
seconde, temps bloqué dans `Present`, utilisation 3D du jeu et puissance de la carte, CPU par
thread.

### Référence du 2026-11-03 (proxy complet, exe DX12)

Captures `20261103-*-banc-{passage,repos,camera}` (`banc-camera` a été prise avant `banc-repos`, sans
effet sur les mesures). Proxy `actif=on`, `mode=coalesce`, `portee=message`, `ameliorations=quiet`,
`rendements=quiet`. Réglages graphiques : 1920×1200, qualité maximale, MSAA ×8, `VSync 1`, écran à
100 Hz. `MaxJobThreads 12`, `MaxGameCoreThreads 10`. Processeur i5-14600KF, carte RTX 4060 Ti.

| scénario | durée | CPU du processus | cœurs occupés | images/s | p50 / p99 (ms) | 3D du jeu | puissance carte |
|---|---:|---:|---:|---:|---:|---:|---:|
| passage 78 → 79 | 4 594 ms | 23,8 s | 5,2 | 95 | 9,9 / 33,3 | 45 % | 112 W |
| tour du joueur, au repos | 60 s | 247 s | 4,1 | 96 | 9,6 / 33,6 | 47 % | 113 W |
| tour du joueur, défilement | 60 s | 244 s | 4,1 | 96 | 9,9 / 32,6 | 50 % | 117 W |

Le passage dure autant qu'au banc du 2026-10-27 avec proxy (4 245 et 4 586 ms).

**Au repos, le jeu coûte autant qu'en mouvement.** Sans que rien ne bouge, il occupe 4 cœurs et
rend 96 images par seconde à 113 W, à 2,8 GHz. Ce n'est pas le rendu de la scène qui coûte, c'est la
boucle d'image elle-même.

**Le pool de tâches de l'exe représente 70 % du CPU.**
- **Qui.** 11 threads, point d'entrée `exe+0xe52d5d`, plus le thread principal : 12, la valeur de
  `MaxJobThreads`. Ils consomment 178 s sur 247 au repos, 19,7 s sur 29,6 dans la fenêtre du
  passage.
- **Ce qu'ils font.** Presque rien de GameCore : ils cèdent le processeur, 5 000 fois par seconde
  chacun, en tranches de 0,06 ms. 82 % de leurs sorties sont des cessions.
- **La boucle.** `exe+0xe51370` (adresse chaude `0xe515a2`) est la boucle de vol de tâches d'Intel
  TBB (`receive_or_steal_task`). On y reconnaît :
  - le choix d'une victime au hasard (`× 0x9E3779B1`, le `FastRandom` de TBB) ;
  - une pause prolongée d'environ 1 000 cycles mesurée par `rdtsc` ;
  - `SwitchToThread` ;
  - après 100 cessions, le test « plus de travail » (`exe+0xe4d9c0`), qui seul laisse le thread
    s'endormir.
- **Pourquoi ils ne dorment jamais.** À 96 images par seconde, chaque image confie de nouvelles
  tâches au pool avant que les threads aient fini leurs 100 cessions. Le pool reste éveillé en
  permanence, même quand rien ne bouge.

**Les présentations viennent du pool.** Les `Present` sont appelés par les threads du pool, environ
toutes les 10 ms : c'est la synchro verticale d'un écran à 100 Hz. Le jeu ne rend pas d'images en
trop, il en manque quelques-unes (96 images/s en moyenne, p95 à 22-24 ms). Le temps bloqué dans
`Present` (2-4 ms en moyenne) est l'attente du prochain rafraîchissement. Une première lecture
supposait un écran à 75 Hz, d'après `RefreshRateInHz 75` (ignoré en mode fenêtré) et la fréquence que
renvoie WMI (`Win32_VideoController`, qui ne décrit qu'un écran) : c'était faux.

**Les deux autres gros threads :**
- **Thread principal** (`exe+0xea3ba7`), 0,6 cœur au repos : 83 % de ses sorties sont des attentes
  sur un verrou ou une variable de condition, 20 000 par seconde. 10 % de ses échantillons sont dans
  `SteamAPI_RunCallbacks` (`exe+0x805c20`).
- **Thread `exe+0x863770`**, 0,5 cœur, 84 % dans le noyau : 26 000 réveils par seconde. C'est une
  boucle autour de l'API Steam (`SteamInternal_ContextInit`), probablement la boucle réseau du
  multijoueur. Elle attend par un appel virtuel qui reste à identifier.

**Pistes, de la moins chère à la plus chère :**
1. **Plafond d'images**, sans rien coder : refaire `banc-repos`. Si le pool s'endort entre deux
   images, le CPU et la puissance de la carte devraient baisser ensemble. Si ça se confirme, le proxy
   pourrait plafonner lui-même, et plus bas pendant le tour des IA.
   - Le limiteur du jeu est `TickIntervalInMS` (`AppOptions.txt`, section `[Performance]`), en
     millisecondes entières. L'écran d'options ne propose que 49, 32 et 16 ms (20, 30, 60 images/s ;
     `Options.lua`, `tickInterval_options`), mais le fichier accepte toute valeur.
   - Dans la boucle principale (`exe+0x6ff6f1`, dans `exe+0x6ff330`), si l'image a duré moins que
     l'intervalle, en millisecondes tronquées, le jeu fait `Sleep` du reste. Le jeu appelle
     `timeBeginPeriod(1)` au démarrage (`exe+0xc731b0`) : ce `Sleep` est précis à la milliseconde.
     Une image dure donc l'intervalle plus 0,5 à 1,5 ms.
   - Pour 75 images/s (13,3 ms), 12 ms donne de 74 à 80 images/s ; 13 ms descendrait sous 75.
   - Sur l'écran à 100 Hz, plafonner coûte de la fluidité : on échange des images contre du CPU et du
     GPU. Le banc mesure ce que chaque palier rapporte (75, puis 60).
   - En plein écran fenêtré, le jeu suit la fréquence du bureau Windows, pas `RefreshRateInHz` (le
     fichier le dit : « Ignored for windowed mode »).
   - **Mesuré à 75 FPS** (12 ms, capture `20261103-183034-banc-repos-75fps`, même scénario que
     `banc-repos`, 60 s caméra immobile) :

     | | sans limite | 75 FPS | écart |
     |---|---:|---:|---:|
     | images/s (moyenne) | 96 | 71 | −26 % |
     | temps d'image p50 / p95 (ms) | 9,6 / 24,0 | 13,4 / 25,3 | |
     | CPU du processus | 247 s (4,1 cœurs) | 180 s (3,0 cœurs) | −27 % |
     | pool TBB (11 threads) | 178 s | 125 s | −30 % |
     | thread principal | 36,8 s | 23,8 s | −35 % |
     | boucle Steam `exe+0x863770` | 29,8 s | 28,7 s | = |
     | puissance de la carte | 113 W | 91 W | −20 % |
     | 3D du jeu | 47 % | 38 % | |

   - **Le CPU suit le nombre d'images.** Le pool coûte environ 30 ms de CPU par image dans les deux
     cas (178 s / 5 731 images, 125 s / 4 281), soit 2,6 ms par thread et par image. Il dort donc
     entre deux images, mais chaque image le réveille pour une rafale de cessions (84-86 % de ses
     sorties). Le coût est par image, pas par seconde : raccourcir cette rafale (piste 3) gagnerait
     sur le CPU sans perdre une image.
   - La médiane tombe à 13,4 ms (75 images/s), mais les accrocs, déjà présents sans limite, ramènent
     la moyenne à 71. `Present` ne bloque plus (0,2 ms) : la carte n'est plus le facteur limitant.
   - **Où part le CPU du pool** (`banc-repos`, 11 threads, 238 s d'échantillons sur la capture
     entière ; première adresse de l'exe en partant de la feuille) :

     | étape | CPU | part |
     |---|---:|---:|
     | tâches du jeu (rendu `exe+0x95xxxx`, pilote `nvwgf2umx`, `d3d12core`…) | ~104 s | ~44 % |
     | pause active de TBB (`0xe515a0`, avec ou sans pile) | 61 s | 26 % |
     | `SwitchToThread`, et piles réduites au noyau | 28 s | 12 % |
     | piles absentes hors de la pause (noyau, `ntdll`) | ~8 s | 3 % |
     | reste de TBB (vol de tâche, boucle du thread, sommeil) | 7 s | 3 % |

     Près de 40 % du pool est de l'attente active, et la pause de 1 000 cycles en fait les deux tiers :
     chaque thread la répète `2 × (n − 1) + 2` fois (24 avec 12 threads) avant sa première cession.
     Le seuil grandit avec le nombre de threads, si bien que réduire `MaxJobThreads` réduit à la fois le
     nombre de threads qui attendent et la durée de l'attente de chacun.
2. **`MaxJobThreads` plus bas** (6, puis 4) : refaire les trois scénarios. Moins de threads qui
   tournent à vide, au risque d'allonger le passage si le pool travaille vraiment par moments.
   - **Le risque, mesuré avant l'essai.** Nombre de threads du pool qui exécutent de vraies tâches
     (hors attente de TBB) à chaque milliseconde, avec 11 threads :

     | | aucun | 1 à 5 | 6 ou plus | 8 ou plus | 10 ou plus |
     |---|---:|---:|---:|---:|---:|
     | au repos | 62 % | 27 % | 11 % | 4,7 % | 1,0 % |
     | passage de tour | 53 % | 37 % | 10 % | 3,2 % | 0,5 % |

     Ce qui dépasse 5 threads pendant les pics fait environ 2,7 ms de travail par image au repos,
     soit 0,5 ms de retard au plus sur 5 threads. C'est moins que les 2 à 4 ms que le jeu passe déjà
     à attendre le rafraîchissement dans `Present`.
   - **Mesuré à 6 threads** (`20261103-19*-6threads`, même protocole, `TickIntervalInMS 0`) :

     | | 12 threads | 6 threads | écart |
     |---|---:|---:|---:|
     | **au repos (60 s)** | | | |
     | CPU du processus | 247 s (4,1 cœurs) | 168 s (2,8 cœurs) | −32 % |
     | pool TBB | 178 s (11 threads) | 97 s (5 threads) | −46 % |
     | thread principal | 36,8 s | 37,5 s | = |
     | images/s, p50 / p95 (ms) | 96, 9,6 / 24,0 | 94, 9,9 / 21,2 | |
     | images > 33 ms | 65 | 31 | |
     | puissance de la carte | 113 W | 114 W | = |
     | **défilement (60 s)** | | | |
     | CPU du processus | 244 s | 169 s | −31 % |
     | images/s, p95 (ms) | 96, 22,2 | 92, 21,4 | |
     | puissance de la carte | 117 W | 122 W | |
     | **passage 78 → 79** | | | |
     | durée | 4 594 ms | 4 890 ms | +6 % |
     | CPU du processus pendant le passage | 23,8 s | 19,8 s | −17 % |
     | CPU du thread GameCore | 2 218 ms | 2 181 ms | = |

   - **Aucune différence ressentie** à l'écran. Le jeu perd 2 à 4 images/s en moyenne, mais il a
     moins d'accrocs (p95 de 24 à 21 ms au repos, deux fois moins d'images au-delà de 33 ms).
   - **Où est passé le gain.** Sur la capture entière (81 s), l'attente active du pool tombe d'environ
     96 s à 34 s (pause 61 → 24 s, `SwitchToThread` et noyau 28 → 7 s). Ses vraies tâches baissent
     aussi, de 98 à 70 s : à travail égal, elles coûtent moins cher, peut-être parce que moins de
     threads se disputent les mêmes données, ou parce qu'elles restent sur les cœurs P. Non vérifié.
   - **Le passage** prend 0,3 s de plus. C'est dans la dispersion des bancs précédents (4 245 à
     4 586 ms avec proxy), mais une seule mesure de chaque côté ne suffit pas à trancher.
   - Avec 5 threads, le pool n'a jamais plus de 5 threads au travail en même temps, et il reste vide
     60 % du temps, comme avant.
   - **Mesuré à 4 threads** (`20261103-192*-4threads`) : trop peu.

     | | 12 threads | 6 threads | 4 threads |
     |---|---:|---:|---:|
     | **au repos** : CPU du processus | 4,1 cœurs | 2,8 cœurs | 2,2 cœurs |
     | images/s, p95 (ms) | 96, 24,0 | 94, 21,2 | 86, 28,5 |
     | images > 33 ms | 65 | 31 | 129 |
     | bloqué dans `Present` (moyenne) | 2,2 ms | 1,8 ms | 1,2 ms |
     | **défilement** : CPU du processus | 4,1 cœurs | 2,8 cœurs | 2,2 cœurs |
     | images/s, p95 (ms) | 96, 22,2 | 92, 21,4 | 74, 32,1 |
     | images > 33 ms | 52 | 33 | 190 |
     | bloqué dans `Present` (moyenne) | 2,9 ms | 1,4 ms | 0,4 ms |
     | **passage 78 → 79** : durée | 4 594 ms | 4 890 ms | 4 711 ms |

     En défilement, `Present` ne bloque presque plus : le jeu n'attend plus l'écran, il attend ses
     3 threads de travail. Les images deviennent irrégulières et la moyenne tombe à 74 images/s. La
     puissance de la carte baisse (97-105 W) seulement parce qu'elle rend moins d'images. Le passage
     ne bouge pas : ses trois durées tiennent dans la dispersion habituelle. **6 threads est le bon
     réglage pour cette machine** (6 cœurs P) : presque tout le gain de CPU, sans perte d'images.
3. **Raccourcir l'attente de TBB** depuis le proxy : moins de cessions avant le test « plus de
   travail », ou une vraie attente à la place de `SwitchToThread`. C'est un détour dans l'exe, à
   régler comme les autres fonctionnalités.
4. **Boucle Steam** `exe+0x863770` : identifier son attente et sa fréquence de réveil.
   - **Ce qu'elle est** (`banc-repos-6threads`). Un thread générique de l'exe (`exe+0x999d10`)
     appelle en boucle `exe+0x863770`, puis attend `WaitForSingleObject(…, délai)` (`+0xac` de
     l'objet thread). La fonction interroge Steam par l'ancienne API P2P : `IsP2PPacketAvailable`
     (`vtable + 8`), puis `ReadP2PPacket` (`vtable + 0x10`, tampon de 0x800 octets) s'il y a un paquet.
     Le délai est vraisemblablement nul : la boucle tourne environ 26 000 fois par seconde.
   - **Ce que coûte chaque tour.** Chaque appel est un aller-retour avec `steam.exe` : le thread du
     jeu attend dans `tier0_s64` (`WaitForSingleObject`) et c'est un thread de `steam.exe` qui le
     réveille, 1,58 million de fois en 81 s. Ce thread de `steam.exe` consomme 0,54 cœur.
   - **L'effet sur le thread principal.** `SteamAPI_RunCallbacks` (`exe+0x805c20`, 10 % du thread
     principal) passe 98 % de son temps dans une section critique de `steamclient64`
     (`steamclient64+0x8f1b58` → `RtlEnterCriticalSection` → rotation à `ntdll+0x1bcaa`, puis
     `NtWaitForAlertByThreadId`). Le thread principal a été réveillé 1 087 089 fois en 60 s, dont 92 %
     au sortir de ce verrou, presque tous par le thread de la boucle P2P. Les deux threads se
     disputent le verrou de `steamclient` 16 700 fois par seconde.
   - **Coût total**, au repos, partie multijoueur : environ 0,5 cœur dans la boucle du jeu,
     0,54 cœur dans `steam.exe`, et 0,06 cœur d'attente du thread principal, soit plus d'un cœur.
   - **Décision (2026-11-03) : on n'y touche pas pour l'instant**, la boucle réseau du multijoueur
     étant trop sensible. Si on y revient, la piste la moins risquée est de n'attendre 1 ms qu'après
     un appel qui n'a trouvé aucun paquet. Une attente fixe après chaque paquet ralentirait les
     transferts en rafale, par exemple l'envoi de la sauvegarde à un joueur qui rejoint.


## Étage 1 : les zones du profileur de l'exe (2026-11-03)

Captures `20261103-201540-zones-passage` et `20261103-201630-zones-repos` : `profil_zones=on`
([startup_proxy](../startup_proxy/README.md#le-profil-des-zones-de-lexe-profil_zones)),
`MaxJobThreads 6`, limiteur désactivé, même partie multijoueur locale. 586 070 zones de 92 noms en
60 s au repos ; 66 991 de 304 noms dans les 5 s du passage. Aucune zone perdue.

**Au repos, le réseau prend les trois quarts de l'image du thread principal.** 5 744 images en
60 s, 7,8 ms en moyenne par `Civ6App_UpdateFrame` :

| zone (thread principal) | par image | max | temps réel en 60 s | CPU en 60 s |
|---|---:|---:|---:|---:|
| `Civ6App_UpdateNetwork` | 3,53 ms | 44,9 ms | 20,3 s | 8,2 s |
| `Civ6App_SteamAPI_RunCallbacks` | 2,54 ms | 40,9 ms | 14,6 s | 5,8 s |
| `WorldView_Render` (rendu, zones filles comprises) | 2,17 ms | 7,1 ms | 12,5 s | 12,2 s |
| dont `WorldView_Render_Models` | 1,13 ms | | 6,5 s | 6,3 s |
| dont `WorldView_Render_VFX` | 0,55 ms | | 3,2 s | 3,1 s |
| `Civ6App_UpdateFrame`, hors zones filles | 1,41 ms | | 8,1 s | 8,1 s |
| `UIManager_Update` (interface, sur le pool) | 0,15 ms | | 0,9 s | 0,8 s |

- Les deux zones du réseau passent plus de temps à attendre qu'à calculer : 35 s de temps réel pour
  14 s de CPU. Ce sont elles qui attendent le verrou de `steamclient`, que la boucle P2P
  (`exe+0x863770`) prend 26 000 fois par seconde. Leurs pointes, 40 à 45 ms, font les accrocs de
  l'image.
- Le rendu de la carte ne coûte que 2,2 ms par image. L'interface Lua est négligeable au repos.

**Pendant le passage (78 → 79, 4 566 ms)**, 447 images en 5 s, 7,9 ms en moyenne :
`SteamAPI_RunCallbacks` 2,3 ms par image, `UpdateNetwork` 2,0 ms une image sur deux, rendu 2,6 ms,
reste de l'image 2,4 ms. Les contextes Lua (`DiplomacyRibbon`, `CityBannerManager`, `TopPanel`,
`WorldTracker`) font moins de 0,2 s en tout. La sauvegarde automatique (`DoGameStateSaveRequest`,
460 ms) tourne sur le thread GameCore. GameCore attend le thread principal entre deux messages : les
~3,3 ms de réseau par image ralentissent donc aussi le passage.

**Conséquence.** La boucle P2P et sa dispute du verrou de `steamclient` sont la plus grosse dépense
qui reste, au repos comme pendant le passage, en partie multijoueur :
- plus d'un cœur au total, dont 0,54 dans `steam.exe` ;
- environ 6 ms sur les 7,8 ms de chaque image du thread principal ;
- les accrocs de 40 ms.

Reste à mesurer une partie solo pour comparer.

### Collecte en vraie partie multijoueur (2026-11-03)

Le banc tourne en multijoueur hébergé seul. Une vraie partie avec d'autres joueurs dira :
- si des pairs réels changent le coût de la boucle P2P et du verrou de `steamclient` ;
- à quelle cadence arrivent réellement les paquets.

C'est ce qui décidera du réglage d'une boucle moins pressante. Le jeu interroge Steam 26 000 fois
par seconde, mais on ignore combien de paquets il y a vraiment à lire.

`civ6.wprp` trace maintenant aussi `Microsoft-Windows-Kernel-Network`. Le rapport y gagne une
section « Réseau » : paquets UDP et TCP reçus et envoyés par seconde, débit et écart entre deux
paquets reçus, pour le jeu, `steam.exe` et les autres processus bavards. Seuls le pid et la taille
sont décodés, pas les adresses.

**Pendant la partie**
- `profil_zones=on` tout du long. Le fichier de zones couvre toute la session, environ 5 Mo par
  minute, sans droits admin.
- Une seule capture légère pour toute la partie, dans un PowerShell administrateur :
  `capture.ps1 -Name mp -Light`, Entrée à la fin avant de quitter le jeu. Les captures normales
  échantillonnent le CPU avec les piles toutes les millisecondes, ce qui fait ramer le jeu en
  partie. `-Light` ne garde que le profil Frames : images, trafic réseau, GPU et zones. Le rapport
  couvre toute la capture, avec les trois derniers passages de tour détaillés.
- Noter l'heure d'un accroc ressenti, pour le retrouver dans les zones.

**Après, avant de relancer le jeu :** `netcode\collect_logs.ps1`. Le jeu réécrit
`net_message_debug.log` et `net_connection_debug.log` à chaque démarrage. Le fichier
`%LOCALAPPDATA%\Civ6RolloverProxy\zones\zones-<pid>-*.bin` de la session, lui, reste.

### Résultats de la partie du 2026-11-03 (20:43 → 00:17)

Capture `captures/20261103-204357-mp`. Quatre parties à quatre humains (l'hôte est un autre
joueur), dont la dernière va du tour 1 au tour 95 : 213 min, 111 passages de tour.
- Proxy de démarrage avec `profil_zones=on`.
- Correctifs POL coupés, car l'hôte n'avait pas le proxy.
- `MaxJobThreads 6`.

Le fichier de zones fait 3,5 Go, soit 143 millions de zones dont aucune perdue. `zones.read` ne
peut pas le charger : `zones_long.py` le dépouille en flux, en moins de 2 minutes, dans
`zones_long.pkl`, qu'`analyze.py` relit pour une capture légère.

**Le réseau ne coûte presque rien en vraie partie.**

| par image du thread principal | banc (hébergé seul, T78) | vraie partie, hors passage |
|---|---:|---:|
| `Civ6App_UpdateNetwork` | 3,53 ms | 0,36 ms |
| `Civ6App_SteamAPI_RunCallbacks` | 2,54 ms | 0,36 ms |
| `WorldView_Render` | 2,17 ms | 1,93 ms |
| image entière (`Civ6App_UpdateFrame`) | 7,8 ms | 8,8 ms |

La dispute du verrou de `steamclient`, qui prenait les trois quarts de l'image au banc, est donc
propre au banc : une partie multijoueur hébergée seul, sans pair. En vraie partie, le réseau
monte de 0,25 à 0,9 ms par image au fil de la partie, sans plus. Une boucle P2P moins pressante
gagnerait peu sur le thread principal. Il resterait le CPU du thread P2P lui-même et celui de
`steam.exe`, que la capture légère ne mesure pas.

**Les paquets.** Le fournisseur `Microsoft-Windows-Kernel-Network` n'a rien émis dans la trace
légère, alors que ses mots-clés (0x10, 0x20) sont bons d'après `logman query providers`. Il faudra
l'essayer en admin avant de s'y fier. À défaut, `net_message_debug.log` compte les messages reçus
des autres joueurs :
- 10 729 messages en 213 min, soit 0,84 par seconde ;
- au plus 29 en une seconde ;
- aucun dans 55 % des secondes.

Cela contre 26 000 interrogations de Steam par seconde mesurées au banc.

**Fluidité.** 99 images par seconde, avec la synchro verticale à 100 Hz :
- 10,0 ms en médiane, 15,4 ms au 99ᵉ centile hors passage ;
- 676 images de plus de 33 ms en 3 h 30.

Les 81 images de plus de 50 ms se répartissent ainsi :
- 48 dans `GameViewState_Tick` (125 ms en moyenne) ;
- 15 dans `Graphics_SubmissionIncFrame` ;
- les autres au chargement de l'interface (`UIUpdateJob_Update`, `UILoadLock`, jusqu'à 1,3 s),
  en début de partie.

Le GPU tourne à 43 % (3D) en moyenne, pour 106 W.

**Le coût du thread principal** est d'environ 8,8 ms sur 10 ms par image :
- `GameViewState_Tick` 2,15 ms ;
- rendu de la carte 1,93 ms ;
- `Graphics_SubmissionIncFrame` 1,81 ms ;
- interface 0,6 ms ;
- réseau 0,7 ms.

C'est là, plus que dans le réseau, qu'il reste à gagner. Une part de ces 8,8 ms est sans doute de
l'attente de l'image précédente, cadencée par la synchro verticale. Ce serait le cas de
`Graphics_SubmissionIncFrame` et de son max de 155 ms. Seule une capture CPU, courte et hors
partie, ferait la part du calcul et de l'attente.
