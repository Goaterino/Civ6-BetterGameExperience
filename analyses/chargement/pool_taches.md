# Le pool de tâches de l'exe et son attente active (2026-11-17)

Reverse seul, aucun correctif écrit. Les adresses sont celles de Ghidra, projet `civ6exe1105` : c'est la
même empreinte que `CivilizationVI_DX12.exe` installé, et les motifs ont été vérifiés dans les octets
du fichier. Les équivalents DX11 (`CivilizationVI.exe`) viennent d'une recherche de motifs dans le
fichier. Dans cette zone, l'exe DX11 est décalé de −0xe5b0. Les captures du 2026-11-03 ont été prises
avec l'exe d'avant la mise à jour du 5 novembre : le code de TBB y est identique, décalé de +0x5d0.

## En bref

- **Le pool est Intel TBB (version 2017-2018), lié statiquement.** Les chaînes de l'exe le confirment :
  `CivTech\SDKSources\TBB\src\tbb\governor.cpp`, « RML might limit the number of workers… ». Par-dessus,
  Firaxis a écrit `Platform_TBBJobManager.cpp`, exporté sous les noms `EXP_JobManager_*`.
- **Les ouvriers savent déjà dormir sur un objet noyau.** Le serveur RML de TBB les endort sur un
  événement Windows (`WaitForSingleObjectEx(…, INFINITE)`) et les réveille par `SetEvent` quand une
  tâche arrive dans une arène vide.
- **Le gaspillage vient du compte à rebours avant le sommeil.** Un ouvrier sans travail fait
  `2 × limite` pauses d'environ 1 000 cycles, puis **au moins 101 `SwitchToThread`**. Seulement
  ensuite il vérifie que l'arène entière est vide (`is_out_of_work`) et s'endort.
  - Ce compte à rebours dure environ 0,15 ms.
  - Le réveil, lui, prend 6 à 8 µs en médiane.
  - Le pool répète ce cycle 576 fois par seconde et par ouvrier pendant un passage de tour.
- **Un deuxième site tourne à vide, côté soumission.** `EXP_JobManager_Wait` finit par une boucle de
  `SwitchToThread` sans limite, tant que des ouvriers sont encore dans l'arène. Elle prend 1,1 à 1,3 s
  de CPU par chargement.
- **Correctif proposé, en trois étapes :**
  1. Baisser le seuil de 100 cessions : un octet, en `140e5100b` (DX11 `140e42a5b`). L'ouvrier
     emprunte ainsi beaucoup plus tôt le chemin de sommeil que TBB prévoit déjà.
  2. Remplacer la boucle de cessions de `JobManager_Wait` (`140e4d3bd`, DX11 `140e3ee0d`) par
     `WaitOnAddress`, avec un `WakeByAddressAll` posé là où un ouvrier quitte l'arène (`140e4d7f1`,
     DX11 `140e3f241`).
  3. Ensuite seulement, choisir le nombre d'ouvriers selon la phase par la limite souple du marché
     (`market::set_active_num_workers`, `140e519c0`, DX11 `140e43400`).
- **Pendant le chargement, l'attente active coûte peu de CPU.** Elle fait 4 à 6 % du CPU des ouvriers
  dans `StartupGame`. Les cessions, elles, font 51 à 85 % de leurs sorties du processeur. En
  revanche, `ProcessTiles` sature les 5 ouvriers (5,0 à 5,2 threads en tâche en moyenne). C'est donc
  l'étape 3 qui fera gagner du temps au chargement : environ 2 s par chargement, estimation à
  mesurer. Les étapes 1 et 2 la rendent possible sans pénaliser les passages de tour.

## 1. Structure du pool

### Création

| quoi | où (DX12) | détail |
|---|---|---|
| lecture de `MaxJobThreads` | `14006cd00` | `[Performance] MaxJobThreads`, défaut −1 (« including main thread »). −1 donne un nombre de cœurs calculé (`140999860`, CPUID ; cas particulier AMD famille 0x15), moins 1. La valeur est bornée à [2, 32] et rangée en `app+0x26c`, avec `n−1` en `app+0x270`. |
| init du gestionnaire de tâches | `140993a80` (`Platform_TBBJobManager.cpp`) | `task_scheduler_init(min(n, 32))` (`140e4ad60` → `governor` `140e4acc0`), puis une `task_arena` principale de concurrence `n` (`140993370`, `140e4dd20`). Le passage de `app+0x26c` à cet argument n'a pas été suivi jusqu'au bout (appelant `140993b30`). |
| marché (`market::global_market`) | `140e515d0` | `n−1` ouvriers : limite dure en `+0x18`, limite souple en `+0x1c`. La limite souple se change à chaud par `140e519c0` (`set_active_num_workers`, utilisé par `global_control`). |
| lancement des threads | `140e527a0` → `140e52300` (`_beginthreadex`) | Les threads ne naissent qu'au premier réveil (« wake or launch »). Leur routine `140e52740` décale la pile (`alloca` de `(index+1)·0x9E37 & 0xFFFF`), puis appelle `private_worker::run` (`140e524d0`). |

### L'API de Firaxis (exportée, donc aussi utilisée par les DLL)

- `EXP_JobManager_CreateJobList` / `PushJobOntoList` / `SpawnList` / `AddJob` / `Wait` /
  `DestroyJobSet` (`140994190` à `1409941f0`).
- Chaque *JobSet* porte sa propre `task_arena` (objet de 0x28 octets, initialisé à la demande par
  `140e4cd30`).
- **Ajout d'une tâche** (`AddJob` → `140993640` → `140e4ce70`) :
  - si l'emplacement 0 de l'arène (le maître, `arena+0x180`) est libre, le thread qui soumet le
    prend un instant, pousse la tâche dans sa propre pile (`local_spawn` `140e4f950`), puis le
    rend ;
  - sinon la tâche va dans le flux FIFO de l'arène (`140e4c9e0` → `140e4d830`, `arena+0x90`).
- **Signal aux ouvriers.** Dans les deux cas, `advertise_new_work` ne réveille les ouvriers que si
  l'arène passe de vide à pleine. Elle appelle alors `adjust_demand` (`140e51340`), depuis
  `140e4fc98` pour un ajout dans la pile et `140e4c360` pour un ajout dans le flux.

### Boucle d'un ouvrier

Les piles relevées pendant les temps morts se lisent ainsi :
- `e5278d` : la routine du thread ;
- `e52548` : `private_worker::run` appelle `client.process` ;
- `e518ca` : `market::process` (`140e51810`), qui choisit une arène qui manque d'ouvriers ;
- `e4d791` / `e4d7ab` : `arena::process` (`140e4d630`), qui prend un emplacement et boucle sur
  `receive_or_steal_task` (vtable +0x30) et `local_wait_for_all` (vtable +0x28, `140e501b0`) ;
- `e50452` : `local_wait_for_all` appelle `receive_or_steal_task` ;
- `e50fd2` : la pause, dans `receive_or_steal_task` (`140e50da0`) ;
- `e51003` : le retour de `SwitchToThread`.

`receive_or_steal_task` (`140e50da0`), en pseudo-code :

```
n = arena.my_limit[+0x88] - 1 ; pauses = 0 ; cessions = 0
boucle :
  si *ref_count_attendu == 1                       → fin (retour nul)
  si ouvrier au niveau extérieur ([sched+0x30] == 0)
     et arena.alloués[+0x80] < arena.références[+0x84] >> 1  → rappelé par le marché, retour nul
  si n et boîte aux lettres non vide               → tâche « affinité » (140e4f610)
  si niveau extérieur et flux FIFO [+0x90] non vide → tâche du flux (140e50630)
  si la pile a de la marge et n                    → vol : victime = emplacements[(aléa >> 16) % n]
        (aléa : x = x·0x9E3779B1 + c, le FastRandom de TBB ; emplacements de 0x100 octets à +0x180)
  pause prolongée                                  (140e50fa9) : PAUSE ×1,2,4,8,16 puis jusqu'à rdtsc+1000
  si pauses >= 2·(n+1) :                           (140e50ff0, lea eax,[rsi*2+2])
      SwitchToThread()                             (140e50ffd)
      si cessions++ >= 100                         (140e51009, cmp eax,0x64)
         et ouvrier au niveau extérieur et arena.is_out_of_work() → retour nul
      pauses = 2·(n+1)       // désormais chaque échec cède le processeur
  pauses++
```

- **Avant tout sommeil**, l'ouvrier fait donc 2·limite pauses d'environ 1 000 ticks (12 avec
  6 threads), puis 101 tours « pause + `SwitchToThread` + tentative de vol ».
- **Durée mesurée** : 0,10 à 0,18 ms en médiane (voir 2). Cela fait environ 1,4 µs par tour.
- **Les maîtres ne s'endorment jamais ici.** Un thread qui attend ses propres tâches (`[sched+0x30]`
  non nul) tourne jusqu'à ce que son compteur tombe à 1.

`is_out_of_work` (`140e4d3f0`), selon l'état de la réserve `arena+0xb0` :
- **0, vide** : renvoie vrai ;
- **−1, pleine** : l'ouvrier passe l'état à « occupé » par CAS, avec une adresse de sa pile comme
  marque. Il parcourt alors les emplacements (pile non vide) et le flux FIFO :
  - tout est vide et personne n'a ajouté de tâche entre-temps : il passe l'état à 0 et appelle
    `adjust_demand(arena, −max_workers[+0xac])`. Le nombre d'ouvriers alloués tombe à 0, **et tous
    les ouvriers de l'arène la quittent** à leur prochain tour de boucle ;
  - sinon : il remet l'état à « pleine ».

**Sortie de l'arène et sommeil :**
1. `arena::process` décrémente les références de 2 (`lock xadd`, `140e4d804`).
2. `market::process` cherche une autre arène. S'il n'en trouve pas, l'ouvrier revient dans
   `private_worker::run` (`140e524d0`).
3. Si le serveur a trop d'ouvriers éveillés (`slack < 0`, `server+0x20`), l'ouvrier s'inscrit dans
   la liste des dormeurs (verrou tournant `server+0x38`, liste `server+0x30`). Il attend ensuite
   `WaitForSingleObjectEx(événement du thread, INFINITE)`.

**Réveil :**
1. `adjust_demand` (`140e51340`), sous le verrou du marché, appelle
   `server.adjust_job_count_estimate(delta)` (vtable +0x28).
2. Celle-ci appelle `wake_some` (`140e52850`) : autant de dormeurs que la demande en retire de la
   liste.
3. `wake_or_launch` (`140e527a0`) fait `SetEvent` (`140e52842`), ou lance le thread s'il n'existe pas
   encore.
4. **Chaque ouvrier réveillé rappelle `wake_some`** (`140e52604`) : le réveil se propage en cascade.

Dans la capture du passage, la moitié des réveils vient du thread principal (spawn → `e4fc9d` →
`adjust_demand` → `wake_some`). L'autre moitié vient de la cascade entre ouvriers.

### Comment le thread qui soumet attend la fin

`EXP_JobManager_Wait` (`1409941f0`) appelle `task_arena::internal_wait` (`140e4d0a0`) :

```
boucle :
  tant que arena.réserve[+0xb0] != vide :
     si l'emplacement 0 est libre (CAS sur [+0x180]) :
         s'y installer ; tant que réserve != vide : local_wait_for_all(dummy)
         → le thread qui attend exécute lui-même des tâches (le maître participe)
     sinon :
         CreateEvent ; mettre en file une wait_task (vtable 1411311a0, execute = 140e4caa0 :
         is_out_of_work puis SetEvent) ; WaitForSingleObjectEx(INFINITE)   ← attente noyau
  si arena.références[+0x84] < 2 et emplacement 0 libre : fin
  SwitchToThread()                                    (140e4d3bd, retour 140e4d3c3)
```

La dernière boucle attend que tous les ouvriers aient quitté l'arène, donc qu'ils aient **fini leurs
tâches en cours** : la réserve peut être vide alors que des tâches s'exécutent encore. C'est de
l'attente active pure, sans limite. C'est elle que le README voyait comme « 3,4 s dans le noyau
depuis `+e4d3c3` » pendant `Block on asset load`.

Il existe d'autres verrous tournants, avec repli sur `SwitchToThread` au-delà de 16 pauses : verrou
du marché `140e57070`, liste des dormeurs `140e52850`, `global_market` `140e515d0`. Ils sont
négligeables dans les mesures.

## 2. Mesures

Méthode :
- pour le chargement, les échantillons CPU à 1 ms (`extrait.pkl`) ;
- pour les sorties du processeur, les changements de contexte (`trace.pkl`) ;
- **ouvriers** : les threads dont la pile contient `exe+e5278d` ;
- **« à vide »** : le premier cadre hors système est dans TBB (`140e4a000`–`140e58000`), sans code du
  jeu au-dessus. Les échantillons dont la pile ne contient que du système sont comptés comme
  travail : l'estimation est prudente.

Les scripts sont restés dans le dossier temporaire de la session. Ils seront à verser dans
`rollover/` si on poursuit.

### Chargement : captures `20261117-132539-apres-leviers-3` et `20261117-141910-sans-vsync`

`MaxJobThreads 6`, soit 5 ouvriers. La phase `WAIT_FOR_PLAYERS` contient `StartupGame`, donc
`ProcessTiles`.

| | apres-leviers-3 | sans-vsync |
|---|---:|---:|
| CPU des ouvriers à vide dans `WAIT_FOR_PLAYERS` | 4 à 6 % (1,1 à 1,3 s par chargement) | 5 à 6 % (1,3 à 1,7 s) |
| cessions parmi les sorties du processeur | 54 à 85 % | 51 à 79 % |
| sorties par seconde et par ouvrier | 1 100 à 1 900 | 1 700 à 2 400 |
| sommeils RML par seconde et par ouvrier | 72 à 148 (médiane 0,3 à 0,6 ms) | 188 à 254 (médiane 0,3 ms) |
| rafale réveil → sommeil (médiane) | 0,13 à 0,18 ms | 0,10 à 0,12 ms |
| attente active de TBB par chargement, tous threads : `receive_or_steal` | 2,0 s | 3,7 s |
| attente active de TBB par chargement, tous threads : boucle finale de `internal_wait` | 1,3 s | 1,1 s |

- **Toutes phases confondues**, l'attente active est de 12 à 22 % du CPU des ouvriers. Elle passe
  97 % dans les 6 s qui précèdent l'écran, où le pool ne fait presque rien.
- **Cessions nombreuses, CPU faible.** Les cessions dominent les sorties du processeur mais coûtent
  peu de CPU. 75 % d'entre elles (65 % au passage de tour) donnent le processeur à un **autre ouvrier** prêt sur le même
  processeur logique : les ouvriers s'empilent sur quelques cœurs et se renvoient la main.
  `SwitchToThread` ne cède qu'aux threads prêts sur le processeur courant.
- **`ProcessTiles` est saturé.** Le nombre de threads du pool qui exécutent une vraie tâche, par
  milliseconde, vaut en moyenne 5,0 à 5,2 de la seconde 1 à la seconde 5 de la phase, à chacun des
  8 chargements. Le pool n'attend pas pendant le terrain : il manque de threads.
- **Aucun passage de tour** dans ces deux captures. Elles s'arrêtent quand le joueur a la main.

### Passage de tour : captures du 2026-11-03 (exe d'avant le 5 novembre, même TBB)

| `banc-passage` (9 à 10 s, dont environ 4,6 à 4,9 s de passage) | 12 threads (11 ouvriers) | 6 threads (5 ouvriers) |
|---|---:|---:|
| CPU des ouvriers | 28,0 s (3,1 cœurs) | 19,7 s (2,0 cœurs) |
| dont TBB à vide (échantillons) | 9,2 s (33 %) | 3,6 s (18 %) |
| cessions parmi les sorties | 72 % | 63 % |
| sommeils RML par seconde et par ouvrier | 802 | 576 |
| durée d'un sommeil (médiane / p90) | 0,14 / 2,2 ms | 0,15 / 2,4 ms |
| rafale réveil → sommeil (médiane) | 0,16 ms | 0,17 ms |
| latence de réveil, de `ReadyThread` à l'arrivée sur le CPU (médiane / p90 / p99) | 8 / 11 / 80 µs | 6 / 10 / 54 µs |
| coût des réveils pour le thread principal (`adjust_demand` + `wake_some` + `SetEvent`) | ~25 ms | ~30 ms |

**Lecture.** Le pool s'endort et se réveille des centaines de fois par seconde. Chaque cycle brûle
environ 0,15 ms de compte à rebours, soit **20 à 25 fois la latence de réveil**. Le commentaire de
TBB lui-même (repris dans oneTBB) dit que le seuil de 100 cessions est « expérimental », et qu'idéalement
l'attente avant `out_of_work` devrait durer à peu près le temps d'un réveil. Cette attente grandit
avec le nombre de threads, par le seuil de pauses `2·limite` et par le nombre de rafales. C'est ce
que la baisse à 6 threads a réduit.

## 3. Correctif proposé

Une fonctionnalité du proxy de démarrage, réglable :
- une clé `read_config`, `install.ps1` et l'onglet d'options ;
- par exemple `pool_cessions=<n>` (100 = jeu d'origine) et `pool_attente_fin=on|off`.

Les motifs ci-dessous sont **uniques dans les deux exes**, vérifiés dans les fichiers installés.

### Étape 1 : s'endormir plus tôt par le chemin de TBB (un octet)

| | DX12 | DX11 |
|---|---|---|
| début du motif | `140e50ff0` | `140e42a40` |
| octet à changer (imm8 de `cmp eax,0x64`) | `140e5100b` | `140e42a5b` |

```
8D 04 75 02 00 00 00 3B E8 7C ?? 8B E8 FF 15 ?? ?? ?? ?? 41 8B C7 41 FF C7 83 F8 64 7C ?? 4D 85 F6 75 ?? 48 8B 4F 20 E8
```

L'octet `0x64` est au déplacement +0x1b. On le remplace par Y, à mesurer parmi 4, 8 et 16 :
- **Rafale attendue** : 2·(n+1) pauses (environ 4 µs) plus Y tours d'environ 1,4 µs. Avec Y = 8, cela
  fait environ 15 µs au lieu de 150 µs : à peu près deux latences de réveil.
- **Aucun mécanisme nouveau.** Le sommeil (événement RML) et le réveil à l'ajout d'une tâche
  (`advertise_new_work` → `adjust_demand` → `SetEvent`) existent et servent déjà 576 à 800 fois par
  seconde. Ils ont fait leurs preuves.
- **Seuil des pauses, optionnel.** Le `lea eax,[rsi*2+2]` (7 octets, début du motif) peut devenir
  `mov eax, imm32` suivi de deux `nop`. C'est inutile au départ : il ne pèse que quelques µs.

**Risques**
- **Plus de cycles sommeil-réveil.** Chaque cycle coûte un `SetEvent` au thread qui soumet (1 à 2 µs)
  et un changement de contexte. Au passage de tour, aujourd'hui, environ 30 ms en 10 s pour le thread
  principal : même doublé, c'est négligeable.
- **Latence ajoutée aux petites tâches** qui arrivent sur un pool endormi : 6 à 8 µs en médiane,
  50 à 80 µs au p99, une fois par lot.
  - Les lots d'une image sont bornés par `Present` (2 à 4 ms d'attente de la VSync).
  - Les tâches à grain fin de `ProcessTiles` ne sont pas concernées : le pool y est saturé et ne
    s'endort pas.
- **Rappel de toute l'arène d'un coup.** Quand `is_out_of_work` réussit, tous les ouvriers de l'arène
  repartent dormir. La tâche suivante les réveille en cascade. C'est déjà le comportement actuel,
  seulement plus fréquent.
- **Interblocage.** Rien de nouveau : même code, moins d'itérations. Les courses sont gérées par le
  protocole de TBB (état « occupé » par CAS, ré-annonce si une tâche arrive pendant le parcours).
- **Ordre mémoire.** Inchangé : x86 et opérations `lock` de TBB.
- **Déterminisme et multijoueur.** On ne touche qu'à l'ordonnancement. Le résultat des tâches ne doit
  déjà pas en dépendre, puisque le nombre de threads varie d'une machine à l'autre.

### Étape 2 : la fin de `JobManager_Wait` sur un objet noyau

| | DX12 | DX11 |
|---|---|---|
| `call [SwitchToThread]` de la boucle finale (`FF 15`, 6 octets) | `140e4d3bd` | `140e3ee0d` |
| motif (le `FF 15` à +0x1a) | `140e4d3a3` | `140e3edf3` |
| sortie d'un ouvrier (`lock xadd [rdi+0x84], −2`), motif de 27 octets | `140e4d7f1` | `140e3f241` |

```
48 8B 06 8B 80 84 00 00 00 83 F8 02 73 ?? 48 8B 06 4C 39 B0 80 01 00 00 74 ?? FF 15 ?? ?? ?? ?? E9
4C 8B 87 E0 00 00 00 B8 FE FF FF FF 48 8B 8F D8 00 00 00 F0 0F C1 87 84 00 00 00
```

**Côté attente.**
1. Remplacer le `FF 15` par `E8 rel32` vers une page proche, suivi d'un `90`. C'est le style de
   `motifs::rediriger_appel`, adapté à un appel indirect.
2. Dans la page, `48 8B 0E` (`mov rcx,[rsi]`, l'arène : `rsi` est la `task_arena*`), puis un saut
   vers le crochet.
3. Le crochet :

```
attendre_sortie(arena) :
  r = &arena.références (+0x84) ; v = charge(r)
  si v < 2 : retour                       // la boucle du jeu revérifie aussi l'emplacement 0
  épisode courant (thread_local, remis à zéro si le dernier appel date de plus de 100 µs)
  si l'épisode dure depuis moins de ~20 µs : SwitchToThread() ; retour     // comme l'original
  WaitOnAddress(r, &v, 4, 1 ms)           // retour immédiat si la valeur a déjà changé
```

**Côté réveil.**
1. Détourner les 27 octets en `140e4d7f1` vers une caverne qui rejoue les quatre instructions. Elles
   sont indépendantes de leur position : `mov r8,[rdi+0xe0]`, `mov eax,−2`, `mov rcx,[rdi+0xd8]`,
   `lock xadd`.
2. La caverne appelle ensuite `WakeByAddressAll(rdi+0x84)`, en gardant `rax`, `rcx` et `r8`, qui
   servent juste après. Elle respecte l'alignement de pile et l'espace réservé de 0x20 octets.
3. Elle revient en `140e4d80c`.

`WakeByAddressAll` sans attente reste en mode utilisateur et coûte quelques dizaines de ns.

**Risques**
- **Réveil manqué.** Les sorties des maîtres (références −1 dans `140e4cd30` et `140e4d040`) et la
  libération de l'emplacement 0 (`[+0x180] = 0`) ne réveillent personne. Le délai de 1 ms les
  couvre.
- **Pas de réveil perdu** entre la lecture et l'attente : `WaitOnAddress` compare atomiquement.
  Le `lock xadd` précède le `WakeByAddressAll`.
- **Le thread principal attend par ce chemin à chaque image.** La phase de cessions (~20 µs) garde
  les attentes courtes identiques à l'original. Il faudra vérifier les temps d'image.

**Gain attendu** : environ 1,1 à 1,3 s de CPU par chargement sur le thread qui attend les assets. Au
passage de tour, à mesurer.

### Étape 3 : plus d'ouvriers pendant le chargement seulement (à n'envisager qu'après 1 et 2)

TBB distingue :
- la **limite dure**, fixée à la création du marché par `MaxJobThreads` ;
- la **limite souple**, `market::set_active_num_workers` (`140e519c0`, DX11 `140e43400`).

La limite souple se change à chaud, sous les verrous du marché. Le motif de 24 octets du prologue est
unique dans les deux exes :

```
48 89 5C 24 18 57 48 83 EC 20 85 C9 BB 01 00 00 00 8B FB 0F B6 D3 0F 45
```

Le principe :
- `MaxJobThreads 12` (11 threads possibles) ;
- le proxy appelle `set_active_num_workers(5)` juste après l'init du JobManager ;
- puis 11 au début d'un chargement, et de nouveau 5 quand le joueur a la main.

En dehors des chargements, les 6 threads en plus ne sont jamais alloués : ils dorment dans RML.

Différences qui restent avec un vrai `MaxJobThreads 6` :
- `my_limit`, la limite haute des emplacements occupés, reste à 12 après un chargement. Le seuil de
  pauses passe donc de 12 à 24, soit quelques µs de plus. L'étape 1 compense.
- Les victimes de vol sont tirées parmi 11 emplacements, dont des vides vite écartés.

**Estimation du gain.** `ProcessTiles` dure 5,2 à 5,6 s (3,5 s au 4e chargement) avec environ 5,1
threads en tâche en continu, soit environ 27 s de CPU. Ce chiffre est cohérent avec les 24 s de passes
du README.

Avec 11 ouvriers sur 6 cœurs P (12 threads logiques) et 8 cœurs E :
- capacité effective estimée à 8-9 « cœurs P », contre 5 aujourd'hui ;
- facteur 1,6 à 1,8 ;
- `ProcessTiles` vers 3 à 3,5 s, soit **environ 2 s de moins par chargement** ;
- un peu plus si d'autres phases parallèles saturent aussi, comme `SharedDataLoad`, tant que le levier
  `PathFileExistsW` n'est pas fait.

C'est une estimation. Le README de `rollover/` a aussi vu les vraies tâches coûter plus cher à
12 threads qu'à 6 (98 s contre 70 s au repos), sans explication. Il faut mesurer.

## 4. Validation

1. **Tests unitaires** : chaque motif trouvé une seule fois dans `CivilizationVI_DX12.exe` et dans
   `CivilizationVI.exe`, comme `signature_dans_les_exes` de `zones.rs`.
2. **Étape 1, `MaxJobThreads 6`.** Bancs `banc-repos`, `banc-passage` et `banc-camera`, chacun 3 fois,
   réglage coupé, puis Y = 16, 8 et 4. On relève :
   - CPU des ouvriers et part à vide ;
   - sommeils par seconde et durée des rafales ;
   - latence de réveil ;
   - CPU du thread principal dans `adjust_demand` et `wake_some` ;
   - images par seconde et p95 ;
   - durée du passage (au moins 3 mesures, à cause de la dispersion de 4,2 à 4,9 s) ;
   - CPU du thread GameCore.

   Critère : moins de CPU au repos et au passage, sans perte d'images ni allongement du passage.
3. **Étape 1 avec `MaxJobThreads 12`**, contre 6 sans correctif, sur les mêmes bancs. Le CPU au repos
   et la durée du passage doivent rester ceux de la référence à 6. C'est la condition pour l'étape 3.
4. **Étape 2.** Temps CPU en `140e4d3c3` sur une capture de chargement (attendu proche de 0), temps
   d'image du thread principal, durée de `Block on asset load`.
5. **Étape 3.** `ProcessTiles` et `SynthesizeTerrain` dans `SynthesisLog.log`, et durée de
   `WAIT_FOR_PLAYERS`, avant et après, sur 4 chargements. Vérifier aussi qu'un passage de tour juste
   après le chargement retrouve le CPU de la référence à 6 threads.
