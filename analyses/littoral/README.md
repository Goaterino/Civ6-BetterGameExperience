# Ville côtière sans navire jusqu'au rechargement

Symptôme rapporté : une ville côtière fondée normalement ne peut ni produire ni acheter d'unité
navale (la galère comme les autres), et tout rentre dans l'ordre après avoir quitté puis rechargé la
partie. Analyse statique du 2026-11-04 sur `GameCore_XP2_FinalRelease.dll` (projet Ghidra
`~/Tools/ghidra-projects/civ6`, relu en `-noanalysis -readOnly` avec `DumpAddrs.java`,
`DisasmAt.java`, `FindOperand.java` ; décompilations dans `~/Tools/ghidra-projects/galere1` à
`galere15`). Correctif proposé (POL, pas encore dans le proxy) : voir [Correctif](#correctif).

## La porte des unités navales

- `CanTrain` (`1800f86d0`) appelle `1800ff9f0` : pour une unité `DOMAIN_SEA`, il faut
  `180131a10(ville, 0)`. La production et l'achat passent par là.
- `180131a10(ville, domaine)` (`City_Instance.cpp`) :
  1. un district achevé de la ville dont le `MilitaryDomain` est ce domaine (`18011ef20`) : vrai ;
  2. domaine terrestre : vrai ;
  3. domaine naval : plot du centre-ville (`180043d80`), bloc annexe présent (vtable `+0x40`) et
     **bit 0 de ses drapeaux** (`18005e880(vtable+0x30(plot), 1)`).

Sans port, la ville ne dépend donc que de ce bit. Les autres appelants de `180131a10` (IA
`18052d500`, placement `180120950`, personnages illustres `180644260`, `180634010`, `1804e67f0`) en
dépendent aussi.

## Le bit « accès naval »

Le plot (vtable `1809c0450`) a un bloc annexe alloué à la demande dans une réserve globale
(`DAT_180b61488`, emplacements de `0x28` octets) ; sa poignée est en `plot+0x4e`. Vtable du plot :
`+0x30` lecture (`18004c210`, emplacement 0 par défaut), `+0x38` lecture-écriture (alloue si
besoin), `+0x40` le bloc existe. Dans le bloc : `+0x0c` ville, `+0x10` district, `+0x14` drapeaux
(16 bits, posés par `18005eb30(bloc, masque, oui)`), `+0x1c` propriétaire.

Le bit 0 vaut « une unité navale peut y être » : eau, centre-ville côtier, canal achevé (et les
aménagements dont `+0x48 & 4`). **Il n'est pas sauvegardé** (`180060950`, lecture d'un plot, ne le
lit pas). Au chargement (`180751b60`), `18004c4e0` le recalcule pour chaque plot qui a un bloc :

```
drapeaux = 0
si eau (180060820)
   ou (district du plot = DISTRICT_CITY_CENTER (18005e770)
       et une eau voisine, hors lac (plot+0x3a & 8), dont l'étendue a au moins 1 plot (18005fe00(plot, 1))) :
    bit 0
(bits 4, 0x10, 2 ensuite)
```

puis ajoute le bit aux canaux achevés (`Districts_XP2.Canal`, drapeau `+0x28 & 1`) et aux
aménagements concernés. `18004c850` fait le même calcul pour un seul plot.

En cours de partie, le bit d'un centre-ville n'est recalculé qu'à des moments précis :
- fondation (`180144c50`, `City_Placement.cpp`) : après la prise de possession du plot (`18004c5f0`,
  sans district, donc sans bit), puis après la création du district du centre-ville ;
- prise de possession d'un plot (`18004c5f0`), annexion (`18011b100`), prise de ville
  (`18004c6f0`, depuis `180142db0`), destruction de la ville (`180144a20`) ;
- inondation ou submersion d'une côte basse (`180739890`, `180739c20`), pour ce plot seul ;
- eau apparue à côté d'un centre-ville (`1803641b0`) : bit ajouté aux centres-villes voisins.

Il est effacé par `18004c850` (remise à zéro avant calcul), à la suppression d'un canal (`18011e220`)
et au pillage d'un canal (`18011f610`).

## Cause

Le rechargement corrige le bug parce qu'il recalcule ce bit, qui n'est qu'un cache. En cours de
partie, il est donc resté faux après un recalcul fait au mauvais moment, ou n'a pas été recalculé
après un changement. Le chemin exact n'est pas identifié : pour une ville fondée normalement, la
lecture statique ne trouve aucun appel qui l'efface à tort. Le journal du correctif donne le plot
et le joueur concernés, ce qui permettra de rejouer la partie avec une trace (`18004c850`,
`18005eb30`) si le cas revient.

## Correctif

`call 18005e880` en `180131a73` est redirigé vers un relais (`mov r8,rbx ; jmp [rip+0]`, `rbx` = plot).
Si le bit est absent, le relais refait le test du chargement ci-dessus. Il **n'écrit rien** :
l'interface appelle aussi `CanTrain`, et une écriture faite depuis elle n'aurait pas lieu au même
moment sur chaque machine. Le résultat est celui qu'aurait donné un rechargement, pour la production
et l'achat comme pour l'IA et le placement des unités. Le déplacement des navires dans la ville
(autre lecteur du bit) n'est pas touché : il reste celui du jeu jusqu'au rechargement.

Correctif POL : il change la simulation, il n'est appliqué que si tous les joueurs ont le proxy
(`BgdPolActifs`).

## À vérifier

- En partie : le journal du proxy (`littoral : bit naval absent sur le centre-ville (x, y) du
  joueur …`) sur une partie où le bug apparaît, et la production d'une galère dans cette ville.
- Le chemin qui laisse le bit faux, avec une trace Frida sur `18004c850` et `18005eb30` pour le plot
  journalisé.
- Si les navires peuvent entrer dans la ville pendant le bug (le bit sert aussi aux déplacements).
