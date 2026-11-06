# Choix des cases travaillées : une 2/1 plutôt qu'une 2/2

Avec le focus nourriture et production, une ville peut faire travailler une case à 2 de nourriture
et 1 de production alors qu'une case 2/2 est libre. Analyse statique de
`GameCore_XP2_FinalRelease.dll` (projet Ghidra `civ6xp2`, SHA-256 `324c51e9…`), correctif dans le
proxy de GameCore ([proxy/src/citoyens.rs](../proxy/src/citoyens.rs)). C'est le premier correctif
**POL** (« proxy-only lobbies ») : il change la simulation, donc il ne s'applique que si tous les
joueurs ont BetterGameExperience à la même version (voir
[le salon multijoueur](../startup_proxy/README.md#le-salon-multijoueur--qui-a-le-proxy-)).

## Le code du jeu

Tout est dans `City_Citizens.cpp` :

| adresse | rôle |
|---|---|
| `180110b30`, `180110ac0` | `SetFavoredYield`, `SetDisfavoredYield` (Lua) : cochent le rendement, puis relancent le placement |
| `180110650` | placement : libère les citoyens non verrouillés (`180110760`), puis les replace (`180110ba0`) |
| `180110ba0` | note chaque emplacement libre et appelle le choix |
| `18010f050` | **le choix** : sac à dos par programmation dynamique |
| `18010fff0` | nombre de citoyens qu'un emplacement peut recevoir (1 pour une case, les sièges de spécialiste d'un district) |

`180110650` est relancé par le focus, la croissance, la file de production, l'annexion d'une case
par la culture, le culture bombing, la construction d'une amélioration.

**La note d'un emplacement** (`180110ba0`) : pour chaque rendement, la valeur de la case
(`18005f6d0`, Rules_Yield `18037ec60`), doublée sauf pour un rendement (l'or, d'après l'ordre des
index), ajoutée au score « favori » si le rendement est favori, au score « autre » sinon. Un
rendement défavorisé ne compte pas. Chaque emplacement donne `(nourriture brute, favori, autre)`,
répété autant de fois qu'il a de places (0x18 octets : case, nourriture, favori, autre, bourrage).

**Le choix** (`18010f050(début, fin, K, T)`) prend au plus `K` emplacements (`K` : citoyens à
placer) et écrit les emplacements choisis au début du tableau. `T` est le déficit de nourriture,
`max(0, consommation − nourriture de la ville)`. Il compare deux combinaisons ainsi :

1. la nouvelle gagne si l'ancienne n'atteint pas `T` et que la nouvelle l'atteint ;
2. sinon, score favori, puis score autre.

## Le bug

Cette comparaison n'est pas un ordre. Une étape peut retomber sous `T` pour un meilleur score
favori, et l'étape suivante « repasse » `T` avec la combinaison qu'elle a sous la main, qui n'est
pas la meilleure. Le résultat dépend de l'ordre des cases autour de la ville.

Exemple : deux citoyens, déficit 5, focus nourriture et production, cases dans l'ordre 2/2, 2/2,
3/2, 1/4, 2/1. Le jeu prend **3/2 + 2/1** ; 3/2 + 2/2 atteint aussi 5 de nourriture et produit
plus.

[simulation.py](simulation.py) réimplémente le choix du jeu : sur 30 000 tirages aléatoires, il fait
moins bien que le choix exact dans 241 cas.

Le bug ne joue que si `T > 0`. `T` est lu dans un cache des rendements de la ville
(`1801332f0`) : on n'a pas établi s'il reflète les cases avant ou après la libération des citoyens,
donc à quelle fréquence le bug se produit.

**Pas les ressources cachées.** La fonction de rendement d'une case (`18037ec60`) ajoute bien les
rendements de la ressource sans vérifier si le joueur la voit. Mais la même fonction sert au
gouverneur, au revenu réel de la ville (`180133780`) et à `Plot:GetYield` en Lua : le gouverneur ne
voit pas d'autre rendement que celui que la ville touche.

## Le correctif

Détour de `18010f050` ([proxy/src/citoyens.rs](../proxy/src/citoyens.rs)). Le choix exact :

- parmi les combinaisons d'au plus `K` emplacements qui atteignent `T`, s'il y en a, le meilleur
  score favori, puis autre ;
- sinon, le meilleur score tout court, comme le jeu dans ce cas ;
- à égalité, le moins d'emplacements, puis le premier trouvé.

Programmation dynamique sur (emplacements pris, nourriture plafonnée à `T`), avec retour arrière.
Elle est validée contre une recherche exhaustive (test `egal_a_l_exhaustif`, 20 000 tirages, et
`simulation.py`). Hors du cadre prévu (nourriture négative, table de plus de 4 millions de cases),
le détour rend la main à l'originale.

**Correctif POL.** Le détour ne l'applique que si `BgdPolActifs`, l'export du proxy de démarrage,
rend 1 : salon où tous les humains ont la même version, ou partie locale (solo, hotseat). Sinon,
l'originale : c'est le cas aussi sans le proxy de démarrage. Le proxy de GameCore lève le bit 16 de
`RolloverProxyStatus` une fois le détour posé ; sans lui, le proxy de démarrage ne s'annonce pas
dans le salon.

**Le journal**, `%LOCALAPPDATA%\Civ6RolloverProxy\RolloverProxy.log` :
- `citoyens : détour posé en 0x18010f050, BetterGameExperience 0.2.0` ;
- `pol : correctifs POL actifs` ou `coupés`, à chaque changement ;
- `citoyens : choix corrigé (n), K citoyens. Jeu : … BGD : …` : les 30 premiers écarts de chaque
  partie avec le choix de l'originale, recalculé sur une copie. Chaque emplacement y est écrit
  `(x,y) nourriture/favori/autre`.

## Tests

```bash
cd proxy && cargo test --release citoyens
```

```bash
python -I citoyens/simulation.py
```

La pose du détour sur la vraie DLL est vérifiée par `proxy/tests/check_hooks.py`.
