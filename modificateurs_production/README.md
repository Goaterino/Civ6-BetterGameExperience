# Modificateurs de production d'unités : dérive au retrait

Gêne : retirer un bonus de production d'unités en pourcentage enlève un peu plus que ce qu'il avait
ajouté. Après un retrait d'Ilkum (+30 % pour les ouvriers), le multiplicateur tombe à 99,61 % au
lieu de revenir à 100 %, et la perte s'accumule à chaque échange de la carte. Signalé et corrigé
pour le joueur par Serianol (PR 8, 2026-11-13), étendu aux villes le 2026-11-14.

## La cause dans GameCore

Projet Ghidra `civ6xp2` (base 0x180000000). Les multiplicateurs sont en virgule fixe 24.8. Le jeu
convertit un pourcentage `p` ainsi : `x = p × 0,01f`, `e = floorf(x)`, puis
`(e << 8) | (trunc((x − e) × 256) & 0xff)`. Pour un positif, c'est `trunc(x × 256)` ; pour un
négatif, `floorf` arrondit vers −∞ : +30 % ajoute 76/256, −30 % retire 77/256. Seuls les multiples
de 25 % tombent juste (960 pourcentages sur 1000 dérivent). Constantes lues dans le fichier :
`0.01f` en `1809c8978`, `256.0f` en `1809c1b78` ; `1809b0fbe` est `floorf`.

`EFFECT_ADJUST_UNIT_PRODUCTION` (vtable `180a8d0f0`, créée par `1808b7780`) : ajout `180880b40`,
retrait `180881800`. Chacun choisit selon la cible :

| cible | ajout | retrait | tableau |
|---|---|---|---|
| joueur (`MODIFIER_PLAYER_UNITS_ADJUST_UNIT_PRODUCTION`, Ilkum) | `180944610` | `180973d50` | `1809497a0(unités du joueur, index, p)`, tableau en `+0x200` |
| ville (`MODIFIER_PLAYER_CITIES_…`, `MODIFIER_SINGLE_CITY_…`) | `1809444e0` | `180973c20` | conversion recopiée dans la boucle, tableau en `(ville+0x10c0)+0x2c0` |

Le retrait passe `−p` à la même conversion. Le tableau de la ville est le `mod_unité[queue+0x2c0]`
que lit le prédicteur des tours ([production_turns](../production_turns/README.md)).

`EFFECT_ADJUST_CITY_PRODUCTION_UNIT` (`MODIFIER_PLAYER_CAPITAL_CITY_…`,
`MODIFIER_PLAYER_DISTRICTS_…`, vtable `180a924f0`) ajoute et retire un entier en `ville+0x440`
(`1808a1050` / `1808a16f0`) : pas de dérive, pas de détour.

## Le correctif (POL)

[proxy/src/production_modifiers.rs](../proxy/src/production_modifiers.rs). Sous POL, une
variation négative applique l'opposé exact de la conversion positive : `−trunc(|p| × 0,01f × 256)`.
Les variations positives, et les négatives qui ne dérivent pas, restent celles du jeu.

- **Joueur** : détour de `1809497a0`. Variation négative sous POL : même borne, même éditeur
  FAutoArchive (`18072a920`), écriture corrigée ; sinon l'originale.
- **Ville** : détours de `1809444e0` et `180973c20`. L'originale s'exécute sauf si l'un des montants
  de l'effet dérive et que POL est actif. Dans ce cas, le proxy refait la boucle : mêmes appels
  dans le même ordre (`18072a920` sur `ville+0x10c0`, `180044f00` pour le propriétaire), mêmes
  sauts (type négatif, index −1, premier montant au-delà du nombre de montants), conversion corrigée.
- **Effet de bord voulu** : une pénalité négative qui dérivait (−30 %) est appliquée à −76/256 au
  lieu de −77/256, si bien que son retrait la compense aussi exactement.
- **Pas de réparation des sauvegardes** : la perte déjà accumulée reste.
- **Garde-fou** : les corps entiers des trois fonctions doivent correspondre, sinon aucun détour et
  les correctifs POL de ce joueur sont coupés (`ST_POL`), pour qu'aucune machine ne calcule
  autrement que les autres.
- **Tests** : `cargo test` (conversion du jeu reproduite, symétrie de 1 à 100 000 %, boucle de ville
  sur des structures factices) ; `check_hooks.py` (sites, sauts, restauration).

Pas encore essayé en partie.
