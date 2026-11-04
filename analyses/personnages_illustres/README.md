# Ville d'arrivée d'un personnage illustre

Question : quand un joueur obtient un personnage illustre (points accumulés, achat en or ou en foi),
dans quelle ville l'unité apparaît-elle ? Analyse statique du 2026-11-04, sur
`GameCore_XP2_FinalRelease.dll` (projet Ghidra `~/Tools/ghidra-projects/civ6`, base 0x180000000),
relu en `-noanalysis -readOnly` avec `DumpAddrs.java`. Les décompilations sont dans
`~/Tools/ghidra-projects/gp_spawn` à `gp_spawn5`. Rien n'a encore été vérifié en partie : voir
[À vérifier](#à-vérifier).

## Conclusion

Le choix dépend du **domaine de l'unité** du personnage (`Units.Domain`), lu dans
`GreatPersonClasses.UnitType` :

- **Unité terrestre** (tous sauf l'amiral) : **la ville la plus peuplée qui a un district achevé du
  type de la classe** (`GreatPersonClasses.DistrictType` : campus pour le scientifique, théâtre pour
  l'écrivain, l'artiste et le musicien, lieu saint pour le prophète, campement pour le général, etc.).
  - Égalité de population : la première ville de la liste des villes du joueur l'emporte
    (comparaison stricte).
  - Aucune ville avec ce district achevé : **la capitale**. Sans capitale, un plot stocké dans le
    joueur (`Player+0x778`).
- **Unité navale** (l'amiral, district `DISTRICT_HARBOR`) : le district `MilitaryDomain="DOMAIN_SEA"`
  (port, cothon, chantier naval royal) **de la ville la plus peuplée**, sur un plot qui passe un test
  de littoral. À défaut, **le dernier autre district littoral** parcouru (centre-ville côtier en
  pratique). Sans aucun des deux, le personnage ne peut pas être obtenu :
  `LOC_GREAT_PEOPLE_CANNOT_EARN_PERSON_NO_VALID_SPAWN_SEA`, « You must have a coastal city center or a
  Harbor district in order to spawn a Great Admiral. »

Ce n'est donc ni la ville qui a produit le plus de points (les points sont au niveau du joueur), ni la
capitale par défaut. L'unité est ensuite posée sur le plot du **district** (et non du centre-ville)
pour la branche navale. Pour la branche terrestre, c'est le plot de la ville (`City+0xc0`).

## Octroi : `FUN_18020cbe0`

`Game_GreatPeople` : la fonction qui accorde un personnage à un joueur (assert Release
`!"kSpawnLocation.IsValid()"`, trace `"Granted to Player"`). Ses appelants sont le recrutement
(`18020d6a0`, assert `CanRecruitPerson`), le traitement du tour (`180209c80`), `18020d950`,
`180208fd0` et `1806e0170`.

1. Elle calcule l'emplacement par `FUN_18020bfa0(this, &loc, classe, joueur)`.
2. Si `loc` est invalide (-9999), elle renvoie `false` **sans rien accorder**.
3. Sinon, elle crée l'unité (`UnitManager` vtable `+0x50`), lui attache le personnage (`180399650`),
   la place sur `loc` (`FUN_1803b11b0(unit, &loc, 1)`), puis retire le personnage du marché et envoie
   les événements.

`FUN_18020a270` (appelée par `1806dfc40`, assert « Great Person unit was not created properly ») crée
aussi l'unité d'un personnage, mais sur un plot fourni par l'appelant : pas de choix de ville.

## Choix de l'emplacement : `FUN_18020bfa0`

Entrées : l'info de classe (`GameInfo+0x13b0`, champ `+0x70` = district, `+0x98` = unité).
`FUN_1807d8a20(unit+0x1c)` convertit le hash du domaine : `DOMAIN_LAND` = 2, `DOMAIN_SEA` = 0,
`DOMAIN_AIR` = 1 (même numérotation que `FUN_180191a50`, qui compare les chaînes).

### Domaine terrestre (2)

```
meilleur = INT_MIN
pour chaque ville de Player+0x6d0 (liste +0xd0) :
    si FUN_18011ee20(ville+0xb58, district_classe, bComplet=1, bRemplacements=0)
       et ville+0x268 > meilleur :
        loc = ville+0xc0 ; meilleur = ville+0x268
si loc invalide :
    capitale = FUN_180265450(villes)      // première ville avec +0x280 (capitale)
    loc = capitale ? capitale+0xc0 : Player+0x778
```

- `City+0x268` est la **population** : c'est le champ que lit le `City:GetPopulation` Lua
  (`1806c91e0`, `Lua_ICity.cpp`).
- `FUN_18011ee20` (`City_Districts.cpp`) parcourt les identifiants de districts de la ville et cherche
  `district+0xf0 == type`. Avec `bComplet`, il exige `district+0x3f8`, le champ de `District:IsComplete`
  (`1806d2760`). Ni le pillage ni la présence de bâtiments ne comptent.
- Le 4ᵉ argument à 0 désactive le test `FUN_18064e010`, qui accepte aussi un district qui en remplace
  un autre (liste `DistrictInfo+0x1e8`). Voir [À vérifier](#à-vérifier) pour les districts uniques.

### Domaine naval (0)

```
meilleur = INT_MIN ; repli = invalide
pour chaque district de Player+0x6e8 (liste +0xd0) :
    plot = plot du district
    si DistrictInfo+0xe1 & 4 == 0
       et bit 0 des drapeaux du plot (données plot +0x14)
       et le district a une ville (FUN_180191580) :
        si MilitaryDomain du district == DOMAIN_SEA :
            si ville.population > meilleur : loc = plot ; meilleur = population
        sinon :
            repli = plot                  // écrasé à chaque district, pas de tri
loc = loc valide ? loc : repli            // peut rester invalide
```

- Le district naval n'a **pas** besoin d'être achevé, à la différence de la branche terrestre.
- Le domaine du district est `Districts.MilitaryDomain` (`DistrictInfo+0x78`, comparé à
  `"DOMAIN_SEA"`) : `DISTRICT_HARBOR`, `DISTRICT_COTHON`, `DISTRICT_ROYAL_NAVY_DOCKYARD`. Le
  `DistrictType` de la classe n'est pas utilisé ici.
- Le repli n'est pas trié par population : c'est le dernier district littoral de la liste.
- `CanRecruitPerson`/`CanEarn` (`FUN_180209280`) appelle aussi `FUN_18020bfa0` et refuse le personnage
  quand la position est invalide pour une unité navale, avec le texte `NO_VALID_SPAWN_SEA`.

## Adresses

| adresse | rôle |
|---|---|
| `18020cbe0` | octroi d'un personnage (création et placement de l'unité) |
| `18020bfa0` | choix de l'emplacement |
| `180209280` | possibilité d'obtenir un personnage (texte `NO_VALID_SPAWN_SEA`) |
| `18020a270` | création de l'unité sur un plot imposé |
| `18011ee20` | la ville a-t-elle un district de ce type (achevé) |
| `18064e010` | un district en remplace-t-il un autre |
| `180265450` | capitale du joueur |
| `180191a50` | `MilitaryDomain` d'un district |
| `1807d8a20` | hash de domaine → `DOMAIN_*` |
| `1803b11b0` | placement de l'unité |

## À vérifier

- **Districts uniques en branche terrestre.** L'appel passe `bRemplacements = 0`. Si un district
  unique est stocké sous son propre type (lavra, acropole, hansa, etc.), une ville qui n'a que lui ne
  compterait pas, et le personnage irait dans une autre ville, ou dans la capitale. À tester avec la
  Russie (prophète, lavra hors capitale) ou l'Allemagne (ingénieur, hansa).
- Le sens exact du bit 0 des drapeaux de plot (`+0x14`). Le même test sert à `FUN_180131a10` pour
  autoriser la production navale d'une ville : c'est probablement « littoral / accès à la mer ».
- Le drapeau `DistrictInfo+0xe1 & 4`, qui exclut des districts du repli naval. C'est probablement
  `InternalOnly` (`DISTRICT_WONDER`), non vérifié.
- L'ordre de la liste des villes (`PlayerCities+0xd0`), qui départage les égalités de population.
- Le comportement de `FUN_1803b11b0` quand le plot est occupé par un autre civil.
- En partie : deux villes avec campus à population égale, puis la plus peuplée sans campus, et un
  amiral sans port mais avec un centre-ville côtier.
