# Unités superposées en fin de déplacement

**Pas encore corrigé** (2026-11-06) : le détour POL proposé plus bas attend une confirmation en partie.

Analyse statique (2026-11-06) de `GameCore_XP2_FinalRelease.dll` (SHA-256 `324c51e9…c98c`, projet
Ghidra `civ6xp2`, base `0x180000000`). Symptôme : deux unités de même classe (limite d'empilement : une
par case) finissent sur la même case. Selon les joueurs, surtout sur des routes de qualités différentes.

Pas d'implémentation. Correctif proposé : **POL**, détour GameCore, sans réglage (bug pur).

## En bref

- **Vérifié.** Le chemin (A*) ne contrôle l'empilement qu'aux cases où l'unité **finit un tour** et à
  la destination. L'exécution entre dans toute case intermédiaire avec le drapeau « ignorer
  l'empilement », sans vérifier qu'elle pourra en repartir.
- **Vérifié.** Le jeu a un filet : à la fin de `MoveTo`, une unité empilée est renvoyée à sa dernière
  case valide. Mais seulement quand l'opération est **terminée**. Une unité qui s'arrête en chemin
  (elle attend le tour suivant, ou n'a plus de mouvements) reste empilée.
- Donc dès que l'unité s'arrête sur une case que le plan prévoyait de traverser, et que cette case
  est occupée, les deux unités sont superposées. Il faut pour cela que le coût réel d'un pas dépasse
  celui du plan. Les causes certaines et probables de cet écart sont plus bas. Celle qui touche les
  routes n'est **pas démontrée**.
- Correctif : étendre le filet existant aux arrêts en chemin (détour de `MoveTo`, 2 entrées de vtable).
  Il ne dépend pas de la cause exacte de l'écart.

## L'interface n'y est pour rien

`MoveUnitToPlot` / `RequestMoveOperation` (`Base/Assets/UI/Civ6Common.lua:102-160`) envoient
`UnitOperationTypes.MOVE_TO` avec `PARAM_MODIFIERS = ATTACK + MOVE_IGNORE_UNEXPLORED_DESTINATION`.
BBG (`2865001760/ui/replacements/civ6common.lua`, identique sur ce point), BSM et MPH
(`unitflagmanager.lua`, `ATTACK`) ne mettent jamais `IGNORE_STACKING`. Le proxy non plus. BBG ne touche
pas aux coûts des routes (seulement `Routes_XP2.BuildWithUnitChargeCost`, `PrereqTech` du rail).

Valeurs de `UnitOperationMoveModifiers`, lues dans l'enregistrement Lua (`1805acd30`) :
`TERRITORY_NO_ENEMY 2`, `IGNORE_STACKING 4`, `IGNORE_DANGER 8`, `ATTACK 0x10`, `DECLARE_WAR 0x20`,
`MAXIMIZE_EXPLORE 0x40`, `STAY_ON_LAND 0x80`, `ANY_ROUTE 0x200`, `ROUTE_ALLOW_UNEXPLORED 0x400`,
`IGNORE_ZOC 0x800`, `IGNORE_STACKING_AFTER_TURN 0x1000`, `MOVE_IGNORE_UNEXPLORED_DESTINATION 0x4000`.

## Où est le code

| adresse | fichier | rôle |
|---|---|---|
| `180040040` | `Unit_Operations_Manager.cpp` | traite les opérations d'une unité : efface le chemin, note la position de départ dans `résultat+8`, puis boucle (100 pas au plus) |
| `18066d0e0` | `Unit_Operations_MoveTo.cpp` | `MoveTo`, entrée `+0x20` des vtables `180a292e0` et `180a2a298` (emplacements `180a29300`, `180a2a2b8`) : un pas, puis le contrôle d'empilement |
| `1801aa510` | `Formation_Movement.cpp` | valide la destination, obtient le chemin (`1801aa820`) |
| `1801aa820` | idem (bornes) | garde le chemin en cache ; ne le refait que si le prochain nœud était hors de vue au calcul |
| `1801a7510` | `Formation_Movement.cpp` | lance l'A* (`180059bf0`, `Pathfinder_TwoLayer.cpp`) |
| `1801a7150` | idem | modèle de mouvement de l'A* : mouvements restants et numéro de tour de chaque nœud |
| `1801a9f00` | idem | validité d'une arête : **contrôle d'empilement aux fins de tour** |
| `1801a9a40` | idem | données d'un nœud : nombre d'unités empilables (`nœud+0x54`), case visible (`+0x50 & 2`) |
| `1801a82a0` | idem | **exécute un pas** du chemin |
| `1801a8da0` | idem | coût réel d'un pas (unités de combat ; `1801a8f00` pour les autres, sans ZDC) |
| `180377640` | `Rules_Movement.cpp` | règle des routes : coût = max(route de départ, route d'arrivée), sauf rivière sans pont |
| `1803b0370` | `Unit_Placement.cpp` | nombre d'unités qui comptent pour l'empilement sur une case (l'unité comprise) |
| `1803b08e0` | idem | `CanMoveInto(unité, case, drapeaux, …)` |
| `1803b11b0` | idem | place l'unité ; mode `& 1` : contrôle et, si besoin, case de repli (`1803affd0`, `FindAlternateLocation`) |

Champs de la formation (`unité+3000`, soit `+0xbb8`) : `+0xc0` position, `+0xd8` propriétaire, `+0xf0`
mouvements restants (virgule fixe, 1 = `0x100`), `+0x198` « a déjà bougé ce tour » (posé à 1 après chaque
pas par `1801a4a90`, recalculé par `1801a4ba0` : une unité a moins que son maximum), `+0x1e8` le chemin.
Nœud du chemin (16 octets, rangé de la destination `[0]` vers l'unité `[n-1]`) : `+0` mouvements
restants prévus, `+4` tour (1 = ce tour-ci), `+8` drapeaux (`1` : hors de vue au calcul), `+0xc/+0xe` x, y.

Limite d'empilement : `GlobalParameters` en `*(0x180b8aa70)+0x58c`.

## Le plan (A*)

`1801a7150` suit les mouvements le long du chemin. Pour l'arête `de → vers` :

```
mvts = de.mvts ; tour = de.tour
si mvts < 1 : tour++ ; mvts = maximum              // nouveau tour
coût = chaîne Rules_Movement (même fonction que l'exécution)
reste = mvts - coût
si reste < 0 :
    si (l'unité est sur `de` et n'a pas bougé) ou tour vient de changer : coût = mvts, reste = 0
    sinon tour++                                    // l'unité attend sur `de`
vers.mvts = reste ; vers.tour = tour
```

`1801a9f00` remonte la chaîne des parents. Une case est contrôlée si elle est la destination, ou si le
nœud suivant est d'un autre tour (fin de tour). Le test :

```
si (nœud+0x50 & 2)                                  // case visible au moment du calcul
   et nœud+0x54 >= limite                           // déjà occupée
   et pas IGNORE_STACKING
   et (pas IGNORE_STACKING_AFTER_TURN ou tour < 2) : arête refusée
```

Les cases traversées dans le tour ne sont pas contrôlées : c'est voulu, on peut traverser ses unités.
Le nombre `nœud+0x54` est pris au moment du calcul (`1801a9a40` → `1803b0490` → `1803b0370`).

## L'exécution (`1801a82a0`)

```
chemin = 1801aa510(...)                      // en cache depuis le début du traitement
si le nœud suivant est d'un tour > 1 : rendre -1     // attendre le tour suivant (prévu)
coût = 1801a8da0(formation, case actuelle, case suivante, destination, ...)
si formation+0x198 (déjà bougé) et coût > mouvements :
    1803afd30 ; rendre -1                    // attendre : NON prévu par le plan
drapeaux = ATTACK→1, DECLARE_WAR→2, IGNORE_STACKING→8, 0x2000→0x80
si case suivante != destination : drapeaux |= 8      // ignorer l'empilement
1803b20a0(formation, case suivante, 3, drapeaux)     // déplacement réel
mouvements -= coût
```

`180040040` arrête aussi la boucle quand la formation n'a plus de mouvements (`1801a3710`).

L'unité peut donc s'arrêter sur une case **intermédiaire** dans trois cas :

1. le nœud suivant est du tour 2 : c'est une fin de tour du plan, contrôlée ;
2. elle n'a plus de mouvements : fin de tour du plan si les coûts concordent ;
3. le coût du pas suivant dépasse les mouvements restants : **jamais** prévu par le plan.

Les cas 2 et 3 sur une case non contrôlée n'arrivent que si l'exécution dépense plus que le plan.

## Le filet, et son trou (`18066d0e0`)

Fin de `MoveTo`, après le pas :

```
résultat+0x10 : terminé ; +0x11 : attend le tour suivant ; +0x12 : a bougé
résultat+8 : dernière case valide (posée par 180040040, mise à jour ci-dessous)
n = 1803b0370(unité, unités de la case de l'unité)
si n <= limite et CanMoveInto(unité, sa case) : résultat+8 = sa case      // case valide
si résultat+0x10 (terminé) et case non valide : 1803b11b0(unité, résultat+8, 1)
```

Le jeu sait donc qu'une unité peut finir empilée, et la renvoie à sa dernière case valide. Mais
seulement quand l'opération est terminée. Quand elle attend (`+0x11`, cas 1 et 3) ou n'a plus de
mouvements (cas 2), rien n'est fait : l'unité reste empilée jusqu'à son prochain pas, au tour suivant.
Le joueur humain voit alors le blocage de fin de tour `ENDTURN_BLOCKING_STACKED_UNITS`
(`ActionPanel.lua:100`), s'il arrive à son tour.

## Pourquoi l'exécution dépense plus que le plan

Les deux appellent la même chaîne de règles (`1803790d0`, `180379200`, `180379120`, `180379190`,
`180377640`, `180377de0`, `180377d00`, `180378010`, `180377e40`) avec les mêmes cases. Contexte :
`+0x30` plafond (routes, villes), `+0x34` coûts ajoutés (terrain, élément), `+0x2c` plancher (ZDC),
`+0x38` report sur le pas suivant. Coût = max(min(`+0x34`, `+0x30`), `+0x2c`), 1 au moins. Écarts
trouvés :

- **ZDC des ennemis découverts en route (vérifié, sens dangereux).** `180378a70` ne compte que les
  unités ennemies **visibles** (`180358010`). Le plan voit l'état du départ. En avançant, l'unité
  découvre des ennemis ; l'exécution applique la ZDC (coût = tous les mouvements restants) sur une case
  que le plan traversait. L'unité y entre avec le drapeau 8 et s'y arrête à 0 mouvement (cas 2).
  Seulement en guerre.
- **Report `+0x38` (vérifié, sens sans danger).** Le plan l'ajoute au pas suivant (`nœud+0x20`),
  l'exécution l'ignore. Le plan est plus pessimiste : l'unité s'arrête à une fin de tour contrôlée.
- **Unités non combattantes (vérifié, sans danger).** Le plan applique la ZDC, l'exécution non
  (`1801a8f00`). Même conclusion.
- **Unités étrangères non visibles (vérifié, rattrapé).** Le contrôle d'empilement exige une case
  visible au calcul. Une unité alliée, neutre ou de cité-État dans le brouillard n'est pas vue. Mais
  `1801aa820` refait le chemin quand le prochain nœud était hors de vue : en principe rattrapé, pas
  vérifié dans tous les cas.
- **Routes (non démontré).** La règle des routes est la même des deux côtés : max des deux routes,
  pont exigé sur les deux cases (`1803789d0`, `plot+0x47` route, `plot+0x4d & 6 == 4` pont). Les
  qualités mixtes rendent les mouvements fractionnaires (0,75 ; 0,5 ; 0,25) et donnent des chemins
  de coût égal en grand nombre. Deux pistes restent ouvertes, à vérifier à l'exécution :
  - le plan prend ses cases par l'interface de carte de l'A* (`vtable+0x80/+0x88`), l'exécution par
    la carte (`180044d40`, `+0x50/+0xb0`). Si la première rend la vue du joueur (route vue en dernier),
    une route pillée ou retirée dans le brouillard coûte plus cher que prévu ;
  - l'A* réaffecte des parents : si un nœud change de parent sans que ses enfants soient recalculés,
    les tours du chemin final ne correspondent plus aux mouvements.

Le correctif ci-dessous ne dépend d'aucune de ces causes.

## Correctif proposé (POL, sans réglage)

Étendre le filet de `MoveTo` aux arrêts en chemin. Détour de `18066d0e0` par ses deux entrées de vtable
(`180a29300` et `180a2a2b8`, pointeurs en `.rdata`, `VirtualProtect`), comme les autres détours du proxy
qui vérifient d'abord les octets attendus. Esquisse :

```rust
// fn(gestionnaire, unité, requête, résultat) -> u64, convention x64
unsafe extern "system" fn move_to(h: usize, unite: usize, req: usize, res: usize) -> u64 {
    let r = ORIGINAL(h, unite, req, res);
    if !pol::actifs() { return r; }
    let termine = rd::<u8>(res + 0x10) != 0;          // déjà traité par le jeu
    let attend  = rd::<u8>(res + 0x11) != 0;
    let formation = rd::<usize>(unite + 0xbb8);
    let a_sec = formation != 0 && rd::<i32>(formation + 0xf0) <= 0;
    if termine || !(attend || a_sec) { return r; }
    let (vx, vy) = (rd::<i32>(res + 8), rd::<i32>(res + 0xc));   // dernière case valide
    let (x, y) = (rd::<i32>(unite + 0xf8), rd::<i32>(unite + 0xfc));
    if vx <= -9999 || vy <= -9999 || (vx, vy) == (x, y) { return r; }
    // même calcul que la fin de MoveTo
    let carte = thunk_180044d40();
    let couche = f_180067990(unite);
    let unites = f_180056a20(vcall(carte, 0xb8), x, y, couche);
    let limite = rd::<u32>(rd::<usize>(BASE + 0xb8aa70) + 0x58c);
    if f_1803b0370(unite, unites, 0) > limite {
        f_1803b11b0(unite, res + 8, 1, 0);            // retour à la dernière case valide
        log(...);                                      // unité, case, mouvements, routes (+0x47)
    }
    r
}
```

- L'appel `1803b11b0(unité, case, 1, 0)` est celui que le jeu fait déjà quand l'opération se termine
  (`18066d0e0`) ; le mode 1 contrôle la case et prend une case de repli si elle n'est plus libre.
- On ne touche pas aux traversées : une unité qui a encore des mouvements et continue n'est pas
  concernée.
- L'unité recule d'une case (souvent) au lieu de s'empiler. Au tour suivant, `180040040` refait le
  chemin depuis cette case.
- Ça change la simulation : POL, actif seulement si tous les joueurs ont le proxy (`pol::actifs()`).
  Bug pur : pas de clé de réglage.
- Variante plus fine, non retenue : ne pas entrer (dans `1801a82a0`) dans une case occupée si
  l'unité ne pourra pas en repartir. Il faut alors recalculer le pas suivant ; plus intrusif.

## Prochaine étape

1. Implémenter le détour avec un journal à chaque correction : unité, case, mouvements, nœud du plan
   (`chemin[n-1]`, mouvements et tour prévus), route des deux cases (`plot+0x47`). En partie, ce
   journal dira laquelle des causes ci-dessus produit l'écart, et si les routes y sont pour quelque
   chose.
2. Si les routes ressortent : comparer ce que rend `vtable+0x88` de la carte de l'A* avec la carte
   réelle (`1801a7150`, `param_1[0]`), et suivre la mise à jour des enfants dans `Pathfinder_Instance.cpp`
   (`180057ff0`–`180058fe0`).

## Refaire

Décompilation : `analyzeHeadless ~/Tools/ghidra-projects civ6xp2 -process GameCore_XP2_FinalRelease.dll
-noanalysis -readOnly -scriptPath ~/Tools/ghidra-scripts -postScript DumpAddrs.java <sortie> <adresses>`
sur les adresses du tableau (et `CallersOf.java`, `DumpByStrings.java` avec `IGNORE_STACKING_AFTER_TURN`).
Vtables lues directement dans le fichier PE.
