# Unités empilées : soutien, contournement et garnison

Analyse statique (2026-11-08) de `GameCore_XP2_FinalRelease.dll` (projet Ghidra `civ6xp2`, base
`0x180000000`), et correctif POL dans le proxy : [proxy/src/empilement.rs](../proxy/src/empilement.rs).
**Essayé en partie le 2026-11-08 : fonctionne avec des unités terrestres** (essais non
exhaustifs : unités navales et avions pas encore essayés).

Deux unités de même classe peuvent se retrouver sur la même case : unité achetée dans une ville déjà
occupée, déplacement interrompu ([unites_superposees](../analyses/unites_superposees/README.md)),
téléportation… Le jeu ne bloque que la fin de tour du joueur humain. Pendant ce temps, l'unité en trop
compte dans les bonus de combat.

## En bref

- **Vérifié.** Le contournement et le soutien comptent toutes les unités des cases voisines, sans
  regarder l'empilement : deux unités empilées donnent +4 au lieu de +2.
- **Vérifié.** La défense d'une ville ne prend qu'une unité, le meilleur défenseur de la case : un
  empilement n'ajoute rien, il permet seulement de garder la meilleure des deux. Pas changé.
- **Correctif.** Une unité en excédent ne compte plus dans le soutien ni dans le contournement.
  POL, sans réglage (correction d'abus).

## Le contournement : `18036c2a0`

`LOC_COMBAT_PREVIEW_FLANKING_BONUS_DESC`. Appelé par `180369400` (force d'attaque d'une unité contre une
unité), `(this, &résultat, attaquant, case du défenseur, type de combat, infobulle ou 0)`.

```
si joueur(attaquant)+0x648 (Tradition militaire) et combat == COMBAT_MELEE (0x2ca3edd1)
   et attaquant non aérien (+0x2a0 != 1) :
  pour les 6 voisins de la case du défenseur, pour chaque couche, pour chaque entrée {i16 joueur, i16 genre, i32 id} :
    u = 1802c4360(joueur+0x6c8, id)                    // en 18036c3e1
    si u et même propriétaire que l'attaquant et u != attaquant
       et force > 0 (18015a2b0) et u non aérien et non embarquée (180066a00) : n++
bonus = GP+0x1b0 (COMBAT_FLANKING_BONUS_MODIFIER, 2) × n × (1 + attaquant+0x7ac %)
```

`18036c890` est la version religieuse (couche 3), hors sujet.

## Le soutien : `18036d1d0`

`LOC_COMBAT_PREVIEW_SUPPORT_BONUS_DESC`. Appelé par `18036ad70` (unité qui se défend contre une unité) et
`18036b800` (contre un district). Mêmes conditions (`+0x648`, `COMBAT_MELEE`). Voisins du défenseur ;
compte les unités du défenseur avec une force, autres que lui (`1802c4360` en `18036d301`). **Ni test
de domaine, ni test d'embarquement.** Bonus `GP+0x234` (`COMBAT_SUPPORT_BONUS_MODIFIER`, 2) × n ×
(1 + `+0x7b0` %).

## La garnison : `180190d90`

`LOC_COMBAT_PREVIEW_DISTRICT_GARRISONED_UNIT_BONUS`. Base `180190cc0` : meilleure force de mêlée de
l'ère moins `GP+0x1b4` (`COMBAT_GARRISON_MILITIA_MODIFIER`). Garnison : `180159250(case, -1, -1, 0,
0x10)` en `180190e3f`, **le meilleur défenseur** (comparé par `18036e340`), non aérien. S'il est plus
fort que la base, le district gagne la différence. Une seule unité compte : rien à corriger.
Le contournement et le soutien ne s'appliquent pas contre un district.

## Ce que le jeu appelle « même place »

Couches de la carte (`MapLayers`) : 0 PRIMARY (unités militaires et civiles), 1 TRADE, 2 SPY,
3 RELIGIOUS. Le compteur d'empilement `1803b0370` compare deux unités du même joueur par
`UnitManager` (`18064fff0`, vtable `180a23c60`) `+0xb8` = `18064f280(type A, type B)` : même domaine
(`DOMAIN_HOVER` en joker), pas aérien, pas marchand, même `FormationClass` (`UnitInfo+0x20`).

Classes (`1807d8a70`) : 0 `LAND_COMBAT` (`0x9ad485f8`), 1 `SUPPORT`, 2 `CIVILIAN`, 3 `NAVAL`
(`0xfeeb5b2f`), 4 `AIR`.

Retenu par l'utilisateur (2026-11-08) : même propriétaire et même classe, **seulement**
`LAND_COMBAT` et `NAVAL`. Les autres classes gardent le comportement du jeu (les unités de soutien et
civiles n'ont de toute façon pas de force ; les avions s'empilent par conception).

## Le correctif

Les deux appels `call 1802c4360` sont redirigés vers un relais `mov r8,rsi ; mov r9,r14 ;
jmp [rip+0]`. À ces deux sites, `rsi` est le porteur du bonus (l'attaquant, ou le défenseur) et
`r14` la liste d'unités de la case voisine en cours de parcours. La fonction du proxy appelle
`1802c4360`, puis rend 0 si l'unité est en excédent : le `test rax,rax ; jz` qui suit l'ignore.

Une unité `u` de classe `LAND_COMBAT` ou `NAVAL` est en excédent si :
- le porteur est sur sa case, au même propriétaire et de la même classe : il occupe la place (un
  attaquant empilé ne reçoit pas le contournement de son voisin de case) ;
- ou une unité vivante (`+0x2e8 == 0`), avec une force (`18015a2b0`), du même propriétaire et de la
  même classe la précède dans la liste de la case.

La limite est d'une unité par classe, fixe : c'est celle du jeu (`GlobalParameters+0x58c`, 1), et la
lire n'apporterait rien tant qu'aucun mod ne la change. Le nombre compté ne dépend pas de l'ordre de
la liste ; seule l'unité retenue en dépend, ce qui n'a pas d'effet sur le bonus.

- **Prévisualisation.** L'infobulle de combat vient des mêmes fonctions : elle suit sans changement.
- **Multijoueur.** Résolution du combat et IA calculées chez tous : correctif POL (`pol::actifs()`),
  `VERSION` à passer à 0.7.0 à la prochaine release (laissée à 0.6.0 pour un essai entre joueurs qui
  ont tous ce build). Sans réglage local, qui désynchroniserait.
- **Hors du correctif** (décidé le 2026-11-08) : garnison, bonus des unités levées (`1803683b0`),
  bonus par voisin (`18064db30`), soutien antiaérien (`180368ca0`) et aérien (`180368690`).

## À vérifier en jeu

Les points 1 à 3 sont vérifiés le 2026-11-08 avec des unités terrestres. Restent les points 4 et 5.


1. Le journal au chargement : `empilement : unités en excédent ignorées par le soutien et le
   contournement (appels redirigés en 0x18036c3e1 et 0x18036d301)`.
2. Avec Tradition militaire, acheter une unité de mêlée dans une ville déjà gardée, voisine d'une unité
   ennemie. Prévisualiser une attaque depuis une autre case : le contournement doit être +2 (une case),
   pas +4. Le journal donne `empilement : unité … en excédent, ignorée pour le contournement …`.
3. Même chose pour le soutien : l'ennemi attaque une unité voisine de la ville où deux unités sont
   empilées.
4. Une unité terrestre et une unité navale dans la même ville comptent toujours toutes les deux.
5. Soutien des avions : `18036d1d0` ne teste pas le domaine. Vérifier si des avions dans une ville
   voisine donnent du soutien dans le jeu d'origine (non corrigé ici).
